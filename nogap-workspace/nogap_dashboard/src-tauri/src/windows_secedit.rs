#[cfg(target_os = "windows")]
use anyhow::{anyhow, Result};
#[cfg(target_os = "windows")]
use std::fs;
#[cfg(target_os = "windows")]
use std::process::Command;

/// Export current security policy configuration to a file using secedit
/// Returns the path to the exported configuration file
#[cfg(target_os = "windows")]
pub fn export_secedit_cfg(path: &str) -> Result<String> {
    log::info!("[secedit] Exporting security policy to: {}", path);
    let output = Command::new("secedit")
        .args(&["/export", "/cfg", path, "/quiet"])
        .output()
        .map_err(|e| {
            log::error!("[secedit] Failed to execute secedit export: {}", e);
            anyhow!("Failed to execute secedit export: {}", e)
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("[secedit] Export failed: {}", stderr);
        return Err(anyhow!("secedit export failed: {}", stderr));
    }

    log::info!("[secedit] Successfully exported security policy");
    Ok(path.to_string())
}

/// Parse a policy value from secedit configuration content
/// Searches across all INI sections for a matching key
#[cfg(target_os = "windows")]
pub fn parse_secedit_value(cfg_content: &str, policy_name: &str) -> Option<String> {
    log::info!("[secedit] Parsing policy value: {}", policy_name);
    for line in cfg_content.lines() {
        let trimmed = line.trim();

        // Skip empty lines, comments, and section headers
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('[') {
            continue;
        }

        // Look for key=value pairs
        if let Some(equals_pos) = trimmed.find('=') {
            let key = trimmed[..equals_pos].trim();
            if key == policy_name {
                let value = trimmed[equals_pos + 1..].trim();
                log::info!("[secedit] Found policy value: {} = {}", policy_name, value);
                return Some(value.to_string());
            }
        }
    }

    log::info!(
        "[secedit] Policy not found in configuration: {}",
        policy_name
    );
    None
}

/// Update a policy value in secedit configuration content
/// If the policy exists, updates its value; otherwise appends to [System Access]
#[cfg(target_os = "windows")]
pub fn update_secedit_cfg(cfg_content: &str, policy_name: &str, new_value: &str) -> Result<String> {
    log::info!(
        "[secedit] Updating configuration: {} = {}",
        policy_name,
        new_value
    );
    let mut lines: Vec<String> = cfg_content.lines().map(|s| s.to_string()).collect();
    let mut found = false;

    // First pass: try to update existing entry
    for line in lines.iter_mut() {
        let trimmed = line.trim();

        if let Some(equals_pos) = trimmed.find('=') {
            let key = trimmed[..equals_pos].trim();
            if key == policy_name {
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

        // Find [System Access] section
        for (idx, line) in lines.iter().enumerate() {
            if line.trim() == "[System Access]" {
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

/// Apply a secedit configuration file using secedit /configure
#[cfg(target_os = "windows")]
pub fn apply_secedit_cfg(path: &str) -> Result<()> {
    log::info!("[secedit] Applying security policy from: {}", path);
    let output = Command::new("secedit")
        .args(&["/configure", "/cfg", path, "/db", "secedit.sdb", "/quiet"])
        .output()
        .map_err(|e| {
            log::error!("[secedit] Failed to execute secedit configure: {}", e);
            anyhow!("Failed to execute secedit configure: {}", e)
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!("[secedit] Configure failed: {}", stderr);
        return Err(anyhow!("secedit configure failed: {}", stderr));
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
