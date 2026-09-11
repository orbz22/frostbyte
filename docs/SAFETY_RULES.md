# Safety Rules & Process Protection Guidelines — FrostByte

**Document Version:** 1.0.0  
**Purpose:** Define non-negotiable boundaries, whitelists, and heuristics to prevent system instability, data loss, and false-positive interventions.

---

## 1. The Core Philosophy: "Do No Harm"

A background process watchdog holds significant power over user workflows. If implemented carelessly, an automated tool can cause Blue Screens of Death (BSOD), terminate unsaved user work, disrupt ongoing audio/video conferences, or break OS functionality.

**FrostByte follows 5 Non-Negotiable Safety Principles:**
1. **Never Kill Windows Subsystem Processes:** System-level binaries are strictly read-only for monitoring; they must never be terminated, suspended, or throttled.
2. **Never Disrupt Active Human Interaction:** If a user is actively typing, listening, presenting, or playing a game, FrostByte must stay invisible and silent.
3. **Prefer Taming Over Killing:** Throttling (Job Object CPU quota / Priority / Affinity) preserves process state and prevents data loss. Process termination (`Kill`) is the absolute last resort.
4. **Transient State Reversibility:** Any restriction applied (Affinity, Priority, Job Quotas) can be rolled back instantly with one click from the System Tray.
5. **Fail-Safe Crash Recovery:** If FrostByte crashes or terminates, Windows automatically tears down anonymous Job Objects, instantly returning all processes to their default state.

---

## 2. Hardcoded System Whitelist (Never Touch)

The following processes are hardcoded into the immutable kernel of `SafetyEngine`. No heuristic, rule, or high temperature can ever trigger an intervention against them:

### 2.1 Critical Windows Kernel & Subsystems
* `ntoskrnl.exe` (NT Kernel)
* `csrss.exe` (Client/Server Runtime Subsystem)
* `smss.exe` (Session Manager)
* `wininit.exe` (Windows Initialization)
* `services.exe` (Service Control Manager)
* `lsass.exe` (Local Security Authority Subsystem)
* `winlogon.exe` (Windows Logon Process)
* `dwm.exe` (Desktop Window Manager)
* `explorer.exe` (Windows Shell & Taskbar)
* `sihost.exe` (Shell Infrastructure Host)
* `fontdrvhost.exe` (Usermode Font Driver Host)
* `taskhostw.exe` (Host Process for Windows Tasks)
* `audiodg.exe` (Windows Audio Device Graph Isolation)

### 2.2 Antivirus & Endpoint Security
* `MsMpEng.exe` (Microsoft Defender Antivirus)
* `NisSrv.exe` (Microsoft Network Realtime Inspection)
* `SecurityHealthService.exe`
* `avp.exe` (Kaspersky)
* `bdservicehost.exe` (Bitdefender)
* `SavService.exe` (Sophos)
* `CSFalconService.exe` (CrowdStrike)

### 2.3 GPU Drivers & Display Infrastructure
* `nvcontainer.exe`, `nvdisplay.container.exe` (NVIDIA)
* `amdfendrsr.exe`, `RadeonSoftware.exe` (AMD)
* `igfxCUIService.exe`, `igfxEM.exe` (Intel Graphics)
* `RtkAudioService64.exe` (Realtek Audio)

---

## 3. Dynamic Context Safeguards

Even if a process is not on the hardcoded whitelist, FrostByte checks contextual telemetry before classifying it as rogue:

### 3.1 Audio Session Protection (WASAPI)
* **Check:** Query `IAudioSessionControl2::GetState()` via Windows Core Audio API.
* **Rule:** If the process is currently streaming or producing audio (`AudioSessionStateActive`), **DO NOT THROTTLE OR TERMINATE**.
* **Protects:** Spotify, Discord, Zoom, Microsoft Teams, YouTube background tabs, DAW/music software.

### 3.2 Fullscreen & Gaming Guard
* **Check:** `SHQueryUserNotificationState()` returning `QUNS_RUNNING_D3D_FULL_SCREEN` or `QUNS_PRESENTATION_MODE`.
* **Rule:** If a fullscreen DirectX / Vulkan game or PowerPoint presentation is detected:
  * Silence all Toast notifications.
  * Disable automated aggressive actions.
  * Maintain gaming performance profile.

### 3.3 Active Foreground Window Exemption
* **Check:** `GetForegroundWindow()` and `GetWindowThreadProcessId()`.
* **Rule:** If the process matches the window currently in foreground focus with active user input within the last 120 seconds, it is **exempt** from rogue classification (e.g., active 3D viewport in Blender, active Premiere Pro export).

---

## 4. Graduated Intervention Hierarchy

FrostByte strictly enforces a 4-tier escalation model:

```
[Level 0: Monitor]
      │
      ▼ (Single thread 100% for > 60s AND CPU Temp > 80°C)
[Level 1: Notify User]
  • Show Windows Toast with options: [Tame] [Kill] [Ignore]
      │
      ▼ (No user response after 60s AND Temp > 88°C in Auto-Tame mode)
[Level 2: Soft Tame (Job Object / Priority)]
  • Assign to Windows Job Object with 10% CPU Hard Cap
  • Lower Process Priority to IDLE_PRIORITY_CLASS
  • Restrict Affinity to Efficiency Cores
      │
      ▼ (Temp > 92°C AND Confirmed Orphan/Zombie with 0 Window Handles)
[Level 3: Smart Power Governor]
  • Toggle Windows Power Plan PROCTHROTTLEMAX to 99% (Bypasses Turbo Boost)
  • Drops CPU temperature by 15°C–25°C immediately
      │
      ▼ (Only if User Configured "Aggressive Mode" for specific non-system binaries)
[Level 4: Terminate Process]
  • TerminateProcess() with full audit trail in history.log
```

---

## 5. False-Positive Prevention for Developers & Creators

Developers frequently run intense, multi-threaded CPU tasks (e.g., `cargo build`, `npm install`, `gcc`, `cl.exe`, Docker, ML training).

### Safeguards for Dev Workloads:
1. **Process Tree Context:** If a compiler or node process has an active, living parent terminal (`wt.exe`, `cmd.exe`, `powershell.exe`, `code.exe`), treat it as intentional compilation rather than an orphan.
2. **Multi-Thread Saturation Check:** Compilers utilize all available cores (e.g., 80%–100% total CPU across all 16 threads). Rogue loops typically saturate **only 1 single core** (e.g., 6.2% on 16 threads) for an indefinite period.
3. **Configurable Project Whitelists:** Users can whitelist entire directory trees (e.g., `C:\Local Disk (D)\.Project\*` or `C:\Users\<user>\*`).

---

## 6. Audit Logging & Undo Buffer
* Every automated action is written to `logs/audit.jsonl`:
  ```json
  {
    "timestamp": "2026-09-12T01:30:00Z",
    "pid": 32696,
    "process_name": "IDMIntegrator64.exe",
    "cpu_percent": 6.25,
    "thread_saturation": 100.0,
    "duration_seconds": 120,
    "cpu_temp_before": 93.0,
    "action_taken": "JOB_OBJECT_RATE_LIMIT_10_PERCENT",
    "cpu_temp_after": 64.5
  }
  ```
* A one-click button in the System Tray: **"Revert All Active Throttles"** immediately clears all Job Object constraints and resets power plans.
