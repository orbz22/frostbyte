use crate::types::ThermalSnapshot;
use std::os::windows::process::CommandExt;
use std::process::Command;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::System::Performance::*;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Native Win32 Performance Data Helper (PDH) query for ACPI Thermal Zones.
/// Operates entirely in-process without spawning console/powershell sub-processes,
/// and works under standard (non-elevated) user permissions.
struct PdhThermalZone {
    query: isize,
    high_precision_counter: Option<isize>,
    standard_counter: Option<isize>,
}

impl PdhThermalZone {
    pub fn new() -> Option<Self> {
        unsafe {
            let mut query: isize = 0;
            if PdhOpenQueryW(None, 0, &mut query) != 0 {
                return None;
            }

            // High Precision Temperature counter (tenths of a Kelvin, e.g. 3682 = 95.0°C)
            let hp_path = HSTRING::from(r"\Thermal Zone Information(*)\High Precision Temperature");
            let mut hp_counter: isize = 0;
            let hp_ok =
                PdhAddEnglishCounterW(query, PCWSTR(hp_path.as_ptr()), 0, &mut hp_counter) == 0;

            // Standard Temperature counter (Kelvin, e.g. 368 = 95°C)
            let std_path = HSTRING::from(r"\Thermal Zone Information(*)\Temperature");
            let mut std_counter: isize = 0;
            let std_ok =
                PdhAddEnglishCounterW(query, PCWSTR(std_path.as_ptr()), 0, &mut std_counter) == 0;

            if !hp_ok && !std_ok {
                let _ = PdhCloseQuery(query);
                return None;
            }

            // Prime the query once
            let _ = PdhCollectQueryData(query);

            Some(Self {
                query,
                high_precision_counter: if hp_ok { Some(hp_counter) } else { None },
                standard_counter: if std_ok { Some(std_counter) } else { None },
            })
        }
    }

    pub fn sample(&mut self) -> Option<f32> {
        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return None;
            }

            // Try High Precision Counter first
            if let Some(counter) = self.high_precision_counter {
                if let Some(temp) = Self::read_counter_array(counter, true) {
                    return Some(temp);
                }
            }

            // Fallback to Standard Counter
            if let Some(counter) = self.standard_counter {
                if let Some(temp) = Self::read_counter_array(counter, false) {
                    return Some(temp);
                }
            }

            None
        }
    }

    unsafe fn read_counter_array(counter: isize, is_high_precision: bool) -> Option<f32> {
        let mut buffer_size: u32 = 0;
        let mut item_count: u32 = 0;
        let _ = PdhGetFormattedCounterArrayW(
            counter,
            PDH_FMT_DOUBLE,
            &mut buffer_size,
            &mut item_count,
            None,
        );

        if buffer_size == 0 || item_count == 0 {
            return None;
        }

        let mut buffer = vec![0u8; buffer_size as usize];
        let status = PdhGetFormattedCounterArrayW(
            counter,
            PDH_FMT_DOUBLE,
            &mut buffer_size,
            &mut item_count,
            Some(buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
        );

        if status != 0 {
            return None;
        }

        let items = std::slice::from_raw_parts(
            buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
            item_count as usize,
        );

        let mut max_celsius: Option<f32> = None;

        for item in items {
            let val = item.FmtValue.Anonymous.doubleValue;
            let celsius = if is_high_precision {
                if val > 2732.0 {
                    Some(((val - 2732.0) / 10.0) as f32)
                } else {
                    None
                }
            } else {
                if val > 273.0 {
                    Some((val - 273.15) as f32)
                } else {
                    None
                }
            };

            if let Some(c) = celsius {
                // Plausible CPU/motherboard thermal range (20°C - 125°C)
                if (20.0..=125.0).contains(&c) {
                    max_celsius = Some(max_celsius.map_or(c, |m| m.max(c)));
                }
            }
        }

        max_celsius
    }
}

impl Drop for PdhThermalZone {
    fn drop(&mut self) {
        unsafe {
            if self.query != 0 {
                let _ = PdhCloseQuery(self.query);
            }
        }
    }
}

