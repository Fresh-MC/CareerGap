/// Example: Autonomous Sensing Loop
///
/// This example demonstrates how to use the autonomous sensor to continuously
/// monitor system compliance without manual intervention.
///
/// Usage:
///   cargo run --example autonomous_sensor
///
/// The sensor will:
/// 1. Load policies from policies.yaml
/// 2. Start background audit scheduler
/// 3. Run audits every hour (configurable)
/// 4. Detect and log compliance drift
/// 5. Store results in snapshot database
///
/// Press Ctrl+C to stop.

use nogap_core::policy_parser;
use nogap_core::sensor_scheduler::{SensorConfig, SensorScheduler};
use std::error::Error;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("🔍 AUTONOMOUS SENSING EXAMPLE");
    println!("=============================\n");

    // Load policies
    println!("Loading policies...");
    let policies = match policy_parser::load_policy("../nogap_core/policies.yaml") {
        Ok(p) => {
            println!("✅ Loaded {} policies\n", p.len());
            p
        }
        Err(e) => {
            eprintln!("❌ Failed to load policies: {}", e);
            eprintln!("   Make sure policies.yaml exists in nogap_core/");
            return Ok(());
        }
    };

    // Configure sensor
    let config = SensorConfig {
        enabled: true,
        interval_hours: 1, // Run every hour for demo
        max_stored_events: 50,
    };

    println!("Sensor Configuration:");
    println!("  • Enabled: {}", config.enabled);
    println!("  • Interval: {} hour(s)", config.interval_hours);
    println!("  • Max Events: {}\n", config.max_stored_events);

    // Create scheduler
    let scheduler = SensorScheduler::new(config);

    // Start sensing loop
    println!("Starting autonomous sensing loop...");
    println!("(This will run indefinitely - press Ctrl+C to stop)\n");

    match scheduler.start(policies) {
        Ok(_) => {
            println!("✅ Sensor started successfully!\n");
            println!("What's happening:");
            println!("  1. Background thread is running audits every hour");
            println!("  2. Results are stored in snapshots.db");
            println!("  3. Drift detection compares consecutive audits");
            println!("  4. All activity is logged\n");
            println!("Watch the logs below for audit cycles...\n");
            println!("─────────────────────────────────────────────────\n");

            // Keep main thread alive and monitor events
            let mut last_event_count = 0;
            loop {
                thread::sleep(Duration::from_secs(5));

                // Check for new events
                let events = scheduler.get_events();
                if events.len() > last_event_count {
                    println!("\n📊 Event Summary:");
                    for (i, event) in events.iter().skip(last_event_count).enumerate() {
                        println!("  Audit #{}: {} policies, {} passed, {} failed, {} drifted",
                            last_event_count + i + 1,
                            event.audit_count,
                            event.passed_count,
                            event.failed_count,
                            event.drift_count
                        );
                    }
                    println!();
                    last_event_count = events.len();
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to start sensor: {}", e);
            eprintln!("\nPossible causes:");
            eprintln!("  • Sensor is disabled in configuration");
            eprintln!("  • Sensor is already running in another process");
            eprintln!("  • Insufficient permissions for system audits");
        }
    }

    Ok(())
}
