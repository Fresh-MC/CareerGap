use crate::types::Policy;
use std::collections::HashMap;

/// Sandbox execution context for policy evaluation
/// 
/// Provides isolated in-memory environment for policy rule execution
/// without executing shell commands or affecting system state.
pub type SandboxContext = HashMap<String, String>;

/// Result of policy execution showing state changes
#[derive(Debug, Clone, PartialEq)]
pub struct PolicyEffect {
    pub key: String,
    pub action: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// Summary of all changes made by policy execution
#[derive(Debug, Clone, PartialEq)]
pub struct SandboxDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub modified: Vec<(String, String, String)>, // (key, old_value, new_value)
}

/// Executes a policy in an isolated sandbox context
/// 
/// Supported actions:
/// - `set`: Sets a key-value pair in context
/// - `enforce`: Validates a required key exists
/// - `audit`: Logs current state (read-only)
/// - `delete`: Removes a key from context
/// 
/// # Arguments
/// * `policy` - The policy to execute
/// * `ctx` - Mutable sandbox context
/// 
/// # Returns
/// `PolicyEffect` describing the change, or error message
pub fn execute_policy_in_sandbox(
    policy: &Policy,
    ctx: &mut SandboxContext,
) -> Result<PolicyEffect, String> {
    let action = policy.action.to_lowercase();
    
    match action.as_str() {
        "set" => {
            let old_value = ctx.insert(policy.name.clone(), policy.value.clone());
            println!("🔧 Policy [{}]: SET '{}' = '{}'", policy.id, policy.name, policy.value);
            Ok(PolicyEffect {
                key: policy.name.clone(),
                action: "set".to_string(),
                old_value,
                new_value: Some(policy.value.clone()),
            })
        }
        
        "enforce" => {
            if !ctx.contains_key(&policy.name) {
                return Err(format!(
                    "Enforcement failed: required key '{}' not found in context",
                    policy.name
                ));
            }
            println!("✅ Policy [{}]: ENFORCE '{}' exists", policy.id, policy.name);
            Ok(PolicyEffect {
                key: policy.name.clone(),
                action: "enforce".to_string(),
                old_value: ctx.get(&policy.name).cloned(),
                new_value: None,
            })
        }
        
        "audit" => {
            let value = ctx.get(&policy.name);
            println!(
                "📋 Policy [{}]: AUDIT '{}' = {:?}",
                policy.id, policy.name, value
            );
            Ok(PolicyEffect {
                key: policy.name.clone(),
                action: "audit".to_string(),
                old_value: value.cloned(),
                new_value: None,
            })
        }
        
        "delete" => {
            let old_value = ctx.remove(&policy.name);
            if old_value.is_some() {
                println!("🗑️  Policy [{}]: DELETE '{}'", policy.id, policy.name);
            } else {
                println!("⚠️  Policy [{}]: DELETE '{}' (key not found)", policy.id, policy.name);
            }
            Ok(PolicyEffect {
                key: policy.name.clone(),
                action: "delete".to_string(),
                old_value,
                new_value: None,
            })
        }
        
        _ => Err(format!("Unsupported policy action: '{}'", policy.action)),
    }
}

/// Validates policy effects and generates a diff summary
/// 
/// Compares initial state with current state to produce structured diff.
/// 
/// # Arguments
/// * `initial_ctx` - Snapshot of context before policy execution
/// * `current_ctx` - Current context after policy execution
/// 
/// # Returns
/// `SandboxDiff` showing additions, deletions, and modifications
pub fn validate_policy_effects(
    initial_ctx: &SandboxContext,
    current_ctx: &SandboxContext,
) -> SandboxDiff {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();
    
    // Find added and modified keys
    for (key, new_value) in current_ctx {
        match initial_ctx.get(key) {
            None => added.push(key.clone()),
            Some(old_value) if old_value != new_value => {
                modified.push((key.clone(), old_value.clone(), new_value.clone()));
            }
            _ => {}
        }
    }
    
    // Find removed keys
    for key in initial_ctx.keys() {
        if !current_ctx.contains_key(key) {
            removed.push(key.clone());
        }
    }
    
    SandboxDiff {
        added,
        removed,
        modified,
    }
}

/// Executes multiple policies in sequence and returns cumulative diff
pub fn execute_policy_batch(
    policies: &[Policy],
    ctx: &mut SandboxContext,
) -> Result<Vec<PolicyEffect>, String> {
    let mut effects = Vec::new();
    
    for policy in policies {
        let effect = execute_policy_in_sandbox(policy, ctx)?;
        effects.push(effect);
    }
    
    Ok(effects)
}

