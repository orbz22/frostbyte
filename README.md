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
1. **The Hidden Loop:** A background process (e.g., an orphaned `node.exe`, a hung integration binary, or a stuck browser helper) gets trapped in an infinite loop, pegging **one single thread at 100%**.
2. **The Deception:** On an 8-core / 16-thread CPU, 1 saturated thread is only `1 / 16 ≈ 6.25%` of total CPU. It looks completely harmless in standard Task Manager graphs.
3. **The Heat Explosion:** The Windows power scheduler sees a thread demanding maximum throughput and triggers **Intel Turbo Boost / AMD Precision Boost**, pinning that core to maximum clock speed (e.g., 4.2+ GHz) at high voltage.
4. **Thermal Throttling:** On modern laptops with compact dies and shared heatpipes, this concentrated hotspot drives temperatures straight to **93°C+**, draining your battery and wearing out hardware while you thought your laptop was idle.

**FrostByte was created to solve this exact problem automatically.**

---

## ✨ Key Features

* 🔍 **Single-Core Saturation Detector:** Analyzes per-thread CPU time deltas to immediately detect runaway loops hiding behind low overall CPU percentages.
* 🧟 **Orphan & Zombie Process Hunter:** Detects background worker tasks whose parent processes have died but continue burning CPU cycles in the background.
* 🛡️ **Non-Destructive Soft-Taming:** Instead of blindly killing processes and causing data loss, FrostByte can gently restrict rogue tasks to a **10% CPU Hard Cap** using native Windows Job Objects, or move them to Efficiency Cores.
* ⚡ **Smart Thermal Governor:** Temporarily toggles Windows Power Management (`PROCTHROTTLEMAX` 99%) when CPU package temperatures cross emergency thresholds (> 90°C), dropping temperatures by **15°C – 25°C in seconds** without needing a reboot.
* 🔒 **Bulletproof Safety Core:**
  * **Immune Whitelist:** Hardcoded protection for Windows core processes (`csrss.exe`, `explorer.exe`, `dwm.exe`, antivirus suites, etc.).
  * **Audio Protection:** Automatically exempts any app actively playing audio via WASAPI (Spotify, YouTube, Zoom, Discord).
  * **Fullscreen Game Guard:** Automatically silences alerts and pauses interventions during active Direct3D/Vulkan fullscreen games.
* 🪶 **Zero Resource Overhead:** Written in **Rust + Tauri v2**. Idles at **< 20 MB RAM** and **< 0.1% CPU**.

---

## 📂 Documentation & Architecture

Comprehensive technical specifications and planning documents are available in the [`docs/`](docs/) directory:

* 📄 [**Product Requirement Document (PRD)**](docs/PRD.md) — Comprehensive functional requirements, user personas, and success metrics.
* 🏗️ [**System Architecture & Design**](docs/ARCHITECTURE.md) — Subsystem breakdowns, Win32 APIs, heuristic formulas, and data flow.
* 🛡️ [**Safety Rules & Whitelisting**](docs/SAFETY_RULES.md) — Non-negotiable safety boundaries, false-positive prevention, and the 4-tier intervention hierarchy.
* ⚙️ [**Technology Stack Choices**](docs/TECH_STACK.md) — Deep dive into why Rust + Tauri v2 was selected over C# and C++.
* 🗺️ [**Product Roadmap & Milestones**](docs/ROADMAP.md) — Phase 0 to Phase 4 development plan.

---

## 🛠️ Tech Stack

* **Core Daemon & Telemetry:** [Rust](https://www.rust-lang.org/) (via `windows-rs`, `ntapi`, `nvml-wrapper`)
* **Desktop Shell & System Tray:** [Tauri v2](https://v2.tauri.app/)
* **Frontend UI:** Modern Tailwind CSS + Windows 11 Fluent Design tokens
* **Windows APIs:** Windows Job Objects, `NtQuerySystemInformation`, Core Audio WASAPI, Power Management (`powrprof.dll`)

---

## 🚀 Development Quick Start (Coming in Phase 1)

### Prerequisites
* Windows 10 (Build 19041+) or Windows 11
* [Rust Toolchain](https://rustup.rs/) (`x86_64-pc-windows-msvc`)
* [Visual Studio 2022 C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
* [Node.js](https://nodejs.org/) (v18+) or [pnpm](https://pnpm.io/)

```bash
# Clone the repository
git clone https://github.com/your-username/frostbyte.git
cd frostbyte

# Run in development mode (Phase 1+)
cargo run
```

---

## 🤝 Contributing

Contributions, issues, and feature requests are welcome!  
FrostByte is a community-driven open-source project. Check out the [Product Roadmap](docs/ROADMAP.md) to see what we're currently building.

---

## 📜 License

Distributed under the **MIT License**. See [`LICENSE`](LICENSE) for more information.
