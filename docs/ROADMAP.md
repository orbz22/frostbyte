# Product Roadmap & Milestones — FrostByte

**Project:** FrostByte  
**License:** Open Source (MIT)  
**Status:** Phase 0 (Planning & Architecture Completed)  

---

## Roadmap Overview

```
[Phase 0: Architecture & Setup]  <-- Current Stage
           │
           ▼
[Phase 1: Headless Engine POC]  (CLI Telemetry, Rogue Thread Detection, Hardcoded Whitelist)
           │
           ▼
[Phase 2: Mitigation Engine]    (Job Object Throttling, Power Governor 99%, Toast Alerts)
           │
           ▼
[Phase 3: Native Tray & UI]     (Tauri v2 System Tray, Dynamic Temp Badge, Dashboard Flyout)
           │
           ▼
[Phase 4: Open-Source Launch]   (GitHub CI/CD, Automated Builds, Winget Package, v1.0.0 Release)
```

---

## Phase 0: Architecture & Foundation (Current)
* [x] Formulate Product Requirements Document (`PRD.md`)
* [x] Design System Architecture & Win32 Integration (`ARCHITECTURE.md`)
* [x] Define Process Safety Rules & Whitelist Specifications (`SAFETY_RULES.md`)
* [x] Select Technology Stack: Rust + Tauri v2 (`TECH_STACK.md`)
* [ ] Initialize Git repository and project scaffolding

---

## Phase 1: Core Engine & Telemetry POC (CLI Milestone)
**Goal:** A standalone, low-overhead CLI daemon that monitors thermals and reliably flags runaway single-core loops without crashing.

* [ ] **Thermal Provider:**
  * Implement WMI / ACPI temperature queries for CPU package.
  * Implement NVIDIA NVML query for discrete GPU temperature and power draw.
* [ ] **Process & Thread Sampler:**
  * Implement `NtQuerySystemInformation` to capture process and thread kernel/user time deltas.
  * Calculate normalized single-core saturation percentage ($>85\%$ threshold).
* [ ] **Orphan & Loop Heuristic:**
  * Track thread CPU saturation history over a rolling window (30s – 60s).
  * Validate parent process alive status (orphan detection).
* [ ] **Safety Core v1:**
  * Implement hardcoded whitelist (critical Windows system binaries, antivirus, display drivers).
* [ ] **CLI Output:**
  * Output formatted real-time status: CPU Temp, GPU Temp, Active High-Load Threads, Rogue Alerts.

---

## Phase 2: Mitigation & Governor Engine (Action Milestone)
**Goal:** Automate cooling and non-destructive process taming.

* [ ] **Job Object CPU Rate Limiter:**
  * Implement `CreateJobObjectW` and `JOBOBJECT_CPU_RATE_CONTROL_INFORMATION`.
  * Support soft-taming rogue processes to a hard 10% CPU cap.
  * Add one-click revert to remove job constraints.
* [ ] **Smart Thermal Power Governor:**
  * Implement Windows Power Scheme API via `powrprof.dll`.
  * Auto-adjust `PROCTHROTTLEMAX` (100% $\to$ 99%) when CPU Package Temp $>90^\circ\text{C}$ on AC power.
  * Implement hysteresis cool-down (restore to 100% once Temp $\le 68^\circ\text{C}$ for 15s).
* [ ] **Context Safeguards:**
  * Windows Core Audio API (WASAPI) check to exempt active audio streams.
  * `SHQueryUserNotificationState` to exempt fullscreen games/presentations.
* [ ] **Native Windows Toast Notifications:**
  * Push interactive toast when a rogue process is flagged with buttons: `[Tame (10% Cap)]`, `[Kill]`, `[Whitelist]`.

---

## Phase 3: Tauri v2 System Tray & Modern UI (User Experience)
**Goal:** A clean, lightweight system tray companion that feels like a native Windows 11 feature.

* [ ] **Tauri v2 Desktop Shell:**
  * Initialize Tauri v2 project structure with Rust backend and Vite frontend.
* [ ] **System Tray Integration:**
  * Dynamic Tray Icon with real-time temperature readout (e.g., "52°", "91°").
  * Tray Menu: Instant Cool Down toggle, Mode selector (Balanced / Silent / Gaming), Exit.
* [ ] **Flyout / Dashboard Window:**
  * Clean dark-mode Windows 11 Fluent aesthetic (Mica / Acrylic backdrop).
  * Real-time thermal sparkline graph (last 5 minutes).
  * Active Rogue Processes card with one-click actions.
  * Whitelist management tab.
  * Audit history log viewer.

---

## Phase 4: Open Source, CI/CD & Community Launch
**Goal:** Public GitHub release with automated releases, clean installer, and community support.

* [ ] **Documentation & Community Assets:**
  * High-quality `README.md` with screenshots and GIFs.
  * Contributing guide (`CONTRIBUTING.md`), Code of Conduct, and Issue Templates.
* [ ] **Automated GitHub Actions CI/CD:**
  * Automated testing and formatting checks (`cargo test`, `cargo clippy`).
  * Release pipeline producing signed `.msi`, `.exe` installer, and standalone portable zip.
  * Support for both `x86_64` (Intel/AMD) and `aarch64` (Snapdragon X Elite / Windows on ARM).
* [ ] **Package Distribution:**
  * Submit package to Windows Package Manager (`winget install FrostByte`).
  * Submit to Scoop / Chocolatey.
