use crate::types::{ProcessSample, ThreadSample};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, Thread32First, Thread32Next,
    PROCESSENTRY32W, TH32CS_SNAPPROCESS, TH32CS_SNAPTHREAD, THREADENTRY32,
};
use windows::Win32::System::Threading::{
    GetProcessTimes, GetThreadTimes, OpenProcess, OpenThread, PROCESS_QUERY_LIMITED_INFORMATION,
    THREAD_QUERY_LIMITED_INFORMATION,
};

#[derive(Clone, Debug)]
struct ProcessPrevTimes {
    kernel: u64,
    user: u64,
    timestamp: Instant,
}

#[derive(Clone, Debug)]
struct ThreadPrevTimes {
    kernel: u64,
    user: u64,
}

pub struct ProcessWatcher {
    logical_cores: u32,
    prev_processes: HashMap<u32, ProcessPrevTimes>,
    prev_threads: HashMap<u32, ThreadPrevTimes>,
}

impl ProcessWatcher {
    pub fn new() -> Self {
        let logical_cores = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(8);

        Self {
            logical_cores,
            prev_processes: HashMap::new(),
            prev_threads: HashMap::new(),
        }
    }

    pub fn logical_cores(&self) -> u32 {
        self.logical_cores
    }

    /// Captures a snapshot of all active processes and threads, computing CPU deltas and single-core saturations.
    pub fn sample(&mut self) -> (f32, Vec<ProcessSample>) {
        let now = Instant::now();
        let (raw_processes, active_pids) = self.get_process_list();
        let active_threads = self.get_thread_list();

        let mut process_samples = Vec::new();
        let mut total_system_delta_100ns = 0u64;
        let mut total_wall_clock_100ns = 0u64;

        // Group threads by process ID
        let mut process_threads_map: HashMap<u32, Vec<u32>> = HashMap::new();
        for (tid, pid) in &active_threads {
            process_threads_map.entry(*pid).or_default().push(*tid);
        }

        for (pid, ppid, name) in raw_processes {
            // Check if parent process is still alive
            let is_orphan = ppid > 0 && !active_pids.contains(&ppid) && pid != 4 && name != "smss.exe";

            // Query process CPU times
            if let Some((kernel, user)) = Self::read_process_times(pid) {
                let mut proc_cpu_pct = 0.0f32;
                let mut top_thread_saturation = 0.0f32;
                let mut inspected_threads = Vec::new();

                if let Some(prev) = self.prev_processes.get(&pid) {
                    let elapsed = now.duration_since(prev.timestamp).as_secs_f32();
                    if elapsed > 0.01 {
                        let delta_kernel = kernel.saturating_sub(prev.kernel);
                        let delta_user = user.saturating_sub(prev.user);
                        let delta_total = delta_kernel + delta_user;

                        let wall_100ns = (elapsed * 10_000_000.0) as u64;
                        total_system_delta_100ns += delta_total;
                        total_wall_clock_100ns = total_wall_clock_100ns.max(wall_100ns);

                        // Process CPU % normalized to all cores
                        proc_cpu_pct = ((delta_total as f32) / (wall_100ns as f32 * self.logical_cores as f32)) * 100.0;

                        // If process is burning significant CPU, inspect its threads for single-core saturation
                        if proc_cpu_pct >= 0.5 {
                            if let Some(tids) = process_threads_map.get(&pid) {
                                for tid in tids {
                                    if let Some((t_kernel, t_user)) = Self::read_thread_times(*tid) {
                                        if let Some(prev_t) = self.prev_threads.get(tid) {
                                            let t_delta = (t_kernel.saturating_sub(prev_t.kernel))
                                                + (t_user.saturating_sub(prev_t.user));
                                            let t_saturation = ((t_delta as f32) / (wall_100ns as f32)) * 100.0;

                                            top_thread_saturation = top_thread_saturation.max(t_saturation);

                                            inspected_threads.push(ThreadSample {
                                                thread_id: *tid,
                                                process_id: pid,
                                                kernel_time_100ns: t_kernel,
                                                user_time_100ns: t_user,
                                                saturation_pct: t_saturation,
                                            });
                                        }

                                        self.prev_threads.insert(
                                            *tid,
                                            ThreadPrevTimes {
                                                kernel: t_kernel,
                                                user: t_user,
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                self.prev_processes.insert(
                    pid,
                    ProcessPrevTimes {
                        kernel,
                        user,
                        timestamp: now,
                    },
                );

                let thread_count = process_threads_map.get(&pid).map(|v| v.len()).unwrap_or(1);

                process_samples.push(ProcessSample {
                    pid,
                    ppid,
                    name,
                    is_orphan,
                    total_cpu_pct: proc_cpu_pct,
                    top_thread_saturation_pct: top_thread_saturation,
                    thread_count,
                });
            }
        }

        // Clean up dead processes and threads from memory
        self.prev_processes.retain(|pid, _| active_pids.contains(pid));
        let active_tid_set: HashSet<u32> = active_threads.into_iter().map(|(tid, _)| tid).collect();
        self.prev_threads.retain(|tid, _| active_tid_set.contains(tid));

        // Calculate overall system CPU %
        let total_system_cpu_pct = if total_wall_clock_100ns > 0 {
            ((total_system_delta_100ns as f32) / (total_wall_clock_100ns as f32 * self.logical_cores as f32)) * 100.0
        } else {
            0.0
        };

        // Sort descending by CPU percentage
        process_samples.sort_by(|a, b| {
            b.total_cpu_pct
                .partial_cmp(&a.total_cpu_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        (total_system_cpu_pct, process_samples)
    }

    /// Read raw process kernel and user time (in 100-nanosecond units)
    fn read_process_times(pid: u32) -> Option<(u64, u64)> {
        unsafe {
            let handle: HANDLE = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut creation = FILETIME::default();
            let mut exit = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();

            let res = GetProcessTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user);
            let _ = CloseHandle(handle);

            if res.is_ok() {
                let k = ((kernel.dwHighDateTime as u64) << 32) | (kernel.dwLowDateTime as u64);
                let u = ((user.dwHighDateTime as u64) << 32) | (user.dwLowDateTime as u64);
                Some((k, u))
            } else {
                None
            }
        }
    }

    /// Read raw thread kernel and user time (in 100-nanosecond units)
    fn read_thread_times(tid: u32) -> Option<(u64, u64)> {
        unsafe {
            let handle: HANDLE = OpenThread(THREAD_QUERY_LIMITED_INFORMATION, false, tid).ok()?;
            let mut creation = FILETIME::default();
            let mut exit = FILETIME::default();
            let mut kernel = FILETIME::default();
            let mut user = FILETIME::default();

            let res = GetThreadTimes(handle, &mut creation, &mut exit, &mut kernel, &mut user);
            let _ = CloseHandle(handle);

            if res.is_ok() {
                let k = ((kernel.dwHighDateTime as u64) << 32) | (kernel.dwLowDateTime as u64);
                let u = ((user.dwHighDateTime as u64) << 32) | (user.dwLowDateTime as u64);
                Some((k, u))
            } else {
                None
            }
        }
    }

    /// Enumerate all running processes: (PID, PPID, ProcessName)
    fn get_process_list(&self) -> (Vec<(u32, u32, String)>, HashSet<u32>) {
        let mut list = Vec::new();
        let mut pids = HashSet::new();

        unsafe {
            let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
                Ok(h) => h,
                Err(_) => return (list, pids),
            };

            let mut entry = PROCESSENTRY32W::default();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    let pid = entry.th32ProcessID;
                    let ppid = entry.th32ParentProcessID;
                    let name = String::from_utf16_lossy(&entry.szExeFile)
                        .trim_matches('\0')
                        .to_string();

                    pids.insert(pid);
                    list.push((pid, ppid, name));

                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }

            let _ = CloseHandle(snapshot);
        }

        (list, pids)
    }

    /// Enumerate all running threads: (ThreadID, ProcessID)
    fn get_thread_list(&self) -> Vec<(u32, u32)> {
        let mut list = Vec::new();

        unsafe {
            let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) {
                Ok(h) => h,
                Err(_) => return list,
            };

            let mut entry = THREADENTRY32::default();
            entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;

            if Thread32First(snapshot, &mut entry).is_ok() {
                loop {
                    let tid = entry.th32ThreadID;
                    let pid = entry.th32OwnerProcessID;
                    list.push((tid, pid));

                    if Thread32Next(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }

            let _ = CloseHandle(snapshot);
        }

        list
    }
}

impl Default for ProcessWatcher {
    fn default() -> Self {
        Self::new()
    }
}
