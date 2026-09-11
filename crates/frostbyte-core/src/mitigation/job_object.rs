use anyhow::{Context, Result};
use std::collections::HashMap;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, SetInformationJobObject,
    JobObjectCpuRateControlInformation, JOBOBJECT_CPU_RATE_CONTROL_INFORMATION,
    JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0, JOB_OBJECT_CPU_RATE_CONTROL_ENABLE,
    JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
};
use windows::Win32::System::Threading::{
    OpenProcess, SetPriorityClass, IDLE_PRIORITY_CLASS,
    PROCESS_SET_INFORMATION, PROCESS_SET_QUOTA,
};

#[derive(Debug)]
pub struct TamedProcessEntry {
    pub pid: u32,
    pub job_handle: isize,
    pub cpu_limit_pct: u32,
    pub tamed_at: chrono::DateTime<chrono::Local>,
}

pub struct JobRateLimiter {
    active_jobs: HashMap<u32, HANDLE>,
}

impl JobRateLimiter {
    pub fn new() -> Self {
        Self {
            active_jobs: HashMap::new(),
        }
    }

    /// Soft-tames a runaway process by assigning it to a Windows Job Object
    /// with a strict CPU rate cap (default: 10%) and sets its priority to IDLE.
    pub fn soft_tame(&mut self, pid: u32, cpu_limit_pct: u32) -> Result<()> {
        let cpu_limit = cpu_limit_pct.clamp(1, 100);
        // Rate in 100ths of a percent (10,000 = 100%, 1,000 = 10%)
        let rate_in_basis_points = cpu_limit * 100;

        unsafe {
            // 1. Create anonymous Job Object
            let job_handle = CreateJobObjectW(None, None)
                .context("Failed to create Windows Job Object")?;

            // 2. Configure CPU rate control
            let mut info = JOBOBJECT_CPU_RATE_CONTROL_INFORMATION {
                ControlFlags: JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP,
                Anonymous: JOBOBJECT_CPU_RATE_CONTROL_INFORMATION_0 {
                    CpuRate: rate_in_basis_points,
                },
            };

            let set_res = SetInformationJobObject(
                job_handle,
                JobObjectCpuRateControlInformation,
                &mut info as *mut _ as *const _,
                std::mem::size_of::<JOBOBJECT_CPU_RATE_CONTROL_INFORMATION>() as u32,
            );

            if let Err(e) = set_res {
                let _ = CloseHandle(job_handle);
                return Err(anyhow::anyhow!("Failed to set job CPU rate control: {:?}", e));
            }

            // 3. Open target process
            let proc_handle = match OpenProcess(PROCESS_SET_QUOTA | PROCESS_SET_INFORMATION, false, pid) {
                Ok(h) => h,
                Err(e) => {
                    let _ = CloseHandle(job_handle);
                    return Err(anyhow::anyhow!("Failed to open process PID {} for quota/information: {:?}", pid, e));
                }
            };

            // 4. Assign process to Job Object
            let assign_res = AssignProcessToJobObject(job_handle, proc_handle);
            if let Err(e) = assign_res {
                let _ = CloseHandle(proc_handle);
                let _ = CloseHandle(job_handle);
                return Err(anyhow::anyhow!("Failed to assign PID {} to Job Object: {:?}", pid, e));
            }

            // 5. Lower priority to IDLE
            let _ = SetPriorityClass(proc_handle, IDLE_PRIORITY_CLASS);
            let _ = CloseHandle(proc_handle);

            // Clean up any previous job for this PID
            if let Some(old_job) = self.active_jobs.remove(&pid) {
                let _ = CloseHandle(old_job);
            }

            self.active_jobs.insert(pid, job_handle);
            Ok(())
        }
    }

    /// Reverts restrictions on a previously tamed process by closing its Job Object handle.
    pub fn revert(&mut self, pid: u32) -> bool {
        if let Some(job) = self.active_jobs.remove(&pid) {
            unsafe {
                let _ = CloseHandle(job);
            }
            true
        } else {
            false
        }
    }

    /// Reverts all active process throttles.
    pub fn revert_all(&mut self) -> usize {
        let count = self.active_jobs.len();
        for (_, job) in self.active_jobs.drain() {
            unsafe {
                let _ = CloseHandle(job);
            }
        }
        count
    }

    pub fn is_tamed(&self, pid: u32) -> bool {
        self.active_jobs.contains_key(&pid)
    }

    pub fn active_tamed_pids(&self) -> Vec<u32> {
        self.active_jobs.keys().copied().collect()
    }
}

impl Drop for JobRateLimiter {
    fn drop(&mut self) {
        self.revert_all();
    }
}

impl Default for JobRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

// Windows kernel handles (such as Job Object handles) are thread-safe and can be managed across threads.
unsafe impl Send for JobRateLimiter {}
unsafe impl Sync for JobRateLimiter {}