pub struct ThermalProvider {
    /// Lazily resolved: `None` until the first GPU query actually needs it.
    /// Probing eagerly would spawn `nvidia-smi` at startup, which on Optimus /
    /// switchable-graphics laptops wakes the discrete GPU before the user's
    /// "GPU Monitoring off" preference has even been applied.
    nvidia_smi_available: Option<bool>,
    pdh_thermal: Option<PdhThermalZone>,
    cpu_temp_monitoring: bool,
    gpu_temp_monitoring: bool,
    gpu_monitoring: bool,
}

impl ThermalProvider {
    pub fn new() -> Self {
        // Initialize native Win32 PDH thermal zone counter (in-process, 0 latency, standard user friendly)
        // No GPU probing happens here on purpose -- see `nvidia_smi_available`.
        let pdh_thermal = PdhThermalZone::new();

        Self {
            nvidia_smi_available: None,
            pdh_thermal,
            cpu_temp_monitoring: true,
            gpu_temp_monitoring: true,
            gpu_monitoring: true,
        }
    }

    /// Enables or disables CPU package temperature sampling.
    /// When disabled, the ACPI thermal zone query is skipped entirely and
    /// `cpu_package_temp` is reported as `None`.
    pub fn set_cpu_temp_monitoring(&mut self, enabled: bool) {
        self.cpu_temp_monitoring = enabled;
    }

    pub fn is_cpu_temp_monitoring(&self) -> bool {
        self.cpu_temp_monitoring
    }

    /// Enables or disables discrete GPU temperature reporting.
    /// Other GPU metrics (power, utilization, clock) keep reporting; use
    /// [`set_gpu_monitoring`](Self::set_gpu_monitoring) to stop querying the GPU entirely.
    pub fn set_gpu_temp_monitoring(&mut self, enabled: bool) {
        self.gpu_temp_monitoring = enabled;
    }

    pub fn is_gpu_temp_monitoring(&self) -> bool {
        self.gpu_temp_monitoring
    }

    /// Master switch for all discrete GPU telemetry. When disabled, `nvidia-smi`
    /// is never spawned, so every GPU metric (temperature, power, utilization,
    /// clock) reports `None` and the per-tick subprocess cost disappears.
    pub fn set_gpu_monitoring(&mut self, enabled: bool) {
        self.gpu_monitoring = enabled;
    }

    pub fn is_gpu_monitoring(&self) -> bool {
        self.gpu_monitoring
    }

    /// Captures a combined snapshot of system thermals and power state.
    pub fn sample(&mut self) -> ThermalSnapshot {
        let (is_ac_online, battery_percent) = self.get_power_status();
        let (raw_gpu_temp, gpu_power_w, gpu_utilization_pct, gpu_clock_mhz) =
            self.get_gpu_metrics();
        let gpu_temp = if self.gpu_temp_monitoring {
            raw_gpu_temp
        } else {
            None
        };

        // 1. Direct hardware temperature via native Win32 PDH (Thermal Zone / ACPI)
        let mut cpu_package_temp = if self.cpu_temp_monitoring {
            self.pdh_thermal.as_mut().and_then(|pdh| pdh.sample())
        } else {
            None
        };

        // 2. Fallback: If hardware thermal zones are not supported by the motherboard DSDT,
        // use discrete GPU temperature as an indicator of shared heatsink load.
        // The raw GPU reading is used so the fallback still works when only the
        // GPU temperature *reporting* has been turned off. Turning off GPU
        // monitoring entirely removes this fallback along with the query.
        if self.cpu_temp_monitoring && cpu_package_temp.is_none() && raw_gpu_temp.is_some() {
            cpu_package_temp = raw_gpu_temp.map(|t| (t + 4.0).min(99.0));
        }

        ThermalSnapshot {
            cpu_package_temp,
            max_core_temp: cpu_package_temp,
            gpu_temp,
            gpu_power_w,
            gpu_utilization_pct,
            gpu_clock_mhz,
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
    fn get_gpu_metrics(&mut self) -> (Option<f32>, Option<f32>, Option<f32>, Option<u32>) {
        // Checked before the availability probe: with GPU monitoring off, `nvidia-smi`
        // must never be executed, not even once to test for its presence.
        if !self.gpu_monitoring {
            return (None, None, None, None);
        }

        let available = match self.nvidia_smi_available {
            Some(known) => known,
            None => {
                let probed = Command::new("nvidia-smi")
                    .creation_flags(CREATE_NO_WINDOW)
                    .arg("--help")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);
                self.nvidia_smi_available = Some(probed);
                probed
            }
        };

        if !available {
            return (None, None, None, None);
        }

        let output = Command::new("nvidia-smi")
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "--query-gpu=temperature.gpu,power.draw,utilization.gpu,clocks.current.graphics",
                "--format=csv,noheader,nounits",
            ])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = text.trim().split(',').map(|s| s.trim()).collect();
                let temp = parts.first().and_then(|s| s.parse::<f32>().ok());
                let power = parts.get(1).and_then(|s| s.parse::<f32>().ok());
                let util = parts.get(2).and_then(|s| s.parse::<f32>().ok());
                let clock = parts.get(3).and_then(|s| s.parse::<u32>().ok());
                return (temp, power, util, clock);
            }
        }

        (None, None, None, None)
    }
}

