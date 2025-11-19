use crate::types::Policy;
use std::error::Error;

pub mod secedit;
use secedit::SeceditExecutor;

#[derive(Debug, Clone, PartialEq)]
pub struct AuditResult {
    pub policy_id: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RemediateResult {
    Success(String),
    Failed(String),
}

// ========== PLATFORM DISPATCHERS ==========

pub fn audit_policy(policy: &Policy) -> Result<AuditResult, Box<dyn Error>> {
    match policy.check_type.as_str() {
        "registry_key" => {
            let registry = RealRegistry;
            // Route based on policy ID to the correct registry check function
            match policy.id.as_str() {
                "A.1.b.i" => check_registry_password_history(policy, &registry),
                "A.1.a.ii" => check_min_password_length(policy, &registry),
                "A.1.a.iii" => check_max_password_age(policy, &registry),
                "A.1.a.iv" => check_registry_min_password_length(policy, &registry),
                "A.1.b.ii" => check_account_lockout_threshold(policy, &registry),
                "A.2.a.ii" => check_admin_account_renamed(policy, &registry),
                "A.2.b.i" => check_lockout_threshold(policy, &registry),
                // A.2.b.ii removed - now handled by local_policy
                "A.3.a.i" => check_smb1_disabled(policy, &registry),
                "A.3.a.ii" => check_firewall_domain_profile(policy, &registry),
                "A.3.a.iii" => check_firewall_private_profile(policy, &registry),
                "A.3.a.iv" => check_firewall_public_profile(policy, &registry),
                "A.4.b.i" => check_autoplay_disabled(policy, &registry),
                "A.4.b.ii" => check_remote_registry_disabled(policy, &RealServiceManager),
                "A.5.a.i" => check_restrict_anonymous_sam(policy, &registry),
                "A.5.a.ii" => check_restrict_anonymous(policy, &registry),
                "A.5.b.i" => check_lm_compatibility(policy, &registry),
                "A.7.b.i" => check_uac_elevation(policy, &registry),
                _ => Err(format!("Unknown registry_key policy: {}", policy.id).into()),
            }
        }
        "local_policy" => {
            #[cfg(target_os = "windows")]
            let executor = secedit::RealSeceditExecutor;
            #[cfg(not(target_os = "windows"))]
            let executor = secedit::MockSeceditExecutor::new();
            audit_local_policy(policy, &executor)
        }
        "service_status" => {
            let service_manager = RealServiceManager;
            match policy.id.as_str() {
                "A.4.a.i" => check_termservice_disabled(policy, &service_manager),
                "A.4.a.ii" => check_spooler_disabled(policy, &service_manager),
                "A.4.b.i" => check_w32time_enabled(policy, &service_manager),
                "A.7.a.i" => check_remote_registry_disabled(policy, &service_manager),
                _ => check_service_status(policy, &service_manager),
            }
        }
        _ => Err(format!(
            "Unsupported check_type for Windows platform: {} (policy: {})",
            policy.check_type, policy.id
        )
        .into()),
    }
}

pub fn remediate_policy(policy: &Policy) -> Result<RemediateResult, Box<dyn Error>> {
    // Ensure admin privileges on Windows
    #[cfg(target_os = "windows")]
    {
        use crate::privilege::ensure_admin;
        if let Err(e) = ensure_admin() {
            return Ok(RemediateResult::Failed(e));
        }
    }

    match policy.check_type.as_str() {
        "registry_key" => {
            let registry = RealRegistry;
            match policy.id.as_str() {
                "A.1.b.i" => remediate_registry_password_history(policy, &registry),
                "A.1.a.ii" => remediate_min_password_length(policy, &registry),
                "A.1.a.iii" => remediate_max_password_age(policy, &registry),
                "A.1.a.iv" => remediate_registry_min_password_length(policy, &registry),
                "A.1.b.ii" => remediate_account_lockout_threshold(policy, &registry),
                "A.2.a.ii" => remediate_admin_account_renamed(policy, &registry),
                "A.2.b.i" => remediate_lockout_threshold(policy, &registry),
                // A.2.b.ii removed - now handled by local_policy
                "A.3.a.i" => remediate_smb1_disabled(policy, &registry),
                "A.3.a.ii" => remediate_firewall_domain_profile(policy, &registry),
                "A.3.a.iii" => remediate_firewall_private_profile(policy, &registry),
                "A.3.a.iv" => remediate_firewall_public_profile(policy, &registry),
                "A.4.b.i" => remediate_autoplay_disabled(policy, &registry),
                "A.4.b.ii" => remediate_remote_registry_disabled(policy, &RealServiceManager),
                "A.5.a.i" => remediate_restrict_anonymous_sam(policy, &registry),
                "A.5.a.ii" => remediate_restrict_anonymous(policy, &registry),
                "A.5.b.i" => remediate_lm_compatibility(policy, &registry),
                "A.7.b.i" => remediate_uac_elevation(policy, &registry),
                _ => Err(format!("Unknown registry_key policy: {}", policy.id).into()),
            }
        }
        "local_policy" => {
            #[cfg(target_os = "windows")]
            let executor = secedit::RealSeceditExecutor;
            #[cfg(not(target_os = "windows"))]
            let executor = secedit::MockSeceditExecutor::new();
            remediate_local_policy(policy, &executor)
        }
        "service_status" => {
            let service_manager = RealServiceManager;
            match policy.id.as_str() {
                "A.4.a.i" => remediate_termservice_disabled(policy, &service_manager),
                "A.4.a.ii" => remediate_spooler_disabled(policy, &service_manager),
                "A.4.b.i" => remediate_w32time_enabled(policy, &service_manager),
                "A.7.a.i" => remediate_remote_registry_disabled(policy, &service_manager),
                _ => remediate_service_disable(policy, &service_manager),
            }
        }
        _ => Err(format!(
            "Unsupported check_type for Windows platform: {} (policy: {})",
            policy.check_type, policy.id
        )
        .into()),
    }
}

pub trait ServiceManager {
    fn is_running(&self, service_name: &str) -> Result<bool, Box<dyn Error>>;
    fn is_enabled(&self, service_name: &str) -> Result<bool, Box<dyn Error>>;
    fn stop(&self, service_name: &str) -> Result<(), Box<dyn Error>>;
    fn disable(&self, service_name: &str) -> Result<(), Box<dyn Error>>;
    fn start(&self, service_name: &str) -> Result<(), Box<dyn Error>>;
    fn enable(&self, service_name: &str) -> Result<(), Box<dyn Error>>;
}

pub struct RealServiceManager;

impl ServiceManager for RealServiceManager {
    fn is_running(&self, service_name: &str) -> Result<bool, Box<dyn Error>> {
        #[cfg(windows)]
        {
            use std::process::Command;
            let output = Command::new("sc").args(&["query", service_name]).output()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.contains("RUNNING"))
        }
        #[cfg(not(windows))]
        {
            let _ = service_name;
            Err("Windows service operations not supported on this platform".into())
        }
    }

