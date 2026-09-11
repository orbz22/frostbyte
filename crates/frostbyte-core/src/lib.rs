pub mod heuristics;
pub mod process;
pub mod safety;
pub mod thermal;
pub mod types;

pub use heuristics::HeuristicEngine;
pub use process::ProcessWatcher;
pub use safety::SafetyEngine;
pub use thermal::ThermalProvider;
pub use types::*;

pub struct Watchdog {
    thermal: ThermalProvider,
    process: ProcessWatcher,
    heuristics: HeuristicEngine,
}

impl Watchdog {
    pub fn new() -> Self {
        let thermal = ThermalProvider::new();
        let process = ProcessWatcher::new();
        let heuristics = HeuristicEngine::default();

        Self {
            thermal,
            process,
            heuristics,
        }
    }

    pub fn with_settings(saturation_threshold: f32, min_consecutive_ticks: u32, tick_interval_secs: u32) -> Self {
        let thermal = ThermalProvider::new();
        let process = ProcessWatcher::new();
        let safety = SafetyEngine::new();
        let heuristics = HeuristicEngine::new(safety, saturation_threshold, min_consecutive_ticks, tick_interval_secs);

        Self {
            thermal,
            process,
            heuristics,
        }
    }

    /// Executes one evaluation tick: samples thermals, processes, and evaluates runaway heuristics.
    pub fn tick(&mut self) -> SystemSnapshot {
        let thermals = self.thermal.sample();
        let (total_cpu_pct, processes) = self.process.sample();
        let rogue_alerts = self.heuristics.evaluate(&processes);

        // Keep top 10 processes by CPU
        let top_processes = processes.into_iter().take(10).collect();

        SystemSnapshot {
            timestamp: chrono::Local::now(),
            thermals,
            total_cpu_pct,
            logical_cores: self.process.logical_cores(),
            top_processes,
            rogue_alerts,
        }
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
