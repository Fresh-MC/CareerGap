use anyhow::Result;
/// NoGap CLI - Operator Cockpit Terminal UI
///
/// Provides TUI, audit, and remediate commands for the NoGap security platform.
use clap::{Parser, Subcommand};
use nogap_cli::ui;
use serde::{Serialize, Deserialize};
use chrono::Utc;
use nogap_core::{policy_parser, engine};
use std::fs;

#[derive(Serialize, Deserialize)]
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
    /// View, filter, and export past audit/remediation results
    Report {
        /// Path to input JSON report file
        #[arg(long)]
        from_json: String,
        /// Export to CSV file
        #[arg(long)]
        to_csv: Option<String>,
        /// Filter by policy ID or severity (high/medium/low)
        #[arg(long)]
        filter: Option<String>,
        /// Show summary statistics only
        #[arg(long)]
        summary: bool,
    },
    /// AI-ASSISTED: Generate risk report from audit results
    #[command(name = "risk-report")]
    RiskReport {
        /// Path to policies YAML file
        #[arg(short, long, default_value = "policies.yaml")]
        policies: String,
        /// Number of top risks to show
        #[arg(short, long, default_value = "10")]
        top: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// AI-ASSISTED: Detect compliance drift since last audit
    #[command(name = "drift")]
    Drift {
        /// Path to policies YAML file
        #[arg(short, long, default_value = "policies.yaml")]
        policies: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// AI-ASSISTED: Get policy recommendations based on system context
    Recommend {
        /// Path to policies YAML file
        #[arg(short, long, default_value = "policies.yaml")]
        policies: String,
        /// System role (e.g., "web server", "database", "workstation")
        #[arg(short, long, default_value = "general")]
        role: String,
        /// Environment (production, development, staging, air-gapped)
        #[arg(short, long, default_value = "production")]
        environment: String,
        /// Max recommendations to show
        #[arg(short, long, default_value = "20")]
        max: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// AUTONOMOUS: Start background sensing loop (continuous observation)
    #[command(name = "sense-start")]
    SenseStart {
        /// Path to policies YAML file
        #[arg(short, long, default_value = "policies.yaml")]
        policies: String,
        /// Audit interval in hours
        #[arg(short, long, default_value = "24")]
        interval: u64,
    },
    /// AUTONOMOUS: View sensing status and history
    #[command(name = "sense-status")]
    SenseStatus {
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
        Commands::Report { from_json, to_csv, filter, summary } => {
            run_report_cli(&from_json, to_csv.as_deref(), filter.as_deref(), summary)?;
        }
        Commands::RiskReport { policies, top, json } => {
            run_risk_report(&policies, top, json)?;
        }
        Commands::Drift { policies, json } => {
            run_drift_detection(&policies, json)?;
        }
        Commands::Recommend { policies, role, environment, max, json } => {
            run_recommendations(&policies, &role, &environment, max, json)?;
        }
        Commands::SenseStart { policies, interval } => {
            run_sense_start(&policies, interval)?;
        }
        Commands::SenseStatus { json } => {
            run_sense_status(json)?;
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
    
    // Resolve path - use default or custom
    let resolved_path = if csv_path == "default" {
        get_default_csv_path()
    } else {
        csv_path.to_string()
    };
    
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

/// View, filter, and export past audit/remediation results
fn run_report_cli(
    json_path: &str,
    csv_path: Option<&str>,
    filter: Option<&str>,
    summary_only: bool,
) -> Result<()> {
    use std::fs;
    
    // Load JSON report
    let json_content = fs::read_to_string(json_path)
        .map_err(|e| anyhow::anyhow!("Failed to read JSON file: {}", e))?;
    
    let report: CliReport = serde_json::from_str(&json_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse JSON report: {}", e))?;
    
    // Apply filter if provided
    let filtered_results: Vec<_> = if let Some(f) = filter {
        let filter_lower = f.to_lowercase();
        
        // Check if filtering by severity
        if filter_lower == "high" || filter_lower == "medium" || filter_lower == "low" {
            // Filter by severity (need to cross-reference with policies)
            // For now, just filter by policy ID since severity is in policies, not results
            report.results.iter()
                .filter(|r| r.policy_id.to_lowercase().contains(&filter_lower))
                .cloned()
                .collect()
        } else {
            // Filter by policy ID
            report.results.iter()
                .filter(|r| r.policy_id.contains(f))
                .cloned()
                .collect()
        }
    } else {
        report.results.clone()
    };
    
    // Show summary
    if summary_only {
        let total = filtered_results.len();
        let passed = filtered_results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        let score = if total > 0 {
            (passed as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        
        println!("Report Summary");
        println!("==============");
        println!("Report Date: {}", report.timestamp);
        println!("Total Policies: {}", total);
        println!("Passed: {}", passed);
        println!("Failed: {}", failed);
        println!("Compliance Score: {:.2}%", score);
        
        if let Some(f) = filter {
            println!("Filter Applied: {}", f);
        }
        
        return Ok(());
    }
    
    // Display filtered results
    println!("Report Date: {}", report.timestamp);
    println!("Compliance Score: {:.2}%", report.compliance_score);
    
    if let Some(f) = filter {
        println!("Filter: {}", f);
    }
    
    println!("\nResults ({} policies):\n", filtered_results.len());
    
    for result in &filtered_results {
        let status = if result.passed { "PASS" } else { "FAIL" };
        println!("  [{}] {} - {}", status, result.policy_id, result.message);
    }
    
    // Export to CSV if requested
    if let Some(csv_output) = csv_path {
        // Create minimal policy stubs for CSV export
        // Since we don't have full policy objects, create them from results
        let stub_policies: Vec<nogap_core::types::Policy> = filtered_results.iter()
            .map(|r| {
                use nogap_core::types::Policy;
                Policy {
                    id: r.policy_id.clone(),
                    title: None,
                    description: Some(r.message.clone()),
                    platform: "unknown".to_string(),
                    check_type: "unknown".to_string(),
                    severity: Some("medium".to_string()),
                    target_file: None,
                    expected_state: None,
                    ..Default::default()
                }
            })
            .collect();
        
        export_csv_report(&filtered_results, &stub_policies, csv_output)?;
    }
    
    Ok(())
}

// ============================================================
// AI-ASSISTED FEATURES (Non-Agentic, Read-Only, User-Controlled)
// ============================================================

/// AI-ASSISTED: Generate risk report from audit results
/// Shows top N policies by risk score (severity × non-compliance)
fn run_risk_report(policies_path: &str, top_n: usize, output_json: bool) -> Result<()> {
    use nogap_core::{policy_parser, engine, risk_scoring};
    
    println!("⚠️  AI-ASSISTED RISK REPORT - Review before taking action\n");
    
    let policies = policy_parser::load_policy(policies_path)?;
    
    let audit_results = match engine::audit(&policies) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Audit error: {}", e);
            return Err(anyhow::anyhow!("Audit failed"));
        }
    };
    
    // Calculate risk scores
    let all_scores = risk_scoring::calculate_all_risk_scores(&policies, &audit_results);
    let summary = risk_scoring::calculate_system_risk(&all_scores);
    let top_risks = risk_scoring::get_top_risks(&all_scores, top_n);
    
    if output_json {
        let output = serde_json::json!({
            "disclaimer": "AI-assisted analysis - review before action",
            "summary": {
                "total_policies": summary.total_policies,
                "compliant": summary.compliant_count,
                "non_compliant": summary.non_compliant_count,
                "normalized_risk_score": summary.normalized_risk_score,
                "total_risk_score": summary.total_risk_score,
            },
            "top_risks": top_risks,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }
    
    // Text output
    println!("🎯 RISK SUMMARY");
    println!("================");
    println!("Total Policies: {}", summary.total_policies);
    println!("Compliant: {} ✅", summary.compliant_count);
    println!("Non-Compliant: {} ❌", summary.non_compliant_count);
    println!("Normalized Risk: {:.1}%", summary.normalized_risk_score * 100.0);
    
    if top_risks.is_empty() {
        println!("\n✅ No high-risk policies found. Your system is well-configured!");
    } else {
        println!("\n🔴 TOP {} RISK PRIORITIES", top_risks.len());
        println!("============================");
        for (i, risk) in top_risks.iter().enumerate() {
            println!(
                "\n{}. {} [{}]",
                i + 1,
                risk.policy_title,
                risk.policy_id
            );
            println!("   Severity: {}", risk.severity.to_uppercase());
            println!("   Risk Score: {:.0}%", risk.risk_score * 100.0);
        }
    }
    
    println!("\n⚠️  These are AI-assisted suggestions. Review each policy before remediation.");
    
    Ok(())
}

/// AI-ASSISTED: Detect compliance drift since last audit
fn run_drift_detection(policies_path: &str, output_json: bool) -> Result<()> {
    use nogap_core::{policy_parser, engine, drift_detection};
    
    println!("⚠️  AI-ASSISTED DRIFT DETECTION - Comparing against previous audit\n");
    
    let policies = policy_parser::load_policy(policies_path)?;
    
    let audit_results = match engine::audit(&policies) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Audit error: {}", e);
            return Err(anyhow::anyhow!("Audit failed"));
        }
    };
    
    // Initialize drift database
    let conn = drift_detection::init_drift_db()
        .map_err(|e| anyhow::anyhow!("Failed to initialize drift database: {}", e))?;
    
    // Detect drift
    let drift_report = drift_detection::detect_drift(&conn, &audit_results)
        .map_err(|e| anyhow::anyhow!("Failed to detect drift: {}", e))?;
    
    // Store current results for future comparison
    drift_detection::store_audit_results(&conn, &audit_results, None)
        .map_err(|e| anyhow::anyhow!("Failed to store audit results: {}", e))?;
    
    if output_json {
        let output = serde_json::json!({
            "disclaimer": "AI-assisted analysis - review before action",
            "drift_report": {
                "timestamp": drift_report.timestamp,
                "total_compared": drift_report.total_compared,
                "regressions": drift_report.regressions.len(),
                "improvements": drift_report.improvements.len(),
                "unchanged": drift_report.unchanged_count,
                "has_regressions": drift_report.has_regressions(),
            },
            "regression_details": drift_report.regressions,
            "improvement_details": drift_report.improvements,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }
    
    // Text output
    println!("📉 COMPLIANCE DRIFT REPORT");
    println!("==========================");
    println!("Policies Compared: {}", drift_report.total_compared);
    println!("Regressions: {} ⚠️", drift_report.regressions.len());
    println!("Improvements: {} ✅", drift_report.improvements.len());
    println!("Unchanged: {}", drift_report.unchanged_count);
    
    if drift_report.has_regressions() {
        println!("\n🔴 SECURITY REGRESSIONS (Compliant → Non-Compliant)");
        println!("===================================================");
        for event in &drift_report.regressions {
            println!("\n  ⚠️  {}", event.policy_id);
            println!("     Previous: Compliant ✅");
            println!("     Current:  Non-Compliant ❌");
        }
    } else {
        println!("\n✅ No regressions detected. Compliance is stable or improving.");
    }
    
    if !drift_report.improvements.is_empty() {
        println!("\n🟢 IMPROVEMENTS (Non-Compliant → Compliant)");
        println!("============================================");
        for event in &drift_report.improvements {
            println!("  ✅ {}", event.policy_id);
        }
    }
    
    println!("\n⚠️  This is AI-assisted analysis. Review before taking action.");
    
    Ok(())
}

/// AI-ASSISTED: Get policy recommendations based on system context
fn run_recommendations(
    policies_path: &str,
    role: &str,
    environment: &str,
    max_results: usize,
    output_json: bool,
) -> Result<()> {
    use nogap_core::{policy_parser, ai_recommender};
    
    println!("⚠️  AI-ASSISTED RECOMMENDATIONS - Review before enabling\n");
    
    let policies = policy_parser::load_policy(policies_path)?;
    
    let context = ai_recommender::SystemContext {
        os: std::env::consts::OS.to_string(),
        role: role.to_string(),
        environment: environment.to_string(),
        additional_context: None,
    };
    
    let recommendations = ai_recommender::keyword_based_recommendations(&context, &policies, max_results);
    
    if output_json {
        let output = serde_json::json!({
            "disclaimer": "AI-assisted suggestions - review each policy before enabling",
            "context": {
                "os": context.os,
                "role": context.role,
                "environment": context.environment,
            },
            "recommendations": recommendations,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }
    
    // Text output
    println!("💡 POLICY RECOMMENDATIONS");
    println!("=========================");
    println!("OS: {}", context.os);
    println!("Role: {}", context.role);
    println!("Environment: {}", context.environment);
    
    if recommendations.is_empty() {
        println!("\nNo specific recommendations for this context.");
        println!("Try adjusting the role or environment parameters.");
    } else {
        println!("\nRecommended Policies ({}):", recommendations.len());
        println!("----------------------------");
        for (i, rec) in recommendations.iter().enumerate() {
            // Find policy for additional details
            if let Some(policy) = policies.iter().find(|p| p.id == rec.policy_id) {
                let title = policy.title.as_deref().unwrap_or("Untitled");
                let severity = policy.severity.as_deref().unwrap_or("medium");
                println!(
                    "\n{}. {} [{}]",
                    i + 1,
                    title,
                    rec.policy_id
                );
                println!("   Severity: {}", severity.to_uppercase());
                println!("   Relevance: {:.0}%", rec.relevance_score * 100.0);
                println!("   Reason: {}", rec.reason);
            }
        }
    }
    
    println!("\n⚠️  These are AI-assisted suggestions. Review each policy before enabling.");
    
    Ok(())
}

/// Start the autonomous sensing loop
fn run_sense_start(policy_path: &str, interval_hours: u64) -> Result<()> {
    use nogap_core::sensor_scheduler::{SensorConfig, SensorScheduler};
    use std::io::{self, Write};

    println!("🔍 AUTONOMOUS SENSING LOOP");
    println!("==========================");
    println!();
    println!("This will start continuous background audits:");
    println!("  • Interval: {} hours", interval_hours);
    println!("  • Policy file: {}", policy_path);
    println!("  • Mode: Observation only (no remediation)");
    println!();
    println!("⚠️  WARNING: The sensing loop will run indefinitely.");
    println!("    Press Ctrl+C to stop.");
    println!();
    print!("Continue? [y/N]: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Cancelled.");
        return Ok(());
    }

    // Load policies
    let policies = match policy_parser::load_policy(policy_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ Failed to load policies from '{}': {}", policy_path, e);
            return Ok(());
        }
    };

    println!("\n✅ Loaded {} policies", policies.len());

    // Create and start sensor
    let config = SensorConfig {
        enabled: true,
        interval_hours,
        max_stored_events: 100,
    };

    let scheduler = SensorScheduler::new(config);
    
    match scheduler.start(policies) {
        Ok(_) => {
            println!("\n🚀 Sensing loop started successfully!");
            println!("📊 Logs will show audit progress");
            println!("Press Ctrl+C to stop...");
            
            // Keep main thread alive
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to start sensing loop: {}", e);
            eprintln!("   Possible causes:");
            eprintln!("   • Sensing is disabled in configuration");
            eprintln!("   • Sensing loop is already running");
        }
    }

    Ok(())
}

/// Show sensing status and history
fn run_sense_status(output_json: bool) -> Result<()> {

    // For now, we'll load status from snapshots database
    // In a full implementation, we'd persist the scheduler state
    
    if output_json {
        // JSON output for automation
        let status = serde_json::json!({
            "sensor_enabled": false,
            "message": "Use 'sense-start' to begin autonomous sensing",
            "events": [],
        });
        println!("{}", serde_json::to_string_pretty(&status)?);
        return Ok(());
    }

    // Text output
    println!("🔍 AUTONOMOUS SENSOR STATUS");
    println!("===========================");
    println!();
    println!("Status: Not running");
    println!("Use 'nogap-cli sense-start' to begin autonomous sensing.");
    println!();
    println!("What is autonomous sensing?");
    println!("  • Background audits run automatically on schedule");
    println!("  • No decision-making or remediation");
    println!("  • Pure observation and drift detection");
    println!("  • Results stored in snapshots database");
    
    Ok(())
}