impl Default for ThermalProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_monitoring_toggles_suppress_readings() {
        let mut provider = ThermalProvider::new();
        assert!(provider.is_cpu_temp_monitoring());
        assert!(provider.is_gpu_temp_monitoring());

        provider.set_cpu_temp_monitoring(false);
        provider.set_gpu_temp_monitoring(false);

        let snapshot = provider.sample();
        assert!(
            snapshot.cpu_package_temp.is_none(),
            "CPU temp must be suppressed while monitoring is off"
        );
        assert!(
            snapshot.max_core_temp.is_none(),
            "Max core temp must be suppressed while monitoring is off"
        );
        assert!(
            snapshot.gpu_temp.is_none(),
            "GPU temp must be suppressed while monitoring is off"
        );

        provider.set_cpu_temp_monitoring(true);
        provider.set_gpu_temp_monitoring(true);
        assert!(provider.is_cpu_temp_monitoring());
        assert!(provider.is_gpu_temp_monitoring());
    }

    /// Guards the "keep the discrete GPU asleep" contract: with GPU monitoring off,
    /// `nvidia-smi` must never be executed -- not even the availability probe.
    #[test]
    fn test_gpu_off_never_probes_nvidia_smi() {
        let mut provider = ThermalProvider::new();
        assert!(
            provider.nvidia_smi_available.is_none(),
            "constructing a provider must not probe the GPU"
        );

        provider.set_gpu_monitoring(false);
        let snapshot = provider.sample();

        assert!(
            provider.nvidia_smi_available.is_none(),
            "sampling with GPU monitoring off must not probe nvidia-smi"
        );
        assert!(snapshot.gpu_temp.is_none());

        println!(
            "CPU temp with the GPU untouched: {:?}",
            snapshot.cpu_package_temp
        );
    }

    #[test]
    fn test_gpu_monitoring_master_switch_silences_all_gpu_metrics() {
        let mut provider = ThermalProvider::new();
        assert!(provider.is_gpu_monitoring());

        provider.set_gpu_monitoring(false);
        // The temperature toggle stays on: the master switch alone must silence everything.
        assert!(provider.is_gpu_temp_monitoring());

        let snapshot = provider.sample();
        assert!(snapshot.gpu_temp.is_none(), "GPU temp must be silent");
        assert!(snapshot.gpu_power_w.is_none(), "GPU power must be silent");
        assert!(
            snapshot.gpu_utilization_pct.is_none(),
            "GPU utilization must be silent"
        );
        assert!(snapshot.gpu_clock_mhz.is_none(), "GPU clock must be silent");

        // Power status is unrelated to the GPU query and must keep working.
        let _ = snapshot.is_ac_online;
    }

    #[test]
    fn test_thermal_provider_real_temperature() {
        let mut provider = ThermalProvider::new();
        let snapshot = provider.sample();
        println!("Sampled thermals: {:?}", snapshot);

        // On physical machines with ACPI/NVML sensors, verify the reading is in plausible range.
        // In cloud CI/VM environments (e.g. GitHub Actions runner), physical sensors are absent (None).
        if let Some(temp) = snapshot.cpu_package_temp {
            println!("Detected CPU Package Temp: {:.1}°C", temp);
            assert!(
                (20.0..=115.0).contains(&temp),
                "CPU temperature should be in plausible range"
            );
        }
    }
}
