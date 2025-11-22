use std::error::Error;

/// Trait for abstracting secedit.exe operations to enable testing
pub trait SeceditExecutor {
    /// Export current security policy as INF file content
    fn export_security_policy(&self) -> Result<String, Box<dyn Error>>;

    /// Configure security policy from INF file content
    fn configure_security_policy(&self, inf_content: &str) -> Result<(), Box<dyn Error>>;
}

/// Real secedit executor that calls actual secedit.exe on Windows
#[cfg(target_os = "windows")]
pub struct RealSeceditExecutor;

#[cfg(target_os = "windows")]
impl SeceditExecutor for RealSeceditExecutor {
    fn export_security_policy(&self) -> Result<String, Box<dyn Error>> {
        use std::fs;
        use std::process::Command;
        use tempfile::NamedTempFile;

        // Create unique temp file for export
        let temp_file = NamedTempFile::new()
            .map_err(|e| format!("Failed to create temp file: {}", e))?;
        let export_path = temp_file.path().with_extension("inf");

        // Export current security policy with /quiet flag
        let output = Command::new("secedit")
            .args(&[
                "/export",
                "/cfg",
                export_path.to_str().unwrap(),
                "/quiet",
            ])
            .output()
            .map_err(|e| format!("Failed to execute secedit.exe: {}", e))?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr);
            let stdout_msg = String::from_utf8_lossy(&output.stdout);

            return Err(format!(
                "SeceditError: Export failed. Exit Code: {:?}. Command: secedit /export /cfg <path> /quiet. Stderr: {}. Stdout: {}",
                output.status.code(),
                stderr_msg.trim(),
                stdout_msg.trim()
            )
            .into());
        }

        // Read the exported INF file
        let raw_bytes = fs::read(&export_path)
            .map_err(|e| format!("Failed to read secedit export file: {}", e))?;

        // Detect and decode UTF-16 LE with BOM or fallback to UTF-8
        let content = if raw_bytes.len() >= 2 && raw_bytes[0] == 0xFF && raw_bytes[1] == 0xFE {
            // UTF-16LE with BOM detected
            let (decoded, _encoding, had_errors) = encoding_rs::UTF_16LE.decode(&raw_bytes[2..]);
            if had_errors {
                return Err("UTF-16 decode error in secedit export".into());
            }
            decoded.into_owned()
        } else {
            // UTF-8 or lossy conversion
            String::from_utf8_lossy(&raw_bytes).to_string()
        };

        // Normalize CRLF to LF
        let normalized = content.replace("\r\n", "\n");

        Ok(normalized)
    }

    fn configure_security_policy(&self, inf_content: &str) -> Result<(), Box<dyn Error>> {
        use std::process::Command;
        use tempfile::NamedTempFile;

        // Create unique temp files using NamedTempFile
        let mut inf_temp = NamedTempFile::new()
            .map_err(|e| format!("Failed to create temp INF file: {}", e))?;
        let inf_path = inf_temp.path().with_extension("inf");

        let db_temp = NamedTempFile::new()
            .map_err(|e| format!("Failed to create temp SDB file: {}", e))?;
        let db_path = db_temp.path().with_extension("sdb");

        // Write INF file atomically
        inf_temp.write_all(inf_content.as_bytes())?;
        inf_temp.flush()?;
        inf_temp.persist(&inf_path)
            .map_err(|e| format!("Failed to persist INF file: {}", e))?;

        // Apply the policy using secedit with /quiet flag
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
            .map_err(|e| format!("Failed to execute secedit.exe: {}", e))?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr);
            let stdout_msg = String::from_utf8_lossy(&output.stdout);

            // Check for access denied
            if stderr_msg.contains("Access is denied") || stdout_msg.contains("Access is denied") {
                return Err(
                    "Administrator privileges required to modify Local Security Policy".into(),
                );
            }

            return Err(format!(
                "SeceditError: Configure failed. Exit Code: {:?}. Command: secedit /configure /db <path> /cfg <path> /quiet. Stderr: {}. Stdout: {}",
                output.status.code(),
                stderr_msg.trim(),
                stdout_msg.trim()
            )
            .into());
        }

        Ok(())
    }
}

