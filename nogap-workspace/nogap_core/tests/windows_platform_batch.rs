// Week 2 Stage 5: Comprehensive tests for 18 Windows policies
use nogap_core::platforms::windows::{
    check_admin_account_renamed, check_autoplay_disabled, check_firewall_domain_profile,
    check_firewall_public_profile, check_lm_compatibility, check_lockout_duration,
    check_lockout_threshold, check_max_password_age, check_min_password_length,
    check_password_complexity, check_remote_registry_disabled, check_restrict_anonymous,
    check_restrict_anonymous_sam, check_smb1_disabled, check_spooler_disabled,
    check_termservice_disabled, check_uac_elevation, check_w32time_enabled,
    remediate_admin_account_renamed, remediate_autoplay_disabled,
    remediate_firewall_domain_profile, remediate_firewall_public_profile,
    remediate_lm_compatibility, remediate_lockout_duration, remediate_lockout_threshold,
    remediate_max_password_age, remediate_min_password_length, remediate_password_complexity,
    remediate_remote_registry_disabled, remediate_restrict_anonymous,
    remediate_restrict_anonymous_sam, remediate_smb1_disabled, remediate_spooler_disabled,
    remediate_termservice_disabled, remediate_uac_elevation, remediate_w32time_enabled,
    MockRegistry, MockServiceManager, RemediateResult,
};
use nogap_core::types::Policy;

// ============================================================================
// STAGE 5: PASSWORD POLICIES (3 tests)
// ============================================================================

