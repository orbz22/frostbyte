use std::collections::HashSet;

/// Immutable list of critical Windows system processes, security products,
/// and core hardware driver services that must NEVER be terminated or throttled.
const HARDCODED_IMMUNE_PROCESSES: &[&str] = &[
    // Core NT & Windows Subsystems
    "system",
    "system idle process",
    "idle",
    "registry",
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "services.exe",
    "lsass.exe",
    "lsm.exe",
    "winlogon.exe",
    "dwm.exe",
    "explorer.exe",
    "sihost.exe",
    "taskhostw.exe",
    "fontdrvhost.exe",
    "audiodg.exe",
    "svchost.exe",

    // Windows Defender & Security
    "msmpeng.exe",
    "nissrv.exe",
    "securityhealthservice.exe",
    "securityhealthsystray.exe",
    "smartscreen.exe",

    // Hardware Display & Audio Drivers
    "nvcontainer.exe",
    "nvdisplay.container.exe",
    "nvsphelper64.exe",
    "amdfendrsr.exe",
    "radeontrafficshaper.exe",
    "igfxcuiservice.exe",
    "igfxem.exe",
    "rtkaudioservice64.exe",

    // FrostByte itself
    "frostbyte.exe",
    "frostbyte-cli.exe",
    "frostbyte-app.exe",
];

pub struct SafetyEngine {
    custom_whitelist: HashSet<String>,
}

impl SafetyEngine {
    pub fn new() -> Self {
        Self {
            custom_whitelist: HashSet::new(),
        }
    }

    pub fn with_custom_whitelist(names: &[&str]) -> Self {
        let mut set = HashSet::new();
        for name in names {
            set.insert(name.to_lowercase());
        }
        Self {
            custom_whitelist: set,
        }
    }

    /// Returns true if the process name is protected from any aggressive action.
    pub fn is_immune(&self, process_name: &str) -> bool {
        let lower = process_name.to_lowercase();
        let trimmed = lower.trim();

        // 1. Check hardcoded immune list
        for immune in HARDCODED_IMMUNE_PROCESSES {
            if trimmed == *immune || trimmed.trim_end_matches(".exe") == immune.trim_end_matches(".exe") {
                return true;
            }
        }

        // 2. Check user-defined custom whitelist
        if self.custom_whitelist.contains(trimmed)
            || self.custom_whitelist.contains(trimmed.trim_end_matches(".exe")) {
            return true;
        }

        false
    }

    pub fn add_to_whitelist(&mut self, process_name: &str) {
        self.custom_whitelist.insert(process_name.to_lowercase());
    }

    pub fn remove_from_whitelist(&mut self, process_name: &str) {
        self.custom_whitelist.remove(&process_name.to_lowercase());
    }
}

impl Default for SafetyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardcoded_immunity() {
        let safety = SafetyEngine::new();
        assert!(safety.is_immune("csrss.exe"));
        assert!(safety.is_immune("CSRSS.EXE"));
        assert!(safety.is_immune("explorer.exe"));
        assert!(safety.is_immune("DWM.exe"));
        assert!(safety.is_immune("MsMpEng.exe"));
        assert!(!safety.is_immune("idmintegrator64.exe"));
        assert!(!safety.is_immune("node.exe"));
    }

    #[test]
    fn test_custom_whitelist() {
        let mut safety = SafetyEngine::new();
        assert!(!safety.is_immune("my_custom_tool.exe"));
        safety.add_to_whitelist("my_custom_tool.exe");
        assert!(safety.is_immune("my_custom_tool.exe"));
    }
}
