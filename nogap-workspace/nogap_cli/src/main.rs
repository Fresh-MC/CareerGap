use anyhow::Result;
/// NoGap CLI - Operator Cockpit Terminal UI
///
/// Provides TUI, audit, and remediate commands for the NoGap security platform.
use clap::{Parser, Subcommand};
use nogap_cli::ui;
use serde::{Serialize, Deserialize};
use chrono::Utc;
use nogap_core::{policy_parser, engine};
use std::path::PathBuf;
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
    /// Scan for USB storage devices and policy repositories
    ScanUsb {
        /// Output as JSON (headless mode)
        #[arg(long)]
        json: bool,
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
        Commands::ScanUsb { json } => {
            if json {
                run_scan_usb_json()?;
            } else {
                run_scan_usb_cli()?;
            }
        }
        Commands::Report { from_json, to_csv, filter, summary } => {
            run_report_cli(&from_json, to_csv.as_deref(), filter.as_deref(), summary)?;
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

#[derive(Serialize)]
struct UsbDeviceInfo {
    name: String,
    mount_path: String,
    size_available: Option<u64>,
    has_policy_folder: bool,
    policy_files_detected: Vec<String>,
}

/// Scan USB devices and output results in plain text format
fn run_scan_usb_cli() -> Result<()> {
    println!("🔍 Scanning for USB storage devices...\n");
    
    let devices = scan_usb_devices()?;
    
    if devices.is_empty() {
        println!("No USB storage devices detected.");
        return Ok(());
    }
    
    println!("Found {} USB device(s):\n", devices.len());
    
    for (idx, device) in devices.iter().enumerate() {
        println!("Device #{}", idx + 1);
        println!("  Name: {}", device.name);
        println!("  Mount: {}", device.mount_path);
        if let Some(size) = device.size_available {
            println!("  Available: {} MB", size / 1024 / 1024);
        } else {
            println!("  Available: N/A");
        }
        println!("  Policy folder: {}", if device.has_policy_folder { "✓ Yes" } else { "✗ No" });
        if !device.policy_files_detected.is_empty() {
            println!("  Policy files:");
            for file in &device.policy_files_detected {
                println!("    - {}", file);
            }
        } else {
            println!("  Policy files: None");
        }
        println!();
    }
    
    Ok(())
}

/// Scan USB devices and output results in JSON format
fn run_scan_usb_json() -> Result<()> {
    let devices = scan_usb_devices()?;
    
    let json = serde_json::to_string_pretty(&devices)?;
    println!("{}", json);
    
    Ok(())
}

/// Core USB scanning logic - reusable for CLI and TUI
fn scan_usb_devices() -> Result<Vec<UsbDeviceInfo>> {
    use nogap_core::ostree_lite;
    
    let mut devices = Vec::new();
    
    // Discover USB repos using existing core functionality
    let repo_paths = ostree_lite::discover_usb_repos()
        .unwrap_or_else(|_| Vec::new());
    
    // Get all removable drives
    #[cfg(target_os = "windows")]
    {
        for letter in b'D'..=b'Z' {
            let drive_path = format!("{}:\\", letter as char);
            let path = PathBuf::from(&drive_path);
            
            if !path.exists() {
                continue;
            }
            
            // Check if removable using Windows API
            use std::os::windows::ffi::OsStrExt;
            use std::ffi::OsString;
            
            let wide: Vec<u16> = OsString::from(&drive_path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            
            let drive_type = unsafe {
                winapi::um::fileapi::GetDriveTypeW(wide.as_ptr())
            };
            
            // Only process removable (2) or fixed (3) drives
            if drive_type != 2 && drive_type != 3 {
                continue;
            }
            
            let name = format!("Drive {}", letter as char);
            let has_policy_folder = repo_paths.iter().any(|p| p.starts_with(&path));
            
            // Get available space
            let size_available = get_drive_space(&drive_path);
            
            // Detect policy files
            let policy_files = detect_policy_files(&drive_path);
            
            devices.push(UsbDeviceInfo {
                name,
                mount_path: drive_path,
                size_available,
                has_policy_folder,
                policy_files_detected: policy_files,
            });
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // Check /media and /mnt for mounted devices
        let mount_points = ["/media", "/mnt"];
        
        for mount_base in &mount_points {
            if let Ok(entries) = fs::read_dir(mount_base) {
                for entry in entries.flatten() {
                    if let Ok(sub_entries) = fs::read_dir(entry.path()) {
                        for sub_entry in sub_entries.flatten() {
                            let mount_path = sub_entry.path();
                            if !mount_path.is_dir() {
                                continue;
                            }
                            
                            let mount_str = mount_path.to_string_lossy().to_string();
                            let name = mount_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown")
                                .to_string();
                            
                            let has_policy_folder = repo_paths.iter().any(|p| p.starts_with(&mount_path));
                            let size_available = get_drive_space(&mount_str);
                            let policy_files = detect_policy_files(&mount_str);
                            
                            devices.push(UsbDeviceInfo {
                                name,
                                mount_path: mount_str,
                                size_available,
                                has_policy_folder,
                                policy_files_detected: policy_files,
                            });
                        }
                    }
                }
            }
        }
    }
    
    Ok(devices)
}

/// Get available space on drive/mount point
fn get_drive_space(path: &str) -> Option<u64> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use std::ffi::OsString;
        use winapi::um::fileapi::GetDiskFreeSpaceExW;
        use winapi::shared::minwindef::FALSE;
        
        let wide: Vec<u16> = OsString::from(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let mut free_bytes_available: u64 = 0;
        
        unsafe {
            let result = GetDiskFreeSpaceExW(
                wide.as_ptr(),
                std::mem::transmute(&mut free_bytes_available),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            
            if result != FALSE {
                return Some(free_bytes_available);
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        
        if let Ok(metadata) = fs::metadata(path) {
            // This is approximate - actual free space requires statvfs
            return Some(metadata.blocks() * 512);
        }
    }
    
    None
}

/// Detect policy-related files on USB device
fn detect_policy_files(base_path: &str) -> Vec<String> {
    let mut files = Vec::new();
    
    // Check for aegis_repo marker
    let aegis_marker = PathBuf::from(base_path).join("aegis_repo");
    if aegis_marker.exists() {
        files.push("aegis_repo/".to_string());
        
        // Check for manifest
        if aegis_marker.join("manifest.json").exists() {
            files.push("aegis_repo/manifest.json".to_string());
        }
        
        // Check for objects directory
        if aegis_marker.join("objects").exists() {
            files.push("aegis_repo/objects/".to_string());
        }
    }
    
    // Check for nogap_usb_repo marker
    let nogap_marker = PathBuf::from(base_path).join("nogap_usb_repo");
    if nogap_marker.exists() {
        files.push("nogap_usb_repo".to_string());
    }
    
    // Check for policies.yaml
    if PathBuf::from(base_path).join("policies.yaml").exists() {
        files.push("policies.yaml".to_string());
    }
    
    // Check for reports directory
    let reports_dir = PathBuf::from(base_path).join("reports");
    if reports_dir.exists() {
        files.push("reports/".to_string());
    }
    
    files
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

