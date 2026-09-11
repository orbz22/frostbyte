use colored::Colorize;
use frostbyte_core::Watchdog;
use std::io::Write;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let once_mode = args.iter().any(|arg| arg == "--once");
    let max_ticks: Option<u64> = args.iter().position(|arg| arg == "--count").and_then(|idx| {
        args.get(idx + 1).and_then(|val| val.parse().ok())
    });

    // Print banner
    println!("{}", "=======================================================".cyan());
    println!("{}", "   ❄️  FrostByte — Thermal & Process Watchdog (v0.1.0)   ".cyan().bold());
    println!("{}", "   Phase 1: Real-time Telemetry & Single-Core Loop Hunter".white());
    println!("{}", "=======================================================".cyan());
    println!("Initializing hardware sensors & process watcher...\n");

    let mut watchdog = Watchdog::with_settings(80.0, 3, 2); // 3 ticks * 2s = 6s for quick detection

    // Initial warm-up tick to establish baseline CPU counters
    println!("Sampling initial process baseline (waiting 2s)...");
    let _ = watchdog.tick();
    tokio::time::sleep(Duration::from_secs(2)).await;

    println!("Starting live monitoring loop. Press Ctrl+C to exit.\n");

    let mut tick_counter = 0u64;

    loop {
        let snapshot = watchdog.tick();
        tick_counter += 1;

        // Clear terminal screen unless running in one-shot mode
        if !once_mode {
            print!("\x1B[2J\x1B[1;1H");
            let _ = std::io::stdout().flush();
        }

        println!(
            "{} | Timestamp: {}",
            "❄️ FrostByte Guardian Active".cyan().bold(),
            snapshot.timestamp.format("%H:%M:%S").to_string().yellow()
        );
        println!("{}", "--------------------------------------------------------------------------------".bright_black());

        // 1. Hardware & Thermals Section
        let cpu_temp_str = match snapshot.thermals.cpu_package_temp {
            Some(t) if t >= 85.0 => format!("{:.1}°C", t).red().bold(),
            Some(t) if t >= 70.0 => format!("{:.1}°C", t).yellow().bold(),
            Some(t) => format!("{:.1}°C", t).green().bold(),
            None => "N/A (Non-Elevated)".bright_black(),
        };

        let gpu_temp_str = match snapshot.thermals.gpu_temp {
            Some(t) if t >= 80.0 => format!("{:.1}°C", t).red().bold(),
            Some(t) if t >= 65.0 => format!("{:.1}°C", t).yellow().bold(),
            Some(t) => format!("{:.1}°C", t).green().bold(),
            None => "N/A".bright_black(),
        };

        let gpu_power_str = match snapshot.thermals.gpu_power_w {
            Some(w) => format!("{:.1} W", w).cyan(),
            None => "N/A".bright_black(),
        };

        let power_str = if snapshot.thermals.is_ac_online {
            "Plugged In (AC Online)".green().bold()
        } else {
            "Battery Mode".yellow()
        };

        println!(
            "CPU Temp: {:<18} | GPU Temp: {:<16} | GPU Power: {}",
            cpu_temp_str, gpu_temp_str, gpu_power_str
        );
        println!(
            "Power State: {:<15} | Cores: {:<19} | Total CPU: {:.1}%",
            power_str,
            format!("{} Threads", snapshot.logical_cores).cyan(),
            snapshot.total_cpu_pct
        );
        println!("{}", "--------------------------------------------------------------------------------".bright_black());

        // 2. Rogue Alert Banner (If Any)
        if !snapshot.rogue_alerts.is_empty() {
            println!("{}", "🚨 [ALERT: RUNAWAY SINGLE-CORE LOOP DETECTED]".red().bold());
            for alert in &snapshot.rogue_alerts {
                println!(
                    "  {} Process: {} (PID: {}, PPID: {})",
                    "->".red(),
                    alert.process_name.yellow().bold(),
                    alert.pid,
                    alert.ppid
                );
                println!(
                    "     Single-Core Saturation: {} (Total CPU: {:.1}%)",
                    format!("{:.1}%", alert.single_core_saturation_pct).red().bold(),
                    alert.total_process_cpu_pct
                );
                println!(
                    "     Sustained Duration:     {} seconds",
                    alert.sustained_seconds.to_string().yellow()
                );
                println!("     Diagnosis:              {}", alert.reason.bright_yellow());
                if alert.is_orphan {
                    println!("     Status:                 {}", "ORPHAN PROCESS (Parent is dead)".red().bold());
                }
            }
            println!("{}", "--------------------------------------------------------------------------------".bright_black());
        } else {
            println!("{}", "Status: All processes normal. No runaway single-core loops detected.".green());
            println!("{}", "--------------------------------------------------------------------------------".bright_black());
        }

        // 3. Top Active Processes Table
        println!(
            "{:<8} {:<8} {:<24} {:<12} {:<16} {:<10}",
            "PID".bold(),
            "PPID".bold(),
            "Process Name".bold(),
            "Total CPU %".bold(),
            "Core Saturation".bold(),
            "Status".bold()
        );

        for p in snapshot.top_processes {
            let sat_str = if p.top_thread_saturation_pct >= 80.0 {
                format!("{:.1}% (HIGH!)", p.top_thread_saturation_pct).red().bold()
            } else if p.top_thread_saturation_pct >= 40.0 {
                format!("{:.1}%", p.top_thread_saturation_pct).yellow()
            } else {
                format!("{:.1}%", p.top_thread_saturation_pct).normal()
            };

            let status_str = if p.is_orphan {
                "Orphan".red()
            } else {
                "Normal".bright_black()
            };

            println!(
                "{:<8} {:<8} {:<24} {:<12.1} {:<25} {:<10}",
                p.pid,
                p.ppid,
                p.name.chars().take(23).collect::<String>(),
                p.total_cpu_pct,
                sat_str,
                status_str
            );
        }

        println!("\nTick: #{} | Refreshing every 2s... (Press Ctrl+C to stop)", tick_counter);

        if once_mode || max_ticks.map_or(false, |m| tick_counter >= m) {
            println!("\nCompleted requested sample ticks. Exiting.");
            break;
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    Ok(())
}
