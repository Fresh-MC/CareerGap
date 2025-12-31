#[cfg(not(target_os = "windows"))]
use crate::platforms::linux;
#[cfg(target_os = "windows")]
use crate::platforms::windows;
use crate::snapshot;
use crate::types::Policy;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditResult {
    pub policy_id: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RemediateResult {
    Success { policy_id: String, message: String },
    Failed { policy_id: String, message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RollbackResult {
    pub policy_id: String,
    pub success: bool,
    pub message: String,
}

/// Trait for exporting and applying policy state for rollback
pub trait PolicyStateProvider {
    fn export_state(&self, policy: &Policy) -> Result<Value, Box<dyn Error>>;
    fn apply_state(&self, policy: &Policy, state: Value) -> Result<(), Box<dyn Error>>;
}

pub trait SnapshotProvider {
    fn save_snapshot(
        &self,
        policy_id: &str,
        stage: &str,
        context: &str,
    ) -> Result<(), Box<dyn Error>>;
}

pub struct RealSnapshotProvider;

impl SnapshotProvider for RealSnapshotProvider {
    fn save_snapshot(
        &self,
        policy_id: &str,
        stage: &str,
        context: &str,
    ) -> Result<(), Box<dyn Error>> {
        // For real implementation, we would use snapshot::init_db() and save_snapshot()
        // For now, just log the snapshot
        println!("📸 Snapshot [{}] - {}: {}", stage, policy_id, context);
        Ok(())
    }
}

pub struct MockSnapshotProvider {
    pub snapshot_count: std::sync::Mutex<usize>,
}

impl Default for MockSnapshotProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSnapshotProvider {
    pub fn new() -> Self {
        Self {
            snapshot_count: std::sync::Mutex::new(0),
        }
    }

    pub fn get_count(&self) -> usize {
        *self.snapshot_count.lock().unwrap()
    }
}

impl SnapshotProvider for MockSnapshotProvider {
    fn save_snapshot(
        &self,
        _policy_id: &str,
        _stage: &str,
        _context: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut count = self.snapshot_count.lock().unwrap();
        *count += 1;
        Ok(())
    }
}

pub fn audit(policies: &[Policy]) -> Result<Vec<AuditResult>, Box<dyn Error>> {
    let mut results = Vec::new();

    for policy in policies {
        let result = match policy.platform.as_str() {
            "windows" => {
                #[cfg(target_os = "windows")]
                {
                    let platform_result = windows::audit_policy(policy)?;
                    AuditResult {
                        policy_id: platform_result.policy_id,
                        passed: platform_result.passed,
                        message: platform_result.message,
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    return Err(format!(
                        "Windows policy '{}' cannot run on this platform",
                        policy.id
                    )
                    .into());
                }
            }
            "linux" => {
                #[cfg(not(target_os = "windows"))]
                {
                    let platform_result = linux::audit_policy(policy)?;
                    AuditResult {
                        policy_id: platform_result.policy_id,
                        passed: platform_result.passed,
                        message: platform_result.message,
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    return Err(
                        format!("Linux policy '{}' cannot run on Windows", policy.id).into(),
                    );
                }
            }
            _ => {
                return Err(format!(
                    "Unsupported platform: {} (policy: {})",
                    policy.platform, policy.id
                )
                .into())
            }
        };

        results.push(result);
    }

    Ok(results)
}

pub fn remediate(
    policies: &[Policy],
    snapshot_provider: &dyn SnapshotProvider,
) -> Result<Vec<RemediateResult>, Box<dyn Error>> {
    let mut results = Vec::new();

    for policy in policies {
        let context = format!(
            "Remediating policy {} on platform {}",
            policy.id, policy.platform
        );

        snapshot_provider.save_snapshot(&policy.id, "BEFORE", &context)?;

        // Delegate all check_type routing to platform-specific dispatchers
        let remediate_result = match policy.platform.as_str() {
            "windows" => {
                #[cfg(target_os = "windows")]
                {
                    let platform_result = windows::remediate_policy(policy)?;
                    match platform_result {
                        windows::RemediateResult::Success(msg) => RemediateResult::Success {
                            policy_id: policy.id.clone(),
                            message: msg,
                        },
                        windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                            policy_id: policy.id.clone(),
                            message: msg,
                        },
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    return Err(format!(
                        "Windows policy '{}' cannot run on this platform",
                        policy.id
                    )
                    .into());
                }
            }
            "linux" => {
                #[cfg(not(target_os = "windows"))]
                {
                    let platform_result = linux::remediate_policy(policy)?;
                    match platform_result {
                        linux::RemediateResult::Success(msg) => RemediateResult::Success {
                            policy_id: policy.id.clone(),
                            message: msg,
                        },
                        linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                            policy_id: policy.id.clone(),
                            message: msg,
                        },
                    }
                }
                #[cfg(target_os = "windows")]
                {
                    return Err(
                        format!("Linux policy '{}' cannot run on Windows", policy.id).into(),
                    );
                }
            }
            _ => {
                return Err(format!(
                    "Unsupported platform: {} (policy: {})",
                    policy.platform, policy.id
                )
                .into())
            }
        };

        snapshot_provider.save_snapshot(&policy.id, "AFTER", &context)?;

        results.push(remediate_result);
    }

    Ok(results)
}

/// Default implementation of PolicyStateProvider
pub struct DefaultPolicyStateProvider;

// Platform state export/apply helpers (minimal implementations)
#[cfg(target_os = "windows")]
mod platform_helpers {
    use super::*;
    use crate::platforms::windows::secedit::SeceditExecutor;
    
    pub fn export_registry_state(_target_path: &str, _value_name: &str) -> Result<Value, Box<dyn Error>> {
        // Stub: Return mock state for now
        // In production, this would call windows module's registry read functions
        Ok(serde_json::json!({
            "type": "registry",
            "value": "stub_value"
        }))
    }
    
    pub fn apply_registry_state(_target_path: &str, _value_name: &str, _state: &Value) -> Result<(), Box<dyn Error>> {
        // Stub: In production, this would call windows module's registry write functions
        Ok(())
    }
    
    pub fn export_secedit_state(policy_name: &str) -> Result<Value, Box<dyn Error>> {
        use crate::platforms::windows::secedit;
        let executor = secedit::RealSeceditExecutor;
        let cfg = executor.export_security_policy()?;
        
        if let Some(value) = secedit::parse_secedit_value(&cfg, policy_name) {
            Ok(serde_json::json!({
                "type": "local_policy",
                "policy": policy_name,
                "value": value
            }))
        } else {
            Err(format!("Policy {} not found in secedit export", policy_name).into())
        }
    }
    
    pub fn apply_secedit_state(policy_name: &str, state: &Value) -> Result<(), Box<dyn Error>> {
        use crate::platforms::windows::secedit;
        let executor = secedit::RealSeceditExecutor;
        
        let value_str = state.get("value")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'value' in state")?;
        
        let cfg = executor.export_security_policy()?;
        let updated_cfg = secedit::update_secedit_cfg(&cfg, policy_name, value_str)?;
        executor.configure_security_policy(&updated_cfg)
    }
    
    pub fn export_service_state(_service_name: &str) -> Result<Value, Box<dyn Error>> {
        // Stub: Return mock state
        Ok(serde_json::json!({
            "type": "service",
            "state": "stub_state"
        }))
    }
    
    pub fn apply_service_state(_service_name: &str, _state: &Value) -> Result<(), Box<dyn Error>> {
        // Stub: In production, this would call service control functions
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
mod platform_helpers {
    use super::*;
    
    pub fn export_sysctl_state(_key: &str) -> Result<Value, Box<dyn Error>> {
        // Stub: Return mock state
        Ok(serde_json::json!({
            "type": "sysctl",
            "value": "stub_value"
        }))
    }
    
    pub fn apply_sysctl_state(_key: &str, _state: &Value) -> Result<(), Box<dyn Error>> {
        // Stub: In production, this would call sysctl write functions
        Ok(())
    }
    
    pub fn export_service_state(_service_name: &str) -> Result<Value, Box<dyn Error>> {
        // Stub: Return mock state
        Ok(serde_json::json!({
            "type": "service",
            "state": "stub_state"
        }))
    }
    
    pub fn apply_service_state(_service_name: &str, _state: &Value) -> Result<(), Box<dyn Error>> {
        // Stub: In production, this would call service control functions
        Ok(())
    }
    
    pub fn export_file_state(target_file: &str, check_type: &str) -> Result<Value, Box<dyn Error>> {
        match check_type {
            "file_permissions" => {
                // Stub: Return mock permissions
                Ok(serde_json::json!({
                    "type": "file_permissions",
                    "file": target_file,
                    "permissions": "644"
                }))
            }
            "file_content" => {
                // Read actual file content
                let content = std::fs::read_to_string(target_file)?;
                Ok(serde_json::json!({
                    "type": "file_content",
                    "file": target_file,
                    "content": content
                }))
            }
            _ => Err(format!("Unsupported file check type: {}", check_type).into())
        }
    }
    
    pub fn apply_file_state(target_file: &str, check_type: &str, state: &Value) -> Result<(), Box<dyn Error>> {
        match check_type {
            "file_permissions" => {
                // Stub: In production, this would call chmod
                Ok(())
            }
            "file_content" => {
                let content = state.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'content' in state")?;
                std::fs::write(target_file, content)?;
                Ok(())
            }
            _ => Err(format!("Unsupported file check type: {}", check_type).into())
        }
    }
}

impl PolicyStateProvider for DefaultPolicyStateProvider {
    fn export_state(&self, policy: &Policy) -> Result<Value, Box<dyn Error>> {
        match policy.check_type.as_str() {
            // Handle both new "registry" and legacy "registry_key" check types
            "registry" | "registry_key" => {
                #[cfg(target_os = "windows")]
                {
                    let target_path = policy.target_path.as_ref()
                        .ok_or("target_path required for registry check")?;
                    let value_name = policy.value_name.as_ref()
                        .ok_or("value_name required for registry check")?;
                    
                    platform_helpers::export_registry_state(target_path, value_name)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Ok(serde_json::json!({"error": "registry checks only supported on Windows"}))
                }
            }
            "local_policy" => {
                #[cfg(target_os = "windows")]
                {
                    let policy_name = policy.policy_name.as_ref()
                        .ok_or("policy_name required for local_policy")?;
                    
                    platform_helpers::export_secedit_state(policy_name)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Ok(serde_json::json!({"error": "local_policy only supported on Windows"}))
                }
            }
            "service_status" => {
                let service_name = policy.service_name.as_ref()
                    .ok_or("service_name required for service_status")?;
                
                #[cfg(target_os = "windows")]
                {
                    platform_helpers::export_service_state(service_name)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    platform_helpers::export_service_state(service_name)
                }
            }
            "sysctl_key" => {
                #[cfg(not(target_os = "windows"))]
                {
                    let key = policy.key.as_ref()
                        .ok_or("key required for sysctl_key")?;
                    
                    platform_helpers::export_sysctl_state(key)
                }
                #[cfg(target_os = "windows")]
                {
                    Ok(serde_json::json!({"error": "sysctl_key only supported on Linux"}))
                }
            }
            "file_content" | "file_permissions" => {
                let _target_file = policy.target_file.as_ref()
                    .ok_or("target_file required for file check")?;
                
                #[cfg(not(target_os = "windows"))]
                {
                    platform_helpers::export_file_state(_target_file, policy.check_type.as_str())
                }
                #[cfg(target_os = "windows")]
                {
                    Ok(serde_json::json!({"error": "file checks on Windows not implemented"}))
                }
            }
            // GPO check type - delegated to platform dispatcher for actual audit
            "gpo" => {
                #[cfg(target_os = "windows")]
                {
                    Ok(serde_json::json!({"error": "GPO state export not implemented"}))
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Ok(serde_json::json!({"error": "gpo only supported on Windows"}))
                }
            }
            _ => Ok(serde_json::json!({"error": format!("Unsupported check_type: {}", policy.check_type)})),
        }
    }

    fn apply_state(&self, policy: &Policy, state: Value) -> Result<(), Box<dyn Error>> {
        // Check for error state
        if let Some(error) = state.get("error") {
            return Err(format!("Cannot apply error state: {}", error).into());
        }

        match policy.check_type.as_str() {
            // Handle both new "registry" and legacy "registry_key" check types
            "registry" | "registry_key" => {
                #[cfg(target_os = "windows")]
                {
                    let target_path = policy.target_path.as_ref()
                        .ok_or("target_path required for registry check")?;
                    let value_name = policy.value_name.as_ref()
                        .ok_or("value_name required for registry check")?;
                    
                    platform_helpers::apply_registry_state(target_path, value_name, &state)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Err("registry checks only supported on Windows".into())
                }
            }
            "local_policy" => {
                #[cfg(target_os = "windows")]
                {
                    let policy_name = policy.policy_name.as_ref()
                        .ok_or("policy_name required for local_policy")?;
                    
                    platform_helpers::apply_secedit_state(policy_name, &state)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Err("local_policy only supported on Windows".into())
                }
            }
            "service_status" => {
                let service_name = policy.service_name.as_ref()
                    .ok_or("service_name required for service_status")?;
                
                #[cfg(target_os = "windows")]
                {
                    platform_helpers::apply_service_state(service_name, &state)
                }
                #[cfg(not(target_os = "windows"))]
                {
                    platform_helpers::apply_service_state(service_name, &state)
                }
            }
            "sysctl_key" => {
                #[cfg(not(target_os = "windows"))]
                {
                    let key = policy.key.as_ref()
                        .ok_or("key required for sysctl_key")?;
                    
                    platform_helpers::apply_sysctl_state(key, &state)
                }
                #[cfg(target_os = "windows")]
                {
                    Err("sysctl_key only supported on Linux".into())
                }
            }
            "file_content" | "file_permissions" => {
                let _target_file = policy.target_file.as_ref()
                    .ok_or("target_file required for file check")?;
                
                #[cfg(not(target_os = "windows"))]
                {
                    platform_helpers::apply_file_state(_target_file, policy.check_type.as_str(), &state)
                }
                #[cfg(target_os = "windows")]
                {
                    Err("file checks on Windows not implemented".into())
                }
            }
            // GPO check type - delegated to platform dispatcher for actual remediation
            "gpo" => {
                #[cfg(target_os = "windows")]
                {
                    Err("GPO state apply not implemented - use remediate() directly".into())
                }
                #[cfg(not(target_os = "windows"))]
                {
                    Err("gpo only supported on Windows".into())
                }
            }
            _ => Err(format!("Unsupported check_type: {}", policy.check_type).into()),
        }
    }
}

/// Rollback a policy to its previous state
///
/// # Arguments
/// * `policy_id` - Policy identifier to rollback
/// * `policies` - All policies (to find the policy definition)
/// * `state_provider` - Implementation of PolicyStateProvider
///
/// # Returns
/// `RollbackResult` indicating success or failure
pub fn rollback(
    policy_id: &str,
    policies: &[Policy],
    state_provider: &dyn PolicyStateProvider,
) -> Result<RollbackResult, Box<dyn Error>> {
    // Find the policy
    let policy = policies.iter()
        .find(|p| p.id == policy_id)
        .ok_or_else(|| format!("Policy {} not found", policy_id))?;

    // Load last snapshot
    let conn = snapshot::init_db()?;
    let rollback_state = snapshot::load_last_snapshot(&conn, policy_id)
        .ok_or_else(|| format!("No rollback snapshot found for policy {}", policy_id))?;

    // Apply the previous state
    match state_provider.apply_state(policy, rollback_state.value) {
        Ok(()) => Ok(RollbackResult {
            policy_id: policy_id.to_string(),
            success: true,
            message: format!("Successfully rolled back policy {}", policy_id),
        }),
        Err(e) => Ok(RollbackResult {
            policy_id: policy_id.to_string(),
            success: false,
            message: format!("Failed to rollback policy {}: {}", policy_id, e),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Policy;

    #[test]
    fn test_audit_unsupported_platform() {
        let policy = Policy {
            id: "TEST.1".to_string(),
            platform: "macos".to_string(),
            check_type: "unknown".to_string(),
            ..Default::default()
        };

        let result = audit(&[policy]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported platform"));
    }

    #[test]
    fn test_remediate_unsupported_type() {
        let policy = Policy {
            id: "TEST.1".to_string(),
            platform: "linux".to_string(),
            remediate_type: Some("unknown".to_string()),
            ..Default::default()
        };

        let snapshot_provider = MockSnapshotProvider::new();
        let result = remediate(&[policy], &snapshot_provider);
        assert!(result.is_err());
    }
}