#[test]
fn test_windows_a_1_a_i_password_complexity() {
    let policy = Policy {
        id: "A.1.a.i".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(r"SYSTEM\CurrentControlSet\Control\Lsa".to_string()),
        value_name: Some("PasswordComplexity".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String("1".to_string())),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_dword(
        r"HKLM\SYSTEM\CurrentControlSet\Control\Lsa",
        "PasswordComplexity",
        0, // vulnerable: complexity disabled
    );

    let audit_result = check_password_complexity(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.1.a.i");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("PasswordComplexity"));

    match remediate_password_complexity(&policy, &mut registry).unwrap() {
        RemediateResult::Success(msg) => {
            assert!(msg.contains("complexity") || msg.contains("PasswordComplexity"));
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    // Verify remediation
    let audit_result_after = check_password_complexity(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}

#[test]
fn test_windows_a_1_a_ii_min_password_length() {
    let policy = Policy {
        id: "A.1.a.ii".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(r"SYSTEM\CurrentControlSet\Control\Lsa".to_string()),
        value_name: Some("MinimumPasswordLength".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String("14".to_string())),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_dword(
        r"SYSTEM\CurrentControlSet\Control\Lsa",
        "MinimumPasswordLength",
        8, // vulnerable: too short
    );

    let audit_result = check_min_password_length(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.1.a.ii");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("MinimumPasswordLength"));

    match remediate_min_password_length(&policy, &mut registry).unwrap() {
        RemediateResult::Success(msg) => {
            assert!(msg.contains("password") || msg.contains("length"));
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_min_password_length(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}

#[test]
fn test_windows_a_1_a_iii_max_password_age() {
    let policy = Policy {
        id: "A.1.a.iii".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(r"SYSTEM\CurrentControlSet\Control\Lsa".to_string()),
        value_name: Some("MaximumPasswordAge".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String("60".to_string())),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_dword(
        r"SYSTEM\CurrentControlSet\Control\Lsa",
        "MaximumPasswordAge",
        180, // vulnerable: too long
    );

    let audit_result = check_max_password_age(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.1.a.iii");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("MaximumPasswordAge"));

    match remediate_max_password_age(&policy, &mut registry).unwrap() {
        RemediateResult::Success(msg) => {
            assert!(msg.contains("password") || msg.contains("age"));
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_max_password_age(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}

// ============================================================================
// STAGE 5: ACCOUNT POLICIES (3 tests)
// ============================================================================

#[test]
fn test_windows_a_2_a_ii_admin_renamed() {
    let policy = Policy {
        id: "A.2.a.ii".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon".to_string()),
        value_name: Some("DefaultUserName".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String(
            "Administrator".to_string(),
        )),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_string(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon",
        "DefaultUserName",
        "Administrator", // vulnerable: default name
    );

    let audit_result = check_admin_account_renamed(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.2.a.ii");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("Administrator"));

    match remediate_admin_account_renamed(&policy, &mut registry).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully renamed - just verify we got Success
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_admin_account_renamed(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}

#[test]
fn test_windows_a_2_b_i_lockout_threshold() {
    let policy = Policy {
        id: "A.2.b.i".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(
            r"SYSTEM\CurrentControlSet\Services\RemoteAccess\Parameters\AccountLockout".to_string(),
        ),
        value_name: Some("MaxDenials".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String("5".to_string())),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_dword(
        r"SYSTEM\CurrentControlSet\Services\RemoteAccess\Parameters\AccountLockout",
        "MaxDenials",
        10, // vulnerable: threshold too high
    );

    let audit_result = check_lockout_threshold(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.2.b.i");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("MaxDenials"));

    match remediate_lockout_threshold(&policy, &mut registry).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully set threshold
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_lockout_threshold(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}

#[test]
fn test_windows_a_2_b_ii_lockout_duration() {
    let policy = Policy {
        id: "A.2.b.ii".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(
            r"SYSTEM\CurrentControlSet\Services\RemoteAccess\Parameters\AccountLockout".to_string(),
        ),
        value_name: Some("ResetTime".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String("15".to_string())),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_dword(
        r"SYSTEM\CurrentControlSet\Services\RemoteAccess\Parameters\AccountLockout",
        "ResetTime",
        5, // vulnerable: duration too short
    );

    let audit_result = check_lockout_duration(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.2.b.ii");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("ResetTime"));

    match remediate_lockout_duration(&policy, &mut registry).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully set duration
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_lockout_duration(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}

// ============================================================================
// STAGE 5: NETWORK POLICIES (3 tests)
// ============================================================================

#[test]
fn test_windows_a_3_a_i_smb1_disabled() {
    let policy = Policy {
        id: "A.3.a.i".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(r"SYSTEM\CurrentControlSet\Services\LanmanServer\Parameters".to_string()),
        value_name: Some("SMB1".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String("0".to_string())),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_dword(
        r"SYSTEM\CurrentControlSet\Services\LanmanServer\Parameters",
        "SMB1",
        1, // vulnerable: SMB1 enabled
    );

    let audit_result = check_smb1_disabled(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.3.a.i");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("SMB1"));

    match remediate_smb1_disabled(&policy, &mut registry).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully disabled SMB1
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_smb1_disabled(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}

#[test]
fn test_windows_a_3_a_ii_firewall_domain_profile() {
    let policy = Policy {
        id: "A.3.a.ii".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(r"SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\DomainProfile".to_string()),
        value_name: Some("EnableFirewall".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String("1".to_string())),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_dword(
        r"SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\DomainProfile",
        "EnableFirewall",
        0, // vulnerable: firewall disabled
    );

    let audit_result = check_firewall_domain_profile(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.3.a.ii");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("Domain"));

    match remediate_firewall_domain_profile(&policy, &mut registry).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully enabled domain firewall
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_firewall_domain_profile(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}

#[test]
fn test_windows_a_3_a_iv_firewall_public_profile() {
    let policy = Policy {
        id: "A.3.a.iv".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(r"SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\PublicProfile".to_string()),
        value_name: Some("EnableFirewall".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String("1".to_string())),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_dword(
        r"SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy\PublicProfile",
        "EnableFirewall",
        0, // vulnerable: firewall disabled
    );

    let audit_result = check_firewall_public_profile(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.3.a.iv");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("Public"));

    match remediate_firewall_public_profile(&policy, &mut registry).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully enabled public firewall
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_firewall_public_profile(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}

// ============================================================================
// STAGE 5: SERVICE POLICIES (4 tests)
// ============================================================================

#[test]
fn test_windows_a_4_a_i_termservice_disabled() {
    let policy = Policy {
        id: "A.4.a.i".to_string(),
        platform: "windows".to_string(),
        check_type: "service_status".to_string(),
        service_name: Some("TermService".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String(
            "stopped_disabled".to_string(),
        )),
        ..Default::default()
    };

    let mut service_manager = MockServiceManager::new();
    service_manager
        .running_services
        .borrow_mut()
        .push("TermService".to_string());
    service_manager
        .enabled_services
        .borrow_mut()
        .push("TermService".to_string());

    let audit_result = check_termservice_disabled(&policy, &service_manager).unwrap();
    assert_eq!(audit_result.policy_id, "A.4.a.i");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("TermService"));

    match remediate_termservice_disabled(&policy, &mut service_manager).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully stopped and disabled TermService
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_termservice_disabled(&policy, &service_manager).unwrap();
    assert!(audit_result_after.passed);
}

#[test]
fn test_windows_a_4_a_ii_spooler_disabled() {
    let policy = Policy {
        id: "A.4.a.ii".to_string(),
        platform: "windows".to_string(),
        check_type: "service_status".to_string(),
        service_name: Some("Spooler".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String(
            "stopped_disabled".to_string(),
        )),
        ..Default::default()
    };

    let mut service_manager = MockServiceManager::new();
    service_manager
        .running_services
        .borrow_mut()
        .push("Spooler".to_string());
    service_manager
        .enabled_services
        .borrow_mut()
        .push("Spooler".to_string());

    let audit_result = check_spooler_disabled(&policy, &service_manager).unwrap();
    assert_eq!(audit_result.policy_id, "A.4.a.ii");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("Spooler"));

    match remediate_spooler_disabled(&policy, &mut service_manager).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully stopped and disabled Spooler
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_spooler_disabled(&policy, &service_manager).unwrap();
    assert!(audit_result_after.passed);
}

#[test]
fn test_windows_a_4_b_i_w32time_enabled() {
    let policy = Policy {
        id: "A.4.b.i".to_string(),
        platform: "windows".to_string(),
        check_type: "service_status".to_string(),
        service_name: Some("W32Time".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String(
            "running".to_string(),
        )),
        ..Default::default()
    };

    let mut service_manager = MockServiceManager::new();
    // vulnerable: W32Time not running and not enabled

    let audit_result = check_w32time_enabled(&policy, &service_manager).unwrap();
    assert_eq!(audit_result.policy_id, "A.4.b.i");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("W32Time"));

    match remediate_w32time_enabled(&policy, &mut service_manager).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully started and enabled W32Time
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_w32time_enabled(&policy, &service_manager).unwrap();
    assert!(audit_result_after.passed);
}

#[test]
fn test_windows_a_7_a_i_remote_registry_disabled() {
    let policy = Policy {
        id: "A.7.a.i".to_string(),
        platform: "windows".to_string(),
        check_type: "service_status".to_string(),
        service_name: Some("RemoteRegistry".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String(
            "stopped_disabled".to_string(),
        )),
        ..Default::default()
    };

    let mut service_manager = MockServiceManager::new();
    service_manager
        .running_services
        .borrow_mut()
        .push("RemoteRegistry".to_string());
    service_manager
        .enabled_services
        .borrow_mut()
        .push("RemoteRegistry".to_string());

    let audit_result = check_remote_registry_disabled(&policy, &service_manager).unwrap();
    assert_eq!(audit_result.policy_id, "A.7.a.i");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("RemoteRegistry"));

    match remediate_remote_registry_disabled(&policy, &mut service_manager).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully stopped and disabled RemoteRegistry
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_remote_registry_disabled(&policy, &service_manager).unwrap();
    assert!(audit_result_after.passed);
}

// ============================================================================
// STAGE 5: SECURITY POLICIES (5 tests)
// ============================================================================

#[test]
fn test_windows_a_5_a_i_restrict_anonymous_sam() {
    let policy = Policy {
        id: "A.5.a.i".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(r"SYSTEM\CurrentControlSet\Control\Lsa".to_string()),
        value_name: Some("RestrictAnonymousSAM".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String("1".to_string())),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_dword(
        r"SYSTEM\CurrentControlSet\Control\Lsa",
        "RestrictAnonymousSAM",
        0, // vulnerable: anonymous SAM enumeration allowed
    );

    let audit_result = check_restrict_anonymous_sam(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.5.a.i");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("RestrictAnonymousSAM"));

    match remediate_restrict_anonymous_sam(&policy, &mut registry).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully restricted anonymous SAM access
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_restrict_anonymous_sam(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}

#[test]
fn test_windows_a_5_a_ii_restrict_anonymous() {
    let policy = Policy {
        id: "A.5.a.ii".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(r"SYSTEM\CurrentControlSet\Control\Lsa".to_string()),
        value_name: Some("RestrictAnonymous".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String("1".to_string())),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_dword(
        r"SYSTEM\CurrentControlSet\Control\Lsa",
        "RestrictAnonymous",
        0, // vulnerable: anonymous access allowed
    );

    let audit_result = check_restrict_anonymous(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.5.a.ii");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("RestrictAnonymous"));

    match remediate_restrict_anonymous(&policy, &mut registry).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully restricted anonymous access
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_restrict_anonymous(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}

#[test]
fn test_windows_a_5_b_i_lm_compatibility() {
    let policy = Policy {
        id: "A.5.b.i".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(r"SYSTEM\CurrentControlSet\Control\Lsa".to_string()),
        value_name: Some("LmCompatibilityLevel".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String("5".to_string())),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_dword(
        r"SYSTEM\CurrentControlSet\Control\Lsa",
        "LmCompatibilityLevel",
        2, // vulnerable: weak authentication
    );

    let audit_result = check_lm_compatibility(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.5.b.i");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("LmCompatibilityLevel"));

    match remediate_lm_compatibility(&policy, &mut registry).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully set LM compatibility level
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_lm_compatibility(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}

#[test]
fn test_windows_a_7_b_i_uac_elevation() {
    let policy = Policy {
        id: "A.7.b.i".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System".to_string()),
        value_name: Some("ConsentPromptBehaviorAdmin".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String("2".to_string())),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_dword(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System",
        "ConsentPromptBehaviorAdmin",
        0, // vulnerable: no UAC prompts
    );

    let audit_result = check_uac_elevation(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.7.b.i");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("ConsentPromptBehaviorAdmin"));

    match remediate_uac_elevation(&policy, &mut registry).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully set UAC elevation prompt
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_uac_elevation(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}

#[test]
fn test_windows_a_8_a_i_autoplay_disabled() {
    let policy = Policy {
        id: "A.8.a.i".to_string(),
        platform: "windows".to_string(),
        check_type: "registry_key".to_string(),
        target_path: Some(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer".to_string(),
        ),
        value_name: Some("NoDriveTypeAutoRun".to_string()),
        expected_state: Some(nogap_core::types::ExpectedState::String("255".to_string())),
        ..Default::default()
    };

    let mut registry = MockRegistry::new();
    registry.set_mock_dword(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer",
        "NoDriveTypeAutoRun",
        0, // vulnerable: autoplay enabled
    );

    let audit_result = check_autoplay_disabled(&policy, &registry).unwrap();
    assert_eq!(audit_result.policy_id, "A.8.a.i");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("NoDriveTypeAutoRun"));

    match remediate_autoplay_disabled(&policy, &mut registry).unwrap() {
        RemediateResult::Success(_msg) => {
            // Successfully disabled autoplay
        }
        RemediateResult::Failed(e) => panic!("Remediation failed: {}", e),
    }

    let audit_result_after = check_autoplay_disabled(&policy, &registry).unwrap();
    assert!(audit_result_after.passed);
}
