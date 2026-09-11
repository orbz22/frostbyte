use crate::types::ThermalSnapshot;
use std::process::Command;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

pub struct ThermalProvider {
    nvidia_smi_available: bool,
}

impl ThermalProvider {
    pub fn new() -> Self {
        // Quick probe for nvidia-smi
        let nvidia_smi_available = Command::new("nvidia-smi")
            .arg("--help")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        Self {
            nvidia_smi_available,
        }
    }

    /// Captures a combined snapshot of system thermals and power state.
    pub fn sample(&self) -> ThermalSnapshot {
        let (is_ac_online, battery_percent) = self.get_power_status();
        let (gpu_temp, gpu_power_w) = self.get_gpu_metrics();
        let cpu_package_temp = self.get_cpu_temperature();

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

    /// Query NVIDIA GPU metrics via nvidia-smi if available.
    fn get_gpu_metrics(&self) -> (Option<f32>, Option<f32>) {
        if !self.nvidia_smi_available {
            return (None, None);
        }

        let output = Command::new("nvidia-smi")
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

    /// Attempt to query CPU temperature via WMI MSAcpi_ThermalZoneTemperature.
    /// Returns None gracefully if not elevated or unsupported by BIOS.
    fn get_cpu_temperature(&self) -> Option<f32> {
        // Run quick PowerShell WMI query with low timeout
        let script = "try { (Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature -ErrorAction Stop | Select-Object -First 1).CurrentTemperature } catch { '' }";
        let output = Command::new("powershell")
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
