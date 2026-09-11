use crate::types::ThermalSnapshot;
use std::os::windows::process::CommandExt;
use std::process::Command;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct ThermalProvider {
    nvidia_smi_available: bool,
    cpu_wmi_available: bool,
}

impl ThermalProvider {
    pub fn new() -> Self {
        // Quick probe for nvidia-smi with CREATE_NO_WINDOW
        let nvidia_smi_available = Command::new("nvidia-smi")
            .creation_flags(CREATE_NO_WINDOW)
            .arg("--help")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // Test CPU WMI temperature once at startup with CREATE_NO_WINDOW.
        // If it fails or returns Access Denied, permanently disable to avoid running PowerShell in a loop.
        let mut provider = Self {
            nvidia_smi_available,
            cpu_wmi_available: false,
        };

        if let Some(_) = provider.probe_cpu_temperature() {
            provider.cpu_wmi_available = true;
        }

        provider
    }

    /// Captures a combined snapshot of system thermals and power state.
    pub fn sample(&self) -> ThermalSnapshot {
        let (is_ac_online, battery_percent) = self.get_power_status();
        let (gpu_temp, gpu_power_w) = self.get_gpu_metrics();

        let mut cpu_package_temp = if self.cpu_wmi_available {
            self.probe_cpu_temperature()
        } else {
            None
        };

        // If CPU direct sensor is unavailable (due to non-elevated WMI restrictions),
        // use discrete GPU temperature as an indicator of shared heatsink / package thermal load.
        if cpu_package_temp.is_none() && gpu_temp.is_some() {
            // On shared-heatpipe laptops, CPU runs approx 3-5°C above idle dGPU
            cpu_package_temp = gpu_temp.map(|t| (t + 4.0).min(99.0));
        }

        ThermalSnapshot {
            cpu_package_temp,
            max_core_temp: cpu_package_temp,
            gpu_temp,
            gpu_power_w,
            is_ac_online,
            battery_percent,
        }
    }

    /// Read AC line status and battery percentage via Win32 GetSystemPowerStatus.
    fn get_power_status(&self) -> (bool, Option<u8>) {
        unsafe {
            let mut status = SYSTEM_POWER_STATUS::default();
            if GetSystemPowerStatus(&mut status).is_ok() {
                let is_ac = status.ACLineStatus == 1;
                let battery = if status.BatteryLifePercent <= 100 {
                    Some(status.BatteryLifePercent)
                } else {
                    None
                };
                (is_ac, battery)
            } else {
                (true, None)
            }
        }
    }

    /// Query NVIDIA GPU metrics via nvidia-smi with CREATE_NO_WINDOW.
    fn get_gpu_metrics(&self) -> (Option<f32>, Option<f32>) {
        if !self.nvidia_smi_available {
            return (None, None);
        }

        let output = Command::new("nvidia-smi")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["--query-gpu=temperature.gpu,power.draw", "--format=csv,noheader,nounits"])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = text.trim().split(',').map(|s| s.trim()).collect();
                if parts.len() >= 2 {
                    let temp = parts[0].parse::<f32>().ok();
                    let power = parts[1].parse::<f32>().ok();
                    return (temp, power);
                } else if parts.len() == 1 {
                    let temp = parts[0].parse::<f32>().ok();
                    return (temp, None);
                }
            }
        }

        (None, None)
    }

    /// Query CPU temperature via WMI MSAcpi_ThermalZoneTemperature with CREATE_NO_WINDOW.
    /// Used only if probe succeeded.
    fn probe_cpu_temperature(&self) -> Option<f32> {
        let script = "try { (Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature -ErrorAction Stop | Select-Object -First 1).CurrentTemperature } catch { '' }";
        let output = Command::new("powershell")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if let Ok(raw_temp) = text.parse::<f32>() {
                    if raw_temp > 2732.0 {
                        // Kelvin in tenths: (T - 2732) / 10.0 = Celsius
                        let celsius = (raw_temp - 2732.0) / 10.0;
                        return Some(celsius);
                    }
                }
            }
        }

        None
    }
}

impl Default for ThermalProvider {
    fn default() -> Self {
        Self::new()
    }
}
