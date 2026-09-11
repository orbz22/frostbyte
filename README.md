<p align="center">
  <h1 align="center">❄️ FrostByte</h1>
  <p align="center">
    <strong>Smart, Lightweight Thermal Guardian & Runaway Process Watchdog for Windows</strong>
  </p>
  <p align="center">
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
    <img src="https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078D6.svg?logo=windows" alt="Platform Windows">
    <img src="https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri%20v2-orange.svg?logo=rust" alt="Rust + Tauri">
    <img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" alt="PRs Welcome">
  </p>
</p>

---

## 💡 The Problem FrostByte Solves

Have you ever noticed your laptop fan screaming like a jet engine and your CPU hitting **90°C – 95°C+**, yet when you open Task Manager, CPU usage looks tiny—**only 4% to 6%**?

Here is what is actually happening behind the scenes:
1. **The Hidden Loop:** A background process (e.g., an orphaned `node.exe`, a hung updater, or a stuck extension helper) gets trapped in an infinite loop, pegging **one single thread at 100%**.
2. **The Deception:** On an 8-core / 16-thread CPU (like an Intel Core i7 or AMD Ryzen 7), 1 saturated thread is only `1 / 16 ≈ 6.25%` of total CPU. It looks completely harmless in standard Task Manager graphs.
3. **The Heat Explosion:** The Windows power scheduler sees a thread demanding maximum throughput and triggers **Intel Turbo Boost / AMD Precision Boost**, pinning that core to maximum clock speed (e.g., 4.2+ GHz) at peak voltage.
4. **Thermal Throttling:** On modern laptops with compact silicon dies and shared heatpipes, this concentrated hotspot drives temperatures straight to **93°C+**, draining your battery and wearing out hardware while you thought your laptop was idle.

**FrostByte was created to solve this exact problem automatically.**

---

## ✨ Key Features

* 🔍 **Single-Core Saturation Detector:** Analyzes per-thread Win32 kernel/user time deltas to detect runaway loops ($>85\%$ saturation) hiding behind low overall CPU percentages.
* 🧟 **Orphan & Zombie Process Hunter:** Detects background worker tasks whose parent processes have died but continue burning CPU cycles in the background.
* 🛡️ **Non-Destructive Soft-Taming:** Instead of blindly killing processes and causing data loss, FrostByte gently restricts rogue tasks to a **10% CPU Hard Cap** and sets them to `IDLE_PRIORITY_CLASS` using native Windows Job Objects (`JOBOBJECT_CPU_RATE_CONTROL_INFORMATION`).
* ⚡ **Smart Thermal Governor:** Temporarily clamps Windows Power Management (`PROCTHROTTLEMAX` 99%) when CPU package temperatures cross emergency thresholds (> 90°C), dropping temperatures by **15°C – 25°C in seconds** without needing a reboot.
* 🔒 **Bulletproof Safety Core:**
  * **Immune Whitelist:** Hardcoded protection for Windows core processes (`csrss.exe`, `explorer.exe`, `dwm.exe`, `lsass.exe`, `audiodg.exe`, Windows Defender, etc.).
  * **Audio Protection:** Exempts apps actively playing audio via WASAPI (Spotify, YouTube, Zoom, Discord).
  * **Interactive Reversion:** Revert throttles at any time or automatically when FrostByte exits.
* 🪶 **Zero Resource Overhead:** Written in pure **Rust + Tauri v2**. Idles at **< 20 MB RAM** and **< 0.1% CPU**.

---

## 🖥️ User Interface & Dashboard

FrostByte offers two high-performance interfaces:

### 1. Modern Windows 11 Desktop UI (`frostbyte-app`)
* **Live Hardware Gauges:** Real-time CPU Package & GPU thermals, power draw, and thread clock state.
* **Instant Cool Toggle:** Clamps Turbo Boost to 99% with a single click, cooling your laptop immediately.
* **Autonomous Auto-Tame:** Background watchdog that immediately tames rogue single-core loops.
* **Active Tamed Manager:** See which processes are currently limited, with one-click undo or safe termination.
* **System Tray Companion:** Minimizes cleanly to the Windows system tray with quick action toggles.

### 2. High-Performance Terminal Dashboard (`frostbyte-cli`)
* High-visibility colorized status tables.
* Live continuous monitoring mode (`--count N` or streaming).
* Fast scriptable command-line flags (`--cool`, `--boost-on`, `--tame <PID>`, `--auto-tame`).

---

## 🚀 Quick Start & Usage

### Running the Desktop App

```powershell
# Run the Tauri v2 Desktop GUI
cargo run -p frostbyte-app --release
```

### Running the CLI Dashboard

```powershell
# Run one diagnostic snapshot
cargo run -p frostbyte-cli -- --once

# Run continuous monitoring with 2-second refresh
cargo run -p frostbyte-cli

# Immediately drop heat by clamping Turbo Boost to 99%
cargo run -p frostbyte-cli -- --cool

# Restore full Turbo Boost performance (100%)
cargo run -p frostbyte-cli -- --boost-on

# Soft-tame a specific runaway process by PID to 10% CPU
cargo run -p frostbyte-cli -- --tame 12345

# Run continuous watchdog with autonomous loop taming
cargo run -p frostbyte-cli -- --auto-tame
```

---

## 🏗️ Workspace Architecture

The FrostByte codebase is organized as an enterprise-grade Rust workspace:

```
frostbyte/
├── crates/
│   ├── frostbyte-core/      # Telemetry, Heuristics, Windows Job Objects, Power Governor
│   │   ├── src/
│   │   │   ├── governor/    # Windows Power Scheme PROCTHROTTLEMAX (100% <-> 99%)
│   │   │   ├── heuristics/  # Single-core saturation & orphan loop detector
│   │   │   ├── mitigation/  # Windows Job Object CPU rate limiting (10% hard cap)
│   │   │   ├── process/     # Win32 Toolhelp32 process & thread delta sampling
│   │   │   ├── safety/      # Immutable system whitelist & immunity validator
│   │   │   ├── thermal/     # WMI / ACPI / NVML temperature telemetry
│   │   │   └── types.rs     # Shared domain data models
│   ├── frostbyte-cli/       # Lightweight terminal monitoring dashboard
│   └── frostbyte-app/       # Tauri v2 Windows 11 system tray & desktop application
│       ├── ui/              # Modern Tailwind CSS / Fluent dark-mode frontend
│       └── src/main.rs      # Native system tray and IPC command bindings
└── docs/                    # Complete PRD, Architecture, Safety Rules, & Roadmap
```

---

## 📂 Documentation

Comprehensive technical specifications are available in [`docs/`](docs/):

* 📄 [**Product Requirements Document (PRD)**](docs/PRD.md)
* 🏗️ [**System Architecture & Design**](docs/ARCHITECTURE.md)
* 🛡️ [**Safety Rules & Immunity Specifications**](docs/SAFETY_RULES.md)
* ⚙️ [**Technology Stack Evaluation**](docs/TECH_STACK.md)
* 🗺️ [**Product Roadmap & Milestones**](docs/ROADMAP.md)

---

## 🤝 Contributing

Contributions, issues, and feature requests are welcome!
Please read our [Contributing Guide](CONTRIBUTING.md) and [Code of Conduct](CODE_OF_CONDUCT.md).

---

## 📜 License

Distributed under the **MIT License**. See [`LICENSE`](LICENSE) for details.
