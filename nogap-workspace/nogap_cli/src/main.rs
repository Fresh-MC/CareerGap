use anyhow::Result;
/// NoGap CLI - Operator Cockpit Terminal UI
///
/// Provides TUI, audit, and remediate commands for the NoGap security platform.
use clap::{Parser, Subcommand};
use nogap_cli::ui;
use serde::Serialize;
use chrono::Utc;
use nogap_core::{policy_parser, engine};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;

#[derive(Serialize)]
struct CliReport {
    timestamp: String,
    compliance_score: f32,
    results: Vec<nogap_core::engine::AuditResult>,
}

#[derive(Parser)]
#[command(name = "nogap-cli")]
#[command(about = "NoGap Security Platform - Operator Cockpit", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the interactive TUI operator cockpit
    Tui {
        /// Path to policies YAML file
        #[arg(short, long, default_value = "policies.yaml")]
        policies: String,
    },
    /// Run audit on policies (non-interactive)
    Audit {
        /// Path to policies YAML file
        #[arg(short, long, default_value = "policies.yaml")]
        policies: String,
        /// Filter by policy ID
        #[arg(short, long)]
        filter: Option<String>,
        /// Output as JSON (headless mode)
        #[arg(long)]
        json: bool,
        /// Export CSV report to specified path
        #[arg(long)]
        export_csv: Option<String>,
    },
    /// Run remediation on policies (non-interactive)
    Remediate {
        /// Path to policies YAML file
        #[arg(short, long, default_value = "policies.yaml")]
        policies: String,
        /// Policy ID to remediate
        #[arg(short, long)]
        id: String,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Export CSV report to specified path
        #[arg(long)]
        export_csv: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Tui { policies } => {
            ui::run_tui(&policies)?;
        }
        Commands::Audit { policies, filter, json, export_csv } => {
            if json {
                run_headless_audit(&policies);
                return Ok(());
            }
            run_audit_cli(&policies, filter.as_deref(), export_csv.as_deref())?;
        }
        Commands::Remediate { policies, id, yes, export_csv } => {
            run_remediate_cli(&policies, &id, yes, export_csv.as_deref())?;
        }
    }

    Ok(())
}

fn run_headless_audit(policies_path: &str) {
    let policies = match policy_parser::load_policy(policies_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{{\"error\": \"Failed to load policies: {}\"}}", e);
            std::process::exit(1);
        }
    };

    let results = match engine::audit(&policies) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{{\"error\": \"Audit failed: {}\"}}", e);
            std::process::exit(1);
        }
    };

    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let score = if total > 0 {
        (passed as f32 / total as f32) * 100.0
    } else {
        0.0
    };

    let report = CliReport {
        timestamp: Utc::now().to_rfc3339(),
        compliance_score: score,
        results,
    };

    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{}", json),
        Err(e) => {
            eprintln!("{{\"error\": \"JSON serialization failed: {}\"}}", e);
            std::process::exit(1);
        }
    }
}

fn run_audit_cli(policies_path: &str, filter: Option<&str>, export_csv: Option<&str>) -> Result<()> {
    use nogap_core::policy_parser;

    let policies = policy_parser::load_policy(policies_path)?;

    let filtered: Vec<_> = if let Some(f) = filter {
        policies.iter().filter(|p| p.id.contains(f)).collect()
    } else {
        policies.iter().collect()
    };

    println!("Running audit on {} policies...", filtered.len());

    // Run audit using the engine
    let results = match engine::audit(&policies) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Audit error: {}", e);
            return Err(anyhow::anyhow!("Audit failed"));
        }
    };

    for policy in &filtered {
        let title = policy.title.as_deref().unwrap_or("Untitled");
        println!("\n  Policy: {} [{}]", title, policy.id);
        println!("  Platform: {}", policy.platform);
        
        // Find result for this policy
        if let Some(result) = results.iter().find(|r| r.policy_id == policy.id) {
            let status = if result.passed { "PASS" } else { "FAIL" };
            println!("  Status: {} - {}", status, result.message);
        } else {
            println!("  Status: [No result]");
        }
    }

    println!("\nAudit complete.");

    // Export CSV if requested
    if let Some(csv_path) = export_csv {
        export_csv_report(&results, &policies, csv_path)?;
    }

    Ok(())
}

