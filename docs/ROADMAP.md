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

## Phase 0: Architecture & Foundation (Completed)
* [x] Formulate Product Requirements Document (`PRD.md`)
* [x] Design System Architecture & Win32 Integration (`ARCHITECTURE.md`)
* [x] Define Process Safety Rules & Whitelist Specifications (`SAFETY_RULES.md`)
* [x] Select Technology Stack: Rust + Tauri v2 (`TECH_STACK.md`)
* [x] Initialize Git repository and project scaffolding

---

## Phase 1: Core Engine & Telemetry POC (Completed)
**Goal:** A standalone, low-overhead CLI daemon that monitors thermals and reliably flags runaway single-core loops without crashing.

* [x] **Thermal Provider:**
  * Implement WMI / ACPI temperature queries for CPU package.
  * Implement NVIDIA NVML query for discrete GPU temperature and power draw.
  * Detect AC / Battery charging state.
* [x] **Process & Thread Sampler:**
  * Implement high-performance Win32 Toolhelp snapshots to capture process and thread kernel/user time deltas.
  * Calculate normalized single-core saturation percentage ($>85\%$ threshold) alongside total machine CPU %.
* [x] **Orphan & Loop Heuristic:**
  * Track thread CPU saturation history over a rolling window.
  * Validate parent process alive status (orphan/zombie detection).
* [x] **Safety Core v1:**
  * Implement hardcoded whitelist (critical Windows system binaries, antivirus, display drivers).
* [x] **CLI Output & Tests:**
  * Output formatted real-time status: CPU Temp, GPU Temp, Power State, Top Processes, Single-Core Saturation, and Rogue Alerts.
  * Automated unit tests for loop detection, whitelisting, and safety immunity.

---

## Phase 2: Mitigation & Governor Engine (Completed)
**Goal:** Automate cooling and non-destructive process taming.

* [x] **Job Object CPU Rate Limiter:**
  * Implement `CreateJobObjectW` and `JOBOBJECT_CPU_RATE_CONTROL_INFORMATION`.
  * Support soft-taming rogue processes to a hard 10% CPU cap.
  * Add automatic and manual revert to remove job constraints on demand and on shutdown.
* [x] **Smart Thermal Power Governor:**
  * Implement Windows Power Scheme API via `powrprof` / `powercfg`.
  * Auto-adjust `PROCTHROTTLEMAX` (100% $\to$ 99%) when CPU Package Temp $>90^\circ\text{C}$ on AC power.
  * Implement hysteresis cool-down (restore to 100% once Temp $\le 65^\circ\text{C}$ for stable duration).
  * One-click CLI controls (`--cool` and `--boost-on`).
* [x] **Safe Process Termination:**
  * Implement `ProcessActionController` with strict `SafetyEngine` immunity checks.
* [x] **CLI Mitigation Integration:**
  * Added `--auto-tame` flag for autonomous background taming of detected runaway loops.
  * Live status display for Turbo Boost mode and currently tamed PIDs.

---

## Phase 3: Tauri v2 System Tray & Modern UI (Completed)
**Goal:** A clean, lightweight system tray companion that feels like a native Windows 11 feature.

* [x] **Tauri v2 Desktop Shell:**
  * Initialize Tauri v2 project structure with Rust backend and Fluent HTML5 frontend.
* [x] **System Tray Integration:**
  * Native Windows 11 System Tray icon with quick actions (Show Dashboard, Instant Cool, Boost Restore, Revert All).
  * System tray left-click to toggle window focus and visibility.
* [x] **Flyout / Dashboard Window:**
  * Clean dark-mode Windows 11 Fluent aesthetic (Acrylic backdrop with Tailwind CSS).
  * Real-time hardware meters (CPU Package Temp, Discrete GPU Temp, Power Draw, Battery/AC).
  * Runaway Process Watchdog Radar banner with live single-core saturation indicators.
  * Interactive process management (Instant Soft-Tame 10% Cap, Undo/Revert, Safe Kill).
  * Dynamic event streaming via Tauri v2 background thread every 2 seconds.

---

## Phase 4: Open Source, CI/CD & Community Launch (Completed)
**Goal:** Public GitHub release with automated releases, clean installer, and community support.

* [x] **Documentation & Community Assets:**
  * High-quality `README.md` with problem explanation, features, and usage instructions.
  * Contributing guide (`CONTRIBUTING.md`), Code of Conduct (`CODE_OF_CONDUCT.md`), and Issue Templates (`bug_report.md`, `feature_request.md`).
* [x] **Automated GitHub Actions CI/CD:**
  * Automated testing and formatting workflow (`.github/workflows/ci.yml`).
  * Automated release workflow producing standalone portable zip and checksums (`.github/workflows/release.yml`).
* [x] **Package Distribution:**
  * Windows Package Manager manifest (`winget/frostbyte.yaml`).
