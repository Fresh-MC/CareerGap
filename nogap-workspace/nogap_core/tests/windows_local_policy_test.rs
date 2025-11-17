/// Integration tests for Windows Local Security Policy (secedit-based)
///
/// These tests validate the audit_local_policy() and remediate_local_policy()
/// functions using MockSeceditExecutor for cross-platform testing without
/// requiring Administrator privileges or making real system calls.
///
/// ALL tests use MockSeceditExecutor - NO real secedit.exe calls
/// ALL tests run on Windows, macOS, and Linux
/// NO tests require Administrator privileges

use nogap_core::types::{ExpectedState, Policy};
use nogap_core::platforms::windows::{audit_local_policy, remediate_local_policy, RemediateResult};
use nogap_core::platforms::windows::secedit::MockSeceditExecutor;

// Helper macro for creating test policies
macro_rules! test_policy {
    ($id:expr, $policy_name:expr, $expected:expr) => {
        Policy {
            id: $id.to_string(),
            platform: "Windows".to_string(),
            check_type: "local_policy".to_string(),
            policy_name: Some($policy_name.to_string()),
            expected_state: Some($expected),
            ..Default::default()
        }
    };
    ($id:expr, $policy_name:expr, $expected:expr, $set_value:expr) => {
        Policy {
            id: $id.to_string(),
            platform: "Windows".to_string(),
            check_type: "local_policy".to_string(),
            policy_name: Some($policy_name.to_string()),
            expected_state: Some($expected),
            set_value: Some($set_value),
            ..Default::default()
        }
    };
}

#[test]
fn test_password_complexity_policy() {
    // Test PasswordComplexity: start non-compliant, remediate, verify compliant
    let executor = MockSeceditExecutor::new();
    
    // Set initial state: PasswordComplexity = 0 (disabled)
    executor.set_export_content(
        "[Unicode]\n\
         Unicode=yes\n\
         \n\
         [System Access]\n\
         PasswordComplexity = 0\n\
         \n\
         [Version]\n\
         signature=\"$CHICAGO$\"\n\
         Revision=1\n".to_string()
    );
    
    // Audit - expect it to be enabled (1)
    let audit_policy = test_policy!(
        "A.1.a.i",
        "PasswordComplexity",
        ExpectedState::String("1".to_string())
    );
    
    let result = audit_local_policy(&audit_policy, &executor)
        .expect("Audit should not error");
    assert!(!result.passed, "Should fail: PasswordComplexity is 0, expected 1");
    
    // Remediate - enable password complexity
    let remediate_policy = test_policy!(
        "A.1.a.i",
        "PasswordComplexity",
        ExpectedState::String("1".to_string()),
        serde_yaml::Value::Number(serde_yaml::Number::from(1))
    );
    
    let remediate_result = remediate_local_policy(&remediate_policy, &executor)
        .expect("Remediation should not error");
    assert!(matches!(remediate_result, RemediateResult::Success(_)));
    
    // Verify - should now pass
    let verify_result = audit_local_policy(&audit_policy, &executor)
        .expect("Verification audit should not error");
    assert!(verify_result.passed, "Should pass after remediation");
}

#[test]
fn test_lockout_duration_policy() {
    // Test LockoutDuration: start with 5 minutes, remediate to 15, verify
    let executor = MockSeceditExecutor::new();
    
    // Initial state from MockSeceditExecutor::new() has LockoutDuration = 5
    let audit_policy = test_policy!(
        "A.2.b.ii",
        "LockoutDuration",
        ExpectedState::Map {
            operator: "gte".to_string(),
            value: serde_yaml::Value::Number(serde_yaml::Number::from(15)),
        }
    );
    
    // Should fail: 5 < 15
    let result = audit_local_policy(&audit_policy, &executor)
        .expect("Audit should not error");
    assert!(!result.passed, "Should fail: LockoutDuration is 5, expected >= 15");
    
    // Remediate to 15
    let remediate_policy = test_policy!(
        "A.2.b.ii",
        "LockoutDuration",
        ExpectedState::Map {
            operator: "gte".to_string(),
            value: serde_yaml::Value::Number(serde_yaml::Number::from(15)),
        },
        serde_yaml::Value::Number(serde_yaml::Number::from(15))
    );
    
    let remediate_result = remediate_local_policy(&remediate_policy, &executor)
        .expect("Remediation should not error");
    assert!(matches!(remediate_result, RemediateResult::Success(_)));
    
    // Verify
    let verify_result = audit_local_policy(&audit_policy, &executor)
        .expect("Verification should not error");
    assert!(verify_result.passed, "Should pass: LockoutDuration now 15");
}

