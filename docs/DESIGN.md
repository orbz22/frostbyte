# FrostByte — UI/UX Design System Specification (DESIGN.md)

**Product:** FrostByte — Thermal Guardian & Process Watchdog  
**Version:** 0.3.0  
**Design Language:** Cyber Frost • Windows 11 Fluent 2.0 (Dark Mica / Acrylic)  
**Target Form Factor:** Desktop Utility Widget (560px × 740px, Fixed Ratio)  
**Target Platform:** Windows 10 / 11 (Tauri v2 + WebView2)  

---

## 1. Design Philosophy: "Cyber Frost"

FrostByte is designed to look like a first-party Windows 11 utility created for high-performance laptops and workstations. The design balances two core themes:
1. **Cooling & Cold Serenity:** Translucent frosted glass (*Acrylic / Dark Mica*), deep space dark grounds (`#030712`), and cold electric cyan/ice-blue accents (`#38bdf8`) signifying optimal cooling and control.
2. **Precision Engineering:** High-contrast monospace typography for raw sensor numbers, live metrics, and thread telemetry, combined with crisp border lines and soft glow lighting.

### Core UX Principles
* **Glanceable & High Contrast:** A user looking at the dashboard for 1 second must immediately know: (1) Is the CPU overheating? (2) Is the guardian active? (3) Which process is consuming single-core heat?
* **Zero Cognitive Clutter:** Avoid dense tables or intimidating system graphs. Keep telemetry organized into clear visual hierarchy cards.
* **Safe & Direct Action:** Destructive actions (Kill) are clearly differentiated in crimson red with confirmation dialogs, while non-destructive mitigations (Instant Cool, Tame) are highlighted in cyan and emerald.

---

## 2. Window Architecture & Dimensions

| Property | Value | Description |
|---|---|---|
| **Width** | `560px` | Fits comfortably alongside IDEs, code editors, or browser windows. |
| **Height** | `740px` | Optimal vertical space displaying thermals, controls, alerts, and top 10 processes without requiring excessive scrolling. |
| **Resizable** | `false` | Fixed widget layout ensures consistent layout and prevents gauge clipping. |
| **Window Frame** | Native Windows 11 rounded corners with dark title bar (`windows_subsystem = "windows"`). |
| **Padding** | `16px` outer body padding with `12px` card spacing. |

---

## 3. Color Palette & Design Tokens

### 3.1 Background & Surface Tokens
| Token | Hex / RGBA | Role / Usage |
|---|---|---|
| `--bg-base` | `#030712` | Deepest slate black root ground. |
| `--surface-acrylic` | `rgba(17, 24, 39, 0.75)` | Card background with `backdrop-filter: blur(16px)`. |
| `--surface-card-hover` | `rgba(30, 41, 59, 0.80)` | Interactive button and card hover state. |
| `--border-subtle` | `rgba(255, 255, 255, 0.08)` | Standard card and section separator border. |
| `--border-highlight` | `rgba(56, 189, 248, 0.25)` | Focused, active, or cyan-highlighted border. |

### 3.2 Functional & Semantic Accents
| Token | Hex | Role / Meaning |
|---|---|---|
| **Frost Cyan** | `#38bdf8` | Primary brand accent, Instant Cool active, cooling states, cold gauge. |
| **Emerald Green** | `#10b981` | Safe thermals (<70°C), Guardian Active, Tamed processes, Auto-Tame. |
| **Warning Amber** | `#f59e0b` | Elevated thermals (70°C–84°C), high single-core saturation (40%–79%), manual mode. |
| **Crimson Red** | `#ef4444` | Critical thermal hotspot (≥85°C), Runaway Loop Rogue alert, Kill button. |
| **Text Primary** | `#f8fafc` | Main headings, current temperatures, process names. |
| **Text Muted** | `#94a3b8` | Subtitles, unit labels, thread IDs, timestamps. |
| **Text Dim** | `#64748b` | Footer, minor hints, column table headers. |

---

## 4. Typography Hierarchy

| Element | Font Family | Size | Weight | Example |
|---|---|---|---|---|
| **App Title** | `Segoe UI Variable Display`, sans-serif | `16px` | Bold (700) | `FrostByte` |
| **Thermal Digits** | `Segoe UI Variable`, monospace numbers | `30px - 32px` | Extrabold (800) | `92.0°C` |
| **Section Labels** | `Segoe UI Variable Text`, sans-serif | `12px` | Semibold (600), Uppercase | `SMART GOVERNOR & MITIGATIONS` |
| **Telemetry & PID** | `Cascadia Code`, `Consolas`, monospace | `11px` | Medium (500) | `PID: 15084 • 13.4%` |
| **Badges & Tags** | `Segoe UI Variable Text`, monospace | `9px - 10px` | Bold (700) | `AUTO`, `99% CLAMPED`, `TAMED` |
| **Footnotes & Hints** | `Segoe UI Variable Text`, sans-serif | `10px` | Regular (400) | `Live 1.5s Refresh • Safe-by-default` |

