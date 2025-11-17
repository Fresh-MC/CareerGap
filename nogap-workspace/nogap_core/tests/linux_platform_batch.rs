use nogap_core::platforms::linux::{
    check_sshd_permit_empty_pw, check_sshd_permit_userenv,
    check_sysctl_aslr, check_sysctl_ptrace, check_ufw_default,
    remediate_sshd_permit_empty_pw, remediate_sshd_permit_userenv, remediate_sysctl_aslr,
    remediate_sysctl_ptrace, remediate_ufw_default, MockSysctlProvider, RemediateResult,
};
use nogap_core::types::Policy;
use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_aslr_sysctl() {
    let mut sysctl = MockSysctlProvider::new();
    sysctl.set_mock_value("kernel.randomize_va_space", "0");

    let policy = Policy {
        id: "B.2.b.i".to_string(),
        platform: "linux".to_string(),
        check_type: "sysctl_check".to_string(),
        key: Some("kernel.randomize_va_space".to_string()),
        ..Default::default()
    };

    // Check vulnerable state
    let audit_result = check_sysctl_aslr(&policy, &sysctl).unwrap();
    assert_eq!(audit_result.policy_id, "B.2.b.i");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("not properly configured"));

    // Remediate
    let remediate_result = remediate_sysctl_aslr(&policy, &mut sysctl).unwrap();
    match remediate_result {
        RemediateResult::Success(msg) => {
            assert!(msg.contains("ASLR enabled"));
        }
        RemediateResult::Failed(_) => panic!("Expected success"),
    }
}

#[test]
fn test_ptrace_sysctl() {
    let mut sysctl = MockSysctlProvider::new();
    sysctl.set_mock_value("kernel.yama.ptrace_scope", "0");

    let policy = Policy {
        id: "B.2.b.ii".to_string(),
        platform: "linux".to_string(),
        check_type: "sysctl_check".to_string(),
        key: Some("kernel.yama.ptrace_scope".to_string()),
        ..Default::default()
    };

    // Check vulnerable state
    let audit_result = check_sysctl_ptrace(&policy, &sysctl).unwrap();
    assert_eq!(audit_result.policy_id, "B.2.b.ii");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("not restricted"));

    // Remediate
    let remediate_result = remediate_sysctl_ptrace(&policy, &mut sysctl).unwrap();
    match remediate_result {
        RemediateResult::Success(msg) => {
            assert!(msg.contains("ptrace_scope restricted"));
        }
        RemediateResult::Failed(_) => panic!("Expected success"),
    }
}

#[test]
fn test_sshd_permit_empty_pw() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "# SSH Configuration").unwrap();
    writeln!(temp_file, "Port 22").unwrap();
    writeln!(temp_file, "PermitEmptyPasswords yes").unwrap();
    temp_file.flush().unwrap();

    let temp_path = temp_file.path().to_string_lossy().to_string();

    let policy = Policy {
        id: "B.3.a.xix".to_string(),
        platform: "linux".to_string(),
        check_type: "file_regex".to_string(),
        target_file: Some(temp_path.clone()),
        regex: Some(r"^\s*PermitEmptyPasswords\s+yes".to_string()),
        remediate_type: Some("file_replace".to_string()),
        replace_regex: Some(r"^\s*PermitEmptyPasswords\s+.*".to_string()),
        replace_with: Some("PermitEmptyPasswords no".to_string()),
        ..Default::default()
    };

    // Check vulnerable state
    let audit_result = check_sshd_permit_empty_pw(&policy).unwrap();
    assert_eq!(audit_result.policy_id, "B.3.a.xix");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("vulnerable pattern"));

    // Remediate
    let remediate_result = remediate_sshd_permit_empty_pw(&policy).unwrap();
    match remediate_result {
        RemediateResult::Success(msg) => {
            assert!(msg.contains("remediated successfully"));
        }
        RemediateResult::Failed(_) => panic!("Expected success"),
    }

    // Verify fix
    let content = fs::read_to_string(&temp_path).unwrap();
    assert!(content.contains("PermitEmptyPasswords no"));
    assert!(!content.contains("PermitEmptyPasswords yes"));
}