#[test]
fn test_history_policy() {
    // Test PasswordHistorySize
    let executor = MockSeceditExecutor::new();
    
    // Set initial state: PasswordHistorySize = 5
    executor.set_export_content(
        "[Unicode]\n\
         Unicode=yes\n\
         \n\
         [System Access]\n\
         PasswordHistorySize = 5\n\
         \n\
         [Version]\n\
         signature=\"$CHICAGO$\"\n\
         Revision=1\n".to_string()
    );
    
    // Audit - expect >= 24
    let audit_policy = test_policy!(
        "A.1.b.i",
        "PasswordHistorySize",
        ExpectedState::Map {
            operator: "gte".to_string(),
            value: serde_yaml::Value::Number(serde_yaml::Number::from(24)),
        }
    );
    
    let result = audit_local_policy(&audit_policy, &executor)
        .expect("Audit should not error");
    assert!(!result.passed, "Should fail: 5 < 24");
    
    // Remediate to 24
    let remediate_policy = test_policy!(
        "A.1.b.i",
        "PasswordHistorySize",
        ExpectedState::Map {
            operator: "gte".to_string(),
            value: serde_yaml::Value::Number(serde_yaml::Number::from(24)),
        },
        serde_yaml::Value::Number(serde_yaml::Number::from(24))
    );
    
    let remediate_result = remediate_local_policy(&remediate_policy, &executor)
        .expect("Remediation should not error");
    assert!(matches!(remediate_result, RemediateResult::Success(_)));
    
    // Verify
    let verify_result = audit_local_policy(&audit_policy, &executor)
        .expect("Verification should not error");
    assert!(verify_result.passed, "Should pass after remediation");
}

#[test]
fn test_guest_account() {
    // Test EnableGuestAccount
    let executor = MockSeceditExecutor::new();
    
    // Set guest account enabled (non-compliant)
    executor.set_export_content(
        "[Unicode]\n\
         Unicode=yes\n\
         \n\
         [System Access]\n\
         EnableGuestAccount = 1\n\
         \n\
         [Version]\n\
         signature=\"$CHICAGO$\"\n\
         Revision=1\n".to_string()
    );
    
    // Audit - expect disabled (0)
    let audit_policy = test_policy!(
        "A.2.a.i",
        "EnableGuestAccount",
        ExpectedState::String("0".to_string())
    );
    
    let result = audit_local_policy(&audit_policy, &executor)
        .expect("Audit should not error");
    assert!(!result.passed, "Should fail: guest account is enabled");
    
    // Remediate - disable guest account
    let remediate_policy = test_policy!(
        "A.2.a.i",
        "EnableGuestAccount",
        ExpectedState::String("0".to_string()),
        serde_yaml::Value::Number(serde_yaml::Number::from(0))
    );
    
    let remediate_result = remediate_local_policy(&remediate_policy, &executor)
        .expect("Remediation should not error");
    assert!(matches!(remediate_result, RemediateResult::Success(_)));
    
    // Verify
    let verify_result = audit_local_policy(&audit_policy, &executor)
        .expect("Verification should not error");
    assert!(verify_result.passed, "Guest account should be disabled");
}

