#[cfg(target_os = "windows")]
use anyhow::{anyhow, Result};
#[cfg(target_os = "windows")]
use serde_yaml::Value as YamlValue;
#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[cfg(target_os = "windows")]
use crate::helpers;

/// Represents the result of auditing a registry value
#[cfg(target_os = "windows")]
#[derive(Debug)]
pub enum RegistryAuditResult {
    /// The key and value exist, audit completed with compliance status
    Compliant(bool),
    /// The registry key does not exist
    KeyNotFound,
    /// The registry key exists but the value does not
    ValueNotFound,
    /// An error occurred during audit (not related to missing key/value)
    Error(String),
}

/// Parse registry path into root key and subkey path
/// Example: "HKLM\\System\\CurrentControlSet\\Services" -> (HKEY_LOCAL_MACHINE, "System\\CurrentControlSet\\Services")
#[cfg(target_os = "windows")]
fn parse_registry_path(target_path: &str) -> Result<(RegKey, String)> {
    if target_path.starts_with("HKLM\\") || target_path.starts_with("HKLM/") {
        let subkey = target_path[5..].replace('/', "\\");
        Ok((RegKey::predef(HKEY_LOCAL_MACHINE), subkey))
    } else if target_path.starts_with("HKCU\\") || target_path.starts_with("HKCU/") {
        let subkey = target_path[5..].replace('/', "\\");
        Ok((RegKey::predef(HKEY_CURRENT_USER), subkey))
    } else {
        Err(anyhow!(
            "Unsupported registry root. Only HKLM and HKCU are supported. Got: {}",
            target_path
        ))
    }
}

/// Audit a registry value against expected state using operator comparison
/// Returns detailed result indicating whether key/value exists or if compliant
#[cfg(target_os = "windows")]
pub fn audit_registry_value(
    target_path: &str,
    value_name: &str,
    expected_state: &YamlValue,
) -> RegistryAuditResult {
    let (root_key, subkey_path) = match parse_registry_path(target_path) {
        Ok(result) => result,
        Err(e) => return RegistryAuditResult::Error(e.to_string()),
    };

    log::info!("[registry] Opening registry key: {}", target_path);
    let subkey = match root_key.open_subkey(&subkey_path) {
        Ok(key) => key,
        Err(e) => {
            // Check if the error is "file not found" (key doesn't exist)
            let error_code = e.raw_os_error().unwrap_or(0);
            if error_code == 2 {
                // ERROR_FILE_NOT_FOUND
                log::info!(
                    "[registry] Key not found (secure default): {}",
                    target_path
                );
                return RegistryAuditResult::KeyNotFound;
            }
            log::error!("[registry] Failed to open key {}: {}", target_path, e);
            return RegistryAuditResult::Error(format!(
                "Failed to open registry key {}: {}",
                target_path, e
            ));
        }
    };

    log::info!("[registry] Reading registry value: {}", value_name);
    // Try to read as DWORD first, then as String
    let actual_value: YamlValue = if let Ok(dword_val) = subkey.get_value::<u32, _>(value_name) {
        log::info!(
            "[registry] Read DWORD value: {} = {}",
            value_name,
            dword_val
        );
        YamlValue::Number(serde_yaml::Number::from(dword_val as i64))
    } else if let Ok(string_val) = subkey.get_value::<String, _>(value_name) {
        log::info!("[registry] Read STRING value: {}", value_name);
        YamlValue::String(string_val)
    } else {
        // Value doesn't exist in the key
        log::info!(
            "[registry] Value not found (secure default): {} in {}",
            value_name,
            target_path
        );
        return RegistryAuditResult::ValueNotFound;
    };

    // Parse expected_state and perform comparison using operator helper
    let compliant = if let Some(obj) = expected_state.as_mapping() {
        // Handle operator-based comparison: { operator: "eq", value: 1 }
        if let (Some(op), Some(expected_val)) = (obj.get("operator"), obj.get("value")) {
            let op_str = op.as_str().unwrap_or("eq");
            helpers::compare_with_operator(&actual_value, expected_val, op_str).unwrap_or(false)
        } else {
            false
        }
    } else {
        // Simple value comparison
        helpers::compare_with_operator(&actual_value, expected_state, "eq").unwrap_or(false)
    };

    RegistryAuditResult::Compliant(compliant)
}

/// Remediate a registry value by setting it to the specified value
#[cfg(target_os = "windows")]
pub fn remediate_registry_value(
    target_path: &str,
    value_name: &str,
    set_value: &YamlValue,
    set_type: &str,
) -> Result<()> {
    // Ensure we have admin privileges
    crate::ensure_admin()?;

    let (root_key, subkey_path) = parse_registry_path(target_path)?;

    log::info!("[registry] Opening registry key for write: {}", target_path);
    let subkey = root_key
        .open_subkey_with_flags(&subkey_path, KEY_WRITE)
        .map_err(|e| {
            log::error!(
                "[registry] Failed to open key {} for writing: {}",
                target_path,
                e
            );
            anyhow!(
                "Failed to open registry key {} for writing: {}",
                target_path,
                e
            )
        })?;

    match set_type {
        "DWORD" => {
            let value = if let Some(num) = set_value.as_i64() {
                num as u32
            } else if let Some(s) = set_value.as_str() {
                s.parse::<u32>()
                    .map_err(|e| anyhow!("Failed to parse DWORD value '{}': {}", s, e))?
            } else {
                return Err(anyhow!(
                    "Invalid DWORD value type: expected number or string, got {:?}",
                    set_value
                ));
            };

            log::info!("[registry] Writing DWORD value: {} = {}", value_name, value);
            subkey.set_value(value_name, &value).map_err(|e| {
                log::error!(
                    "[registry] Failed to write DWORD value {}: {}",
                    value_name,
                    e
                );
                anyhow!("Failed to set DWORD registry value {}: {}", value_name, e)
            })?;
            log::info!("[registry] Successfully wrote DWORD value: {}", value_name);
        }
        "STRING" => {
            let value = set_value.as_str().ok_or_else(|| {
                anyhow!(
                    "Invalid STRING value type: expected string, got {:?}",
                    set_value
                )
            })?;

            log::info!("[registry] Writing STRING value: {}", value_name);
            subkey.set_value(value_name, &value).map_err(|e| {
                log::error!(
                    "[registry] Failed to write STRING value {}: {}",
                    value_name,
                    e
                );
                anyhow!("Failed to set STRING registry value {}: {}", value_name, e)
            })?;
            log::info!("[registry] Successfully wrote STRING value: {}", value_name);
        }
        _ => {
            return Err(anyhow!(
                "Unsupported registry value type '{}'. Only DWORD and STRING are supported.",
                set_type
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::*;

    #[test]
    fn test_parse_registry_path_hklm() {
        let (_, subkey) = parse_registry_path("HKLM\\System\\Test").unwrap();
        assert_eq!(subkey, "System\\Test");
    }

    #[test]
    fn test_parse_registry_path_hkcu() {
        let (_, subkey) = parse_registry_path("HKCU\\Software\\Test").unwrap();
        assert_eq!(subkey, "Software\\Test");
    }

    #[test]
    fn test_parse_registry_path_forward_slash() {
        let (_, subkey) = parse_registry_path("HKLM/System/Test").unwrap();
        assert_eq!(subkey, "System\\Test");
    }

    #[test]
    fn test_parse_registry_path_invalid() {
        let result = parse_registry_path("HKEY_LOCAL_MACHINE\\System\\Test");
        assert!(result.is_err());
    }
}