    fn is_enabled(&self, service_name: &str) -> Result<bool, Box<dyn Error>> {
        #[cfg(windows)]
        {
            use std::process::Command;
            let output = Command::new("sc").args(&["qc", service_name]).output()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(!stdout.contains("DISABLED"))
        }
        #[cfg(not(windows))]
        {
            let _ = service_name;
            Err("Windows service operations not supported on this platform".into())
        }
    }

    fn stop(&self, service_name: &str) -> Result<(), Box<dyn Error>> {
        #[cfg(windows)]
        {
            use std::process::Command;
            let output = Command::new("sc").args(&["stop", service_name]).output()?;
            if output.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Failed to stop service: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into())
            }
        }
        #[cfg(not(windows))]
        {
            let _ = service_name;
            Err("Windows service operations not supported on this platform".into())
        }
    }

    fn disable(&self, service_name: &str) -> Result<(), Box<dyn Error>> {
        #[cfg(windows)]
        {
            use std::process::Command;
            let output = Command::new("sc")
                .args(&["config", service_name, "start=", "disabled"])
                .output()?;
            if output.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Failed to disable service: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into())
            }
        }
        #[cfg(not(windows))]
        {
            let _ = service_name;
            Err("Windows service operations not supported on this platform".into())
        }
    }

    fn start(&self, service_name: &str) -> Result<(), Box<dyn Error>> {
        #[cfg(windows)]
        {
            use std::process::Command;
            let output = Command::new("sc").args(&["start", service_name]).output()?;
            if output.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Failed to start service: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into())
            }
        }
        #[cfg(not(windows))]
        {
            let _ = service_name;
            Err("Windows service operations not supported on this platform".into())
        }
    }

    fn enable(&self, service_name: &str) -> Result<(), Box<dyn Error>> {
        #[cfg(windows)]
        {
            use std::process::Command;
            let output = Command::new("sc")
                .args(&["config", service_name, "start=", "auto"])
                .output()?;
            if output.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Failed to enable service: {}",
                    String::from_utf8_lossy(&output.stderr)
                )
                .into())
            }
        }
        #[cfg(not(windows))]
        {
            let _ = service_name;
            Err("Windows service operations not supported on this platform".into())
        }
    }
}

pub struct MockServiceManager {
    pub running_services: std::cell::RefCell<Vec<String>>,
    pub enabled_services: std::cell::RefCell<Vec<String>>,
    pub stopped_services: std::cell::RefCell<Vec<String>>,
    pub disabled_services: std::cell::RefCell<Vec<String>>,
}

impl Default for MockServiceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MockServiceManager {
    pub fn new() -> Self {
        Self {
            running_services: std::cell::RefCell::new(vec!["RemoteRegistry".to_string()]),
            enabled_services: std::cell::RefCell::new(vec!["RemoteRegistry".to_string()]),
            stopped_services: std::cell::RefCell::new(Vec::new()),
            disabled_services: std::cell::RefCell::new(Vec::new()),
        }
    }
}

impl ServiceManager for MockServiceManager {
    fn is_running(&self, service_name: &str) -> Result<bool, Box<dyn Error>> {
        Ok(self
            .running_services
            .borrow()
            .contains(&service_name.to_string())
            && !self
                .stopped_services
                .borrow()
                .contains(&service_name.to_string()))
    }

    fn is_enabled(&self, service_name: &str) -> Result<bool, Box<dyn Error>> {
        Ok(self
            .enabled_services
            .borrow()
            .contains(&service_name.to_string())
            && !self
                .disabled_services
                .borrow()
                .contains(&service_name.to_string()))
    }

    fn stop(&self, service_name: &str) -> Result<(), Box<dyn Error>> {
        self.stopped_services
            .borrow_mut()
            .push(service_name.to_string());
        self.running_services
            .borrow_mut()
            .retain(|s| s != service_name);
        Ok(())
    }

    fn disable(&self, service_name: &str) -> Result<(), Box<dyn Error>> {
        self.disabled_services
            .borrow_mut()
            .push(service_name.to_string());
        self.enabled_services
            .borrow_mut()
            .retain(|s| s != service_name);
        Ok(())
    }

    fn start(&self, service_name: &str) -> Result<(), Box<dyn Error>> {
        self.stopped_services
            .borrow_mut()
            .retain(|s| s != service_name);
        self.running_services
            .borrow_mut()
            .push(service_name.to_string());
        Ok(())
    }

