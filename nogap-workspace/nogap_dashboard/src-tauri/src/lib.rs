// NoGap Dashboard - Tauri Backend
// IPC commands for native policy auditing and remediation

mod atomic;
mod elev_checks;
mod helpers;
mod privilege;
mod reporting;
mod reporting_csv;
mod utils;
mod commands_ostree;

#[cfg(target_os = "windows")]
mod windows_registry;

#[cfg(target_os = "windows")]
mod windows_secedit;

use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Policy {
    pub id: String,
    pub title: String,
    pub description: String,
    pub platform: String,
    pub severity: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FullPolicy {
    pub id: String,
    pub title: String,
    pub description: String,
    pub platform: String,
    pub severity: String,
    pub reversible: bool,
    pub check_type: String,
    pub expected_state: YamlValue,
    pub remediate_type: String,
    pub post_reboot_required: bool,
    pub reference: String,
    #[serde(default)]
    pub policy_name: Option<String>,
    #[serde(default)]
    pub target_path: Option<String>,
    #[serde(default)]
    pub value_name: Option<String>,
    #[serde(default)]
    pub set_value: Option<YamlValue>,
    #[serde(default)]
    pub set_type: Option<String>,
    #[serde(default)]
    pub service_name: Option<String>,
    #[serde(default)]
    pub remediate_params: Option<HashMap<String, YamlValue>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditResult {
    pub policy_id: String,
    pub compliant: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemediateResult {
    pub policy_id: String,
    pub success: bool,
    pub message: String,
    pub reboot_required: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RollbackResult {
    pub policy_id: String,
    pub success: bool,
    pub message: String,
}

// ========== PRIVILEGE ENFORCEMENT ==========

/// Check if the current process has elevated/admin privileges
#[tauri::command]
fn cmd_check_elevation() -> Result<bool, String> {
    elev_checks::is_elevated()
        .map_err(|e| format!("Failed to check elevation status: {}", e))
}

/// Require that the current process has elevated/admin privileges
#[tauri::command]
fn cmd_require_elevation() -> Result<(), String> {
    elev_checks::ensure_admin()
        .map_err(|e| format!("Admin privilege required: {}", e))
}

// Load policies from YAML/JSON
#[tauri::command]
fn load_policies(app_handle: tauri::AppHandle) -> Result<Vec<Policy>, String> {
    let full_policies = load_full_policies(&app_handle)?;

    let policies = full_policies
        .into_iter()
        .map(|fp| Policy {
            id: fp.id,
            title: fp.title,
            description: fp.description,
            platform: fp.platform,
            severity: fp.severity,
            status: "pending".to_string(),
        })
        .collect();

    Ok(policies)
}

// Internal: Load full policy details for audit/remediation
fn load_full_policies(_app_handle: &tauri::AppHandle) -> Result<Vec<FullPolicy>, String> {
    use std::fs;

    // Use standardized policy path resolution from nogap_core
    let policies_path = nogap_core::policy_parser::resolve_policy_path();

    let yaml_content = fs::read_to_string(&policies_path)
        .map_err(|e| format!("Failed to read policies.yaml at {:?}: {}", policies_path, e))?;

    let full_policies: Vec<FullPolicy> =
        serde_yaml::from_str(&yaml_content).map_err(|e| format!("Failed to parse YAML: {}", e))?;

    Ok(full_policies)
}

// Internal: Find a policy by ID
fn find_policy(app_handle: &tauri::AppHandle, policy_id: &str) -> Result<FullPolicy, String> {
    let policies = load_full_policies(app_handle)?;
    policies
        .into_iter()
        .find(|p| p.id == policy_id)
        .ok_or_else(|| format!("Policy {} not found", policy_id))
}

// Audit a single policy
#[tauri::command]
fn audit_policy(app_handle: tauri::AppHandle, policy_id: String) -> Result<AuditResult, String> {
    let policy = find_policy(&app_handle, &policy_id)?;

    // Detect current OS
    let current_os = std::env::consts::OS;
    let policy_platform = policy.platform.as_str();

    // Check platform compatibility
    if (policy_platform == "windows" && current_os != "windows")
        || (policy_platform == "linux" && current_os != "linux")
    {
        return Ok(AuditResult {
            policy_id,
            compliant: false,
            message: format!(
                "Policy is for {} but current OS is {}",
                policy_platform, current_os
            ),
        });
    }

    // Perform audit based on check_type
    match policy.check_type.as_str() {
        "local_policy" => audit_local_policy(&policy),
        "registry_key" => audit_registry_key(&policy),
        "service_status" => audit_service_status(&policy),
        "file_permission" => audit_file_permission(&policy),
        "sysctl" => audit_sysctl(&policy),
        "ssh_config" => audit_ssh_config(&policy),
        _ => Ok(AuditResult {
            policy_id,
            compliant: false,
            message: format!("Unknown check_type: {}", policy.check_type),
        }),
    }
}

// Audit all policies
#[tauri::command]
fn audit_all_policies(app_handle: tauri::AppHandle) -> Result<Vec<AuditResult>, String> {
    let policies = load_full_policies(&app_handle)?;
    let current_os = std::env::consts::OS;

    let results: Vec<AuditResult> = policies
        .into_iter()
        .filter(|p| {
            (p.platform == "windows" && current_os == "windows")
                || (p.platform == "linux" && current_os == "linux")
        })
        .map(|p| {
            let policy_id = p.id.clone();
            audit_policy(app_handle.clone(), policy_id).unwrap_or_else(|e| AuditResult {
                policy_id: p.id.clone(),
                compliant: false,
                message: format!("Error: {}", e),
            })
        })
        .collect();

    Ok(results)
}

// Remediate a single policy
#[tauri::command]
fn remediate_policy(
    app_handle: tauri::AppHandle,
    policy_id: String,
) -> Result<RemediateResult, String> {
    // Enforce admin privileges before remediation
    elev_checks::ensure_admin()
        .map_err(|e| format!("Admin privilege required for remediation: {}", e))?;

    // Check privileges before attempting remediation
    #[cfg(target_os = "windows")]
    if let Err(e) = privilege::ensure_admin() {
        return Ok(RemediateResult {
            policy_id,
            success: false,
            message: e,
            reboot_required: false,
        });
    }

    #[cfg(target_os = "linux")]
    if let Err(e) = privilege::ensure_root() {
        return Ok(RemediateResult {
            policy_id,
            success: false,
            message: e,
            reboot_required: false,
        });
    }

    let policy = find_policy(&app_handle, &policy_id)?;

    // Detect current OS
    let current_os = std::env::consts::OS;
    let policy_platform = policy.platform.as_str();

    // Check platform compatibility
    if (policy_platform == "windows" && current_os != "windows")
        || (policy_platform == "linux" && current_os != "linux")
    {
        return Ok(RemediateResult {
            policy_id,
            success: false,
            message: format!(
                "Policy is for {} but current OS is {}",
                policy_platform, current_os
            ),
            reboot_required: false,
        });
    }

    // Perform remediation based on remediate_type
    match policy.remediate_type.as_str() {
        "local_policy_set" => remediate_local_policy(&policy),
        "registry_set" => remediate_registry_set(&policy),
        "service_disable" => remediate_service_disable(&policy),
        "file_permission_set" => remediate_file_permission(&policy),
        "sysctl_set" => remediate_sysctl(&policy),
        "ssh_config_set" => remediate_ssh_config(&policy),
        _ => Ok(RemediateResult {
            policy_id,
            success: false,
            message: format!("Unknown remediate_type: {}", policy.remediate_type),
            reboot_required: false,
        }),
    }
}

// Remediate all policies
#[tauri::command]
fn remediate_all_policies(app_handle: tauri::AppHandle) -> Result<Vec<RemediateResult>, String> {
    // Enforce admin privileges before bulk remediation
    elev_checks::ensure_admin()
        .map_err(|e| format!("Admin privilege required for bulk remediation: {}", e))?;

    let policies = load_full_policies(&app_handle)?;
    let current_os = std::env::consts::OS;

    let results: Vec<RemediateResult> = policies
        .into_iter()
        .filter(|p| {
            (p.platform == "windows" && current_os == "windows")
                || (p.platform == "linux" && current_os == "linux")
        })
        .map(|p| {
            let policy_id = p.id.clone();
            remediate_policy(app_handle.clone(), policy_id).unwrap_or_else(|e| RemediateResult {
                policy_id: p.id.clone(),
                success: false,
                message: format!("Error: {}", e),
                reboot_required: false,
            })
        })
        .collect();

    Ok(results)
}

// Get system information
#[tauri::command]
fn get_system_info() -> Result<String, String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    Ok(format!("{} ({})", os, arch))
}

// Rollback a single policy to its previous state
#[tauri::command]
fn rollback_policy(policy_id: String) -> Result<RollbackResult, String> {
    // Enforce admin privileges before rollback
    elev_checks::ensure_admin()
        .map_err(|e| format!("Admin privilege required for rollback: {}", e))?;

    log::info!("[ROLLBACK] Attempting rollback for policy: {}", policy_id);
    
    // Load all policies to find the one being rolled back
    let policies_yaml = match std::fs::read_to_string("../../nogap_core/policies.yaml") {
        Ok(content) => content,
        Err(e) => return Err(format!("Failed to load policies: {}", e)),
    };
    
    let policies: Vec<FullPolicy> = match serde_yaml::from_str(&policies_yaml) {
        Ok(p) => p,
        Err(e) => return Err(format!("Failed to parse policies: {}", e)),
    };
    
    // Convert FullPolicy to nogap_core Policy
    let policy = policies.iter()
        .find(|p| p.id == policy_id)
        .ok_or_else(|| format!("Policy {} not found", policy_id))?;
    
    let core_policy = convert_to_core_policy(policy);
    
    // Perform rollback using engine
    let state_provider = nogap_core::engine::DefaultPolicyStateProvider;
    let result = nogap_core::engine::rollback(&policy_id, &[core_policy], &state_provider)
        .map_err(|e| format!("Rollback failed: {}", e))?;
    
    log::info!(
        "[ROLLBACK] Policy {} rollback result: success={}",
        policy_id,
        result.success
    );
    
    Ok(RollbackResult {
        policy_id: result.policy_id,
        success: result.success,
        message: result.message,
    })
}

// Rollback all policies that have rollback snapshots
#[tauri::command]
fn rollback_all() -> Result<Vec<RollbackResult>, String> {
    // Enforce admin privileges before bulk rollback
    elev_checks::ensure_admin()
        .map_err(|e| format!("Admin privilege required for bulk rollback: {}", e))?;

    log::info!("[ROLLBACK] Attempting rollback for all policies");
    
    // Load all policies
    let policies_yaml = match std::fs::read_to_string("../../nogap_core/policies.yaml") {
        Ok(content) => content,
        Err(e) => return Err(format!("Failed to load policies: {}", e)),
    };
    
    let policies: Vec<FullPolicy> = match serde_yaml::from_str(&policies_yaml) {
        Ok(p) => p,
        Err(e) => return Err(format!("Failed to parse policies: {}", e)),
    };
    
    let mut results = Vec::new();
    let state_provider = nogap_core::engine::DefaultPolicyStateProvider;
    
    // Try to rollback each policy
    for policy in &policies {
        let core_policy = convert_to_core_policy(policy);
        
        match nogap_core::engine::rollback(&policy.id, &[core_policy], &state_provider) {
            Ok(result) => results.push(RollbackResult {
                policy_id: result.policy_id,
                success: result.success,
                message: result.message,
            }),
            Err(e) => {
                // Skip policies without rollback snapshots
                log::debug!("[ROLLBACK] Skipping policy {}: {}", policy.id, e);
            }
        }
    }
    
    log::info!("[ROLLBACK] Rolled back {} policies", results.len());
    Ok(results)
}

// Helper function to convert FullPolicy to nogap_core::types::Policy
fn convert_to_core_policy(policy: &FullPolicy) -> nogap_core::types::Policy {
    nogap_core::types::Policy {
        id: policy.id.clone(),
        title: Some(policy.title.clone()),
        description: Some(policy.description.clone()),
        platform: policy.platform.clone(),
        severity: Some(policy.severity.clone()),
        reversible: Some(policy.reversible),
        check_type: policy.check_type.clone(),
        target_file: None,
        target_glob: None,
        regex: None,
        replace_regex: None,
        replace_with: None,
        key: None,
        expected_state: None, // Not needed for rollback
        package_name: None,
        service_name: policy.service_name.clone(),
        value_name: policy.value_name.clone(),
        target_path: policy.target_path.clone(),
        policy_name: policy.policy_name.clone(),
        right_name: None,
        port: None,
        protocol: None,
        remediate_type: Some(policy.remediate_type.clone()),
        value: None,
        set_value: policy.set_value.as_ref().map(|v| v.clone()),
        set_type: policy.set_type.clone(),
        remediate_params: policy.remediate_params.as_ref().map(|map| {
            nogap_core::types::RemediateParams {
                stop: map.get("stop").and_then(|v| v.as_bool()),
                disable: map.get("disable").and_then(|v| v.as_bool()),
                start: map.get("start").and_then(|v| v.as_bool()),
                enable: map.get("enable").and_then(|v| v.as_bool()),
            }
        }),
        chmod_mode: None,
        reference: Some(policy.reference.clone()),
        post_reboot_required: Some(policy.post_reboot_required),
    }
}

// Helper function to check for admin privileges (stub for now)
#[cfg(target_os = "windows")]
fn ensure_admin() -> anyhow::Result<()> {
    // In a real implementation, this would check if running as administrator
    // For now, just return Ok - actual admin check happens at registry/system level
    Ok(())
}

// ============================================================================
// AUDIT FUNCTIONS
// ============================================================================

#[cfg(target_os = "windows")]
fn audit_local_policy(policy: &FullPolicy) -> Result<AuditResult, String> {
    use std::env;

    log::info!(
        "[policy:{}] Audit start: check_type=local_policy",
        policy.id
    );

    let policy_name = policy
        .policy_name
        .as_ref()
        .ok_or_else(|| format!("No policy_name for policy {}", policy.id))?;

    // Export secedit configuration to temporary file
    let temp_dir = env::temp_dir();
    let temp_cfg = temp_dir.join("nogap_audit_cfg.inf");
    let temp_cfg_path = temp_cfg
        .to_str()
        .ok_or_else(|| "Failed to get temp path".to_string())?;

    match windows_secedit::export_secedit_cfg() {
        Ok(cfg_content) => {
            // Parse the current value
            let current_value = windows_secedit::parse_secedit_value(&cfg_content, policy_name);

            let compliant = if let Some(current) = current_value {
                // Convert current value to YamlValue
                let actual_yaml = if let Ok(num) = current.parse::<i64>() {
                    YamlValue::Number(serde_yaml::Number::from(num))
                } else {
                    YamlValue::String(current.clone())
                };

                // Parse expected_state and perform comparison
                if let Some(obj) = policy.expected_state.as_mapping() {
                    // Handle operator-based comparison: { operator: "gte", value: 14 }
                    if let (Some(op), Some(expected_val)) = (obj.get("operator"), obj.get("value"))
                    {
                        let op_str = op.as_str().unwrap_or("eq");
                        helpers::compare_with_operator(&actual_yaml, expected_val, op_str)
                            .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    // Simple value comparison
                    helpers::compare_with_operator(&actual_yaml, &policy.expected_state, "eq")
                        .unwrap_or(false)
                }
            } else {
                false // Policy not found in configuration
            };

            // Clean up temp file
            let _ = std::fs::remove_file(temp_cfg_path);

            log::info!(
                "[policy:{}] Audit result: compliant={}",
                policy.id,
                compliant
            );
            Ok(AuditResult {
                policy_id: policy.id.clone(),
                compliant,
                message: if compliant {
                    format!("Local policy {} is correctly set", policy_name)
                } else {
                    format!("Local policy {} does not meet expected state", policy_name)
                },
            })
        }
        Err(e) => {
            log::error!("[policy:{}] Audit error: {}", policy.id, e);
            Ok(AuditResult {
                policy_id: policy.id.clone(),
                compliant: false,
                message: format!("Failed to audit local policy: {}", e),
            })
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn audit_local_policy(policy: &FullPolicy) -> Result<AuditResult, String> {
    Ok(AuditResult {
        policy_id: policy.id.clone(),
        compliant: false,
        message: "Local policy checks only supported on Windows".to_string(),
    })
}

#[cfg(target_os = "windows")]
fn audit_registry_key(policy: &FullPolicy) -> Result<AuditResult, String> {
    log::info!(
        "[policy:{}] Audit start: check_type=registry_key",
        policy.id
    );

    let target_path = policy
        .target_path
        .as_ref()
        .ok_or_else(|| format!("No target_path for policy {}", policy.id))?;

    let value_name = policy
        .value_name
        .as_ref()
        .ok_or_else(|| format!("No value_name for policy {}", policy.id))?;

    match windows_registry::audit_registry_value(target_path, value_name, &policy.expected_state) {
        Ok(compliant) => {
            log::info!(
                "[policy:{}] Audit result: compliant={}",
                policy.id,
                compliant
            );
            Ok(AuditResult {
                policy_id: policy.id.clone(),
                compliant,
                message: if compliant {
                    format!("Registry value {} is correctly set", value_name)
                } else {
                    format!("Registry value {} does not meet expected state", value_name)
                },
            })
        }
        Err(e) => {
            log::error!("[policy:{}] Audit error: {}", policy.id, e);
            Ok(AuditResult {
                policy_id: policy.id.clone(),
                compliant: false,
                message: format!("Failed to audit registry: {}", e),
            })
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn audit_registry_key(policy: &FullPolicy) -> Result<AuditResult, String> {
    Ok(AuditResult {
        policy_id: policy.id.clone(),
        compliant: false,
        message: "Registry checks only supported on Windows".to_string(),
    })
}

#[cfg(target_os = "windows")]
fn audit_service_status(policy: &FullPolicy) -> Result<AuditResult, String> {
    log::info!(
        "[policy:{}] Audit start: check_type=service_status",
        policy.id
    );

    let service_name = policy
        .service_name
        .as_ref()
        .ok_or_else(|| format!("No service_name for policy {}", policy.id))?;
    // Validate identifier to avoid shell injection or unsafe names
    if !utils::validate_identifier(service_name) {
        return Ok(AuditResult {
            policy_id: policy.id.clone(),
            compliant: false,
            message: format!("Invalid service name: {}", service_name),
        });
    }
    // Use sc query to check service status
    let output = match utils::run_command_safe("sc", &["query", service_name.as_str()]) {
        Ok(o) => o,
        Err(e) => {
            return Ok(AuditResult {
                policy_id: policy.id.clone(),
                compliant: false,
                message: format!("Failed to query service {}: {}", service_name, e),
            })
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Determine actual status
    let actual_status = if stdout.contains("STOPPED") || !output.status.success() {
        "stopped_disabled"
    } else if stdout.contains("RUNNING") {
        "running"
    } else {
        "unknown"
    };

    // Parse expected_state and perform comparison
    let compliant = if let Some(obj) = policy.expected_state.as_mapping() {
        // Handle operator-based comparison: { operator: "eq", value: "stopped_disabled" }
        if let (Some(op), Some(expected_val)) = (obj.get("operator"), obj.get("value")) {
            let op_str = op.as_str().unwrap_or("eq");
            let actual_yaml = YamlValue::String(actual_status.to_string());
            helpers::compare_with_operator(&actual_yaml, expected_val, op_str).unwrap_or(false)
        } else {
            false
        }
    } else {
        // Simple string comparison
        let expected_str = policy.expected_state.as_str().unwrap_or("");
        let actual_yaml = YamlValue::String(actual_status.to_string());
        let expected_yaml = YamlValue::String(expected_str.to_string());
        helpers::compare_with_operator(&actual_yaml, &expected_yaml, "eq").unwrap_or(false)
    };

    log::info!(
        "[policy:{}] Audit result: compliant={}, service_status={}",
        policy.id,
        compliant,
        actual_status
    );
    Ok(AuditResult {
        policy_id: policy.id.clone(),
        compliant,
        message: if compliant {
            format!("Service {} is in expected state", service_name)
        } else {
            format!(
                "Service {} is NOT in expected state (current: {})",
                service_name, actual_status
            )
        },
    })
}

#[cfg(target_os = "linux")]
fn audit_service_status(policy: &FullPolicy) -> Result<AuditResult, String> {
    use std::process::Command;

    log::info!(
        "[policy:{}] Audit start: check_type=service_status",
        policy.id
    );

    let service_name = policy
        .service_name
        .as_ref()
        .ok_or_else(|| format!("No service_name for policy {}", policy.id))?;

    // Validate identifier
    if !utils::validate_identifier(service_name) {
        return Ok(AuditResult {
            policy_id: policy.id.clone(),
            compliant: false,
            message: format!("Invalid service name: {}", service_name),
        });
    }

    // Use systemctl to check service status
    let output = match utils::run_command_safe("systemctl", &["is-active", service_name.as_str()]) {
        Ok(o) => o,
        Err(e) => {
            return Ok(AuditResult {
                policy_id: policy.id.clone(),
                compliant: false,
                message: format!("Failed to query service {}: {}", service_name, e),
            })
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Map systemctl status to normalized status
    let actual_status = if stdout == "inactive" || stdout == "failed" {
        "stopped_disabled"
    } else if stdout == "active" {
        "running"
    } else {
        &stdout
    };

    // Parse expected_state and perform comparison
    let compliant = if let Some(obj) = policy.expected_state.as_mapping() {
        // Handle operator-based comparison: { operator: "eq", value: "stopped_disabled" }
        if let (Some(op), Some(expected_val)) = (obj.get("operator"), obj.get("value")) {
            let op_str = op.as_str().unwrap_or("eq");
            let actual_yaml = YamlValue::String(actual_status.to_string());
            helpers::compare_with_operator(&actual_yaml, expected_val, op_str).unwrap_or(false)
        } else {
            false
        }
    } else {
        // Simple string comparison
        let expected_str = policy.expected_state.as_str().unwrap_or("");
        let actual_yaml = YamlValue::String(actual_status.to_string());
        let expected_yaml = YamlValue::String(expected_str.to_string());
        helpers::compare_with_operator(&actual_yaml, &expected_yaml, "eq").unwrap_or(false)
    };

    log::info!(
        "[policy:{}] Audit result: compliant={}, service_status={}",
        policy.id,
        compliant,
        actual_status
    );
    Ok(AuditResult {
        policy_id: policy.id.clone(),
        compliant,
        message: if compliant {
            format!("Service {} is in expected state", service_name)
        } else {
            format!(
                "Service {} is NOT in expected state (currently: {})",
                service_name, stdout
            )
        },
    })
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn audit_service_status(policy: &FullPolicy) -> Result<AuditResult, String> {
    Ok(AuditResult {
        policy_id: policy.id.clone(),
        compliant: false,
        message: "Service checks not supported on this platform".to_string(),
    })
}

#[cfg(target_os = "linux")]
fn audit_file_permission(policy: &FullPolicy) -> Result<AuditResult, String> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    log::info!(
        "[policy:{}] Audit start: check_type=file_permission",
        policy.id
    );

    let target_path = policy
        .target_path
        .as_ref()
        .ok_or_else(|| format!("No target_path for policy {}", policy.id))?;

    match fs::metadata(target_path) {
        Ok(metadata) => {
            let perms = metadata.permissions();
            let mode = perms.mode();
            let actual_mode = mode & 0o777;

            // Convert actual mode to YAML value for comparison
            let actual_yaml = YamlValue::Number(serde_yaml::Number::from(actual_mode));

            // Parse expected_state and perform comparison
            let compliant = if let Some(obj) = policy.expected_state.as_mapping() {
                // Handle operator-based comparison: { operator: "eq", value: 0o644 }
                if let (Some(op), Some(expected_val)) = (obj.get("operator"), obj.get("value")) {
                    let op_str = op.as_str().unwrap_or("eq");
                    helpers::compare_with_operator(&actual_yaml, expected_val, op_str)
                        .unwrap_or(false)
                } else {
                    false
                }
            } else {
                // Simple value comparison - handle string or number
                let expected_mode = if let Some(mode_str) = policy.expected_state.as_str() {
                    u32::from_str_radix(mode_str.trim_start_matches("0o"), 8).unwrap_or(0)
                } else if let Some(mode_int) = policy.expected_state.as_i64() {
                    mode_int as u32
                } else {
                    0
                };
                let expected_yaml =
                    YamlValue::Number(serde_yaml::Number::from(expected_mode & 0o777));
                helpers::compare_with_operator(&actual_yaml, &expected_yaml, "eq").unwrap_or(false)
            };

            log::info!(
                "[policy:{}] Audit result: compliant={}, file_mode={:o}",
                policy.id,
                compliant,
                actual_mode
            );
            Ok(AuditResult {
                policy_id: policy.id.clone(),
                compliant,
                message: if compliant {
                    format!("File {} has correct permissions", target_path)
                } else {
                    format!(
                        "File {} has incorrect permissions (current: {:o})",
                        target_path, actual_mode
                    )
                },
            })
        }
        Err(e) => {
            log::error!("[policy:{}] Audit error: {}", policy.id, e);
            Ok(AuditResult {
                policy_id: policy.id.clone(),
                compliant: false,
                message: format!("Failed to check file {}: {}", target_path, e),
            })
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn audit_file_permission(policy: &FullPolicy) -> Result<AuditResult, String> {
    Ok(AuditResult {
        policy_id: policy.id.clone(),
        compliant: false,
        message: "File permission checks only supported on Linux".to_string(),
    })
}

#[cfg(target_os = "linux")]
fn audit_sysctl(policy: &FullPolicy) -> Result<AuditResult, String> {
    use std::process::Command;

    log::info!("[policy:{}] Audit start: check_type=sysctl", policy.id);

    let policy_name = policy
        .policy_name
        .as_ref()
        .ok_or_else(|| format!("No policy_name for policy {}", policy.id))?;

    let output = Command::new("sysctl").arg(policy_name).output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let parts: Vec<&str> = stdout.trim().split('=').collect();

            if parts.len() == 2 {
                let current_value = parts[1].trim();

                // Try to parse as number, fall back to string
                let actual_yaml = if let Ok(num) = current_value.parse::<i64>() {
                    YamlValue::Number(serde_yaml::Number::from(num))
                } else {
                    YamlValue::String(current_value.to_string())
                };

                // Parse expected_state and perform comparison
                let compliant = if let Some(obj) = policy.expected_state.as_mapping() {
                    // Handle operator-based comparison: { operator: "gte", value: 1 }
                    if let (Some(op), Some(expected_val)) = (obj.get("operator"), obj.get("value"))
                    {
                        let op_str = op.as_str().unwrap_or("eq");
                        helpers::compare_with_operator(&actual_yaml, expected_val, op_str)
                            .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    // Simple comparison - convert expected_state to appropriate type
                    helpers::compare_with_operator(&actual_yaml, &policy.expected_state, "eq")
                        .unwrap_or(false)
                };

                log::info!(
                    "[policy:{}] Audit result: compliant={}, sysctl_value={}",
                    policy.id,
                    compliant,
                    current_value
                );
                Ok(AuditResult {
                    policy_id: policy.id.clone(),
                    compliant,
                    message: if compliant {
                        format!("sysctl {} is correctly set", policy_name)
                    } else {
                        format!(
                            "sysctl {} is incorrect (current: {})",
                            policy_name, current_value
                        )
                    },
                })
            } else {
                log::error!(
                    "[policy:{}] Audit error: Failed to parse sysctl output",
                    policy.id
                );
                Ok(AuditResult {
                    policy_id: policy.id.clone(),
                    compliant: false,
                    message: format!("Failed to parse sysctl output for {}", policy_name),
                })
            }
        }
        Err(e) => {
            log::error!("[policy:{}] Audit error: {}", policy.id, e);
            Ok(AuditResult {
                policy_id: policy.id.clone(),
                compliant: false,
                message: format!("Failed to query sysctl {}: {}", policy_name, e),
            })
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn audit_sysctl(policy: &FullPolicy) -> Result<AuditResult, String> {
    Ok(AuditResult {
        policy_id: policy.id.clone(),
        compliant: false,
        message: "sysctl checks only supported on Linux".to_string(),
    })
}

#[cfg(target_os = "linux")]
fn audit_ssh_config(policy: &FullPolicy) -> Result<AuditResult, String> {
    use std::fs;

    log::info!("[policy:{}] Audit start: check_type=ssh_config", policy.id);

    let target_path = policy
        .target_path
        .as_ref()
        .unwrap_or(&"/etc/ssh/sshd_config".to_string());

    match fs::read_to_string(target_path) {
        Ok(content) => {
            let policy_name = policy
                .policy_name
                .as_ref()
                .ok_or_else(|| format!("No policy_name for policy {}", policy.id))?;

            // Find the current value in the config file
            let mut current_value: Option<String> = None;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') {
                    continue;
                }
                if trimmed.starts_with(policy_name) {
                    // Extract value after the policy name
                    let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
                    if parts.len() == 2 {
                        current_value = Some(parts[1].trim().to_string());
                        break;
                    }
                }
            }

            let compliant = if let Some(actual_value) = current_value {
                let actual_yaml = YamlValue::String(actual_value.clone());

                // Parse expected_state and perform comparison
                if let Some(obj) = policy.expected_state.as_mapping() {
                    // Handle operator-based comparison: { operator: "eq", value: "no" }
                    if let (Some(op), Some(expected_val)) = (obj.get("operator"), obj.get("value"))
                    {
                        let op_str = op.as_str().unwrap_or("eq");
                        helpers::compare_with_operator(&actual_yaml, expected_val, op_str)
                            .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    // Simple string comparison
                    helpers::compare_with_operator(&actual_yaml, &policy.expected_state, "eq")
                        .unwrap_or(false)
                }
            } else {
                false // Setting not found in config
            };

            log::info!(
                "[policy:{}] Audit result: compliant={}",
                policy.id,
                compliant
            );
            Ok(AuditResult {
                policy_id: policy.id.clone(),
                compliant,
                message: if compliant {
                    format!("SSH config {} is correctly set", policy_name)
                } else {
                    format!("SSH config {} is not set to expected value", policy_name)
                },
            })
        }
        Err(e) => {
            log::error!("[policy:{}] Audit error: {}", policy.id, e);
            Ok(AuditResult {
                policy_id: policy.id.clone(),
                compliant: false,
                message: format!("Failed to read SSH config: {}", e),
            })
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn audit_ssh_config(policy: &FullPolicy) -> Result<AuditResult, String> {
    Ok(AuditResult {
        policy_id: policy.id.clone(),
        compliant: false,
        message: "SSH config checks only supported on Linux".to_string(),
    })
}

// ============================================================================
// REMEDIATION FUNCTIONS
// ============================================================================

#[cfg(target_os = "windows")]
fn remediate_local_policy(policy: &FullPolicy) -> Result<RemediateResult, String> {
    use std::env;

    log::info!(
        "[policy:{}] Remediate start: remediate_type=local_policy_set",
        policy.id
    );

    // Ensure admin privileges
    if let Err(e) = privilege::ensure_admin() {
        return Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: false,
            message: format!("Administrator privileges required: {}", e),
            reboot_required: false,
        });
    }

    let policy_name = policy
        .policy_name
        .as_ref()
        .ok_or_else(|| format!("No policy_name for policy {}", policy.id))?;

    let set_value = policy
        .set_value
        .as_ref()
        .ok_or_else(|| format!("No set_value for policy {}", policy.id))?;

    // Convert set_value to string
    let value_str = if let Some(s) = set_value.as_str() {
        s.to_string()
    } else if let Some(num) = set_value.as_i64() {
        num.to_string()
    } else {
        return Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: false,
            message: format!("Invalid set_value type: {:?}", set_value),
            reboot_required: false,
        });
    };

    // Export secedit configuration to temporary file
    let temp_dir = env::temp_dir();
    let temp_cfg = temp_dir.join("nogap_remediate_cfg.inf");
    let temp_cfg_path = temp_cfg
        .to_str()
        .ok_or_else(|| "Failed to get temp path".to_string())?;

    match windows_secedit::export_secedit_cfg() {
        Ok(cfg_content) => {
            // Update the configuration
            match windows_secedit::update_secedit_cfg(&cfg_content, policy_name, &value_str) {
                Ok(updated_cfg) => {
                    // Apply the configuration
                    match windows_secedit::apply_secedit_cfg(&updated_cfg) {
                        Ok(_) => {
                            log::info!(
                                "[policy:{}] Remediate success: set {} to {}",
                                policy.id,
                                policy_name,
                                value_str
                            );
                            Ok(RemediateResult {
                                policy_id: policy.id.clone(),
                                success: true,
                                message: format!(
                                    "Successfully set local policy {} to {}",
                                    policy_name, value_str
                                ),
                                reboot_required: policy.post_reboot_required,
                            })
                        }
                        Err(e) => {
                            // Clean up temp file
                            let _ = std::fs::remove_file(temp_cfg_path);

                            log::error!(
                                "[policy:{}] Remediate failure: apply secedit failed: {}",
                                policy.id,
                                e
                            );
                            Ok(RemediateResult {
                                policy_id: policy.id.clone(),
                                success: false,
                                message: format!("Failed to apply secedit configuration: {}", e),
                                reboot_required: false,
                            })
                        }
                    }
                }
                Err(e) => {
                    // Clean up temp file
                    let _ = std::fs::remove_file(temp_cfg_path);

                    log::error!(
                        "[policy:{}] Remediate failure: update secedit failed: {}",
                        policy.id,
                        e
                    );
                    Ok(RemediateResult {
                        policy_id: policy.id.clone(),
                        success: false,
                        message: format!("Failed to update secedit configuration: {}", e),
                        reboot_required: false,
                    })
                }
            }
        }
        Err(e) => {
            log::error!(
                "[policy:{}] Remediate failure: export secedit failed: {}",
                policy.id,
                e
            );
            Ok(RemediateResult {
                policy_id: policy.id.clone(),
                success: false,
                message: format!("Failed to export secedit configuration: {}", e),
                reboot_required: false,
            })
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn remediate_local_policy(policy: &FullPolicy) -> Result<RemediateResult, String> {
    Ok(RemediateResult {
        policy_id: policy.id.clone(),
        success: false,
        message: "Local policy remediation only supported on Windows".to_string(),
        reboot_required: false,
    })
}

#[cfg(target_os = "windows")]
fn remediate_registry_set(policy: &FullPolicy) -> Result<RemediateResult, String> {
    log::info!(
        "[policy:{}] Remediate start: remediate_type=registry_set",
        policy.id
    );

    // Ensure admin privileges
    if let Err(e) = privilege::ensure_admin() {
        return Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: false,
            message: format!("{}", e),
            reboot_required: false,
        });
    }

    let target_path = policy
        .target_path
        .as_ref()
        .ok_or_else(|| format!("No target_path for policy {}", policy.id))?;

    let value_name = policy
        .value_name
        .as_ref()
        .ok_or_else(|| format!("No value_name for policy {}", policy.id))?;

    let set_value = policy
        .set_value
        .as_ref()
        .ok_or_else(|| format!("No set_value for policy {}", policy.id))?;

    let set_type = policy
        .set_type
        .as_ref()
        .ok_or_else(|| format!("No set_type for policy {}", policy.id))?;

    match windows_registry::remediate_registry_value(target_path, value_name, set_value, set_type) {
        Ok(()) => {
            log::info!(
                "[policy:{}] Remediate success: set registry value {}",
                policy.id,
                value_name
            );
            Ok(RemediateResult {
                policy_id: policy.id.clone(),
                success: true,
                message: format!(
                    "Successfully set registry value {} to {:?}",
                    value_name, set_value
                ),
                reboot_required: policy.post_reboot_required,
            })
        }
        Err(e) => {
            log::error!("[policy:{}] Remediate failure: {}", policy.id, e);
            Ok(RemediateResult {
                policy_id: policy.id.clone(),
                success: false,
                message: format!("Failed to set registry value: {}", e),
                reboot_required: false,
            })
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn remediate_registry_set(policy: &FullPolicy) -> Result<RemediateResult, String> {
    Ok(RemediateResult {
        policy_id: policy.id.clone(),
        success: false,
        message: "Registry remediation only supported on Windows".to_string(),
        reboot_required: false,
    })
}

#[cfg(target_os = "windows")]
fn remediate_service_disable(policy: &FullPolicy) -> Result<RemediateResult, String> {
    log::info!(
        "[policy:{}] Remediate start: remediate_type=service_disable",
        policy.id
    );

    // Ensure admin privileges
    if let Err(e) = privilege::ensure_admin() {
        return Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: false,
            message: format!("{}", e),
            reboot_required: false,
        });
    }

    let service_name = policy
        .service_name
        .as_ref()
        .ok_or_else(|| format!("No service_name for policy {}", policy.id))?;

    // Validate identifier
    if !utils::validate_identifier(service_name) {
        return Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: false,
            message: format!("Invalid service name: {}", service_name),
            reboot_required: false,
        });
    }

    // Stop the service
    let stop_output = match utils::run_command_safe("sc", &["stop", service_name.as_str()]) {
        Ok(o) => o,
        Err(e) => {
            return Ok(RemediateResult {
                policy_id: policy.id.clone(),
                success: false,
                message: format!("Failed to stop service {}: {}", service_name, e),
                reboot_required: false,
            })
        }
    };

    // Disable the service
    let disable_output = match utils::run_command_safe(
        "sc",
        &["config", service_name.as_str(), "start=", "disabled"],
    ) {
        Ok(o) => o,
        Err(e) => {
            return Ok(RemediateResult {
                policy_id: policy.id.clone(),
                success: false,
                message: format!("Failed to disable service {}: {}", service_name, e),
                reboot_required: false,
            })
        }
    };

    if stop_output.status.success() && disable_output.status.success() {
        log::info!(
            "[policy:{}] Remediate success: service {} stopped and disabled",
            policy.id,
            service_name
        );
        Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: true,
            message: format!("Service {} stopped and disabled successfully", service_name),
            reboot_required: policy.post_reboot_required,
        })
    } else {
        log::error!(
            "[policy:{}] Remediate failure: failed to stop/disable service {}",
            policy.id,
            service_name
        );
        Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: false,
            message: format!(
                "Failed to stop/disable service {}. May require administrator privileges.",
                service_name
            ),
            reboot_required: false,
        })
    }
}

#[cfg(target_os = "linux")]
fn remediate_service_disable(policy: &FullPolicy) -> Result<RemediateResult, String> {
    use std::process::Command;

    log::info!(
        "[policy:{}] Remediate start: remediate_type=service_disable",
        policy.id
    );

    // Ensure root privileges
    if let Err(e) = privilege::ensure_root() {
        return Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: false,
            message: format!("{}", e),
            reboot_required: false,
        });
    }

    let service_name = policy
        .service_name
        .as_ref()
        .ok_or_else(|| format!("No service_name for policy {}", policy.id))?;

    // Validate identifier
    if !utils::validate_identifier(service_name) {
        return Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: false,
            message: format!("Invalid service name: {}", service_name),
            reboot_required: false,
        });
    }

    // Stop the service
    let stop_output = match utils::run_command_safe("systemctl", &["stop", service_name.as_str()]) {
        Ok(o) => o,
        Err(e) => {
            return Ok(RemediateResult {
                policy_id: policy.id.clone(),
                success: false,
                message: format!("Failed to stop service {}: {}", service_name, e),
                reboot_required: false,
            })
        }
    };

    // Disable the service
    let disable_output =
        match utils::run_command_safe("systemctl", &["disable", service_name.as_str()]) {
            Ok(o) => o,
            Err(e) => {
                return Ok(RemediateResult {
                    policy_id: policy.id.clone(),
                    success: false,
                    message: format!("Failed to disable service {}: {}", service_name, e),
                    reboot_required: false,
                })
            }
        };

    if stop_output.status.success() && disable_output.status.success() {
        log::info!(
            "[policy:{}] Remediate success: service {} stopped and disabled",
            policy.id,
            service_name
        );
        Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: true,
            message: format!("Service {} stopped and disabled successfully", service_name),
            reboot_required: policy.post_reboot_required,
        })
    } else {
        log::error!(
            "[policy:{}] Remediate failure: failed to stop/disable service {}",
            policy.id,
            service_name
        );
        Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: false,
            message: format!(
                "Failed to stop/disable service {}. May require root privileges.",
                service_name
            ),
            reboot_required: false,
        })
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn remediate_service_disable(policy: &FullPolicy) -> Result<RemediateResult, String> {
    Ok(RemediateResult {
        policy_id: policy.id.clone(),
        success: false,
        message: "Service remediation not supported on this platform".to_string(),
        reboot_required: false,
    })
}

#[cfg(target_os = "linux")]
fn remediate_file_permission(policy: &FullPolicy) -> Result<RemediateResult, String> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    log::info!(
        "[policy:{}] Remediate start: remediate_type=file_permission_set",
        policy.id
    );

    // Ensure root privileges
    if let Err(e) = privilege::ensure_root() {
        return Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: false,
            message: format!("{}", e),
            reboot_required: false,
        });
    }

    let target_path = policy
        .target_path
        .as_ref()
        .ok_or_else(|| format!("No target_path for policy {}", policy.id))?;

    let set_value = policy
        .set_value
        .as_ref()
        .ok_or_else(|| format!("No set_value for policy {}", policy.id))?;

    let mode = if let Some(mode_str) = set_value.as_str() {
        u32::from_str_radix(mode_str.trim_start_matches("0o"), 8).unwrap_or(0o644)
    } else if let Some(mode_int) = set_value.as_i64() {
        mode_int as u32
    } else {
        0o644
    };

    match fs::set_permissions(target_path, fs::Permissions::from_mode(mode)) {
        Ok(_) => {
            log::info!(
                "[policy:{}] Remediate success: set file {} permissions to {:o}",
                policy.id,
                target_path,
                mode
            );
            Ok(RemediateResult {
                policy_id: policy.id.clone(),
                success: true,
                message: format!("File {} permissions set to {:o}", target_path, mode),
                reboot_required: policy.post_reboot_required,
            })
        }
        Err(e) => {
            log::error!("[policy:{}] Remediate failure: {}", policy.id, e);
            Ok(RemediateResult {
                policy_id: policy.id.clone(),
                success: false,
                message: format!(
                    "Failed to set permissions on {}: {}. May require root privileges.",
                    target_path, e
                ),
                reboot_required: false,
            })
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn remediate_file_permission(policy: &FullPolicy) -> Result<RemediateResult, String> {
    Ok(RemediateResult {
        policy_id: policy.id.clone(),
        success: false,
        message: "File permission remediation only supported on Linux".to_string(),
        reboot_required: false,
    })
}

#[cfg(target_os = "linux")]
fn remediate_sysctl(policy: &FullPolicy) -> Result<RemediateResult, String> {
    use std::process::Command;

    log::info!(
        "[policy:{}] Remediate start: remediate_type=sysctl_set",
        policy.id
    );

    // Ensure root privileges
    if let Err(e) = privilege::ensure_root() {
        return Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: false,
            message: format!("{}", e),
            reboot_required: false,
        });
    }

    let policy_name = policy
        .policy_name
        .as_ref()
        .ok_or_else(|| format!("No policy_name for policy {}", policy.id))?;

    let set_value = policy
        .set_value
        .as_ref()
        .ok_or_else(|| format!("No set_value for policy {}", policy.id))?;

    let value_str = set_value
        .as_str()
        .or_else(|| set_value.as_i64().map(|v| v.to_string()).as_deref())
        .unwrap_or("");

    let output = Command::new("sysctl")
        .args(&["-w", &format!("{}={}", policy_name, value_str)])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            log::info!(
                "[policy:{}] Remediate success: set sysctl {} to {}",
                policy.id,
                policy_name,
                value_str
            );
            Ok(RemediateResult {
                policy_id: policy.id.clone(),
                success: true,
                message: format!("sysctl {} set to {}", policy_name, value_str),
                reboot_required: policy.post_reboot_required,
            })
        }
        _ => {
            log::error!(
                "[policy:{}] Remediate failure: failed to set sysctl {}",
                policy.id,
                policy_name
            );
            Ok(RemediateResult {
                policy_id: policy.id.clone(),
                success: false,
                message: format!(
                    "Failed to set sysctl {}. May require root privileges.",
                    policy_name
                ),
                reboot_required: false,
            })
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn remediate_sysctl(policy: &FullPolicy) -> Result<RemediateResult, String> {
    Ok(RemediateResult {
        policy_id: policy.id.clone(),
        success: false,
        message: "sysctl remediation only supported on Linux".to_string(),
        reboot_required: false,
    })
}

#[cfg(target_os = "linux")]
fn remediate_ssh_config(policy: &FullPolicy) -> Result<RemediateResult, String> {
    use std::fs;
    use std::io::Write;

    log::info!(
        "[policy:{}] Remediate start: remediate_type=ssh_config_set",
        policy.id
    );

    // Ensure root privileges
    if let Err(e) = privilege::ensure_root() {
        return Ok(RemediateResult {
            policy_id: policy.id.clone(),
            success: false,
            message: format!("{}", e),
            reboot_required: false,
        });
    }

    let target_path = policy
        .target_path
        .as_ref()
        .unwrap_or(&"/etc/ssh/sshd_config".to_string());
    let policy_name = policy
        .policy_name
        .as_ref()
        .ok_or_else(|| format!("No policy_name for policy {}", policy.id))?;
    let set_value = policy
        .set_value
        .as_ref()
        .ok_or_else(|| format!("No set_value for policy {}", policy.id))?;

    let value_str = set_value.as_str().unwrap_or("");

    match fs::read_to_string(target_path) {
        Ok(mut content) => {
            // Comment out existing lines with this setting
            content = content
                .lines()
                .map(|line| {
                    let trimmed = line.trim();
                    if !trimmed.starts_with('#') && trimmed.starts_with(policy_name) {
                        format!("# {}", line)
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            // Add new setting
            content.push_str(&format!("\n{} {}\n", policy_name, value_str));

            // Write back atomically
            match atomic::atomic_write(target_path, &content) {
                Ok(_) => {
                    log::info!(
                        "[policy:{}] Remediate success: set SSH config {} to {}",
                        policy.id,
                        policy_name,
                        value_str
                    );
                    Ok(RemediateResult {
                        policy_id: policy.id.clone(),
                        success: true,
                        message: format!("SSH config {} set to {}", policy_name, value_str),
                        reboot_required: policy.post_reboot_required,
                    })
                }
                Err(e) => {
                    log::error!(
                        "[policy:{}] Remediate failure: atomic_write failed: {}",
                        policy.id,
                        e
                    );
                    Ok(RemediateResult {
                        policy_id: policy.id.clone(),
                        success: false,
                        message: format!(
                            "Failed to write SSH config: {}. May require root privileges.",
                            e
                        ),
                        reboot_required: false,
                    })
                }
            }
        }
        Err(e) => {
            log::error!(
                "[policy:{}] Remediate failure: read SSH config failed: {}",
                policy.id,
                e
            );
            Ok(RemediateResult {
                policy_id: policy.id.clone(),
                success: false,
                message: format!("Failed to read SSH config: {}", e),
                reboot_required: false,
            })
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn remediate_ssh_config(policy: &FullPolicy) -> Result<RemediateResult, String> {
    Ok(RemediateResult {
        policy_id: policy.id.clone(),
        success: false,
        message: "SSH config remediation only supported on Linux".to_string(),
        reboot_required: false,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logger (only once)
    let _ = env_logger::try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            cmd_check_elevation,
            cmd_require_elevation,
            load_policies,
            audit_policy,
            audit_all_policies,
            remediate_policy,
            remediate_all_policies,
            rollback_policy,
            rollback_all,
            get_system_info,
            reporting::generate_html_report,
            reporting::export_pdf,
            reporting_csv::generate_csv_report,
            commands_ostree::cmd_scan_usb_repos,
            commands_ostree::cmd_preview_repo,
            commands_ostree::cmd_import_repo,
            commands_ostree::cmd_export_commit,
            commands_ostree::cmd_list_all_drives
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