/// Creates a fresh sandbox context with optional initial state
pub fn create_sandbox(initial_state: Option<HashMap<String, String>>) -> SandboxContext {
    initial_state.unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_set_policy() {
        let mut ctx = create_sandbox(None);
        let policy = Policy {
            id: "P001".to_string(),
            name: "security_level".to_string(),
            action: "set".to_string(),
            value: "high".to_string(),
        };
        
        let effect = execute_policy_in_sandbox(&policy, &mut ctx);
        assert!(effect.is_ok());
        
        let effect = effect.unwrap();
        assert_eq!(effect.action, "set");
        assert_eq!(effect.new_value, Some("high".to_string()));
        assert_eq!(ctx.get("security_level"), Some(&"high".to_string()));
    }

    #[test]
    fn test_execute_enforce_policy() {
        let mut ctx = create_sandbox(None);
        ctx.insert("required_key".to_string(), "exists".to_string());
        
        let policy = Policy {
            id: "P002".to_string(),
            name: "required_key".to_string(),
            action: "enforce".to_string(),
            value: "".to_string(),
        };
        
        let effect = execute_policy_in_sandbox(&policy, &mut ctx);
        assert!(effect.is_ok());
    }

    #[test]
    fn test_execute_enforce_policy_fails() {
        let mut ctx = create_sandbox(None);
        
        let policy = Policy {
            id: "P003".to_string(),
            name: "missing_key".to_string(),
            action: "enforce".to_string(),
            value: "".to_string(),
        };
        
        let effect = execute_policy_in_sandbox(&policy, &mut ctx);
        assert!(effect.is_err());
        assert!(effect.unwrap_err().contains("Enforcement failed"));
    }

    #[test]
    fn test_execute_audit_policy() {
        let mut ctx = create_sandbox(None);
        ctx.insert("config".to_string(), "value123".to_string());
        
        let policy = Policy {
            id: "P004".to_string(),
            name: "config".to_string(),
            action: "audit".to_string(),
            value: "".to_string(),
        };
        
        let effect = execute_policy_in_sandbox(&policy, &mut ctx);
        assert!(effect.is_ok());
        assert_eq!(effect.unwrap().action, "audit");
    }

    #[test]
    fn test_execute_delete_policy() {
        let mut ctx = create_sandbox(None);
        ctx.insert("temp_data".to_string(), "remove_me".to_string());
        
        let policy = Policy {
            id: "P005".to_string(),
            name: "temp_data".to_string(),
            action: "delete".to_string(),
            value: "".to_string(),
        };
        
        let effect = execute_policy_in_sandbox(&policy, &mut ctx);
        assert!(effect.is_ok());
        assert!(!ctx.contains_key("temp_data"));
    }

    #[test]
    fn test_validate_policy_effects() {
        let mut initial = create_sandbox(None);
        initial.insert("key1".to_string(), "old".to_string());
        initial.insert("key2".to_string(), "stays".to_string());
        
        let mut current = initial.clone();
        current.insert("key1".to_string(), "new".to_string()); // modified
        current.insert("key3".to_string(), "added".to_string()); // added
        current.remove("key2"); // removed
        
        let diff = validate_policy_effects(&initial, &current);
        
        assert_eq!(diff.added, vec!["key3"]);
        assert_eq!(diff.removed, vec!["key2"]);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0], ("key1".to_string(), "old".to_string(), "new".to_string()));
    }

    #[test]
    fn test_execute_policy_batch() {
        let mut ctx = create_sandbox(None);
        
        let policies = vec![
            Policy {
                id: "P001".to_string(),
                name: "var1".to_string(),
                action: "set".to_string(),
                value: "value1".to_string(),
            },
            Policy {
                id: "P002".to_string(),
                name: "var2".to_string(),
                action: "set".to_string(),
                value: "value2".to_string(),
            },
            Policy {
                id: "P003".to_string(),
                name: "var1".to_string(),
                action: "enforce".to_string(),
                value: "".to_string(),
            },
        ];
        
        let effects = execute_policy_batch(&policies, &mut ctx);
        assert!(effects.is_ok());
        assert_eq!(effects.unwrap().len(), 3);
        assert_eq!(ctx.len(), 2);
    }

    #[test]
    fn test_unsupported_action() {
        let mut ctx = create_sandbox(None);
        let policy = Policy {
            id: "P999".to_string(),
            name: "test".to_string(),
            action: "invalid_action".to_string(),
            value: "".to_string(),
        };
        
        let effect = execute_policy_in_sandbox(&policy, &mut ctx);
        assert!(effect.is_err());
        assert!(effect.unwrap_err().contains("Unsupported policy action"));
    }
}