    fn enable(&self, service_name: &str) -> Result<(), Box<dyn Error>> {
        self.disabled_services
            .borrow_mut()
            .retain(|s| s != service_name);
        self.enabled_services
            .borrow_mut()
            .push(service_name.to_string());
        Ok(())
    }
}

pub fn check_service_status(
    policy: &Policy,
    service_manager: &dyn ServiceManager,
) -> Result<AuditResult, Box<dyn Error>> {
    let service_name = policy
        .service_name
        .as_ref()
        .ok_or("service_name is required for service_status check")?;

    let is_running = service_manager.is_running(service_name)?;
    let is_enabled = service_manager.is_enabled(service_name)?;

    // Extract the expected_state string
    let expected_state_str = match &policy.expected_state {
        Some(crate::types::ExpectedState::String(s)) => s.as_str(),
        Some(crate::types::ExpectedState::Map { .. }) => {
            return Err(format!(
                "check_service_status expected a String variant, got Map for policy {}",
                policy.id
            )
            .into());
        }
        None => "stopped_disabled",
    };

    let passed = match expected_state_str {
        "stopped_disabled" => !is_running && !is_enabled,
        "running" => is_running,
        _ => false,
    };

    let message = if passed {
        format!(
            "Service '{}' is in expected state: {}",
            service_name, expected_state_str
        )
    } else {
        format!(
            "Service '{}' is not in expected state. Running: {}, Enabled: {}",
            service_name, is_running, is_enabled
        )
    };

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message,
    })
}

pub fn remediate_service_disable(
    policy: &Policy,
    service_manager: &dyn ServiceManager,
) -> Result<RemediateResult, Box<dyn Error>> {
    let service_name = policy
        .service_name
        .as_ref()
        .ok_or("service_name is required for service_disable remediation")?;

    match service_manager.stop(service_name) {
        Ok(_) => {}
        Err(e) => {
            return Ok(RemediateResult::Failed(format!(
                "Failed to stop service '{}': {}",
                service_name, e
            )));
        }
    }

    match service_manager.disable(service_name) {
        Ok(_) => {}
        Err(e) => {
            return Ok(RemediateResult::Failed(format!(
                "Failed to disable service '{}': {}",
                service_name, e
            )));
        }
    }

    Ok(RemediateResult::Success(format!(
        "Service '{}' stopped and disabled successfully",
        service_name
    )))
}

// ========== STAGE 4: REGISTRY OPERATIONS ==========

pub trait Registry {
    fn get_dword(&self, path: &str, value_name: &str) -> Result<u32, Box<dyn Error>>;
    fn set_dword(&self, path: &str, value_name: &str, value: u32) -> Result<(), Box<dyn Error>>;
    fn get_string(&self, path: &str, value_name: &str) -> Result<String, Box<dyn Error>>;
    fn set_string(&self, path: &str, value_name: &str, value: &str) -> Result<(), Box<dyn Error>>;
}

#[cfg(target_os = "windows")]
pub struct RealRegistry;

#[cfg(not(target_os = "windows"))]
pub struct RealRegistry;

#[cfg(target_os = "windows")]
fn split_registry_path(path: &str) -> Result<(&str, &str), Box<dyn Error>> {
    let parts: Vec<&str> = path.splitn(2, '\\').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid registry path format: {}", path).into());
    }
    Ok((parts[0], parts[1]))
}

#[cfg(target_os = "windows")]
fn open_hive(hive_name: &str) -> Result<winreg::RegKey, Box<dyn Error>> {
    use winreg::enums::*;
    use winreg::RegKey;

    match hive_name.to_uppercase().as_str() {
        "HKEY_LOCAL_MACHINE" | "HKLM" => Ok(RegKey::predef(HKEY_LOCAL_MACHINE)),
        "HKEY_CURRENT_USER" | "HKCU" => Ok(RegKey::predef(HKEY_CURRENT_USER)),
        _ => Err(format!(
            "Unsupported registry hive: {}. Only HKLM and HKCU are supported.",
            hive_name
        )
        .into()),
    }
}

#[cfg(target_os = "windows")]
impl Registry for RealRegistry {
    fn get_dword(&self, path: &str, value_name: &str) -> Result<u32, Box<dyn Error>> {
        use winreg::enums::*;

        let (hive_name, subkey) = split_registry_path(path)?;
        let hive = open_hive(hive_name)?;
        let key = hive.open_subkey_with_flags(subkey, KEY_READ)?;
        let value: u32 = key.get_value(value_name)?;
        Ok(value)
    }

    fn set_dword(&self, path: &str, value_name: &str, value: u32) -> Result<(), Box<dyn Error>> {
        use winreg::enums::*;

        let (hive_name, subkey) = split_registry_path(path)?;
        let hive = open_hive(hive_name)?;
        let key = hive.open_subkey_with_flags(subkey, KEY_WRITE)?;
        key.set_value(value_name, &value)?;
        Ok(())
    }

    fn get_string(&self, path: &str, value_name: &str) -> Result<String, Box<dyn Error>> {
        use winreg::enums::*;

        let (hive_name, subkey) = split_registry_path(path)?;
        let hive = open_hive(hive_name)?;
        let key = hive.open_subkey_with_flags(subkey, KEY_READ)?;
        let value: String = key.get_value(value_name)?;
        Ok(value)
    }

