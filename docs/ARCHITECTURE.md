# System Architecture & Technical Design — FrostByte

**Document Version:** 1.0.0  
**Target OS:** Windows 10 (Build 19041+) / Windows 11 (All Builds)  
**Architecture Pattern:** Modular Daemon / Tray Service + Decoupled Native UI  

---

## 1. High-Level Architecture Overview

FrostByte operates on an event-driven and sampled pipeline. It decouples the **Hardware/Process Sampling Engine** from the **Mitigation & Heuristics Core**, which interfaces with the **Windows System API** and the **User Interface**.

```
+-------------------------------------------------------------------------+
|                              FrostByte UI                               |
|        [System Tray Icon] <---> [Quick Flyout] <---> [Dashboard]        |
+-------------------------------------------------------------------------+
                                    | (IPC / Native Messages)
                                    v
+-------------------------------------------------------------------------+
|                            Core Orchestrator                            |
|             (Tick Manager, State Machine, Event Dispatcher)             |
+-------------------------------------------------------------------------+
      |                             |                             |
      v                             v                             v
+------------------+      +-------------------+      +--------------------+
|  Thermal Engine  |      |  Process Watcher  |      |   Safety Engine    |
| - WMI ACPI Zones |      | - Thread Deltas   |      | - Kernel Whitelist |
| - LibreHW API    |      | - CPU Saturation  |      | - Audio Detector   |
| - NVML (GPU)     |      | - Orphan Detector |      | - Fullscreen Check |
+------------------+      +-------------------+      +--------------------+
      |                             |                             |
      +-----------------------------+-----------------------------+
                                    |
                                    v
                      +---------------------------+
                      |     Heuristics Engine     |
                      |  (Score Rogue Likelihood) |
                      +---------------------------+
                                    |
                                    v
                      +---------------------------+
                      |    Mitigation Governor    |
                      | - Job Object CPU Quota    |
                      | - Priority / Affinity     |
                      | - Power Plan (Boost Cap)  |
                      | - Controlled Termination  |
                      +---------------------------+
                                    |
                                    v
                      +---------------------------+
                      |   Windows OS / Hardware   |
                      +---------------------------+
```

---

## 2. Core Subsystems & Components

### 2.1 Thermal Engine (`ThermalProvider`)
Captures real-time temperature data from CPU and GPU sensors without crashing or locking up the application thread.

* **Primary Backend: LibreHardwareMonitorLib / WinRing0 / WMI ACPI**
  * Reads Intel DTS (Digital Thermal Sensor) / AMD Tctl/Tdie via MSRs or OS driver.
  * Queries `root\WMI:MSAcpi_ThermalZoneTemperature` when elevated drivers are not present.
* **Secondary Backend: NVIDIA NVML / DXGI**
  * Direct query to `nvmlDeviceGetTemperature` for discrete GPU thermals.
* **Adaptive Sampling:**
  * **Cool State (< 70°C):** Sample every 3000ms.
  * **Warm State (70°C – 85°C):** Sample every 1500ms.
  * **Thermal Alert (> 85°C):** Sample every 500ms.

---

### 2.2 Process & Thread Sampler (`ProcessWatcher`)
Detects processes that cause thermal spikes even when total CPU appears deceptively low.

* **Thread-Level Delta Formula:**
  For each thread $i$ belonging to process $P$:
  $$\Delta t_{\text{cpu}} = (t_{\text{kernel}}[k] - t_{\text{kernel}}[k-1]) + (t_{\text{user}}[k] - t_{\text{user}}[k-1])$$
  $$\text{Saturation}_{\text{thread}} = \frac{\Delta t_{\text{cpu}}}{\Delta t_{\text{wall\_clock}} \times 10{,}000{,}000} \times 100\%$$
* **Single-Core Peak Detection:**
  If $\text{Saturation}_{\text{thread}} \ge 85\%$ sustained across $N$ consecutive ticks ($N \ge 30$, approx. 60 seconds):
  $\implies$ Flag as **Candidate Rogue Loop**.

* **Data Collection Method:**
  * Uses `NtQuerySystemInformation(SystemProcessInformation)` for high-performance single-syscall snapshot of all running processes and threads.
  * Bypasses slow WMI process queries, keeping sampling overhead under **< 0.1% CPU**.

---

### 2.3 Heuristics & Evaluation Engine (`HeuristicEngine`)
Before any process is marked as rogue, it must pass through the **Rogue Probability Scorer ($S_{\text{rogue}}$)**:

$$S_{\text{rogue}} = w_1 \cdot C_{\text{single\_core}} + w_2 \cdot O_{\text{orphan}} + w_3 \cdot W_{\text{no\_window}} + w_4 \cdot T_{\text{thermal\_impact}} - P_{\text{safeguards}}$$

#### Factor Scoring:
1. **$C_{\text{single\_core}}$ (Single-core saturation):**
   * $+40$ points if 1 thread $>90\%$ for $>60\text{s}$.
2. **$O_{\text{orphan}}$ (Orphaned status):**
   * $+30$ points if Parent PID no longer exists or belongs to an unrelated new process.