/// Parse a policy value from secedit INF content with robust parsing
/// Returns the value for the given policy_name, case-insensitive, last value wins
pub fn parse_secedit_value(cfg_content: &str, policy_name: &str) -> Option<String> {
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
                result = Some(value); // Last value wins
            }
        }
    }

    result
}

/// Update a policy value in secedit INF content with robust update logic
/// Replaces existing key or adds under [System Access], preserves unrelated lines
pub fn update_secedit_cfg(cfg_content: &str, policy_name: &str, new_value: &str) -> Result<String, Box<dyn Error>> {
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
                break;
            }
        }
    }

    // If not found, append to [System Access] section
    if !found {
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

/// Mock secedit executor for testing and non-Windows platforms
/// Always available for testing, and also on non-Windows platforms
pub struct MockSeceditExecutor {
    pub export_content: std::cell::RefCell<String>,
    pub last_configured_inf: std::cell::RefCell<Option<String>>,
}

impl Default for MockSeceditExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSeceditExecutor {
    pub fn new() -> Self {
        Self {
            export_content: std::cell::RefCell::new(
                "[Unicode]\nUnicode=yes\n\n[System Access]\nLockoutDuration = 5\nLockoutBadCount = 3\nPasswordComplexity = 1\nEnableGuestAccount = 0\n\n[Version]\nsignature=\"$CHICAGO$\"\nRevision=1\n".to_string(),
            ),
            last_configured_inf: std::cell::RefCell::new(None),
        }
    }

    pub fn set_export_content(&self, content: String) {
        *self.export_content.borrow_mut() = content;
    }

    pub fn get_last_configured_inf(&self) -> Option<String> {
        self.last_configured_inf.borrow().clone()
    }
}

impl SeceditExecutor for MockSeceditExecutor {
    fn export_security_policy(&self) -> Result<String, Box<dyn Error>> {
        Ok(self.export_content.borrow().clone())
    }

    fn configure_security_policy(&self, inf_content: &str) -> Result<(), Box<dyn Error>> {
        // Store the INF content for test assertions
        *self.last_configured_inf.borrow_mut() = Some(inf_content.to_string());

        // Parse the INF to update our mock state
        let mut in_system_access = false;
        let mut new_values = std::collections::HashMap::new();

        for line in inf_content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with('[') {
                in_system_access = trimmed.eq_ignore_ascii_case("[System Access]");
                continue;
            }

            if trimmed.is_empty() || trimmed.starts_with(';') {
                continue;
            }

            if in_system_access {
                if let Some(eq_pos) = trimmed.find('=') {
                    let key = trimmed[..eq_pos].trim();
                    let value = trimmed[eq_pos + 1..].trim();
                    new_values.insert(key.to_string(), value.to_string());
                }
            }
        }

        // Update export_content with new values
        let mut lines = vec![];
        let mut in_system_access = false;

        for line in self.export_content.borrow().lines() {
            let trimmed = line.trim();

            if trimmed.starts_with('[') {
                in_system_access = trimmed.eq_ignore_ascii_case("[System Access]");
                lines.push(line.to_string());
                continue;
            }

            if in_system_access && !trimmed.is_empty() && !trimmed.starts_with(';') {
                if let Some(eq_pos) = trimmed.find('=') {
                    let key = trimmed[..eq_pos].trim();
                    if let Some(new_value) = new_values.get(key) {
                        lines.push(format!("{} = {}", key, new_value));
                        continue;
                    }
                }
            }

            lines.push(line.to_string());
        }

        *self.export_content.borrow_mut() = lines.join("\n");

        Ok(())
    }
}