    fn set_string(&self, path: &str, value_name: &str, value: &str) -> Result<(), Box<dyn Error>> {
        use winreg::enums::*;

        let (hive_name, subkey) = split_registry_path(path)?;
        let hive = open_hive(hive_name)?;
        let key = hive.open_subkey_with_flags(subkey, KEY_WRITE)?;
        key.set_value(value_name, &value)?;
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
impl Registry for RealRegistry {
    fn get_dword(&self, _path: &str, _value_name: &str) -> Result<u32, Box<dyn Error>> {
        Err("Registry operations not supported on this platform".into())
    }

    fn set_dword(&self, _path: &str, _value_name: &str, _value: u32) -> Result<(), Box<dyn Error>> {
        Err("Registry operations not supported on this platform".into())
    }

    fn get_string(&self, _path: &str, _value_name: &str) -> Result<String, Box<dyn Error>> {
        Err("Registry operations not supported on this platform".into())
    }

    fn set_string(
        &self,
        _path: &str,
        _value_name: &str,
        _value: &str,
    ) -> Result<(), Box<dyn Error>> {
        Err("Registry operations not supported on this platform".into())
    }
}

pub struct MockRegistry {
    pub dword_values: std::cell::RefCell<std::collections::HashMap<(String, String), u32>>,
    pub string_values: std::cell::RefCell<std::collections::HashMap<(String, String), String>>,
}

impl Default for MockRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MockRegistry {
    pub fn new() -> Self {
        MockRegistry {
            dword_values: std::cell::RefCell::new(std::collections::HashMap::new()),
            string_values: std::cell::RefCell::new(std::collections::HashMap::new()),
        }
    }

    pub fn set_mock_dword(&mut self, path: &str, value_name: &str, value: u32) {
        self.dword_values
            .borrow_mut()
            .insert((path.to_string(), value_name.to_string()), value);
    }

    pub fn set_mock_string(&mut self, path: &str, value_name: &str, value: &str) {
        self.string_values.borrow_mut().insert(
            (path.to_string(), value_name.to_string()),
            value.to_string(),
        );
    }
}

impl Registry for MockRegistry {
    fn get_dword(&self, path: &str, value_name: &str) -> Result<u32, Box<dyn Error>> {
        self.dword_values
            .borrow()
            .get(&(path.to_string(), value_name.to_string()))
            .copied()
            .ok_or_else(|| format!("Registry value not found: {}\\{}", path, value_name).into())
    }

    fn set_dword(&self, path: &str, value_name: &str, value: u32) -> Result<(), Box<dyn Error>> {
        self.dword_values
            .borrow_mut()
            .insert((path.to_string(), value_name.to_string()), value);
        Ok(())
    }

    fn get_string(&self, path: &str, value_name: &str) -> Result<String, Box<dyn Error>> {
        self.string_values
            .borrow()
            .get(&(path.to_string(), value_name.to_string()))
            .cloned()
            .ok_or_else(|| format!("Registry value not found: {}\\{}", path, value_name).into())
    }

    fn set_string(&self, path: &str, value_name: &str, value: &str) -> Result<(), Box<dyn Error>> {
        self.string_values.borrow_mut().insert(
            (path.to_string(), value_name.to_string()),
            value.to_string(),
        );
        Ok(())
    }
}

// ========== LOCAL SECURITY POLICY (SECEDIT) OPERATIONS ==========

/// Audit a local security policy using secedit executor
pub fn audit_local_policy<E: SeceditExecutor>(
    policy: &Policy,
    executor: &E,
) -> Result<AuditResult, Box<dyn Error>> {
    let policy_name = policy
        .policy_name
        .as_ref()
        .ok_or("policy_name is required for local_policy check")?;

    // Map policy_name to INF key name
    let inf_key = match policy_name.as_str() {
        "LockoutDuration" => "LockoutDuration",
        "LockoutBadCount" => "LockoutBadCount",
        "PasswordComplexity" => "PasswordComplexity",
        "EnableGuestAccount" => "EnableGuestAccount",
        "MinimumPasswordLength" => "MinimumPasswordLength",
        "MaximumPasswordAge" => "MaximumPasswordAge",
        "MinimumPasswordAge" => "MinimumPasswordAge",
        "PasswordHistorySize" => "PasswordHistorySize",
        "LmCompatibilityLevel" => "LmCompatibilityLevel",
        "ConsentPromptBehaviorAdmin" => "ConsentPromptBehaviorAdmin",
        _ => return Err(format!("Unknown policy_name: {}", policy_name).into()),
    };

    // Export current security policy via executor
    let inf_content = executor.export_security_policy()?;

    // Parse the INF content
    let mut in_system_access = false;
    let mut found_value: Option<String> = None;

    for line in inf_content.lines() {
        let trimmed = line.trim();

        // Track section
        if trimmed.starts_with('[') {
            in_system_access = trimmed.eq_ignore_ascii_case("[System Access]");
            continue;
        }

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with(';') {
            continue;
        }

        // Parse key=value in [System Access] section
        if in_system_access {
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim();
                let value = trimmed[eq_pos + 1..].trim();

                if key.eq_ignore_ascii_case(inf_key) {
                    found_value = Some(value.to_string());
                    break;
                }
            }
        }
    }

    // Compare with expected state
    let current_value = found_value.ok_or(format!(
        "Policy '{}' not found in secedit export",
        policy_name
    ))?;

    let passed = if let Some(expected) = &policy.expected_state {
        match expected {
            crate::types::ExpectedState::String(expected_str) => current_value == *expected_str,
            crate::types::ExpectedState::Map { operator, value } => {
                let current_num: i64 = current_value.parse().map_err(|_| {
                    format!(
                        "Failed to parse current value '{}' as number",
                        current_value
                    )
                })?;

                // Convert serde_yaml::Value to i64
                let expected_num: i64 = match value {
                    serde_yaml::Value::Number(n) => {
                        n.as_i64().ok_or("Expected value is not a valid integer")?
                    }
                    serde_yaml::Value::String(s) => s
                        .parse()
                        .map_err(|_| "Failed to parse expected value as integer")?,
                    _ => return Err("Expected value must be a number or string".into()),
                };

                match operator.as_str() {
                    "eq" => current_num == expected_num,
                    "gte" => current_num >= expected_num,
                    "lte" => current_num <= expected_num,
                    "ne" => current_num != expected_num,
                    _ => return Err(format!("Unknown operator: {}", operator).into()),
                }
            }
        }
    } else {
        return Err("expected_state is required for local_policy check".into());
    };

    let message = if passed {
        format!(
            "Local policy '{}' is compliant: {} (expected)",
            policy_name, current_value
        )
    } else {
        format!(
            "Local policy '{}' is not compliant: found {}, expected {:?}",
            policy_name, current_value, policy.expected_state
        )
    };

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message,
    })
}

