# Technology Stack & Implementation Choices — FrostByte

**Document Version:** 1.0.0  
**Target:** Production-Grade Open-Source Desktop Utility  

---

## 1. Evaluation Criteria & Constraints

A background system guardian has strict non-functional requirements:
1. **Ultra-Low Memory Footprint:** Must idle at `< 25 MB RAM`. A tool designed to optimize resource usage cannot itself be a resource hog.
2. **Sub-Millisecond CPU Overhead:** Sampling processes and thermals must consume `< 0.2% CPU` average.
3. **Deep Win32 OS Integration:** Needs low-level access to:
   * Windows Job Objects (`CreateJobObjectW`, `SetInformationJobObject`)
   * Power Scheme APIs (`powrprof.dll`, `PowerWriteACValueIndex`)
   * Native Process Snapshots (`NtQuerySystemInformation` / Toolhelp32)
   * Core Audio WASAPI (`IAudioSessionManager2`)
   * Shell Notification State (`SHQueryUserNotificationState`)
4. **Modern, Responsive UI:** Sleek, dark-mode native Windows 11 Fluent aesthetic with real-time graphs and tray flyouts.

---

## 2. Comparison of Evaluated Stacks

| Metric / Stack | **Rust + Tauri v2 (Selected)** | **C# .NET 8 + WinUI 3** | **C++20 + WinUI / ImGui** | **Electron + Node.js** |
| :--- | :--- | :--- | :--- | :--- |
| **Idle RAM** | **~15 – 22 MB** | ~65 – 110 MB | **~8 – 15 MB** | ~180 – 300 MB |
| **CPU Overhead** | **< 0.1%** | ~0.3% – 0.5% | **< 0.1%** | ~0.8% – 1.5% |
| **Win32 API Access** | Native FFI via `windows-rs` | P/Invoke / CSWin32 | Direct C++ headers | C++ Addon (N-API) required |
| **UI Flexibility** | High (Webview2 + Tailwind) | High (XAML / WinUI) | Low to Medium | High (HTML/CSS) |
| **Safety & Stability** | Memory safe, no GC spikes | Safe, but GC pauses | Manual memory management | Memory heavy, V8 GC pauses |
| **Binary Size** | **~8 – 12 MB** | ~35 – 60 MB | **~5 – 8 MB** | ~80 – 120 MB |

---

## 3. Selected Stack Architecture

### 3.1 Backend Core Engine: **Rust**
* **Why Rust?**
  * **Zero-Cost Abstractions & No Garbage Collector:** Unlike C# or Java, Rust has zero GC pauses, ensuring deterministic tick timing for the thermal watchdog.
  * **First-Class Windows Support:** Microsoft actively develops and maintains the `windows` crate (`windows-rs`), providing complete, safe, typed access to the entire Win32, COM, and Windows Runtime (WinRT) APIs.
  * **Memory Safety Without Overhead:** Eliminates buffer overflows, dangling pointers, and race conditions in multi-threaded background workers.

### 3.2 Frontend & GUI: **Tauri v2 + Svelte / React + Tailwind CSS**
* **Why Tauri v2?**
  * Uses the existing Windows 11 native **Microsoft Edge WebView2** runtime, resulting in an installer under 10 MB and minuscule memory consumption compared to Electron.
  * Built-in support for native System Tray icons, global shortcuts, and native Windows notifications.
  * Can run completely "headless" (icon in tray only), initializing the GUI window only when the user clicks the icon.

---

## 4. Key Libraries & Crates Breakdown

### 4.1 Windows Low-Level & System
* **`windows` crate (Microsoft):**
  * Features: `Win32_Foundation`, `Win32_System_Threading`, `Win32_System_JobObjects`, `Win32_System_Power`, `Win32_System_Diagnostics_ToolHelp`, `Win32_Media_Audio`.
* **`ntapi` / native bindings:**
  * Direct fast access to `NtQuerySystemInformation(SystemProcessInformation)` for 0.05ms full-process snapshots.
* **`nvml-wrapper`:**
  * Direct query to NVIDIA GPU thermal and power sensors.

### 4.2 Application & State Management
* **`tauri` (v2.x):** Application framework, window management, and native system tray.
* **`tokio`:** Asynchronous runtime for non-blocking I/O, timers, and background thread execution.
* **`serde` & `serde_json`:** Serialization of configuration (`settings.json`) and audit logs (`audit.jsonl`).
* **`tracing` & `tracing-appender`:** Low-overhead structured logging.

### 4.3 Frontend UI (Tray Flyout & Dashboard)
* **Framework:** Svelte 5 or React with Vite (Fast, minimal bundle size).
* **Styling:** Tailwind CSS + Radix/shadcn-style design tokens tuned to Windows 11 Fluent UI guidelines (Mica / Acrylic backdrop blur effects).
* **Charts:** Lightweight Canvas / SVG sparklines for real-time CPU/GPU temperature history.

---

## 5. Development Prerequisites

To build and contribute to FrostByte:
* **Operating System:** Windows 10 (Build 19041+) or Windows 11
* **Compiler:** Rust toolchain (`rustup install stable`, MSVC toolchain: `x86_64-pc-windows-msvc`)
* **Build Tools:** Visual Studio 2022 C++ Build Tools (with Windows 10/11 SDK)
* **Frontend Runtime:** Node.js (v18+) or Bun / pnpm
* **WebView2:** Pre-installed on Windows 11 and modern Windows 10