#[test]
fn test_invalid_inf_missing_key() {
    // Test error handling when INF is missing the required key
    let executor = MockSeceditExecutor::new();
    
    // Set INF without LockoutDuration
    executor.set_export_content(
        "[Unicode]\n\
         Unicode=yes\n\
         \n\
         [System Access]\n\
         PasswordComplexity = 1\n\
         \n\
         [Version]\n\
         signature=\"$CHICAGO$\"\n\
         Revision=1\n".to_string()
    );
    
    let policy = test_policy!(
        "A.2.b.ii",
        "LockoutDuration",
        ExpectedState::String("15".to_string())
    );
    
    let result = audit_local_policy(&policy, &executor);
    assert!(result.is_err(), "Should error when key is missing");
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn test_invalid_value_non_numeric() {
    // Test error handling when INF value is non-numeric but comparison expects numeric
    let executor = MockSeceditExecutor::new();
    
    // Set INF with invalid non-numeric value
    executor.set_export_content(
        "[Unicode]\n\
         Unicode=yes\n\
         \n\
         [System Access]\n\
         LockoutDuration = abc\n\
         \n\
         [Version]\n\
         signature=\"$CHICAGO$\"\n\
         Revision=1\n".to_string()
    );
    
    let policy = test_policy!(
        "A.2.b.ii",
        "LockoutDuration",
        ExpectedState::Map {
            operator: "gte".to_string(),
            value: serde_yaml::Value::Number(serde_yaml::Number::from(15)),
        }
    );
    
    let result = audit_local_policy(&policy, &executor);
    assert!(result.is_err(), "Should error when value is non-numeric");
    assert!(result.unwrap_err().to_string().contains("Failed to parse"));
}

#[test]
fn test_roundtrip_all_policies() {
    // Test multiple policies in sequence: audit non-compliant, remediate all, verify all compliant
    let executor = MockSeceditExecutor::new();
    
    // Set initial non-compliant state for all policies
    executor.set_export_content(
        "[Unicode]\n\
         Unicode=yes\n\
         \n\
         [System Access]\n\
         PasswordComplexity = 0\n\
         MinimumPasswordLength = 6\n\
         MaximumPasswordAge = 120\n\
         MinimumPasswordAge = 0\n\
         PasswordHistorySize = 5\n\
         LockoutDuration = 5\n\
         LockoutBadCount = 10\n\
         EnableGuestAccount = 1\n\
         LmCompatibilityLevel = 2\n\
         ConsentPromptBehaviorAdmin = 0\n\
         \n\
         [Version]\n\
         signature=\"$CHICAGO$\"\n\
         Revision=1\n".to_string()
    );
    
    // Define all policies with expected compliant states
    let policies = vec![
        ("PasswordComplexity", "1", 1),
        ("MinimumPasswordLength", "14", 14),
        ("MaximumPasswordAge", "60", 60),
        ("MinimumPasswordAge", "1", 1),
        ("PasswordHistorySize", "24", 24),
        ("LockoutDuration", "15", 15),
        ("LockoutBadCount", "5", 5),
        ("EnableGuestAccount", "0", 0),
        ("LmCompatibilityLevel", "5", 5),
        ("ConsentPromptBehaviorAdmin", "2", 2),
    ];
    
    // Phase 1: Audit all - should all fail
    for (policy_name, expected_str, _) in &policies {
        let policy = test_policy!(
            "TEST",
            policy_name,
            ExpectedState::String(expected_str.to_string())
        );
        
        let result = audit_local_policy(&policy, &executor)
            .expect(&format!("Audit {} should not error", policy_name));
        assert!(!result.passed, "{} should fail initial audit", policy_name);
    }
    
    // Phase 2: Remediate all
    for (policy_name, expected_str, set_value) in &policies {
        let policy = test_policy!(
            "TEST",
            policy_name,
            ExpectedState::String(expected_str.to_string()),
            serde_yaml::Value::Number(serde_yaml::Number::from(*set_value))
        );
        
        let result = remediate_local_policy(&policy, &executor)
            .expect(&format!("Remediate {} should not error", policy_name));
        assert!(matches!(result, RemediateResult::Success(_)), 
                "{} remediation should succeed", policy_name);
    }
    
    // Phase 3: Audit all again - should all pass
    for (policy_name, expected_str, _) in &policies {
        let policy = test_policy!(
            "TEST",
            policy_name,
            ExpectedState::String(expected_str.to_string())
        );
        
        let result = audit_local_policy(&policy, &executor)
            .expect(&format!("Final audit {} should not error", policy_name));
        assert!(result.passed, "{} should pass final audit", policy_name);
    }
}