/// Remediate a local security policy using secedit executor
pub fn remediate_local_policy<E: SeceditExecutor>(
    policy: &Policy,
    executor: &E,
) -> Result<RemediateResult, Box<dyn Error>> {
    let policy_name = policy
        .policy_name
        .as_ref()
        .ok_or("policy_name is required for local_policy remediation")?;

    let set_value = policy
        .set_value
        .as_ref()
        .ok_or("set_value is required for local_policy remediation")?;

    // Convert serde_yaml::Value to string
    let set_value_str = match set_value {
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
        _ => return Err("set_value must be a number, string, or boolean".into()),
    };

    // Map policy_name to INF key name
    let inf_key = match policy_name.as_str() {
        "LockoutDuration" => "LockoutDuration",
        "LockoutBadCount" => "LockoutBadCount",
        "PasswordComplexity" => "PasswordComplexity",
        "EnableGuestAccount" => "EnableGuestAccount",
        "MinimumPasswordLength" => "MinimumPasswordLength",
        "MaximumPasswordAge" => "MaximumPasswordAge",
        "MinimumPasswordAge" => "MinimumPasswordAge",
        "PasswordHistorySize" => "PasswordHistorySize",
        "LmCompatibilityLevel" => "LmCompatibilityLevel",
        "ConsentPromptBehaviorAdmin" => "ConsentPromptBehaviorAdmin",
        _ => return Err(format!("Unknown policy_name: {}", policy_name).into()),
    };

    // Create INF content
    let inf_content = format!(
        "[Unicode]\nUnicode=yes\n\n[System Access]\n{} = {}\n\n[Version]\nsignature=\"$CHICAGO$\"\nRevision=1\n",
        inf_key, set_value_str
    );

    // Apply the policy via executor
    if let Err(e) = executor.configure_security_policy(&inf_content) {
        if e.to_string().contains("Administrator privileges required") {
            return Ok(RemediateResult::Failed(
                "Administrator privileges required to modify Local Security Policy".to_string(),
            ));
        }
        return Ok(RemediateResult::Failed(format!(
            "Failed to configure security policy: {}",
            e
        )));
    }

    // Verify the change
    match audit_local_policy(policy, executor) {
        Ok(audit_result) if audit_result.passed => Ok(RemediateResult::Success(format!(
            "Local policy '{}' set to {} successfully",
            policy_name, set_value_str
        ))),
        Ok(audit_result) => Ok(RemediateResult::Failed(format!(
            "Policy applied but verification failed: {}",
            audit_result.message
        ))),
        Err(e) => Ok(RemediateResult::Failed(format!(
            "Policy applied but verification error: {}",
            e
        ))),
    }
}

// 1) A.1.a.i — Enforce password history
pub fn check_registry_password_history<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy
        .target_path
        .as_ref()
        .ok_or("target_path required for registry check")?;
    let value_name = policy
        .value_name
        .as_ref()
        .ok_or("value_name required for registry check")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let expected_min = 24;

    let passed = current_value >= expected_min;
    let message = if passed {
        format!(
            "Password history is configured correctly: {} (expected >= {})",
            current_value, expected_min
        )
    } else {
        format!(
            "Password history is insufficient: {} (expected >= {})",
            current_value, expected_min
        )
    };

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message,
    })
}

pub fn remediate_registry_password_history<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy
        .target_path
        .as_ref()
        .ok_or("target_path required for registry remediation")?;
    let value_name = policy
        .value_name
        .as_ref()
        .ok_or("value_name required for registry remediation")?;

    registry.set_dword(target_path, value_name, 24)?;

    Ok(RemediateResult::Success(format!(
        "Password history set to 24 at {}\\{}",
        target_path, value_name
    )))
}

// 2) A.1.a.iv — Minimum password length (12)
pub fn check_registry_min_password_length<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy
        .target_path
        .as_ref()
        .ok_or("target_path required for registry check")?;
    let value_name = policy
        .value_name
        .as_ref()
        .ok_or("value_name required for registry check")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let expected_min = 12;

    let passed = current_value >= expected_min;
    let message = if passed {
        format!(
            "Minimum password length is configured correctly: {} (expected >= {})",
            current_value, expected_min
        )
    } else {
        format!(
            "Minimum password length is insufficient: {} (expected >= {})",
            current_value, expected_min
        )
    };

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message,
    })
}

pub fn remediate_registry_min_password_length<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy
        .target_path
        .as_ref()
        .ok_or("target_path required for registry remediation")?;
    let value_name = policy
        .value_name
        .as_ref()
        .ok_or("value_name required for registry remediation")?;

    registry.set_dword(target_path, value_name, 12)?;

    Ok(RemediateResult::Success(format!(
        "Minimum password length set to 12 at {}\\{}",
        target_path, value_name
    )))
}

