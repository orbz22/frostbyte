# Product Requirement Document (PRD) — FrostByte

**Project Name:** FrostByte  
**Tagline:** Smart, Lightweight Thermal Guardian & Runaway Process Watchdog for Windows  
**License:** Open Source (MIT License)  
**Target Platform:** Windows 10 & Windows 11 (x64 / ARM64)  
**Document Version:** 1.0.0  
**Status:** In Review / Ready for Implementation  

---

## 1. Problem Statement & Background

### 1.1 The Real-World Scenario
Modern multi-core CPUs (e.g., Intel 11th–14th Gen, AMD Ryzen 5000–8000 series) pack 8 to 24+ logical cores into small silicon dies. On these architectures:
* A single runaway background thread (e.g., an orphaned `node.exe`, a buggy browser helper, or a hung integration binary like `IDMIntegrator64.exe`) occupies **100% of one core**.
* In Windows Task Manager, this looks harmless: **only 4% – 6% total CPU usage**.
* However, Windows Power Scheduler sees a thread demanding maximum throughput and triggers **Intel Turbo Boost / AMD Precision Boost**, driving that core to maximum clock speed (e.g., 4.2 – 5.0 GHz) with peak voltage.
* In compact laptops with shared heatpipes, this concentrated heat density (*thermal hotspot*) causes CPU package temperatures to spike instantly to **90°C – 95°C+**, triggering loud fan noise, thermal throttling, and battery drain—even while the laptop is seemingly "idle".

### 1.2 Current Tool Limitations
* **Monitoring Tools (HWiNFO, Core Temp, HWMonitor):** Strictly passive. They display the heat but take zero corrective action.
* **Heavy Utility Suites (Process Lasso):** Overly complex, steep learning curve for everyday users, proprietary / closed source, and intimidating UI.
* **Default Windows Task Manager:** Doesn't explain *why* heat is spiking, offers no automated intervention, and hides single-core saturation behind averaged percentages.

---

## 2. Product Vision & Value Proposition

**FrostByte** is an automated, lightweight, "set-and-forget" open-source desktop guardian that bridges the gap between hardware thermals and process management.

### Key Value Pillars:
1. **Automated Runaway Detection:** Detects runaway single-thread loops and orphaned processes that hide behind low total CPU usage.
2. **Graduated Mitigation:** Doesn't just blindly kill tasks. Offers gentle graduated remedies: Notification → CPU Throttling (Affinity/Priority/Job Quota) → Safe Termination.
3. **Smart Thermal Governor:** Temporarily tames aggressive Turbo Boost when temperatures cross critical thresholds and the system is not running active foreground games/workloads.
4. **Zero Overhead:** Consumes `< 25 MB RAM` and `< 0.2% CPU` while running in the system tray.
5. **Absolute Safety First:** Bulletproof whitelists prevent touching critical Windows subsystems, drivers, audio pipelines, or active foreground games.

---

## 3. Target Audience & Personas

1. **Laptop Users & Casual Gamers:** Experience sudden heat spikes and jet-engine fan noise on their laps or desks; need an automated solution without touching BIOS or complex power settings.
2. **Software Developers & Engineers:** Frequently suffer from orphaned build processes, stuck dev servers (`node`, `python`, `cargo`, `docker-proxy`) that burn background CPU for hours unnoticed.
3. **System Administrators & Power Users:** Want a transparent, open-source tool with clean logs and customizable YAML/JSON rulesets.

---

## 4. Functional Requirements

### 4.1 Real-Time Thermal & CPU Monitoring (Module: `MonitorEngine`)
* **FR-1.1:** Read real-time CPU package temperature, maximum core temperature, and GPU temperature at configurable intervals (Default: 2s normal, 1s under warning state).
* **FR-1.2:** Monitor per-process and per-thread CPU consumption using native Windows APIs (`NtQuerySystemInformation`, PDH counters, or ETW).
* **FR-1.3:** Calculate both **Total CPU %** and **Normalized Single-Core Saturation %** (e.g., detecting if a single thread is maxing out 1 core for sustained periods).
* **FR-1.4:** Monitor CPU frequency relative to base clock (detecting aggressive Turbo Boost states).

### 4.2 Runaway Loop & Orphan Process Detection (Module: `WatchdogEngine`)
* **FR-2.1 (Sustained Single-Core Saturation):** Flag any background process where a single thread remains near 100% saturation for longer than $T_{threshold}$ (Default: 60 seconds).
* **FR-2.2 (Orphaned Process Detection):** Identify background worker processes whose parent process has terminated (e.g., CLI tools, child node runners, build tools) that continue burning CPU cycles without a parent.
* **FR-2.3 (Background vs. Foreground Disambiguation):** Differentiate active foreground user windows (e.g., video editing export, 3D rendering, gaming) from hidden background tasks. Foreground processes are exempt from automatic aggressive actions.
* **FR-2.4 (Cumulative Time Anomalies):** Detect processes with abnormally high cumulative CPU time relative to process lifespan without window focus.