---

## 5. Screen Breakdown & UI Components

```
┌─────────────────────────────────────────────────────────────┐
│ ❄️ FrostByte [v0.3.0]                    ● GUARDIAN ACTIVE ⚙️ │  <- Header
├──────────────────────────────┬──────────────────────────────┤
│ 🖥️ CPU PACKAGE    16 Threads │ ⚡ DISCRETE GPU    AC Online  │  <- Dual Gauges
│    92.0°C         Boost On   │    52.0°C          15.1 W    │
│    [████████████████░░░░░]   │    [████████░░░░░░░░░░░░░]   │
├──────────────────────────────┴──────────────────────────────┤
│ SMART GOVERNOR & MITIGATIONS               Revert All Tames │  <- Smart Governor
│ ┌──────────────────────────┐  ┌──────────────────────────┐  │
│ │ ❄️ Instant Cool   [AUTO] │  │ 🛡️ Auto-Tame     [ON]   │  │
│ │ Auto ≥ 88°C (99% Cap)    │  │ 10% CPU Hard Cap         │  │
│ └──────────────────────────┘  └──────────────────────────┘  │
│ 🚀 Instant Cool: Auto (88°C) • Tray: On           Configure │
├─────────────────────────────────────────────────────────────┤
│ 🚨 Runaway Single-Core Loop Detected                        │  <- Rogue Alert
│ stuck_worker.exe (PID: 15084)           [🛡️ Tame] [❌ Kill] │
│ Core Saturation: 98.5% • Duration: 12s                      │
├─────────────────────────────────────────────────────────────┤
│ ACTIVE PROCESSES  [CPU: 13.4%]               Live 1.5s Ref  │  <- Process Table
│ PROCESS              TOTAL CPU    CORE SATURATION    ACTION │
│ claude.exe (15084)      6.1%           64.5%          Tame  │
│ node.exe (3088)         2.5%           15.0%          Tame  │
│ chrome.exe (35608)      1.0%           13.1%          Tame  │
├─────────────────────────────────────────────────────────────┤
│ FrostByte open-source guardian • Safe-by-default            │  <- Footer
└─────────────────────────────────────────────────────────────┘
```

### 5.1 Top Navigation & Header
* **App Branding:** Left-aligned 36×36px gradient rounded icon (frost snowflake `❄️`) + title `FrostByte` + pill tag `v0.3.0` + subtitle `Thermal Guardian & Process Watchdog`.
* **Live Guardian Status Badge:** Green pill capsule with soft pulsing circular light: `● GUARDIAN ACTIVE`. If rogue loop is detected, switches to pulsing Red `● ROGUE DETECTED (1)`.
* **Settings Gear Button:** 32×32px circular acrylic button (`⚙️`) with subtle border and hover glow.

### 5.2 Dual Hardware Thermal Cards (2-Column Grid)
* **Left Card (CPU PACKAGE):**
  * Reads real hardware thermal zones via Native Win32 PDH (`\Thermal Zone Information(*)\High Precision Temperature`).
  * Big bold digits (`--.-°C`). Color shifts:
    * `< 70°C`: Emerald Green (`#10b981`)
    * `70°C – 84°C`: Warning Amber (`#f59e0b`)
    * `≥ 85°C`: Alert Crimson (`#ef4444`)
  * Dynamic Progress Bar: Reflects temperature percentage relative to 100°C TjMax.
  * Boost Tag: `Boost Active` (100% boost allowed) or `99% Clamped` (Turbo Boost disabled).
* **Right Card (DISCRETE GPU):**
  * Reads discrete NVIDIA GPU metrics via `nvidia-smi`.
  * Shows GPU Core Temp (`--.-°C`), Power Draw (`--.- W`), and Power Line Status (`AC Online` / `Battery`).

### 5.3 Smart Governor & Mitigations Panel
* **Instant Cool Toggle Card:**
  * Displays feature title with an interactive mode pill:
    * `[AUTO]` (Cyan): Indicates active temperature governor listening for trigger threshold.
    * `[MANUAL]` (Amber): Indicates manual-only mode without automatic temperature-based intervention.
  * Subtext: Dynamically displays `Auto ≥ 88°C (99% Cap)` or `Manual Toggle (99% Cap)`.
  * Clicking the card manually engages/disengages boost clamp (100% $\leftrightarrow$ 99%) in both modes.
* **Auto-Tame Toggle Card:**
  * When enabled (green glowing indicator), rogue threads exceeding single-core threshold for $\ge 6\text{s}$ are automatically placed in a Windows Job Object capped at 10% CPU.
* **Status Shortcut Bar:**
  * Shows concise summary: `Instant Cool: Auto (88°C) • Tray: On` with a clickable `Configure` shortcut that opens the Settings modal.