// 3) A.1.b.ii — Account lockout threshold
pub fn check_account_lockout_threshold<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy
        .target_path
        .as_ref()
        .ok_or("target_path required for policy check")?;
    let value_name = policy
        .value_name
        .as_ref()
        .ok_or("value_name required for policy check")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let expected_max = 5;

    let passed = current_value <= expected_max && current_value > 0;
    let message = if passed {
        format!(
            "Account lockout threshold is configured correctly: {} (expected <= {} and > 0)",
            current_value, expected_max
        )
    } else {
        format!(
            "Account lockout threshold is misconfigured: {} (expected <= {} and > 0)",
            current_value, expected_max
        )
    };

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message,
    })
}

pub fn remediate_account_lockout_threshold<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy
        .target_path
        .as_ref()
        .ok_or("target_path required for policy remediation")?;
    let value_name = policy
        .value_name
        .as_ref()
        .ok_or("value_name required for policy remediation")?;

    registry.set_dword(target_path, value_name, 5)?;

    Ok(RemediateResult::Success(format!(
        "Account lockout threshold set to 5 at {}\\{}",
        target_path, value_name
    )))
}

// 4) A.4.b.xxx — Disable Windows Error Reporting Service (WerSvc)
pub fn check_service_wersvc<S: ServiceManager>(
    policy: &Policy,
    service_manager: &S,
) -> Result<AuditResult, Box<dyn Error>> {
    check_service_status(policy, service_manager)
}

pub fn remediate_service_wersvc<S: ServiceManager>(
    policy: &Policy,
    service_manager: &S,
) -> Result<RemediateResult, Box<dyn Error>> {
    remediate_service_disable(policy, service_manager)
}

// 5) A.5.a.i — Firewall private profile enabled
pub fn check_firewall_private_profile<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy
        .target_path
        .as_ref()
        .ok_or("target_path required for firewall check")?;
    let value_name = policy
        .value_name
        .as_ref()
        .ok_or("value_name required for firewall check")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let expected_enabled = 1;

    let passed = current_value == expected_enabled;
    let message = if passed {
        "Windows Firewall private profile is enabled".to_string()
    } else {
        format!(
            "Windows Firewall private profile is disabled: {} (expected 1)",
            current_value
        )
    };

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message,
    })
}

pub fn remediate_firewall_private_profile<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy
        .target_path
        .as_ref()
        .ok_or("target_path required for firewall remediation")?;
    let value_name = policy
        .value_name
        .as_ref()
        .ok_or("value_name required for firewall remediation")?;

    registry.set_dword(target_path, value_name, 1)?;

    Ok(RemediateResult::Success(format!(
        "Windows Firewall private profile enabled at {}\\{}",
        target_path, value_name
    )))
}

// 6) A.2.a.v — Backup files and directories right
pub fn check_backup_privilege<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy
        .target_path
        .as_ref()
        .ok_or("target_path required for privilege check")?;
    let value_name = policy
        .value_name
        .as_ref()
        .ok_or("value_name required for privilege check")?;

    let current_value = registry.get_string(target_path, value_name)?;
    let expected = "Administrators";

    let passed = current_value.contains(expected);
    let message = if passed {
        format!("SeBackupPrivilege is correctly assigned: {}", current_value)
    } else {
        format!(
            "SeBackupPrivilege is misconfigured: {} (expected to contain '{}')",
            current_value, expected
        )
    };

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message,
    })
}

pub fn remediate_backup_privilege<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy
        .target_path
        .as_ref()
        .ok_or("target_path required for privilege remediation")?;
    let value_name = policy
        .value_name
        .as_ref()
        .ok_or("value_name required for privilege remediation")?;

    registry.set_string(target_path, value_name, "Administrators")?;

    Ok(RemediateResult::Success(format!(
        "SeBackupPrivilege set to Administrators at {}\\{}",
        target_path, value_name
    )))
}

// ========== Additional Registry-Based Policies ==========

// A.1.a.i: Password complexity (local_policy simulated via registry for testing)
pub fn check_password_complexity<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let path = "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Lsa";
    let value_name = "PasswordComplexity";

    let current_value = registry.get_dword(path, value_name)?;
    let passed = current_value == 1;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "Password complexity: {} (expected: 1) at {}\\{}",
            current_value, path, value_name
        ),
    })
}

pub fn remediate_password_complexity<R: Registry>(
    _policy: &Policy, // TODO: remove allow - not yet wired to engine
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let path = "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Lsa";
    let value_name = "PasswordComplexity";

    registry.set_dword(path, value_name, 1)?;

    Ok(RemediateResult::Success(format!(
        "Password complexity enabled at {}\\{}",
        path, value_name
    )))
}

// A.1.a.ii: Minimum password length
pub fn check_min_password_length<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let passed = current_value >= 14;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "Minimum password length: {} (expected: >=14) at {}\\{}",
            current_value, target_path, value_name
        ),
    })
}

pub fn remediate_min_password_length<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    registry.set_dword(target_path, value_name, 14)?;

    Ok(RemediateResult::Success(format!(
        "Minimum password length set to 14 at {}\\{}",
        target_path, value_name
    )))
}

// A.1.a.iii: Maximum password age
pub fn check_max_password_age<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let passed = current_value > 0 && current_value <= 60;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "Maximum password age: {} days (expected: <=60 and >0) at {}\\{}",
            current_value, target_path, value_name
        ),
    })
}

pub fn remediate_max_password_age<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    registry.set_dword(target_path, value_name, 60)?;

    Ok(RemediateResult::Success(format!(
        "Maximum password age set to 60 days at {}\\{}",
        target_path, value_name
    )))
}

// A.2.a.ii: Rename Administrator account
pub fn check_admin_account_renamed<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    let current_value = registry.get_string(target_path, value_name)?;
    let passed = current_value != "Administrator";

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "Default username: '{}' (expected: not 'Administrator') at {}\\{}",
            current_value, target_path, value_name
        ),
    })
}