#[test]
fn test_sshd_permit_userenv() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "# SSH Configuration").unwrap();
    writeln!(temp_file, "Port 22").unwrap();
    writeln!(temp_file, "PermitUserEnvironment yes").unwrap();
    temp_file.flush().unwrap();

    let temp_path = temp_file.path().to_string_lossy().to_string();

    let policy = Policy {
        id: "B.3.a.xx".to_string(),
        platform: "linux".to_string(),
        check_type: "file_regex".to_string(),
        target_file: Some(temp_path.clone()),
        regex: Some(r"^\s*PermitUserEnvironment\s+yes".to_string()),
        remediate_type: Some("file_replace".to_string()),
        replace_regex: Some(r"^\s*PermitUserEnvironment\s+.*".to_string()),
        replace_with: Some("PermitUserEnvironment no".to_string()),
        ..Default::default()
    };

    // Check vulnerable state
    let audit_result = check_sshd_permit_userenv(&policy).unwrap();
    assert_eq!(audit_result.policy_id, "B.3.a.xx");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("vulnerable pattern"));

    // Remediate
    let remediate_result = remediate_sshd_permit_userenv(&policy).unwrap();
    match remediate_result {
        RemediateResult::Success(msg) => {
            assert!(msg.contains("remediated successfully"));
        }
        RemediateResult::Failed(_) => panic!("Expected success"),
    }

    // Verify fix
    let content = fs::read_to_string(&temp_path).unwrap();
    assert!(content.contains("PermitUserEnvironment no"));
    assert!(!content.contains("PermitUserEnvironment yes"));
}

#[test]
#[cfg(unix)]
fn test_ssh_host_key_perms() {
    use std::os::unix::fs::PermissionsExt;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "FAKE SSH HOST KEY").unwrap();
    temp_file.flush().unwrap();

    let temp_path = temp_file.path().to_string_lossy().to_string();

    // Make file world-writable
    let mut perms = fs::metadata(&temp_path).unwrap().permissions();
    perms.set_mode(0o666);
    fs::set_permissions(&temp_path, perms).unwrap();

    let policy = Policy {
        id: "B.6.a.i".to_string(),
        platform: "linux".to_string(),
        check_type: "file_permissions_glob".to_string(),
        target_glob: Some(temp_path.clone()),
        ..Default::default()
    };

    // Check vulnerable state
    let audit_result = check_ssh_host_key_perms(&policy).unwrap();
    assert_eq!(audit_result.policy_id, "B.6.a.i");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("world-writable"));

    // Remediate
    let remediate_result = remediate_ssh_host_key_perms(&policy).unwrap();
    match remediate_result {
        RemediateResult::Success(msg) => {
            assert!(msg.contains("permissions corrected"));
        }
        RemediateResult::Failed(_) => panic!("Expected success"),
    }

    // Verify fix
    let metadata = fs::metadata(&temp_path).unwrap();
    let mode = metadata.permissions().mode();
    assert_eq!(mode & 0o002, 0, "World-writable bit should be removed");
}

#[test]
fn test_ufw_default_deny() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "# UFW Configuration").unwrap();
    writeln!(temp_file, "DEFAULT_INPUT_POLICY=\"ACCEPT\"").unwrap();
    writeln!(temp_file, "DEFAULT_OUTPUT_POLICY=\"ACCEPT\"").unwrap();
    temp_file.flush().unwrap();

    let temp_path = temp_file.path().to_string_lossy().to_string();

    let policy = Policy {
        id: "B.5.a.vii".to_string(),
        platform: "linux".to_string(),
        check_type: "file_regex".to_string(),
        target_file: Some(temp_path.clone()),
        regex: Some(r#"DEFAULT_INPUT_POLICY\s*=\s*"(ACCEPT|ALLOW)""#.to_string()),
        remediate_type: Some("file_replace".to_string()),
        replace_regex: Some(r#"DEFAULT_INPUT_POLICY\s*=\s*".*""#.to_string()),
        replace_with: Some(r#"DEFAULT_INPUT_POLICY="DROP""#.to_string()),
        ..Default::default()
    };

    // Check vulnerable state
    let audit_result = check_ufw_default(&policy).unwrap();
    assert_eq!(audit_result.policy_id, "B.5.a.vii");
    assert!(!audit_result.passed);
    assert!(audit_result.message.contains("vulnerable pattern"));

    // Remediate
    let remediate_result = remediate_ufw_default(&policy).unwrap();
    match remediate_result {
        RemediateResult::Success(msg) => {
            assert!(msg.contains("remediated successfully"));
        }
        RemediateResult::Failed(_) => panic!("Expected success"),
    }

    // Verify fix
    let content = fs::read_to_string(&temp_path).unwrap();
    assert!(content.contains(r#"DEFAULT_INPUT_POLICY="DROP""#));
    assert!(!content.contains(r#"DEFAULT_INPUT_POLICY="ACCEPT""#));
}