### 5.4 Dynamic Rogue Loop Alert Banner
* Appears dynamically between Governor controls and Process Table whenever a runaway thread is detected.
* Styled with an alert red translucent background and animated glow (`box-shadow: 0 0 25px -5px rgba(239, 68, 68, 0.4)`).
* Includes PID, parent process details, sustained duration, and two direct action buttons:
  * `🛡️ Tame (10%)`: Applies instant Job Object hard quota.
  * `❌ Kill`: Safely terminates after verifying it is not in the system whitelist.

### 5.5 Active Process Watchlist Table
* Shows top 10 background processes sorted by CPU usage.
* Columns:
  1. **Process:** Executable name + PID + `ORPHAN` badge if parent PID is dead.
  2. **Total CPU:** Monospace percentage across all cores.
  3. **Core Saturation:** Individual thread percentage normalized to single core (color-coded red if $\ge 80\%$, amber if $\ge 40\%$).
  4. **Action:** `Tame` button or green `TAMED` badge with undo capability.

---

## 6. Settings & Preferences Modal

The settings modal is an acrylic floating dialog with backdrop blur (`z-index: 50`) covering the dashboard:

```
┌──────────────────────────────────────────────┐
│ ⚙️ Settings & Preferences                  ✕ │
├──────────────────────────────────────────────┤
│ ❄️ Instant Cool Mode                 [Auto]  │
│    Otomatis aktif saat suhu ≥ 88°C           │
│                                              │
│ 🌡️ Auto-Cool Temp Trigger              88°C  │
│    [━━━━━━━●━━━━━━━━━━━━━━]                  │
│    75°C (Agresif)      88°C     95°C (Toleran)│
│                                              │
│ 🚀 Auto-Start on Boot              [Enabled] │
│    Start otomatis bersama Windows ke tray.   │
│                                              │
│ 🗔 Close (X) to Tray               [Enabled] │
│    Tombol 'X' minimize ke tray alih-alih exit│
│                                              │
│ 🎯 Saturation Trigger                   80%  │
│    [━━━━━━━━━━━━●━━━━━━━━━]                  │
│    60% (Sensitif)      80%      95% (Ketat)  │
├──────────────────────────────────────────────┤
│ Saved in %LOCALAPPDATA%               [Done] │
└──────────────────────────────────────────────┘
```

### Configurable Parameters
1. **Instant Cool Mode:** Toggle button switching between `Auto` (Cyan) and `Manual` (Amber). When in Manual, the temperature trigger slider is dimmed (`opacity-40 pointer-events-none`).
2. **Auto-Cool Temp Trigger Slider:** Step-based slider from $75^\circ\text{C}$ to $95^\circ\text{C}$ (step: 1°C, default: $88^\circ\text{C}$).
3. **Auto-Start on Boot:** Manages Windows registry key `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\FrostByte` with `--minimized` flag.
4. **Close (X) to Tray:** Toggles between `Enabled` (minimize to tray) and `Full Exit (Quit)` (clean process termination and clamp revert).
5. **Saturation Trigger Slider:** Adjusts single-thread threshold ($60\%$ to $95\%$, default: $80\%$).

---

## 7. Responsive State Matrix

| State | Visual Manifestation |
|---|---|
| **Normal / Healthy** | Status badge green (`GUARDIAN ACTIVE`), CPU temp green (<70°C), alert box displays green checkmark `Thermals & Threads Healthy`. |
| **Warm / Working** | CPU temp yellow (70°C–84°C), Turbo Boost On, no alerts. |
| **Thermal Clamping (Cooling)** | CPU temp $\ge 88^\circ\text{C}$ triggers Instant Cool; Boost indicator changes to `99% Clamped` (cyan glow); cooling in progress. |
| **Rogue Loop Alert** | Red banner appears with pulsing red status badge (`ROGUE DETECTED (1)`), saturation highlighted in bold red. |
| **Tamed State** | Green `TAMED (10% CAP)` badge appears on rogue banner and process table row with `Undo` button. |
| **Settings Open** | Backdrop dimming `rgba(0, 0, 0, 0.70)` with centered frosted glass card and active sliders. |

---

## 8. Export & Integration Guide (Google Stitch / Figma / v0)

When importing into **Google Stitch**, **Figma**, or frontend prototyping tools:
1. **Container:** Set canvas to `560px` width by `740px` height.
2. **Glassmorphism Layering:**
   - Base canvas: `#030712`
   - Cards: `#111827` at `75%` opacity, `backdrop-filter: blur(16px)`, border: `1px solid rgba(255, 255, 255, 0.08)`, border-radius: `16px`.
3. **Glow Shadows:** Use colored drop shadows for active elements:
   - Cyan: `0 0 20px -5px rgba(56, 189, 248, 0.3)`
   - Red: `0 0 25px -5px rgba(239, 68, 68, 0.4)`
   - Emerald: `0 0 20px -5px rgba(16, 185, 129, 0.3)`
4. **Asset Icons:** Standard Unicode / SVG minimal icons for snowflake (`❄️`), shield (`🛡️`), rocket (`🚀`), target (`🎯`), and gear (`⚙️`).