3. **$W_{\text{no\_window}}$ (Window Visibility):**
   * $+20$ points if process has no visible window, or is not in the active foreground.
4. **$T_{\text{thermal\_impact}}$ (Thermal Correlation):**
   * $+20$ points if CPU package temperature $>85^\circ\text{C}$ and frequency $>130\%$ base clock.
5. **$P_{\text{safeguards}}$ (Deductions):**
   * $-100$ points if found in Protected Whitelist.
   * $-80$ points if active audio stream detected.
   * $-100$ points if user is in active fullscreen game.

**Decision Thresholds:**
* $S_{\text{rogue}} \ge 70$: **Warning / Recommend Mitigation**
* $S_{\text{rogue}} \ge 90$ with CPU Temp $>90^\circ\text{C}$: **Immediate Action (Auto-Tame / Emergency Cool)**

---

### 2.4 Safety Engine & Safeguards (`SafetyEngine`)
Safety is the paramount constraint of FrostByte.

1. **Kernel & Critical Subsystem Whitelist:**
   * Hardcoded hash and executable name verification (`ntoskrnl.exe`, `csrss.exe`, `services.exe`, `lsass.exe`, `winlogon.exe`, `dwm.exe`, `explorer.exe`, `audiodg.exe`, antivirus suites).
2. **Audio Playback Detection:**
   * Queries Windows `IAudioSessionManager2` (`Audioclient.h`).
   * If a process has an active audio session with state `AudioSessionStateActive`, it is never killed or throttled (protects YouTube, media players, Spotify, meetings).
3. **Fullscreen / Presentation Detection:**
   * Calls `SHQueryUserNotificationState(&state)`.
   * If state is `QUNS_RUNNING_D3D_FULL_SCREEN` or `QUNS_PRESENTATION_MODE`, automated kills are suppressed, and notifications are silenced.

---

### 2.5 Mitigation & Governor Engine (`MitigationController`)
Provides four non-destructive steps before resorting to process termination:

1. **Step 1: Windows Job Object CPU Rate Limiting (Soft Tame)**
   * Assigns the rogue process to an anonymous Windows Job Object.
   * Configures `JOBOBJECT_CPU_RATE_CONTROL_INFORMATION` with `JOB_OBJECT_CPU_RATE_CONTROL_ENABLE | JOB_OBJECT_CPU_RATE_CONTROL_HARD_CAP` set to 10% (1,000 in basis points).
   * Result: The process still functions, but cannot consume more than 10% of a core, instantly shedding heat.
2. **Step 2: CPU Affinity Pinning**
   * Uses `SetProcessAffinityMask` to move the process to a single Efficiency Core (on Intel hybrid CPUs) or the lowest-priority core.
3. **Step 3: Dynamic Turbo Boost Toggle (`PowerSchemeController`)**
   * Uses Windows Power Management API (`powrprof.dll`):
     ```c
     PowerWriteACValueIndex(NULL, &GUID_CURRENT_SCHEME, &GUID_PROCESSOR_SETTINGS_SUBGROUP, &GUID_PROCTHROTTLEMAX, 99);
     PowerSetActiveScheme(NULL, &GUID_CURRENT_SCHEME);
     ```
   * Changing `PROCTHROTTLEMAX` from 100% to 99% disables Intel Turbo Boost / AMD Precision Boost immediately without rebooting, dropping temperature by 15°C–25°C in seconds.
4. **Step 4: Graceful Termination (`TerminateProcess` fallback)**
   * Used only for confirmed orphaned CLI/background worker tasks or with explicit user consent via Toast notification.

---

## 3. Data Flow & Execution Loop

```
Every Tick (e.g. 2000ms):
  1. Read CPU Package Temperature (T_cpu)
  2. Query Process Snapshot via NtQuerySystemInformation
  3. Calculate Thread CPU Time Deltas
  4. Is T_cpu > ThermalWarningThreshold (e.g. 85°C)?
     YES:
       - Run Heuristics on top 5 offending threads
       - If RogueScore >= 70:
           - Check Safety Rules (Whitelist, Audio, Fullscreen)
           - Trigger Mitigation Strategy:
               Mode 1: Send Interactive Toast Notification
               Mode 2: Apply Job Object CPU Rate Cap (10%)
               Mode 3: Toggle Turbo Boost to 99%
     NO:
       - If system was previously throttled and T_cpu <= 65°C for 15s:
           - Restore normal Power Plan (PROCTHROTTLEMAX = 100%)
  5. Update Tray Icon & Shared Memory State
```

---

## 4. Security & Permissions Architecture
* **Standard User Mode (Unprivileged):**
  * Process monitoring (`OpenProcess` with `PROCESS_QUERY_LIMITED_INFORMATION`), process priority adjustment, and Job Object assignment for processes owned by the current user.
  * System Tray UI, notifications, and logging.
* **Elevated / Administrator Mode (Optional Service):**
  * Access to MSR/Ring0 temperature sensors if WMI is unavailable.
  * Throttling/terminating elevated background processes or system-wide power plan writes.
  * Running as a split architecture: Unprivileged UI Client + Optional Elevated Worker Service communicating via Named Pipes.