### 4.3 Graduated Mitigation & Action System (Module: `MitigationEngine`)
When a rogue process is identified, FrostByte applies one of three user-configurable modes:

#### Mode A: Interactive (Notify First - Default)
* Sends a native Windows notification (Toast) with actionable buttons:
  * **[Cool Down / Throttle]** (Limit to 1 core, reduce priority to Idle)
  * **[Kill Process]** (Safe termination)
  * **[Ignore / Whitelist for Session]**
  * **[Always Ignore this Process]**

#### Mode B: Auto-Tame (Balanced)
* Automatically lowers process priority to `IDLE_PRIORITY_CLASS`.
* Restricts CPU Affinity to 1 or 2 efficiency cores (or lowest cores).
* Applies Windows Job Object CPU Rate Limiting (e.g., max 10% CPU allocation).
* Notifies user of the action taken with an option to revert.

#### Mode C: Aggressive / Eco-Mode (Auto-Kill for known non-essential binaries)
* Automatically terminates verified non-critical rogue processes if they exceed sustained threshold and CPU package temperature exceeds $90^\circ\text{C}$.

### 4.4 Smart Thermal Governor (Module: `GovernorEngine`)
* **FR-4.1:** Dynamic Boost Management: Automatically adjust Windows Processor Performance Boost or `PROCTHROTTLEMAX` (100% $\to$ 99%) when CPU temperature crosses an emergency limit (e.g., $92^\circ\text{C}$) while no fullscreen game/benchmark is active.
* **FR-4.2:** Restore normal boost mode once CPU temperature cools down below safe hysteresis threshold (e.g., $\le 68^\circ\text{C}$ for 15 seconds).
* **FR-4.3:** Profile Presets:
  * **Silent / Cool:** Strict thermal caps, boost limited during idle/work tasks.
  * **Balanced (Recommended):** Dynamic intervention only when rogue processes trigger hotspots.
  * **Performance / Gaming:** Whitelist active games; relax thermal limits during intentional heavy loads.

### 4.5 Whitelist & Safety Core (Module: `SafetyEngine`)
* **FR-5.1 (Immune System Processes):** Hardcoded, tamper-proof blacklist-from-killing containing essential Windows components (`csrss.exe`, `services.exe`, `lsass.exe`, `smss.exe`, `explorer.exe`, `dwm.exe`, `audiodg.exe`, antivirus software).
* **FR-5.2 (Audio Stream Protection):** Never kill or throttle processes currently producing audio (via `WASAPI` / Windows Core Audio API sessions).
* **FR-5.3 (Fullscreen / Game Guard):** Automatically detect active Direct3D/Vulkan fullscreen applications and suspend background interventions.
* **FR-5.4 (User Whitelist):** Allow users to add any application or folder path to a permanent whitelist via UI or config file (`whitelist.json`).

### 4.6 User Interface & Experience (Module: `UI / Tray`)
* **FR-6.1:** System Tray Icon with real-time temperature badge or dynamic color (Green: < 60°C, Yellow: 60–80°C, Red: > 85°C).
* **FR-6.2:** Quick Tray Menu:
  * One-Click "Instant Cool Down" (Throttles background hogs & clamps boost for 3 minutes).
  * Mode Switcher: Balanced / Gaming / Silent.
  * Recent Mitigations list.
* **FR-6.3:** Main Dashboard:
  * Real-time mini CPU & GPU temperature sparkline.
  * Top offending processes list with 1-click actions.
  * Audit log of past prevented runaway loops and thermal events.
  * Settings & Whitelist editor.

---

## 5. Non-Functional Requirements

### 5.1 Performance & Resource Footprint
* **Idle Memory:** $\le 25 \text{ MB}$ RAM.
* **Idle CPU Overhead:** $\le 0.2\%$ average across 1 minute on modern quad-core or higher.
* **Binary Size:** Self-contained installer $\le 15 \text{ MB}$.

### 5.2 Reliability & Fault Tolerance
* **Fail-Safe Operation:** If FrostByte encounters an unhandled exception or terminates unexpectedly, all process throttling or power plan modifications must revert back to Windows default states automatically.
* **Zero BSOD Guarantee:** Never inject kernel drivers or perform unsafe memory writes; rely strictly on stable user-mode Win32 and WMI/Pdh APIs.

### 5.3 Privacy & Security
* **Zero Telemetry by Default:** 100% offline functionality. No telemetry or process names are transmitted outside the local machine.
* **Standard User Compatibility:** Most monitoring and user-process management features run without Administrator privileges. Elevation is requested cleanly only for power plan switching or hardware sensor access if needed.

---

## 6. Success Metrics (KPIs)
1. **Thermal Drop Verification:** Able to reduce CPU temperature by $\ge 20^\circ\text{C}$ within 120 seconds of identifying and mitigating a runaway process.
2. **Zero False-Positive Terminations:** 0 critical system processes or active user work lost in automated tests.
3. **Community Adoption:** Clear GitHub documentation, easy 1-click releases, and high star-to-fork ratio on GitHub.