pub fn remediate_admin_account_renamed<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    registry.set_string(target_path, value_name, "WinAdmin")?;

    Ok(RemediateResult::Success(format!(
        "Administrator account renamed to WinAdmin at {}\\{}",
        target_path, value_name
    )))
}

// A.2.b.i: Account lockout threshold
pub fn check_lockout_threshold<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let passed = current_value > 0 && current_value <= 5;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "Account lockout threshold: {} (expected: 1-5) at {}\\{}",
            current_value, target_path, value_name
        ),
    })
}

pub fn remediate_lockout_threshold<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    registry.set_dword(target_path, value_name, 5)?;

    Ok(RemediateResult::Success(format!(
        "Account lockout threshold set to 5 at {}\\{}",
        target_path, value_name
    )))
}

// A.2.b.ii: Account lockout duration
pub fn check_lockout_duration<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let passed = current_value >= 15;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "Account lockout duration: {} minutes (expected: >=15) at {}\\{}",
            current_value, target_path, value_name
        ),
    })
}

pub fn remediate_lockout_duration<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    registry.set_dword(target_path, value_name, 15)?;

    Ok(RemediateResult::Success(format!(
        "Account lockout duration set to 15 minutes at {}\\{}",
        target_path, value_name
    )))
}

// A.3.a.i: Disable SMBv1
pub fn check_smb1_disabled<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let passed = current_value == 0;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "SMBv1: {} (expected: 0/disabled) at {}\\{} (reboot required after change)",
            current_value, target_path, value_name
        ),
    })
}

pub fn remediate_smb1_disabled<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    registry.set_dword(target_path, value_name, 0)?;

    Ok(RemediateResult::Success(format!(
        "SMBv1 disabled at {}\\{} (reboot required)",
        target_path, value_name
    )))
}

// A.3.a.ii: Firewall Domain profile
pub fn check_firewall_domain_profile<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let passed = current_value == 1;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "Firewall Domain profile: {} (expected: 1/enabled) at {}\\{}",
            current_value, target_path, value_name
        ),
    })
}

pub fn remediate_firewall_domain_profile<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    registry.set_dword(target_path, value_name, 1)?;

    Ok(RemediateResult::Success(format!(
        "Firewall Domain profile enabled at {}\\{}",
        target_path, value_name
    )))
}

// A.3.a.iv: Firewall Public profile
pub fn check_firewall_public_profile<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let passed = current_value == 1;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "Firewall Public profile: {} (expected: 1/enabled) at {}\\{}",
            current_value, target_path, value_name
        ),
    })
}

pub fn remediate_firewall_public_profile<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    registry.set_dword(target_path, value_name, 1)?;

    Ok(RemediateResult::Success(format!(
        "Firewall Public profile enabled at {}\\{}",
        target_path, value_name
    )))
}

// A.5.a.i: Restrict anonymous SID/Name translation
pub fn check_restrict_anonymous_sam<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let passed = current_value == 1;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "Restrict anonymous SAM: {} (expected: 1/restricted) at {}\\{}",
            current_value, target_path, value_name
        ),
    })
}

pub fn remediate_restrict_anonymous_sam<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    registry.set_dword(target_path, value_name, 1)?;

    Ok(RemediateResult::Success(format!(
        "Anonymous SAM restriction enabled at {}\\{}",
        target_path, value_name
    )))
}

// A.5.a.ii: Restrict anonymous enumeration
pub fn check_restrict_anonymous<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let passed = current_value == 1;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "Restrict anonymous: {} (expected: 1/restricted) at {}\\{}",
            current_value, target_path, value_name
        ),
    })
}

pub fn remediate_restrict_anonymous<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    registry.set_dword(target_path, value_name, 1)?;

    Ok(RemediateResult::Success(format!(
        "Anonymous enumeration restriction enabled at {}\\{}",
        target_path, value_name
    )))
}

// A.5.b.i: LAN Manager authentication level
pub fn check_lm_compatibility<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let passed = current_value >= 5;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "LM compatibility level: {} (expected: >=5/NTLMv2 only) at {}\\{}",
            current_value, target_path, value_name
        ),
    })
}

pub fn remediate_lm_compatibility<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    registry.set_dword(target_path, value_name, 5)?;

    Ok(RemediateResult::Success(format!(
        "LM compatibility level set to 5 (NTLMv2 only) at {}\\{}",
        target_path, value_name
    )))
}

// A.7.b.i: UAC elevation prompt
pub fn check_uac_elevation<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let passed = current_value >= 2;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "UAC elevation prompt: {} (expected: >=2) at {}\\{}",
            current_value, target_path, value_name
        ),
    })
}

pub fn remediate_uac_elevation<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    registry.set_dword(target_path, value_name, 2)?;

    Ok(RemediateResult::Success(format!(
        "UAC elevation prompt set to 2 at {}\\{}",
        target_path, value_name
    )))
}

// A.8.a.i: Disable AutoPlay
pub fn check_autoplay_disabled<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<AuditResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    let current_value = registry.get_dword(target_path, value_name)?;
    let passed = current_value == 255;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "AutoPlay disabled: {} (expected: 255) at {}\\{}",
            current_value, target_path, value_name
        ),
    })
}

pub fn remediate_autoplay_disabled<R: Registry>(
    policy: &Policy,
    registry: &R,
) -> Result<RemediateResult, Box<dyn Error>> {
    let target_path = policy.target_path.as_ref().ok_or("target_path required")?;
    let value_name = policy.value_name.as_ref().ok_or("value_name required")?;

    registry.set_dword(target_path, value_name, 255)?;

    Ok(RemediateResult::Success(format!(
        "AutoPlay disabled for all drives at {}\\{}",
        target_path, value_name
    )))
}

