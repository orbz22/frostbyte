use crate::safety::SafetyEngine;
use crate::types::{ProcessSample, RogueAlert};
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct TrackedHighLoad {
    consecutive_ticks: u32,
    peak_saturation: f32,
}

pub struct HeuristicEngine {
    // Key: (PID, ThreadID)
    tracked_threads: HashMap<(u32, u32), TrackedHighLoad>,
    safety: SafetyEngine,
    saturation_threshold: f32,
    min_consecutive_ticks: u32,
    tick_interval_secs: u32,
}

impl HeuristicEngine {
    pub fn new(safety: SafetyEngine, saturation_threshold: f32, min_consecutive_ticks: u32, tick_interval_secs: u32) -> Self {
        Self {
            tracked_threads: HashMap::new(),
            safety,
            saturation_threshold,
            min_consecutive_ticks,
            tick_interval_secs,
        }
    }

    /// Evaluates current process samples and returns alerts for confirmed runaway loops.
    pub fn evaluate(&mut self, processes: &[ProcessSample]) -> Vec<RogueAlert> {
        let mut alerts = Vec::new();
        let mut active_keys = Vec::new();

        for proc in processes {
            // Never flag immune system processes
            if self.safety.is_immune(&proc.name) {
                continue;
            }

            // Check if top thread exceeds single-core saturation threshold (e.g. >= 80%)
            if proc.top_thread_saturation_pct >= self.saturation_threshold {
                let key = (proc.pid, 0); // Using 0 as representative or top thread key
                active_keys.push(key);

                let entry = self.tracked_threads.entry(key).or_insert(TrackedHighLoad {
                    consecutive_ticks: 0,
                    peak_saturation: 0.0,
                });

                entry.consecutive_ticks += 1;
                entry.peak_saturation = entry.peak_saturation.max(proc.top_thread_saturation_pct);

                let sustained_seconds = entry.consecutive_ticks * self.tick_interval_secs;

                if entry.consecutive_ticks >= self.min_consecutive_ticks {
                    let mut reason_parts = Vec::new();
                    reason_parts.push(format!(
                        "Sustained single-core saturation: {:.1}% for {}s",
                        entry.peak_saturation, sustained_seconds
                    ));

                    if proc.is_orphan {
                        reason_parts.push("Orphaned background process (parent dead)".to_string());
                    }

                    alerts.push(RogueAlert {
                        pid: proc.pid,
                        ppid: proc.ppid,
                        process_name: proc.name.clone(),
                        thread_id: 0,
                        single_core_saturation_pct: entry.peak_saturation,
                        total_process_cpu_pct: proc.total_cpu_pct,
                        sustained_seconds,
                        is_orphan: proc.is_orphan,
                        reason: reason_parts.join(" | "),
                    });
                }
            }
        }

        // Clean up threads that have calmed down
        self.tracked_threads.retain(|key, _| active_keys.contains(key));

        alerts
    }

    pub fn set_saturation_threshold(&mut self, threshold: f32) {
        self.saturation_threshold = threshold;
    }

    pub fn saturation_threshold(&self) -> f32 {
        self.saturation_threshold
    }
}

impl Default for HeuristicEngine {
    fn default() -> Self {
        Self::new(SafetyEngine::default(), 80.0, 5, 2) // 5 ticks * 2s = 10 seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rogue_loop_detection() {
        let mut engine = HeuristicEngine::new(SafetyEngine::default(), 80.0, 3, 2);

        let rogue_proc = ProcessSample {
            pid: 9999,
            ppid: 1234,
            name: "stuck_worker.exe".to_string(),
            is_orphan: true,
            total_cpu_pct: 6.25, // 1 core on 16 cores
            top_thread_saturation_pct: 98.5,
            thread_count: 2,
        };

        // Tick 1
        let alerts = engine.evaluate(&[rogue_proc.clone()]);
        assert!(alerts.is_empty(), "Should not alert on first tick");

        // Tick 2
        let alerts = engine.evaluate(&[rogue_proc.clone()]);
        assert!(alerts.is_empty(), "Should not alert on second tick");

        // Tick 3: reaches threshold
        let alerts = engine.evaluate(&[rogue_proc.clone()]);
        assert_eq!(alerts.len(), 1, "Should alert after 3 consecutive high-load ticks");
        assert_eq!(alerts[0].pid, 9999);
        assert_eq!(alerts[0].process_name, "stuck_worker.exe");
        assert!(alerts[0].single_core_saturation_pct >= 98.0);
        assert!(alerts[0].is_orphan);
    }

    #[test]
    fn test_whitelisted_process_never_alerts() {
        let mut engine = HeuristicEngine::new(SafetyEngine::default(), 80.0, 1, 2);

        let system_proc = ProcessSample {
            pid: 100,
            ppid: 4,
            name: "explorer.exe".to_string(),
            is_orphan: false,
            total_cpu_pct: 20.0,
            top_thread_saturation_pct: 100.0,
            thread_count: 30,
        };

        let alerts = engine.evaluate(&[system_proc]);
        assert!(alerts.is_empty(), "Whitelisted process explorer.exe must never trigger an alert");
    }
}
