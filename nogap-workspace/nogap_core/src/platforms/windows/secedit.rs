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
        use std::process::Command;
        use std::fs;
        
        let temp_dir = std::env::temp_dir();
        let export_path = temp_dir.join(format!("nogap_secedit_export_{}.inf", std::process::id()));
        
        // Export current security policy
        let output = Command::new("secedit")
            .args(&["/export", "/cfg", export_path.to_str().unwrap()])
            .output()
            .map_err(|e| format!("Failed to execute secedit.exe: {}", e))?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr);
            let stdout_msg = String::from_utf8_lossy(&output.stdout);

            return Err(format!(
                "secedit export failed:\n\
                 Exit Code: {:?}\n\
                 STDERR: {}\n\
                 STDOUT: {}",
                output.status.code(),
                stderr_msg.trim(),
                stdout_msg.trim()
            ).into());
        }

        // Read the exported INF file (secedit exports as UTF-16LE with BOM)
        let raw_bytes = fs::read(&export_path)
            .map_err(|e| format!("Failed to read secedit export file: {}", e))?;
        
        // Cleanup temp file
        let _ = fs::remove_file(&export_path);
        
        // Detect encoding and decode appropriately
        let content = if raw_bytes.len() >= 2 && raw_bytes[0] == 0xFF && raw_bytes[1] == 0xFE {
            // UTF-16LE with BOM - skip BOM and decode
            let u16_values: Vec<u16> = raw_bytes[2..]
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();
            
            char::decode_utf16(u16_values)
                .collect::<Result<String, _>>()
                .map_err(|e| format!("UTF-16 decode error: {:?}", e))?
        } else {
            // Fallback to UTF-8 (or lossy conversion)
            String::from_utf8_lossy(&raw_bytes).to_string()
        };
        
        Ok(content)
    }

    fn configure_security_policy(&self, inf_content: &str) -> Result<(), Box<dyn Error>> {
        use std::process::Command;
        use std::fs;
        use std::io::Write;
        
        let temp_dir = std::env::temp_dir();
        let inf_path = temp_dir.join(format!("nogap_secedit_patch_{}.inf", std::process::id()));
        let db_path = temp_dir.join(format!("nogap_secedit_{}.sdb", std::process::id()));

        // Write INF file
        let mut file = fs::File::create(&inf_path)?;
        file.write_all(inf_content.as_bytes())?;
        file.sync_all()?;

        // Apply the policy using secedit
        let output = Command::new("secedit")
            .args(&[
                "/configure",
                "/db",
                db_path.to_str().unwrap(),
                "/cfg",
                inf_path.to_str().unwrap(),
                "/areas",
                "SECURITYPOLICY",
            ])
            .output()
            .map_err(|e| format!("Failed to execute secedit.exe: {}", e))?;

        // Cleanup temp files
        let _ = fs::remove_file(&inf_path);
        let _ = fs::remove_file(&db_path);

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr);
            let stdout_msg = String::from_utf8_lossy(&output.stdout);
            
            // Check for access denied
            if stderr_msg.contains("Access is denied") || stdout_msg.contains("Access is denied") {
                return Err("Administrator privileges required to modify Local Security Policy".into());
            }
            
            return Err(format!(
                "secedit configure failed:\n\
                 Exit Code: {:?}\n\
                 STDERR: {}\n\
                 STDOUT: {}",
                output.status.code(),
                stderr_msg.trim(),
                stdout_msg.trim()
            ).into());
        }

        Ok(())
    }
}

/// Mock secedit executor for testing and non-Windows platforms
/// Always available for testing, and also on non-Windows platforms
pub struct MockSeceditExecutor {
    pub export_content: std::cell::RefCell<String>,
    pub last_configured_inf: std::cell::RefCell<Option<String>>,
}

impl MockSeceditExecutor {
    pub fn new() -> Self {
        Self {
            export_content: std::cell::RefCell::new(
                "[Unicode]\n\
                 Unicode=yes\n\
                 \n\
                 [System Access]\n\
                 LockoutDuration = 5\n\
                 LockoutBadCount = 3\n\
                 PasswordComplexity = 1\n\
                 EnableGuestAccount = 0\n\
                 \n\
                 [Version]\n\
                 signature=\"$CHICAGO$\"\n\
                 Revision=1\n".to_string()
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
