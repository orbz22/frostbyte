use crate::safety::SafetyEngine;
use anyhow::{bail, Result};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

pub struct ProcessActionController;

impl ProcessActionController {
    /// Safely terminates a confirmed rogue process after verifying immunity rules.
    pub fn terminate(pid: u32, process_name: &str, safety: &SafetyEngine) -> Result<()> {
        if safety.is_immune(process_name) {
            bail!(
                "Refused to terminate '{}' (PID {}): Process is protected by SafetyEngine.",
                process_name,
                pid
            );
        }

        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, false, pid).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to open PID {} with terminate permission: {:?}",
                    pid,
                    e
                )
            })?;

            let term_res = TerminateProcess(handle, 1);
            let _ = CloseHandle(handle);

            if term_res.is_ok() {
                Ok(())
            } else {
                Err(anyhow::anyhow!("TerminateProcess failed for PID {}", pid))
            }
        }
    }
}