fn run_remediate_cli(policies_path: &str, policy_id: &str, skip_confirm: bool, export_csv: Option<&str>) -> Result<()> {
    use nogap_core::policy_parser;

    let policies = policy_parser::load_policy(policies_path)?;

    let policy = policies
        .iter()
        .find(|p| p.id == policy_id)
        .ok_or_else(|| anyhow::anyhow!("Policy {} not found", policy_id))?;

    let title = policy.title.as_deref().unwrap_or("Untitled");

    if !skip_confirm {
        println!("Remediate policy: {} [{}]", title, policy_id);
        println!("Platform: {}", policy.platform);
        println!("\nProceed? (y/N): ");

        use std::io::{self, BufRead};
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;

        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    println!("Running remediation...");
    
    // Run remediation using the engine (pass single policy as slice)
    let snapshot_provider = engine::RealSnapshotProvider;
    let result = match engine::remediate(std::slice::from_ref(policy), &snapshot_provider) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Remediation error: {}", e);
            return Err(anyhow::anyhow!("Remediation failed"));
        }
    };

    println!("Remediation complete.");
    
    if let Some(first_result) = result.first() {
        match first_result {
            engine::RemediateResult::Success { message, .. } => {
                println!("Success: {}", message);
            }
            engine::RemediateResult::Failed { message, .. } => {
                println!("Failed: {}", message);
            }
        }
    }

    // Export CSV if requested
    if let Some(csv_path) = export_csv {
        // Re-run audit to get current state
        let audit_results = match engine::audit(&policies) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Post-remediation audit error: {}", e);
                return Err(anyhow::anyhow!("Audit failed"));
            }
        };
        export_csv_report(&audit_results, &policies, csv_path)?;
    }

    Ok(())
}

/// Export audit results to CSV file
fn export_csv_report(
    results: &[engine::AuditResult],
    policies: &[nogap_core::types::Policy],
    csv_path: &str,
) -> Result<()> {
    use std::path::Path;
    
    // Resolve path - could be default path or custom
    let resolved_path = resolve_csv_path(csv_path)?;
    
    // Create parent directories if needed
    if let Some(parent) = Path::new(&resolved_path).parent() {
        fs::create_dir_all(parent)?;
    }

    let mut wtr = csv::Writer::from_path(&resolved_path)?;

    // Write header
    wtr.write_record(&[
        "policy_id",
        "description",
        "expected",
        "actual",
        "status",
        "severity",
        "timestamp",
    ])?;

    let timestamp = Utc::now().to_rfc3339();

    // Write results
    for result in results {
        // Find corresponding policy for additional metadata
        let policy = policies.iter().find(|p| p.id == result.policy_id);
        
        let description = policy
            .and_then(|p| p.description.as_deref())
            .unwrap_or("N/A");
        
        let severity = policy
            .and_then(|p| p.severity.as_deref())
            .unwrap_or("medium");

        let expected = policy
            .and_then(|p| p.expected_state.as_ref())
            .map(|e| format!("{:?}", e))
            .unwrap_or_else(|| "N/A".to_string());

        let status = if result.passed { "PASS" } else { "FAIL" };
        let actual = &result.message;

        wtr.write_record(&[
            &result.policy_id,
            description,
            &expected,
            actual,
            status,
            severity,
            &timestamp,
        ])?;
    }

    wtr.flush()?;
    
    println!("✓ CSV report generated: {}", resolved_path);
    
    Ok(())
}

/// Resolve CSV path - handle default paths and USB-B mode
fn resolve_csv_path(csv_path: &str) -> Result<String> {
    // Check if running in USB-B mode (offline)
    if let Some(usb_path) = detect_usb_b_mount() {
        // Get hostname
        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());
        
        let usb_report_path = format!("{}/reports/{}/report.csv", usb_path, hostname);
        println!("✓ USB-B detected, writing to: {}", usb_report_path);
        return Ok(usb_report_path);
    }
    
    // Check if default path requested
    if csv_path == "default" {
        return Ok(get_default_csv_path());
    }
    
    // Use provided path
    Ok(csv_path.to_string())
}

/// Get platform-specific default CSV path
fn get_default_csv_path() -> String {
    #[cfg(target_os = "windows")]
    {
        "C:\\nogap\\report.csv".to_string()
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        "/opt/nogap/report.csv".to_string()
    }
}

/// Detect USB-B mount path (for offline mode)
fn detect_usb_b_mount() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        // Check common Windows USB drive letters
        for letter in 'D'..='Z' {
            let path = format!("{}:\\", letter);
            if std::path::Path::new(&path).exists() {
                // Check if it's a USB-B repository (look for marker file)
                let marker = format!("{}:\\nogap_usb_repo", letter);
                if std::path::Path::new(&marker).exists() {
                    return Some(format!("{}:", letter));
                }
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // Check common Linux mount points
        let common_mounts = [
            "/media/usb",
            "/mnt/usb",
            "/media/nogap",
            "/mnt/nogap",
        ];
        
        for mount in &common_mounts {
            if std::path::Path::new(mount).exists() {
                let marker = format!("{}/nogap_usb_repo", mount);
                if std::path::Path::new(&marker).exists() {
                    return Some(mount.to_string());
                }
            }
        }
        
        // Check /media/$USER/* for USB drives
        if let Ok(entries) = fs::read_dir("/media") {
            for entry in entries.flatten() {
                if let Ok(sub_entries) = fs::read_dir(entry.path()) {
                    for sub_entry in sub_entries.flatten() {
                        let marker = sub_entry.path().join("nogap_usb_repo");
                        if marker.exists() {
                            if let Some(path_str) = sub_entry.path().to_str() {
                                return Some(path_str.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    
    None
}
