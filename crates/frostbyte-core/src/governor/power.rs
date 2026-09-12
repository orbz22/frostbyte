use anyhow::{Context, Result};
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const SUB_PROCESSOR_GUID: &str = "54533251-82be-4824-96c1-47b60b740d00";
const PROCTHROTTLEMAX_GUID: &str = "bc5038f7-23e0-4960-96da-33abaf5935ec";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernorState {
    Normal,
    CoolingClamped,
}

pub struct PowerGovernor {
    state: GovernorState,
    high_temp_threshold: f32,
    cool_temp_threshold: f32,
    consecutive_cool_ticks: u32,
    required_cool_ticks: u32,
    auto_cool_enabled: bool,
}

impl PowerGovernor {
    pub fn new(
        high_temp_threshold: f32,
        cool_temp_threshold: f32,
        required_cool_ticks: u32,
        auto_cool_enabled: bool,
    ) -> Self {
        Self {
            state: GovernorState::Normal,
            high_temp_threshold,
            cool_temp_threshold,
            consecutive_cool_ticks: 0,
            required_cool_ticks,
            auto_cool_enabled,
        }
    }

    pub fn current_state(&self) -> GovernorState {
        self.state
    }

    pub fn set_auto_cool(&mut self, enabled: bool) {
        self.auto_cool_enabled = enabled;
    }

    pub fn is_auto_cool_enabled(&self) -> bool {
        self.auto_cool_enabled
    }

    pub fn set_high_temp_threshold(&mut self, threshold: f32) {
        self.high_temp_threshold = threshold;
    }

    pub fn high_temp_threshold(&self) -> f32 {
        self.high_temp_threshold
    }

    /// Toggles Intel Turbo Boost / AMD Precision Boost by adjusting
    /// Maximum Processor State (100% = Boost Enabled, 99% = Boost Disabled).
    pub fn set_turbo_boost(&mut self, enabled: bool) -> Result<()> {
        let value = if enabled { 100 } else { 99 };

        // 1. Write value for AC power
        let write_output = Command::new("powercfg")
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "/setacvalueindex",
                "SCHEME_CURRENT",
                SUB_PROCESSOR_GUID,
                PROCTHROTTLEMAX_GUID,
                &value.to_string(),
            ])
            .output()
            .context("Failed to execute powercfg /setacvalueindex")?;

        if !write_output.status.success() {
            let err = String::from_utf8_lossy(&write_output.stderr);
            return Err(anyhow::anyhow!("powercfg /setacvalueindex failed: {}", err));
        }

        // 2. Activate scheme change
        let activate_output = Command::new("powercfg")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["/setactive", "SCHEME_CURRENT"])
            .output()
            .context("Failed to execute powercfg /setactive")?;

        if !activate_output.status.success() {
            let err = String::from_utf8_lossy(&activate_output.stderr);
            return Err(anyhow::anyhow!("powercfg /setactive failed: {}", err));
        }

        self.state = if enabled {
            GovernorState::Normal
        } else {
            GovernorState::CoolingClamped
        };

        Ok(())
    }

    /// Evaluates current temperature and automatically intervenes if necessary.
    /// Returns Some(true) if boost was clamped (cooled), Some(false) if boost was restored, None if unchanged.
    pub fn evaluate_thermals(
        &mut self,
        current_temp: Option<f32>,
        is_ac_online: bool,
    ) -> Option<bool> {
        // If user set Instant Cool to manual mode, do not auto-intervene
        if !self.auto_cool_enabled {
            return None;
        }

        let temp = current_temp?;

        // Only govern when plugged in (battery usually has boost restricted by default)
        if !is_ac_online {
            return None;
        }

        match self.state {
            GovernorState::Normal => {
                if temp >= self.high_temp_threshold {
                    // Emergency: temperature is dangerously high! Clamp boost to 99%
                    if self.set_turbo_boost(false).is_ok() {
                        self.consecutive_cool_ticks = 0;
                        return Some(true); // Clamped
                    }
                }
            }
            GovernorState::CoolingClamped => {
                if temp <= self.cool_temp_threshold {
                    self.consecutive_cool_ticks += 1;
                    if self.consecutive_cool_ticks >= self.required_cool_ticks {
                        // Temperature has been safe and stable, restore normal boost
                        if self.set_turbo_boost(true).is_ok() {
                            self.consecutive_cool_ticks = 0;
                            return Some(false); // Restored
                        }
                    }
                } else {
                    self.consecutive_cool_ticks = 0;
                }
            }
        }

        None
    }
}

impl Default for PowerGovernor {
    fn default() -> Self {
        // High trigger: 88.0°C, Safe cool: 65.0°C, Hysteresis ticks: 5 (10 seconds at 2s/tick), Auto-cool enabled: true
        Self::new(88.0, 65.0, 5, true)
    }
}
