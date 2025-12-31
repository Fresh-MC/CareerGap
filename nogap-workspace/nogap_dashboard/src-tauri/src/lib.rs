// NoGap Dashboard - Tauri Backend
// IPC commands for native policy auditing and remediation

mod atomic;
mod elev_checks;
mod helpers;
mod privilege;
mod reporting;
mod reporting_csv;
mod utils;

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
    /// If true, missing registry key or value indicates compliance (secure default behavior)
    #[serde(default)]
    pub missing_is_compliant: bool,
    /// Message to display when key/value is missing and missing_is_compliant is true
    #[serde(default)]
    pub missing_compliant_message: Option<String>,
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

    // Save snapshot before remediation for rollback support
    log::info!("[SNAPSHOT] Capturing pre-remediation state for policy: {}", policy_id);
    let core_policy = convert_to_core_policy(&policy);
    let state_provider = nogap_core::engine::DefaultPolicyStateProvider;

    match nogap_core::engine::PolicyStateProvider::export_state(&state_provider, &core_policy) {
        Ok(before_state) => {
            match nogap_core::snapshot::init_db() {
                Ok(conn) => {
                    match serde_json::to_string(&before_state) {
                        Ok(state_json) => {
                            if let Err(e) = nogap_core::snapshot::save_rollback(&conn, &policy_id, &state_json) {
                                log::warn!("[SNAPSHOT] Failed to save rollback point for {}: {}", policy_id, e);
                            } else {
                                log::info!("[SNAPSHOT] ✅ Saved rollback point for {}", policy_id);
                            }
                        }
                        Err(e) => log::warn!("[SNAPSHOT] Failed to serialize state for {}: {}", policy_id, e),
                    }
                }
                Err(e) => log::warn!("[SNAPSHOT] DB initialization failed for {}: {}", policy_id, e),
            }
        }
        Err(e) => log::warn!("[SNAPSHOT] Failed to export state for {}: {}", policy_id, e),
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
        service_name: policy.service_name.clone(),
        value_name: policy.value_name.clone(),
        target_path: policy.target_path.clone(),
        policy_name: policy.policy_name.clone(),
        remediate_type: Some(policy.remediate_type.clone()),
        set_value: policy.set_value.clone(),
        set_type: policy.set_type.clone(),
        remediate_params: policy.remediate_params.as_ref().map(|map| {
            nogap_core::types::RemediateParams {
                stop: map.get("stop").and_then(|v| v.as_bool()),
                disable: map.get("disable").and_then(|v| v.as_bool()),
                start: map.get("start").and_then(|v| v.as_bool()),
                enable: map.get("enable").and_then(|v| v.as_bool()),
            }
        }),
        reference: Some(policy.reference.clone()),
        post_reboot_required: Some(policy.post_reboot_required),
        // Use defaults for all other fields (including missing_is_compliant, registry, gpo)
        ..Default::default()
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

    use windows_registry::RegistryAuditResult;
    match windows_registry::audit_registry_value(target_path, value_name, &policy.expected_state) {
        RegistryAuditResult::Compliant(compliant) => {
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
        RegistryAuditResult::KeyNotFound => {
            // Key doesn't exist - check if this policy treats missing as compliant
            if policy.missing_is_compliant {
                let msg = policy
                    .missing_compliant_message
                    .clone()
                    .unwrap_or_else(|| {
                        format!(
                            "Registry key not present; secure default applies for {}",
                            value_name
                        )
                    });
                log::info!(
                    "[policy:{}] Key not found, missing_is_compliant=true: {}",
                    policy.id,
                    msg
                );
                Ok(AuditResult {
                    policy_id: policy.id.clone(),
                    compliant: true,
                    message: msg,
                })
            } else {
                log::warn!(
                    "[policy:{}] Key not found, missing_is_compliant=false",
                    policy.id
                );
                Ok(AuditResult {
                    policy_id: policy.id.clone(),
                    compliant: false,
                    message: format!(
                        "Registry key {} does not exist; policy requires explicit configuration",
                        target_path
                    ),
                })
            }
        }
        RegistryAuditResult::ValueNotFound => {
            // Key exists but value doesn't - check if this policy treats missing as compliant
            if policy.missing_is_compliant {
                let msg = policy
                    .missing_compliant_message
                    .clone()
                    .unwrap_or_else(|| {
                        format!(
                            "Registry value {} not present; feature disabled by default (secure)",
                            value_name
                        )
                    });
                log::info!(
                    "[policy:{}] Value not found, missing_is_compliant=true: {}",
                    policy.id,
                    msg
                );
                Ok(AuditResult {
                    policy_id: policy.id.clone(),
                    compliant: true,
                    message: msg,
                })
            } else {
                log::warn!(
                    "[policy:{}] Value not found, missing_is_compliant=false",
                    policy.id
                );
                Ok(AuditResult {
                    policy_id: policy.id.clone(),
                    compliant: false,
                    message: format!(
                        "Registry value {} does not exist in {}; policy requires explicit configuration",
                        value_name, target_path
                    ),
                })
            }
        }
        RegistryAuditResult::Error(e) => {
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

// ========== CSV IMPORT COMMANDS ==========

/// Detect USB-B devices and find CSV reports
#[tauri::command]
fn detect_usb_csv_reports() -> Result<Vec<String>, String> {
    use std::fs;

    let mut report_paths = Vec::new();

    #[cfg(target_os = "windows")]
    {
        // Check drive letters D-Z for USB-B marker
        for letter in b'D'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            let marker_path = format!("{}nogap_usb_repo", drive);
            
            if std::path::Path::new(&marker_path).exists() {
                // Found USB-B, look for reports
                let reports_dir = format!("{}reports", drive);
                if let Ok(entries) = fs::read_dir(&reports_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            // Check for report.csv in hostname directory
                            let csv_path = path.join("report.csv");
                            if csv_path.exists() {
                                if let Some(path_str) = csv_path.to_str() {
                                    report_paths.push(path_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Check common Linux mount points
        let mount_points = ["/media", "/mnt", "/run/media"];
        
        for mount_point in &mount_points {
            if let Ok(entries) = fs::read_dir(mount_point) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let marker_path = path.join("nogap_usb_repo");
                    
                    if marker_path.exists() {
                        // Found USB-B, look for reports
                        let reports_dir = path.join("reports");
                        if let Ok(host_entries) = fs::read_dir(&reports_dir) {
                            for host_entry in host_entries.flatten() {
                                let host_path = host_entry.path();
                                if host_path.is_dir() {
                                    // Check for report.csv in hostname directory
                                    let csv_path = host_path.join("report.csv");
                                    if csv_path.exists() {
                                        if let Some(path_str) = csv_path.to_str() {
                                            report_paths.push(path_str.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if report_paths.is_empty() {
        Err("No USB-B device with reports found".to_string())
    } else {
        log::info!("Found {} CSV reports on USB-B", report_paths.len());
        Ok(report_paths)
    }
}

/// Read CSV file contents
#[tauri::command]
fn read_csv_file(path: String) -> Result<String, String> {
    use std::fs;
    
    log::info!("Reading CSV file: {}", path);
    
    fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read CSV file {}: {}", path, e))
}

/// Read binary file contents
#[tauri::command]
fn read_binary_file(path: String) -> Result<Vec<u8>, String> {
    use std::fs;
    
    log::info!("Reading binary file: {}", path);
    
    fs::read(&path)
        .map_err(|e| format!("Failed to read binary file {}: {}", path, e))
}

/// Write binary file contents
#[tauri::command]
fn write_binary_file(path: String, data: Vec<u8>) -> Result<(), String> {
    use std::fs;
    
    log::info!("Writing binary file: {}", path);
    
    fs::write(&path, data)
        .map_err(|e| format!("Failed to write binary file {}: {}", path, e))
}

// ============================================================
// AI-ASSISTED FEATURES (Non-Agentic, Read-Only, User-Controlled)
// ============================================================

/// Risk Scoring Report - Calculates deterministic risk scores for policies
/// 
/// Formula: risk_score = severity_weight × (1 - compliance_state)
/// - severity_weight: critical=1.0, high=0.75, medium=0.5, low=0.25
/// - compliance_state: 1.0 if compliant, 0.0 if non-compliant
/// 
/// This is AI-ASSISTED and read-only - no automatic remediation.
#[tauri::command]
fn cmd_get_risk_report(app_handle: tauri::AppHandle) -> Result<RiskReportResponse, String> {
    log::info!("[AI-ASSIST] Generating risk scoring report");
    
    // Get audit results for all policies
    let audit_results = audit_all_policies(app_handle.clone())?;
    
    // Load full policies to get severity information
    let policies = load_full_policies(&app_handle)?;
    
    // Convert to core types for risk scoring
    let core_policies: Vec<nogap_core::types::Policy> = policies
        .iter()
        .map(|p| convert_to_core_policy(p))
        .collect();
    
    let core_results: Vec<nogap_core::engine::AuditResult> = audit_results
        .iter()
        .map(|r| nogap_core::engine::AuditResult {
            policy_id: r.policy_id.clone(),
            passed: r.compliant,
            message: r.message.clone(),
        })
        .collect();
    
    // Calculate risk scores using core library
    let all_scores = nogap_core::risk_scoring::calculate_all_risk_scores(&core_policies, &core_results);
    let summary = nogap_core::risk_scoring::calculate_system_risk(&all_scores);
    let top_risks_data = nogap_core::risk_scoring::get_top_risks(&all_scores, 10);
    
    // Convert to response format
    let top_risks: Vec<ScoredPolicyResponse> = top_risks_data
        .iter()
        .map(|sp| {
            ScoredPolicyResponse {
                policy_id: sp.policy_id.clone(),
                title: sp.policy_title.clone(),
                severity: sp.severity.clone(),
                risk_score: sp.risk_score,
                compliant: sp.is_compliant,
            }
        })
        .collect();
    
    log::info!(
        "[AI-ASSIST] Risk report: {} total policies, {} compliant, aggregate score: {:.2}",
        summary.total_policies,
        summary.compliant_count,
        summary.normalized_risk_score
    );
    
    Ok(RiskReportResponse {
        top_risks,
        aggregate_score: summary.normalized_risk_score,
        total_policies: summary.total_policies,
        compliant_count: summary.compliant_count,
        non_compliant_count: summary.non_compliant_count,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScoredPolicyResponse {
    pub policy_id: String,
    pub title: String,
    pub severity: String,
    pub risk_score: f32,
    pub compliant: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskReportResponse {
    pub top_risks: Vec<ScoredPolicyResponse>,
    pub aggregate_score: f32,
    pub total_policies: usize,
    pub compliant_count: usize,
    pub non_compliant_count: usize,
}

/// Compliance Drift Detection - Detects policies that changed from compliant to non-compliant
/// 
/// Compares current audit results against the last saved snapshot.
/// Only reports transitions: compliant → non-compliant (drift events)
/// 
/// This is AI-ASSISTED and read-only - alerts only, no automatic action.
#[tauri::command]
fn cmd_detect_drift(app_handle: tauri::AppHandle) -> Result<DriftReportResponse, String> {
    log::info!("[AI-ASSIST] Detecting compliance drift");
    
    // Get current audit results
    let current_results = audit_all_policies(app_handle.clone())?;
    
    // Load full policies for title lookup
    let policies = load_full_policies(&app_handle)?;
    
    let core_results: Vec<nogap_core::engine::AuditResult> = current_results
        .iter()
        .map(|r| nogap_core::engine::AuditResult {
            policy_id: r.policy_id.clone(),
            passed: r.compliant,
            message: r.message.clone(),
        })
        .collect();
    
    // Initialize drift detection database
    let conn = nogap_core::drift_detection::init_drift_db()
        .map_err(|e| format!("Failed to initialize drift database: {}", e))?;
    
    // Detect drift by comparing against last snapshot
    let drift_report = nogap_core::drift_detection::detect_drift(&conn, &core_results)
        .map_err(|e| format!("Failed to detect drift: {}", e))?;
    
    // Save current audit as new snapshot for future comparisons
    nogap_core::drift_detection::store_audit_results(&conn, &core_results, None)
        .map_err(|e| format!("Failed to save audit snapshot: {}", e))?;
    
    // Convert regressions (compliant → non-compliant) to response format
    let events: Vec<DriftEventResponse> = drift_report.regressions
        .iter()
        .map(|e| {
            // Find policy title for display
            let title = policies.iter()
                .find(|p| p.id == e.policy_id)
                .map(|p| p.title.clone())
                .unwrap_or_default();
            
            DriftEventResponse {
                policy_id: e.policy_id.clone(),
                title,
                previous_state: if e.previous_state { "compliant" } else { "non-compliant" }.to_string(),
                current_state: if e.current_state { "compliant" } else { "non-compliant" }.to_string(),
                severity: "high".to_string(), // Regressions are always concerning
                timestamp: e.detected_at.to_string(),
            }
        })
        .collect();
    
    // Build summary message
    let summary = format!(
        "Compared {} policies: {} regressions, {} improvements, {} unchanged",
        drift_report.total_compared,
        drift_report.regressions.len(),
        drift_report.improvements.len(),
        drift_report.unchanged_count
    );
    
    log::info!(
        "[AI-ASSIST] Drift detection complete: {} regressions found",
        events.len()
    );
    
    Ok(DriftReportResponse {
        events,
        summary,
        has_drift: drift_report.has_regressions(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DriftEventResponse {
    pub policy_id: String,
    pub title: String,
    pub previous_state: String,
    pub current_state: String,
    pub severity: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DriftReportResponse {
    pub events: Vec<DriftEventResponse>,
    pub summary: String,
    pub has_drift: bool,
}

/// Policy Recommendations - Returns keyword-matched policy recommendations
/// 
/// This uses keyword matching (not LLM) to suggest relevant policies
/// based on system context (OS, role, environment).
/// 
/// This is AI-ASSISTED and advisory only - user must review and approve.
#[tauri::command]
fn cmd_get_recommendations(
    app_handle: tauri::AppHandle,
    role: String,
    environment: String,
    additional_context: Option<String>,
) -> Result<RecommendationsResponse, String> {
    log::info!(
        "[AI-ASSIST] Generating policy recommendations for role='{}', env='{}'",
        role, environment
    );
    
    // Load all policies
    let policies = load_full_policies(&app_handle)?;
    
    // Convert to core types
    let core_policies: Vec<nogap_core::types::Policy> = policies
        .iter()
        .map(|p| convert_to_core_policy(p))
        .collect();
    
    // Build system context
    let context = nogap_core::ai_recommender::SystemContext {
        os: std::env::consts::OS.to_string(),
        role: role.clone(),
        environment: environment.clone(),
        additional_context,
    };
    
    // Get keyword-based recommendations (no LLM required)
    let recommended = nogap_core::ai_recommender::keyword_based_recommendations(
        &context,
        &core_policies,
        20, // max recommendations
    );
    
    // Build response with full policy details
    let recommendations: Vec<RecommendedPolicyResponse> = recommended
        .iter()
        .filter_map(|rec| {
            policies.iter().find(|p| p.id == rec.policy_id).map(|p| {
                RecommendedPolicyResponse {
                    policy_id: p.id.clone(),
                    title: p.title.clone(),
                    description: p.description.clone(),
                    severity: p.severity.clone(),
                    platform: p.platform.clone(),
                    relevance_score: rec.relevance_score,
                    reason: rec.reason.clone(),
                }
            })
        })
        .collect();
    
    log::info!(
        "[AI-ASSIST] Generated {} recommendations for context",
        recommendations.len()
    );
    
    Ok(RecommendationsResponse {
        recommendations,
        context_summary: format!(
            "Recommendations for {} environment with {} role",
            std::env::consts::OS,
            role
        ),
        note: "These are AI-assisted suggestions. Please review each policy before applying.".to_string(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendedPolicyResponse {
    pub policy_id: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub platform: String,
    pub relevance_score: f32,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendationsResponse {
    pub recommendations: Vec<RecommendedPolicyResponse>,
    pub context_summary: String,
    pub note: String,
}

// ============================================================
// AUTONOMOUS MONITORING - Sensor State Management (GUI Integration)
// ============================================================
// These commands expose the sensor state to the dashboard UI.
// No sensing logic modification - only state read/write.

use std::sync::{Arc, Mutex, OnceLock};

/// Global sensor scheduler instance for dashboard access
static SENSOR_SCHEDULER: OnceLock<Arc<Mutex<Option<SensorState>>>> = OnceLock::new();

fn get_sensor_state_store() -> &'static Arc<Mutex<Option<SensorState>>> {
    SENSOR_SCHEDULER.get_or_init(|| Arc::new(Mutex::new(None)))
}

/// Persisted sensor state (survives between page reloads)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorState {
    pub enabled: bool,
    pub interval_hours: u64,
    pub last_run_timestamp: Option<u64>,
    pub next_run_timestamp: Option<u64>,
    pub last_scan_summary: Option<SensorScanSummary>,
    pub is_running: bool,
}

impl Default for SensorState {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_hours: 24,
            last_run_timestamp: None,
            next_run_timestamp: None,
            last_scan_summary: None,
            is_running: false,
        }
    }
}

/// Summary of the last autonomous scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorScanSummary {
    pub policies_checked: usize,
    pub compliant: usize,
    pub non_compliant: usize,
    pub drift_events: usize,
    pub status: String,
}

/// Response for sensor state query
#[derive(Debug, Serialize, Deserialize)]
pub struct SensorStateResponse {
    pub enabled: bool,
    pub interval_hours: u64,
    pub last_run: Option<String>,      // Formatted timestamp
    pub next_run: Option<String>,      // Formatted timestamp
    pub last_run_timestamp: Option<u64>,
    pub next_run_timestamp: Option<u64>,
    pub last_scan_summary: Option<SensorScanSummary>,
    pub is_running: bool,
    pub has_history: bool,
}

/// Request to update sensor configuration
#[derive(Debug, Deserialize)]
pub struct SensorConfigUpdate {
    pub enabled: Option<bool>,
    pub interval_hours: Option<u64>,
}

/// Get current sensor state for the dashboard
#[tauri::command]
fn cmd_get_sensor_state() -> Result<SensorStateResponse, String> {
    log::info!("[SENSOR-GUI] Getting sensor state");
    
    let store = get_sensor_state_store();
    let guard = store.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    let state = guard.clone().unwrap_or_default();
    
    // Try to load history from database
    let has_history = check_sensor_history();
    
    // Format timestamps for display
    let last_run = state.last_run_timestamp.map(format_sensor_timestamp);
    let next_run = state.next_run_timestamp.map(format_sensor_timestamp);
    
    Ok(SensorStateResponse {
        enabled: state.enabled,
        interval_hours: state.interval_hours,
        last_run,
        next_run,
        last_run_timestamp: state.last_run_timestamp,
        next_run_timestamp: state.next_run_timestamp,
        last_scan_summary: state.last_scan_summary,
        is_running: state.is_running,
        has_history,
    })
}

/// Update sensor configuration from dashboard
#[tauri::command]
fn cmd_update_sensor_config(config: SensorConfigUpdate) -> Result<SensorStateResponse, String> {
    log::info!("[SENSOR-GUI] Updating sensor config: {:?}", config);
    
    let store = get_sensor_state_store();
    let mut guard = store.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    let mut state = guard.clone().unwrap_or_default();
    
    // Apply updates
    if let Some(enabled) = config.enabled {
        state.enabled = enabled;
        
        // If disabling, also stop running state
        if !enabled {
            state.is_running = false;
            state.next_run_timestamp = None;
        }
    }
    
    if let Some(interval) = config.interval_hours {
        // Validate interval (minimum 1 hour, max 168 hours = 1 week)
        if interval < 1 || interval > 168 {
            return Err("Interval must be between 1 and 168 hours".to_string());
        }
        state.interval_hours = interval;
        
        // Recalculate next run if enabled and was previously running
        if state.enabled && state.last_run_timestamp.is_some() {
            let last_run = state.last_run_timestamp.unwrap();
            state.next_run_timestamp = Some(last_run + (interval * 3600));
        }
    }
    
    *guard = Some(state.clone());
    drop(guard);
    
    // Return updated state
    cmd_get_sensor_state()
}

/// Start the autonomous sensor from dashboard
#[tauri::command]
fn cmd_start_sensor(app_handle: tauri::AppHandle) -> Result<SensorStateResponse, String> {
    log::info!("[SENSOR-GUI] Starting sensor from dashboard");
    
    let store = get_sensor_state_store();
    let mut guard = store.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    let mut state = guard.clone().unwrap_or_default();
    
    if !state.enabled {
        return Err("Sensor is disabled. Enable it first.".to_string());
    }
    
    if state.is_running {
        return Err("Sensor is already running".to_string());
    }
    
    // Mark as running
    state.is_running = true;
    
    // Calculate next run time
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    state.next_run_timestamp = Some(now + (state.interval_hours * 3600));
    
    *guard = Some(state.clone());
    drop(guard);
    
    // Spawn background sensing (simplified for GUI - actual implementation would use sensor_scheduler)
    let interval_hours = state.interval_hours;
    std::thread::spawn(move || {
        run_sensor_loop(app_handle, interval_hours);
    });
    
    cmd_get_sensor_state()
}

/// Stop the autonomous sensor from dashboard
#[tauri::command]
fn cmd_stop_sensor() -> Result<SensorStateResponse, String> {
    log::info!("[SENSOR-GUI] Stopping sensor from dashboard");
    
    let store = get_sensor_state_store();
    let mut guard = store.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    let mut state = guard.clone().unwrap_or_default();
    
    state.is_running = false;
    state.next_run_timestamp = None;
    
    *guard = Some(state);
    
    cmd_get_sensor_state()
}

/// Run a single autonomous scan now (on-demand)
#[tauri::command]
fn cmd_run_sensor_scan_now(app_handle: tauri::AppHandle) -> Result<SensorScanSummary, String> {
    log::info!("[SENSOR-GUI] Running on-demand sensor scan");
    
    // Run the actual audit
    let audit_results = audit_all_policies(app_handle)?;
    
    let compliant_count = audit_results.iter().filter(|r| r.compliant).count();
    let non_compliant_count = audit_results.len() - compliant_count;
    
    // Calculate drift by comparing with previous snapshot
    let drift_events = calculate_drift_from_snapshot(&audit_results);
    
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let summary = SensorScanSummary {
        policies_checked: audit_results.len(),
        compliant: compliant_count,
        non_compliant: non_compliant_count,
        drift_events,
        status: "Completed".to_string(),
    };
    
    // Store in sensor snapshot database
    store_sensor_snapshot(&audit_results, &summary);
    
    // Update sensor state
    let store = get_sensor_state_store();
    if let Ok(mut guard) = store.lock() {
        let mut state = guard.clone().unwrap_or_default();
        state.last_run_timestamp = Some(now);
        state.last_scan_summary = Some(summary.clone());
        
        if state.is_running && state.enabled {
            state.next_run_timestamp = Some(now + (state.interval_hours * 3600));
        }
        
        *guard = Some(state);
    }
    
    log::info!("[SENSOR-GUI] Scan completed: {} policies, {} drift events", 
        summary.policies_checked, summary.drift_events);
    
    Ok(summary)
}

/// Get the last autonomous scan report
#[tauri::command]
fn cmd_get_last_sensor_report() -> Result<SensorReportResponse, String> {
    log::info!("[SENSOR-GUI] Getting last sensor report");
    
    // Try to load from database
    let report = load_last_sensor_report()?;
    
    Ok(report)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SensorReportResponse {
    pub timestamp: Option<String>,
    pub timestamp_raw: Option<u64>,
    pub summary: Option<SensorScanSummary>,
    pub audit_results: Vec<SensorAuditItem>,
    pub source: String, // "agent_sense" to clearly label as autonomous
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SensorAuditItem {
    pub policy_id: String,
    pub compliant: bool,
    pub message: String,
}

// ============== Internal Helper Functions ==============

fn format_sensor_timestamp(ts: u64) -> String {
    use chrono::{DateTime, Utc};
    let datetime = DateTime::<Utc>::from_timestamp(ts as i64, 0)
        .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap());
    datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

fn check_sensor_history() -> bool {
    // Check if there are any agent_sense snapshots in the database
    if let Ok(conn) = nogap_core::snapshot::init_db() {
        let count: Result<i64, _> = conn.query_row(
            "SELECT COUNT(*) FROM snapshots WHERE policy_id = 'agent_sense'",
            [],
            |row| row.get(0),
        );
        return count.unwrap_or(0) > 0;
    }
    false
}

fn calculate_drift_from_snapshot(current_results: &[AuditResult]) -> usize {
    // Load previous sensor snapshot and compare
    if let Ok(conn) = nogap_core::snapshot::init_db() {
        let mut stmt = match conn.prepare(
            "SELECT after_state FROM snapshots 
             WHERE policy_id = 'agent_sense' 
             ORDER BY timestamp DESC 
             LIMIT 1"
        ) {
            Ok(s) => s,
            Err(_) => return 0,
        };
        
        let prev_json: Result<String, _> = stmt.query_row([], |row| row.get(0));
        
        if let Ok(json_str) = prev_json {
            if let Ok(prev_results) = serde_json::from_str::<Vec<AuditResult>>(&json_str) {
                // Count state changes
                let mut drift = 0;
                for curr in current_results {
                    if let Some(prev) = prev_results.iter().find(|p| p.policy_id == curr.policy_id) {
                        if prev.compliant != curr.compliant {
                            drift += 1;
                        }
                    }
                }
                return drift;
            }
        }
    }
    0
}

fn store_sensor_snapshot(results: &[AuditResult], _summary: &SensorScanSummary) {
    if let Ok(conn) = nogap_core::snapshot::init_db() {
        let results_json = serde_json::to_string(results).unwrap_or_default();
        let _ = nogap_core::snapshot::save_snapshot(
            &conn,
            Some("agent_sense"),
            "Autonomous scan from dashboard",
            &results_json,
            &results_json,
        );
    }
}

fn load_last_sensor_report() -> Result<SensorReportResponse, String> {
    let conn = nogap_core::snapshot::init_db()
        .map_err(|e| format!("Failed to open database: {}", e))?;
    
    let mut stmt = conn.prepare(
        "SELECT timestamp, after_state FROM snapshots 
         WHERE policy_id = 'agent_sense' 
         ORDER BY timestamp DESC 
         LIMIT 1"
    ).map_err(|e| format!("Query prepare failed: {}", e))?;
    
    let result: Result<(u64, String), _> = stmt.query_row([], |row| {
        Ok((row.get(0)?, row.get(1)?))
    });
    
    match result {
        Ok((timestamp, json_str)) => {
            let audit_results: Vec<AuditResult> = serde_json::from_str(&json_str)
                .unwrap_or_default();
            
            let compliant = audit_results.iter().filter(|r| r.compliant).count();
            let non_compliant = audit_results.len() - compliant;
            
            let items: Vec<SensorAuditItem> = audit_results.iter().map(|r| {
                SensorAuditItem {
                    policy_id: r.policy_id.clone(),
                    compliant: r.compliant,
                    message: r.message.clone(),
                }
            }).collect();
            
            Ok(SensorReportResponse {
                timestamp: Some(format_sensor_timestamp(timestamp)),
                timestamp_raw: Some(timestamp),
                summary: Some(SensorScanSummary {
                    policies_checked: audit_results.len(),
                    compliant,
                    non_compliant,
                    drift_events: 0, // Can't calculate drift for historical report without prior data
                    status: "Historical".to_string(),
                }),
                audit_results: items,
                source: "agent_sense".to_string(),
            })
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            Ok(SensorReportResponse {
                timestamp: None,
                timestamp_raw: None,
                summary: None,
                audit_results: vec![],
                source: "agent_sense".to_string(),
            })
        }
        Err(e) => Err(format!("Failed to load report: {}", e)),
    }
}

fn run_sensor_loop(app_handle: tauri::AppHandle, interval_hours: u64) {
    log::info!("[SENSOR-GUI] Background sensor loop started");
    
    let interval_secs = interval_hours * 3600;
    
    loop {
        // Check if still supposed to be running
        let store = get_sensor_state_store();
        let should_run = {
            let guard = store.lock();
            if let Ok(g) = guard {
                g.as_ref().map(|s| s.is_running && s.enabled).unwrap_or(false)
            } else {
                false
            }
        };
        
        if !should_run {
            log::info!("[SENSOR-GUI] Sensor loop stopping (disabled or stopped)");
            break;
        }
        
        // Run the scan
        log::info!("[SENSOR-GUI] Running scheduled autonomous scan");
        let _ = cmd_run_sensor_scan_now(app_handle.clone());
        
        // Sleep in chunks for responsive shutdown
        for _ in 0..60 {
            std::thread::sleep(std::time::Duration::from_secs(interval_secs / 60));
            
            // Check if still running
            let store = get_sensor_state_store();
            if let Ok(guard) = store.lock() {
                if !guard.as_ref().map(|s| s.is_running).unwrap_or(false) {
                    log::info!("[SENSOR-GUI] Sensor loop interrupted");
                    return;
                }
            }
        }
    }
}

// ============================================================
// REMEDIATION PLANNER GUI INTEGRATION
// ============================================================
// This section exposes the planner module to the GUI.
// The planner is READ-ONLY - it generates plans but does NOT execute.
// Human approval is mandatory before any action.

// Note: OnceLock is already imported via `use std::sync::{Arc, Mutex, OnceLock}` at the top.
use std::sync::Mutex as StdMutex;

/// Global storage for the latest generated plan
static LATEST_PLAN: OnceLock<StdMutex<Option<PlannerGuiPlan>>> = OnceLock::new();

fn get_plan_store() -> &'static StdMutex<Option<PlannerGuiPlan>> {
    LATEST_PLAN.get_or_init(|| StdMutex::new(None))
}

/// GUI-friendly representation of the remediation plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerGuiPlan {
    pub plan_id: String,
    pub generated_at: String,
    pub goal_description: String,
    pub goal_type: String,
    pub snapshot_timestamp: String,
    pub compliance_rate: f32,
    pub policy_count: usize,
    pub steps: Vec<PlannerGuiStep>,
    pub deferred: Vec<PlannerGuiDeferred>,
    pub excluded: Vec<PlannerGuiExcluded>,  // Manually excluded by user
    pub requires_human_approval: bool,
    pub is_approved: bool,
    pub approved_at: Option<String>,
    pub is_user_modified: bool,  // True if user made any edits
    pub metadata: PlannerGuiMetadata,
}

/// GUI-friendly representation of a plan step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerGuiStep {
    pub policy_id: String,
    pub priority: u32,
    pub reason: String,
    pub risk_score: f32,
    pub confidence: f32,
    pub constraints_considered: Vec<String>,
    pub expected_impact: Option<String>,
    pub estimated_duration_minutes: Option<u32>,
    pub source: StepSource,  // Track who added this step
}

/// Source of a plan step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepSource {
    #[serde(rename = "planner")]
    Planner,  // System-proposed
    #[serde(rename = "user")]
    User,     // Manually added by user
}

/// GUI-friendly representation of a deferred policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerGuiDeferred {
    pub policy_id: String,
    pub reason: String,
    pub blocking_constraints: Vec<String>,
}

/// GUI-friendly representation of a manually excluded policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerGuiExcluded {
    pub policy_id: String,
    pub reason: String,
    pub excluded_at: String,
    pub original_priority: u32,
    pub original_source: StepSource,
}

/// GUI-friendly metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerGuiMetadata {
    pub llm_assisted: bool,
    pub used_deterministic_fallback: bool,
    pub candidates_considered: usize,
    pub planning_duration_ms: u64,
    pub warnings: Vec<String>,
}

/// Generate a remediation plan using latest sensor data
/// 
/// This command is READ-ONLY - it generates a plan but does NOT execute anything.
/// The plan requires explicit human approval.
#[tauri::command]
fn cmd_generate_remediation_plan(app_handle: tauri::AppHandle) -> Result<PlannerGuiPlan, String> {
    log::info!("[PLANNER-GUI] Generating remediation plan");
    
    // Load latest sensor snapshot
    let sensor_report = load_last_sensor_report()?;
    
    if sensor_report.timestamp.is_none() {
        return Err("No sensor data available. Run an autonomous scan first.".to_string());
    }
    
    // Load policies using the existing function
    let policies_yaml = load_full_policies(&app_handle)?;
    
    // Convert to core Policy type
    let core_policies: Vec<nogap_core::types::Policy> = policies_yaml
        .iter()
        .filter_map(|p| {
            Some(nogap_core::types::Policy {
                id: p.id.clone(),
                title: Some(p.title.clone()),
                description: Some(p.description.clone()),
                platform: p.platform.clone(),
                severity: Some(p.severity.clone()),
                reversible: Some(p.reversible),
                check_type: p.check_type.clone(),
                post_reboot_required: Some(p.post_reboot_required),
                ..Default::default()
            })
        })
        .collect();
    
    // Convert sensor audit results to core AuditResult
    let current_audit: Vec<nogap_core::engine::AuditResult> = sensor_report
        .audit_results
        .iter()
        .map(|r| nogap_core::engine::AuditResult {
            policy_id: r.policy_id.clone(),
            passed: r.compliant,
            message: r.message.clone(),
        })
        .collect();
    
    // Calculate risk scores
    let risk_scores = nogap_core::risk_scoring::calculate_all_risk_scores(
        &core_policies,
        &current_audit,
    );
    
    // Generate recommendations (candidate list)
    let recommendations = generate_default_recommendations(&core_policies, &current_audit);
    
    // Create planner input
    let planner_input = nogap_core::planner::PlannerInput {
        current_audit: current_audit.clone(),
        previous_audit: None, // Could load from DB if needed
        risk_scores,
        policies: core_policies,
        execution_history: vec![], // TODO: Load from history
        recommendations,
        drift_report: None,
        disabled_policies: std::collections::HashSet::new(),
        current_platform: get_current_platform(),
    };
    
    // Create default goal: 80% compliance on all policies
    let goal = nogap_core::planner::PlanningGoal::compliance_threshold(0.80, None);
    
    // Generate plan (READ-ONLY operation)
    let planner = nogap_core::planner::Planner::with_defaults();
    let plan = planner.generate_plan(goal, &planner_input)
        .map_err(|e| format!("Failed to generate plan: {}", e))?;
    
    // Convert to GUI-friendly format
    let gui_plan = PlannerGuiPlan {
        plan_id: plan.plan_id.clone(),
        generated_at: plan.generated_at.clone(),
        goal_description: plan.goal.description.clone(),
        goal_type: format!("{:?}", plan.goal.goal_type),
        snapshot_timestamp: sensor_report.timestamp.unwrap_or_else(|| "Unknown".to_string()),
        compliance_rate: plan.source_snapshot.compliance_rate,
        policy_count: plan.source_snapshot.policy_count,
        steps: plan.steps.iter().map(|s| PlannerGuiStep {
            policy_id: s.policy_id.clone(),
            priority: s.priority,
            reason: s.reason.clone(),
            risk_score: s.risk_score,
            confidence: s.confidence,
            constraints_considered: s.constraints_considered.clone(),
            expected_impact: s.expected_impact.clone(),
            estimated_duration_minutes: s.estimated_duration_minutes,
            source: StepSource::Planner,  // All initially from planner
        }).collect(),
        deferred: plan.deferred.iter().map(|d| PlannerGuiDeferred {
            policy_id: d.policy_id.clone(),
            reason: d.reason.clone(),
            blocking_constraints: d.blocking_constraints.clone(),
        }).collect(),
        excluded: vec![],  // Empty initially
        requires_human_approval: plan.requires_human_approval,
        is_approved: false,
        approved_at: None,
        is_user_modified: false,  // Not modified yet
        metadata: PlannerGuiMetadata {
            llm_assisted: plan.metadata.llm_assisted,
            used_deterministic_fallback: plan.metadata.used_deterministic_fallback,
            candidates_considered: plan.metadata.candidates_considered,
            planning_duration_ms: plan.metadata.planning_duration_ms,
            warnings: plan.metadata.warnings.clone(),
        },
    };
    
    // Store the plan
    if let Ok(mut guard) = get_plan_store().lock() {
        *guard = Some(gui_plan.clone());
    }
    
    log::info!(
        "[PLANNER-GUI] Plan generated: {} steps, {} deferred",
        gui_plan.steps.len(),
        gui_plan.deferred.len()
    );
    
    Ok(gui_plan)
}

/// Get the latest generated plan
#[tauri::command]
fn cmd_get_latest_plan() -> Result<Option<PlannerGuiPlan>, String> {
    log::info!("[PLANNER-GUI] Getting latest plan");
    
    if let Ok(guard) = get_plan_store().lock() {
        Ok(guard.clone())
    } else {
        Err("Failed to access plan store".to_string())
    }
}

/// Approve the plan (UI acknowledgment only - NO execution)
/// 
/// This marks the plan as approved by the user but does NOT execute anything.
/// Execution is a separate step that requires additional user action.
#[tauri::command]
fn cmd_approve_plan(plan_id: String) -> Result<PlannerGuiPlan, String> {
    log::info!("[PLANNER-GUI] Approving plan: {}", plan_id);
    
    if let Ok(mut guard) = get_plan_store().lock() {
        if let Some(ref mut plan) = *guard {
            if plan.plan_id != plan_id {
                return Err("Plan ID mismatch - plan may have been regenerated".to_string());
            }
            
            plan.is_approved = true;
            plan.approved_at = Some(chrono::Utc::now().to_rfc3339());
            
            log::info!("[PLANNER-GUI] Plan approved (UI acknowledgment only - NO execution)");
            
            return Ok(plan.clone());
        }
    }
    
    Err("No plan available to approve".to_string())
}

/// Clear the current plan
#[tauri::command]
fn cmd_clear_plan() -> Result<(), String> {
    log::info!("[PLANNER-GUI] Clearing plan");
    
    if let Ok(mut guard) = get_plan_store().lock() {
        *guard = None;
    }
    
    Ok(())
}

// ============== Plan Editing Commands ==============

/// Remove a step from the plan (move to excluded list)
/// 
/// This does NOT re-run the planner. The step is marked as excluded by user.
#[tauri::command]
fn cmd_remove_plan_step(policy_id: String) -> Result<PlannerGuiPlan, String> {
    log::info!("[PLANNER-GUI] Removing step from plan: {}", policy_id);
    
    if let Ok(mut guard) = get_plan_store().lock() {
        if let Some(ref mut plan) = *guard {
            // Find and remove the step
            if let Some(pos) = plan.steps.iter().position(|s| s.policy_id == policy_id) {
                let removed_step = plan.steps.remove(pos);
                
                // Add to excluded list
                plan.excluded.push(PlannerGuiExcluded {
                    policy_id: removed_step.policy_id.clone(),
                    reason: "Removed by user".to_string(),
                    excluded_at: chrono::Utc::now().to_rfc3339(),
                    original_priority: removed_step.priority,
                    original_source: removed_step.source.clone(),
                });
                
                // Re-number priorities
                for (i, step) in plan.steps.iter_mut().enumerate() {
                    step.priority = (i + 1) as u32;
                }
                
                // Mark as modified
                plan.is_user_modified = true;
                plan.is_approved = false;  // Reset approval on modification
                plan.approved_at = None;
                
                log::info!("[PLANNER-GUI] Step removed and moved to excluded list");
                return Ok(plan.clone());
            }
            
            return Err(format!("Step {} not found in plan", policy_id));
        }
    }
    
    Err("No plan available".to_string())
}

/// Restore an excluded step back to the plan
#[tauri::command]
fn cmd_restore_excluded_step(policy_id: String) -> Result<PlannerGuiPlan, String> {
    log::info!("[PLANNER-GUI] Restoring excluded step: {}", policy_id);
    
    if let Ok(mut guard) = get_plan_store().lock() {
        if let Some(ref mut plan) = *guard {
            // Find and remove from excluded list
            if let Some(pos) = plan.excluded.iter().position(|e| e.policy_id == policy_id) {
                let excluded = plan.excluded.remove(pos);
                
                // Re-add to steps at the end
                let new_priority = plan.steps.len() as u32 + 1;
                plan.steps.push(PlannerGuiStep {
                    policy_id: excluded.policy_id,
                    priority: new_priority,
                    reason: "Restored by user".to_string(),
                    risk_score: 5.0,  // Default risk score
                    confidence: 0.5,  // Default confidence for restored
                    constraints_considered: vec![],
                    expected_impact: None,
                    estimated_duration_minutes: None,
                    source: excluded.original_source,
                });
                
                // Mark as modified
                plan.is_user_modified = true;
                plan.is_approved = false;
                plan.approved_at = None;
                
                log::info!("[PLANNER-GUI] Step restored from excluded list");
                return Ok(plan.clone());
            }
            
            return Err(format!("Excluded step {} not found", policy_id));
        }
    }
    
    Err("No plan available".to_string())
}

/// Eligible policy for adding to plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligiblePolicy {
    pub policy_id: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub platform: String,
    pub is_compliant: bool,
    pub can_add: bool,
    pub block_reason: Option<String>,  // Why it can't be added (hard constraint)
}

/// Get list of policies eligible to add to the plan
#[tauri::command]
fn cmd_get_eligible_policies(app_handle: tauri::AppHandle) -> Result<Vec<EligiblePolicy>, String> {
    log::info!("[PLANNER-GUI] Getting eligible policies for plan");
    
    // Load all policies
    let policies = load_full_policies(&app_handle)?;
    
    // Load sensor report for compliance status
    let sensor_report = load_last_sensor_report().ok();
    let audit_map: std::collections::HashMap<String, bool> = sensor_report
        .as_ref()
        .map(|r| {
            r.audit_results
                .iter()
                .map(|a| (a.policy_id.clone(), a.compliant))
                .collect()
        })
        .unwrap_or_default();
    
    // Get current plan to exclude already-added policies
    let plan_policies: std::collections::HashSet<String> = get_plan_store()
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .map(|p| {
            let mut set: std::collections::HashSet<String> = p.steps.iter().map(|s| s.policy_id.clone()).collect();
            set.extend(p.deferred.iter().map(|d| d.policy_id.clone()));
            set.extend(p.excluded.iter().map(|e| e.policy_id.clone()));
            set
        })
        .unwrap_or_default();
    
    // Get current platform for hard constraint check
    let current_platform = get_current_platform();
    
    // Build eligible list
    let eligible: Vec<EligiblePolicy> = policies
        .iter()
        .filter(|p| !plan_policies.contains(&p.id))
        .map(|p| {
            let is_compliant = audit_map.get(&p.id).copied().unwrap_or(true);
            
            // Check hard constraints
            let platform_mismatch = p.platform != current_platform && p.platform != "all";
            let block_reason = if platform_mismatch {
                Some(format!("Policy is for {} but current platform is {}", p.platform, current_platform))
            } else {
                None
            };
            
            EligiblePolicy {
                policy_id: p.id.clone(),
                title: p.title.clone(),
                description: p.description.clone(),
                severity: p.severity.clone(),
                platform: p.platform.clone(),
                is_compliant,
                can_add: block_reason.is_none(),
                block_reason,
            }
        })
        .collect();
    
    log::info!("[PLANNER-GUI] Found {} eligible policies", eligible.len());
    Ok(eligible)
}

/// Add a policy to the plan manually
/// 
/// This does NOT re-run the planner. The step is added as user-sourced.
/// Hard constraints are enforced - policies blocked by hard constraints cannot be added.
#[tauri::command]
fn cmd_add_policy_to_plan(app_handle: tauri::AppHandle, policy_id: String) -> Result<PlannerGuiPlan, String> {
    log::info!("[PLANNER-GUI] Adding policy to plan: {}", policy_id);
    
    // First verify the policy exists and check hard constraints
    let policies = load_full_policies(&app_handle)?;
    let policy = policies
        .iter()
        .find(|p| p.id == policy_id)
        .ok_or_else(|| format!("Policy {} not found", policy_id))?;
    
    // Check hard constraint: platform mismatch
    let current_platform = get_current_platform();
    if policy.platform != current_platform && policy.platform != "all" {
        return Err(format!(
            "Cannot add policy: Platform mismatch. Policy is for '{}' but current platform is '{}'",
            policy.platform, current_platform
        ));
    }
    
    if let Ok(mut guard) = get_plan_store().lock() {
        if let Some(ref mut plan) = *guard {
            // Check if already in plan
            if plan.steps.iter().any(|s| s.policy_id == policy_id) {
                return Err(format!("Policy {} is already in the plan", policy_id));
            }
            
            // Check if in excluded list - remove from there first
            if let Some(pos) = plan.excluded.iter().position(|e| e.policy_id == policy_id) {
                plan.excluded.remove(pos);
            }
            
            // Check if in deferred list - remove from there
            if let Some(pos) = plan.deferred.iter().position(|d| d.policy_id == policy_id) {
                plan.deferred.remove(pos);
            }
            
            // Add as new step at the end
            let new_priority = plan.steps.len() as u32 + 1;
            
            // Get severity-based risk score
            let risk_score = match policy.severity.as_str() {
                "critical" => 9.0,
                "high" => 7.0,
                "medium" => 5.0,
                "low" => 3.0,
                _ => 5.0,
            };
            
            plan.steps.push(PlannerGuiStep {
                policy_id: policy_id.clone(),
                priority: new_priority,
                reason: "Manually added by user".to_string(),
                risk_score,
                confidence: 0.5,  // Default confidence for user-added (not auto-updated)
                constraints_considered: vec![],
                expected_impact: Some(format!("{} severity policy", policy.severity)),
                estimated_duration_minutes: Some(5),
                source: StepSource::User,
            });
            
            // Mark as modified
            plan.is_user_modified = true;
            plan.is_approved = false;
            plan.approved_at = None;
            
            log::info!("[PLANNER-GUI] Policy added to plan as user step");
            return Ok(plan.clone());
        }
    }
    
    Err("No plan available. Generate a plan first.".to_string())
}

// ============== Planner Helper Functions ==============

fn get_current_platform() -> String {
    #[cfg(target_os = "windows")]
    { "windows".to_string() }
    
    #[cfg(target_os = "linux")]
    { "linux".to_string() }
    
    #[cfg(target_os = "macos")]
    { "macos".to_string() }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    { "unknown".to_string() }
}

fn generate_default_recommendations(
    policies: &[nogap_core::types::Policy],
    audit_results: &[nogap_core::engine::AuditResult],
) -> nogap_core::ai_recommender::RecommendationResult {
    // Build a set of non-compliant policy IDs
    let non_compliant: std::collections::HashSet<&str> = audit_results
        .iter()
        .filter(|r| !r.passed)
        .map(|r| r.policy_id.as_str())
        .collect();
    
    // Generate recommendations for non-compliant policies
    let recommendations: Vec<nogap_core::ai_recommender::PolicyRecommendation> = policies
        .iter()
        .filter(|p| non_compliant.contains(p.id.as_str()))
        .map(|p| {
            let severity_score = match p.severity.as_deref().unwrap_or("medium") {
                "critical" => 1.0,
                "high" => 0.85,
                "medium" => 0.7,
                "low" => 0.5,
                _ => 0.6,
            };
            
            nogap_core::ai_recommender::PolicyRecommendation {
                policy_id: p.id.clone(),
                relevance_score: severity_score,
                reason: format!(
                    "Non-compliant {} severity policy requiring remediation",
                    p.severity.as_deref().unwrap_or("medium")
                ),
            }
        })
        .collect();
    
    nogap_core::ai_recommender::RecommendationResult {
        recommendations,
        invalid_suggestions: vec![],
        warnings: vec![],
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logger (only once)
    let _ = env_logger::try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
            detect_usb_csv_reports,
            read_csv_file,
            read_binary_file,
            write_binary_file,
            // AI-Assisted Features (Non-Agentic)
            cmd_get_risk_report,
            cmd_detect_drift,
            cmd_get_recommendations,
            // Autonomous Monitoring (Sensor GUI Integration)
            cmd_get_sensor_state,
            cmd_update_sensor_config,
            cmd_start_sensor,
            cmd_stop_sensor,
            cmd_run_sensor_scan_now,
            cmd_get_last_sensor_report,
            // Remediation Planner (Read-Only Plan Generation)
            cmd_generate_remediation_plan,
            cmd_get_latest_plan,
            cmd_approve_plan,
            cmd_clear_plan,
            // Plan Editing (User Modifications)
            cmd_remove_plan_step,
            cmd_restore_excluded_step,
            cmd_get_eligible_policies,
            cmd_add_policy_to_plan
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
