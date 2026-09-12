use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalSnapshot {
    pub cpu_package_temp: Option<f32>,
    pub max_core_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub gpu_power_w: Option<f32>,
    pub gpu_utilization_pct: Option<f32>,
    pub gpu_clock_mhz: Option<u32>,
    pub is_ac_online: bool,
    pub battery_percent: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadSample {
    pub thread_id: u32,
    pub process_id: u32,
    pub kernel_time_100ns: u64,
    pub user_time_100ns: u64,
    pub saturation_pct: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSample {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub is_orphan: bool,
    pub total_cpu_pct: f32,
    pub top_thread_saturation_pct: f32,
    pub thread_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RogueAlert {
    pub pid: u32,
    pub ppid: u32,
    pub process_name: String,
    pub thread_id: u32,
    pub single_core_saturation_pct: f32,
    pub total_process_cpu_pct: f32,
    pub sustained_seconds: u32,
    pub is_orphan: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub thermals: ThermalSnapshot,
    pub total_cpu_pct: f32,
    pub logical_cores: u32,
    pub is_turbo_boost_clamped: bool,
    pub auto_tame_enabled: bool,
    pub auto_cool_enabled: bool,
    pub tamed_pids: Vec<u32>,
    pub top_processes: Vec<ProcessSample>,
    pub rogue_alerts: Vec<RogueAlert>,
    pub os_version: String,
}