// ========== Service-Based Policies ==========

// A.4.a.i: Disable Remote Desktop
pub fn check_termservice_disabled<S: ServiceManager>(
    policy: &Policy,
    service_manager: &S,
) -> Result<AuditResult, Box<dyn Error>> {
    check_service_status(policy, service_manager)
}

pub fn remediate_termservice_disabled<S: ServiceManager>(
    policy: &Policy,
    service_manager: &S,
) -> Result<RemediateResult, Box<dyn Error>> {
    remediate_service_disable(policy, service_manager)
}

// A.4.a.ii: Disable Print Spooler
pub fn check_spooler_disabled<S: ServiceManager>(
    policy: &Policy,
    service_manager: &S,
) -> Result<AuditResult, Box<dyn Error>> {
    check_service_status(policy, service_manager)
}

pub fn remediate_spooler_disabled<S: ServiceManager>(
    policy: &Policy,
    service_manager: &S,
) -> Result<RemediateResult, Box<dyn Error>> {
    remediate_service_disable(policy, service_manager)
}

// A.4.b.i: Enable Windows Time service
pub fn check_w32time_enabled<S: ServiceManager>(
    policy: &Policy,
    service_manager: &S,
) -> Result<AuditResult, Box<dyn Error>> {
    let service_name = policy
        .service_name
        .as_ref()
        .ok_or("service_name required")?;

    let is_running = service_manager.is_running(service_name)?;
    let is_enabled = service_manager.is_enabled(service_name)?;
    let passed = is_running && is_enabled;

    Ok(AuditResult {
        policy_id: policy.id.clone(),
        passed,
        message: format!(
            "Service '{}': running={}, enabled={} (expected: both true)",
            service_name, is_running, is_enabled
        ),
    })
}

pub fn remediate_w32time_enabled<S: ServiceManager>(
    policy: &Policy,
    service_manager: &S,
) -> Result<RemediateResult, Box<dyn Error>> {
    let service_name = policy
        .service_name
        .as_ref()
        .ok_or("service_name required")?;

    // Start and enable the service
    match service_manager.start(service_name) {
        Ok(_) => {}
        Err(e) => {
            return Ok(RemediateResult::Failed(format!(
                "Failed to start service '{}': {}",
                service_name, e
            )));
        }
    }

    match service_manager.enable(service_name) {
        Ok(_) => {}
        Err(e) => {
            return Ok(RemediateResult::Failed(format!(
                "Failed to enable service '{}': {}",
                service_name, e
            )));
        }
    }

    Ok(RemediateResult::Success(format!(
        "Service '{}' started and enabled successfully",
        service_name
    )))
}

// A.7.a.i: Disable Remote Registry
pub fn check_remote_registry_disabled<S: ServiceManager>(
    policy: &Policy,
    service_manager: &S,
) -> Result<AuditResult, Box<dyn Error>> {
    check_service_status(policy, service_manager)
}

pub fn remediate_remote_registry_disabled<S: ServiceManager>(
    policy: &Policy,
    service_manager: &S,
) -> Result<RemediateResult, Box<dyn Error>> {
    remediate_service_disable(policy, service_manager)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Policy;

    #[test]
    fn test_check_service_status_vulnerable() {
        let policy = Policy {
            id: "A.4.b.x".to_string(),
            title: Some("Disable Remote Registry".to_string()),
            description: Some("Ensure Remote Registry service is disabled".to_string()),
            platform: "windows".to_string(),
            severity: Some("high".to_string()),
            reversible: Some(true),
            check_type: "service_status".to_string(),
            service_name: Some("RemoteRegistry".to_string()),
            expected_state: Some(crate::types::ExpectedState::String(
                "stopped_disabled".into(),
            )),
            remediate_type: Some("service_disable".to_string()),
            ..Default::default()
        };

        let service_manager = MockServiceManager::new();
        let result = check_service_status(&policy, &service_manager).unwrap();

        assert_eq!(result.policy_id, "A.4.b.x");
        assert!(!result.passed);
        assert!(result.message.contains("not in expected state"));
    }

    #[test]
    fn test_remediate_service_disable_success() {
        let policy = Policy {
            id: "A.4.b.x".to_string(),
            service_name: Some("RemoteRegistry".to_string()),
            ..Default::default()
        };

        let service_manager = MockServiceManager::new();
        let result = remediate_service_disable(&policy, &service_manager).unwrap();

        match result {
            RemediateResult::Success(msg) => {
                assert!(msg.contains("stopped and disabled successfully"));
            }
            RemediateResult::Failed(_) => panic!("Expected success"),
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_real_registry_simple_roundtrip() {
        use winreg::enums::*;
        use winreg::RegKey;

        // Create a test subkey under HKCU\Software\NoGapTest
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let test_subkey = "Software\\NoGapTest";
        let (_key, _disp) = hkcu
            .create_subkey(test_subkey)
            .expect("Failed to create test subkey");

        // Write a DWORD using RealRegistry
        let registry = RealRegistry;
        let test_path = format!("HKCU\\{}", test_subkey);
        let test_value_name = "TestDword";
        let test_value = 42u32;

        registry
            .set_dword(&test_path, test_value_name, test_value)
            .expect("Failed to set DWORD");

        // Read it back using RealRegistry
        let read_value = registry
            .get_dword(&test_path, test_value_name)
            .expect("Failed to get DWORD");

        assert_eq!(read_value, test_value, "DWORD roundtrip failed");

        // Delete the subkey at the end
        hkcu.delete_subkey_all(test_subkey)
            .expect("Failed to delete test subkey");
    }
}
