pub mod governor;
pub mod heuristics;
pub mod mitigation;
pub mod process;
pub mod safety;
pub mod thermal;
pub mod types;

pub use governor::{GovernorState, PowerGovernor};
pub use heuristics::HeuristicEngine;
pub use mitigation::{JobRateLimiter, ProcessActionController};
pub use process::ProcessWatcher;
pub use safety::SafetyEngine;
pub use thermal::ThermalProvider;
pub use types::*;

use anyhow::Result;
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

fn detect_os_version() -> String {
    let output = Command::new("reg")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["query", r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion", "/v", "CurrentBuild"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            if let Some(build_str) = text.split_whitespace().last() {
                if let Ok(build_num) = build_str.parse::<u32>() {
                    let win_name = if build_num >= 22000 { "Windows 11" } else { "Windows 10" };
                    return format!("{} (Build {}) x64", win_name, build_num);
                }
            }
        }
    }

    "Windows 11 x64".to_string()
}

pub struct Watchdog {
    thermal: ThermalProvider,
    process: ProcessWatcher,
    heuristics: HeuristicEngine,
    safety: SafetyEngine,
    mitigation: JobRateLimiter,
    governor: PowerGovernor,
    auto_tame_enabled: bool,
    os_version: String,
}

impl Watchdog {
    pub fn new() -> Self {
        Self {
            thermal: ThermalProvider::new(),
            process: ProcessWatcher::new(),
            heuristics: HeuristicEngine::default(),
            safety: SafetyEngine::new(),
            mitigation: JobRateLimiter::new(),
            governor: PowerGovernor::default(),
            auto_tame_enabled: false,
            os_version: detect_os_version(),
        }
    }

    pub fn with_settings(
        saturation_threshold: f32,
        min_consecutive_ticks: u32,
        tick_interval_secs: u32,
        auto_tame: bool,
    ) -> Self {
        Self::with_full_settings(
            saturation_threshold,
            min_consecutive_ticks,
            tick_interval_secs,
            auto_tame,
            true,
            88.0,
        )
    }

    pub fn with_full_settings(
        saturation_threshold: f32,
        min_consecutive_ticks: u32,
        tick_interval_secs: u32,
        auto_tame: bool,
        auto_cool: bool,
        auto_cool_temp: f32,
    ) -> Self {
        let safety = SafetyEngine::new();
        let heuristics = HeuristicEngine::new(
            SafetyEngine::new(),
            saturation_threshold,
            min_consecutive_ticks,
            tick_interval_secs,
        );

        let mut governor = PowerGovernor::default();
        governor.set_auto_cool(auto_cool);
        governor.set_high_temp_threshold(auto_cool_temp);

        Self {
            thermal: ThermalProvider::new(),
            process: ProcessWatcher::new(),
            heuristics,
            safety,
            mitigation: JobRateLimiter::new(),
            governor,
            auto_tame_enabled: auto_tame,
            os_version: detect_os_version(),
        }
    }

    /// Executes one evaluation tick: samples thermals, processes, runs heuristics,
    /// evaluates thermal governor, and executes auto-tame if enabled.
    pub fn tick(&mut self) -> SystemSnapshot {
        let thermals = self.thermal.sample();
        let (total_cpu_pct, processes) = self.process.sample();
        let rogue_alerts = self.heuristics.evaluate(&processes);

        // Evaluate smart thermal power governor
        let _ = self.governor.evaluate_thermals(thermals.cpu_package_temp, thermals.is_ac_online);

        // If auto-tame is active, apply Job Object rate limiting to newly flagged rogue processes
        if self.auto_tame_enabled {
            for alert in &rogue_alerts {
                if !self.mitigation.is_tamed(alert.pid) && !self.safety.is_immune(&alert.process_name) {
                    let _ = self.mitigation.soft_tame(alert.pid, 10);
                }
            }
        }

        let is_turbo_boost_clamped = self.governor.current_state() == GovernorState::CoolingClamped;
        let tamed_pids = self.mitigation.active_tamed_pids();

        // Keep top 10 processes by CPU
        let top_processes = processes.into_iter().take(10).collect();

        SystemSnapshot {
            timestamp: chrono::Local::now(),
            thermals,
            total_cpu_pct,
            logical_cores: self.process.logical_cores(),
            is_turbo_boost_clamped,
            auto_tame_enabled: self.auto_tame_enabled,
            auto_cool_enabled: self.governor.is_auto_cool_enabled(),
            tamed_pids,
            top_processes,
            rogue_alerts,
            os_version: self.os_version.clone(),
        }
    }

    /// Soft-tame a specific process by PID (limits to given CPU % cap, e.g. 10%)
    pub fn soft_tame_process(&mut self, pid: u32, cpu_limit_pct: u32) -> Result<()> {
        self.mitigation.soft_tame(pid, cpu_limit_pct)
    }

    /// Safely terminates a verified rogue process
    pub fn terminate_process(&mut self, pid: u32, process_name: &str) -> Result<()> {
        ProcessActionController::terminate(pid, process_name, &self.safety)
    }

    /// Reverts restrictions on a previously tamed process
    pub fn revert_tame(&mut self, pid: u32) -> bool {
        self.mitigation.revert(pid)
    }

    /// Reverts all active process throttles
    pub fn revert_all_tames(&mut self) -> usize {
        self.mitigation.revert_all()
    }

    /// Manually toggle Turbo Boost (true = 100% Boost On, false = 99% Boost Clamped)
    pub fn set_turbo_boost(&mut self, enabled: bool) -> Result<()> {
        self.governor.set_turbo_boost(enabled)
    }

    pub fn set_auto_tame(&mut self, enabled: bool) {
        self.auto_tame_enabled = enabled;
    }

    pub fn set_auto_cool(&mut self, enabled: bool) {
        self.governor.set_auto_cool(enabled);
    }

    pub fn is_auto_cool_enabled(&self) -> bool {
        self.governor.is_auto_cool_enabled()
    }

    pub fn set_auto_cool_threshold(&mut self, threshold: f32) {
        self.governor.set_high_temp_threshold(threshold);
    }

    pub fn auto_cool_threshold(&self) -> f32 {
        self.governor.high_temp_threshold()
    }

    pub fn set_saturation_threshold(&mut self, threshold: f32) {
        self.heuristics.set_saturation_threshold(threshold);
    }

    pub fn is_auto_tame_enabled(&self) -> bool {
        self.auto_tame_enabled
    }

    pub fn logical_cores(&self) -> u32 {
        self.process.logical_cores()
    }
}

impl Default for Watchdog {
    fn default() -> Self {
        Self::new()
    }
}
