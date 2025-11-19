#[cfg(target_os = "windows")]
use anyhow::{anyhow, Result};
#[cfg(target_os = "windows")]
use std::process::Command;
#[cfg(target_os = "windows")]
use tempfile::NamedTempFile;

use crate::atomic::atomic_write;

/// Export current security policy configuration using secedit
/// Returns the exported INF content as a string
#[cfg(target_os = "windows")]
pub fn export_secedit_cfg() -> Result<String> {
    log::info!("[secedit] Exporting security policy");

    // Create unique temp file for export
    let temp_file = NamedTempFile::new()
        .map_err(|e| anyhow!("Failed to create temp file: {}", e))?;
    let export_path = temp_file.path().with_extension("inf");

    let output = Command::new("secedit")
        .args(&[
            "/export",
            "/cfg",
            export_path.to_str().unwrap(),
            "/quiet",
        ])
        .output()
        .map_err(|e| {
            log::error!("[secedit] Failed to execute secedit export: {}", e);
            anyhow!("Failed to execute secedit export: {}", e)
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        log::error!("[secedit] Export failed. Stderr: {}. Stdout: {}", stderr, stdout);
        return Err(anyhow!(
            "SeceditError: Export failed. Exit Code: {:?}. Command: secedit /export /cfg <path> /quiet. Stderr: {}. Stdout: {}",
            output.status.code(),
            stderr.trim(),
            stdout.trim()
        ));
    }

    // Read the exported INF file
    let raw_bytes = std::fs::read(&export_path)
        .map_err(|e| anyhow!("Failed to read secedit export file: {}", e))?;

    // Detect and decode UTF-16 LE with BOM or fallback to UTF-8
    let content = if raw_bytes.len() >= 2 && raw_bytes[0] == 0xFF && raw_bytes[1] == 0xFE {
        // UTF-16LE with BOM detected
        let (decoded, _encoding, had_errors) = encoding_rs::UTF_16LE.decode(&raw_bytes[2..]);
        if had_errors {
            return Err(anyhow!("UTF-16 decode error in secedit export"));
        }
        decoded.into_owned()
    } else {
        // UTF-8 or lossy conversion
        String::from_utf8_lossy(&raw_bytes).to_string()
    };

    // Normalize CRLF to LF
    let normalized = content.replace("\r\n", "\n");

    log::info!("[secedit] Successfully exported security policy");
    Ok(normalized)
}

/// Parse a policy value from secedit configuration content with robust parsing
/// Searches across all INI sections for a matching key (case-insensitive)
#[cfg(target_os = "windows")]
pub fn parse_secedit_value(cfg_content: &str, policy_name: &str) -> Option<String> {
    log::info!("[secedit] Parsing policy value: {}", policy_name);
    let normalized = cfg_content.replace("\r\n", "\n");
    let policy_lower = policy_name.to_lowercase();
    let mut result = None;

    for line in normalized.lines() {
        let trimmed = line.trim();

        // Skip empty lines, comments, and section headers
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('[') {
            continue;
        }

        // Look for key=value pairs (case-insensitive key matching)
        if let Some(equals_pos) = trimmed.find('=') {
            let key = trimmed[..equals_pos].trim().to_lowercase();
            if key == policy_lower {
                let value = trimmed[equals_pos + 1..].trim().to_string();
                result = Some(value.clone()); // Last value wins
                log::info!("[secedit] Found policy value: {} = {}", policy_name, value);
            }
        }
    }

    if result.is_none() {
        log::info!("[secedit] Policy not found in configuration: {}", policy_name);
    }

    result
}

/// Update a policy value in secedit configuration content with robust update logic
/// If the policy exists, updates its value; otherwise appends to [System Access]
#[cfg(target_os = "windows")]
pub fn update_secedit_cfg(cfg_content: &str, policy_name: &str, new_value: &str) -> Result<String> {
    log::info!("[secedit] Updating configuration: {} = {}", policy_name, new_value);
    let normalized = cfg_content.replace("\r\n", "\n");
    let mut lines: Vec<String> = normalized.lines().map(|s| s.to_string()).collect();
    let policy_lower = policy_name.to_lowercase();
    let mut found = false;

    // First pass: try to update existing entry (case-insensitive)
    for line in lines.iter_mut() {
        let trimmed = line.trim();

        if let Some(equals_pos) = trimmed.find('=') {
            let key = trimmed[..equals_pos].trim().to_lowercase();
            if key == policy_lower {
                *line = format!("{} = {}", policy_name, new_value);
                found = true;
                log::info!("[secedit] Updated existing policy entry: {}", policy_name);
                break;
            }
        }
    }

    // If not found, append to [System Access] section
    if !found {
        log::info!("[secedit] Policy not found, adding to [System Access] section");
        let mut system_access_idx = None;

        // Find [System Access] section (case-insensitive)
        for (idx, line) in lines.iter().enumerate() {
            if line.trim().to_lowercase() == "[system access]" {
                system_access_idx = Some(idx);
                break;
            }
        }

        if let Some(idx) = system_access_idx {
            // Find the end of this section (next section or end of file)
            let mut insert_idx = idx + 1;
            for i in (idx + 1)..lines.len() {
                if lines[i].trim().starts_with('[') {
                    insert_idx = i;
                    break;
                }
                insert_idx = i + 1;
            }

            // Insert the new entry
            lines.insert(insert_idx, format!("{} = {}", policy_name, new_value));
        } else {
            // [System Access] section doesn't exist, create it
            lines.push(String::new());
            lines.push("[System Access]".to_string());
            lines.push(format!("{} = {}", policy_name, new_value));
        }
    }

    Ok(lines.join("\n"))
}

/// Apply a secedit configuration using atomic writes and robust execution
#[cfg(target_os = "windows")]
pub fn apply_secedit_cfg(cfg_content: &str) -> Result<()> {
    log::info!("[secedit] Applying security policy configuration");

    // Create unique temp files using NamedTempFile
    let inf_temp = NamedTempFile::new()
        .map_err(|e| anyhow!("Failed to create temp INF file: {}", e))?;
    let inf_path = inf_temp.path().with_extension("inf");

    let db_temp = NamedTempFile::new()
        .map_err(|e| anyhow!("Failed to create temp SDB file: {}", e))?;
    let db_path = db_temp.path().with_extension("sdb");

    // Write INF file atomically
    atomic_write(
        inf_path.to_str().unwrap(),
        cfg_content,
    )
    .map_err(|e| anyhow!("Failed to write INF file: {}", e))?;

    let output = Command::new("secedit")
        .args(&[
            "/configure",
            "/db",
            db_path.to_str().unwrap(),
            "/cfg",
            inf_path.to_str().unwrap(),
            "/quiet",
        ])
        .output()
        .map_err(|e| {
            log::error!("[secedit] Failed to execute secedit configure: {}", e);
            anyhow!("Failed to execute secedit configure: {}", e)
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        log::error!("[secedit] Configure failed. Stderr: {}. Stdout: {}", stderr, stdout);
        return Err(anyhow!(
            "SeceditError: Configure failed. Exit Code: {:?}. Command: secedit /configure /db <path> /cfg <path> /quiet. Stderr: {}. Stdout: {}",
            output.status.code(),
            stderr.trim(),
            stdout.trim()
        ));
    }

    log::info!("[secedit] Successfully applied security policy");
    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::*;

    #[test]
    fn test_parse_secedit_value_found() {
        let cfg = r#"
[System Access]
MinimumPasswordLength = 14
MaximumPasswordAge = 60

[Event Audit]
AuditSystemEvents = 3
"#;
        assert_eq!(
            parse_secedit_value(cfg, "MinimumPasswordLength"),
            Some("14".to_string())
        );
        assert_eq!(
            parse_secedit_value(cfg, "AuditSystemEvents"),
            Some("3".to_string())
        );
    }

    #[test]
    fn test_parse_secedit_value_not_found() {
        let cfg = r#"
[System Access]
MinimumPasswordLength = 14
"#;
        assert_eq!(parse_secedit_value(cfg, "NonExistent"), None);
    }

    #[test]
    fn test_update_secedit_cfg_existing() {
        let cfg = r#"[System Access]
MinimumPasswordLength = 8
MaximumPasswordAge = 60"#;

        let result = update_secedit_cfg(cfg, "MinimumPasswordLength", "14").unwrap();
        assert!(result.contains("MinimumPasswordLength = 14"));
        assert!(result.contains("MaximumPasswordAge = 60"));
    }

    #[test]
    fn test_update_secedit_cfg_new_entry() {
        let cfg = r#"[System Access]
MinimumPasswordLength = 8

[Event Audit]
AuditSystemEvents = 3"#;

        let result = update_secedit_cfg(cfg, "MaximumPasswordAge", "60").unwrap();
        assert!(result.contains("MaximumPasswordAge = 60"));
        assert!(result.contains("MinimumPasswordLength = 8"));
    }
}
