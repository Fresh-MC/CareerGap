use nogap_core::engine::{MockSnapshotProvider, SnapshotProvider};
use nogap_core::platforms::linux;
use nogap_core::platforms::windows::{self, MockServiceManager, ServiceManager};
use nogap_core::types::Policy;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_windows_service_full_transaction() {
    let snapshot_provider = MockSnapshotProvider::new();
    let service_manager = MockServiceManager::new();

    let mut policy = Policy::default();
    policy.id = "A.7.a.i".to_string();
    policy.platform = "windows".to_string();
    policy.check_type = "service_status".to_string();
    policy.service_name = Some("RemoteRegistry".to_string());
    policy.expected_state = Some(nogap_core::types::ExpectedState::String(
        "stopped_disabled".to_string(),
    ));

    // Initial check - service is running and enabled (vulnerable)
    let check_result_1 = windows::check_service_status(&policy, &service_manager).unwrap();
    assert_eq!(check_result_1.policy_id, "A.7.a.i");
    assert!(
        !check_result_1.passed,
        "Initial check should fail (service running/enabled)"
    );

    // Save snapshot before remediation
    snapshot_provider
        .save_snapshot(&policy.id, "before", "Service running and enabled")
        .unwrap();

    // Remediate - stop and disable the service
    let remediate_result = windows::remediate_service_disable(&policy, &service_manager).unwrap();
    match remediate_result {
        windows::RemediateResult::Success(message) => {
            assert!(message.contains("stopped and disabled"));
        }
        windows::RemediateResult::Failed(_) => panic!("Remediation should succeed"),
    }

    // Verify service state changed
    assert!(
        !service_manager.is_running("RemoteRegistry").unwrap(),
        "Service should be stopped"
    );
    assert!(
        !service_manager.is_enabled("RemoteRegistry").unwrap(),
        "Service should be disabled"
    );

    // Save snapshot after remediation
    snapshot_provider
        .save_snapshot(&policy.id, "after", "Service stopped and disabled")
        .unwrap();

    // Final check - service is stopped and disabled (secure)
    let check_result_2 = windows::check_service_status(&policy, &service_manager).unwrap();
    assert_eq!(check_result_2.policy_id, "A.7.a.i");
    assert!(
        check_result_2.passed,
        "Final check should pass (service stopped/disabled)"
    );

    // Verify snapshots were called exactly twice
    assert_eq!(
        snapshot_provider.get_count(),
        2,
        "Snapshot provider should be called exactly twice"
    );
}

#[test]
fn test_linux_file_regex_full_transaction() {
    let snapshot_provider = MockSnapshotProvider::new();

    // Create temp file with vulnerable config
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "# SSH Configuration").unwrap();
    writeln!(temp_file, "PermitRootLogin yes").unwrap();
    writeln!(temp_file, "Port 22").unwrap();
    temp_file.flush().unwrap();

    let temp_path = temp_file.path().to_string_lossy().to_string();

    let mut policy = Policy::default();
    policy.id = "B.1.a.i".to_string();
    policy.platform = "linux".to_string();
    policy.check_type = "file_regex".to_string();
    policy.target_file = Some(temp_path.clone());
    policy.regex = Some(r"^PermitRootLogin\s+yes".to_string());
    policy.replace_regex = Some(r"^PermitRootLogin\s+yes".to_string());
    policy.replace_with = Some("PermitRootLogin no".to_string());

    // Initial check - should fail (vulnerable pattern exists)
    let check_result_1 = linux::check_permit_root_login(&policy).unwrap();
    assert_eq!(check_result_1.policy_id, "B.1.a.i");
    assert!(
        !check_result_1.passed,
        "Initial check should fail (PermitRootLogin yes found)"
    );
    assert!(check_result_1.message.contains("vulnerable pattern"));

    // Save snapshot before remediation
    snapshot_provider
        .save_snapshot(&policy.id, "before", "PermitRootLogin yes")
        .unwrap();

    // Remediate - replace vulnerable pattern
    let remediate_result = linux::remediate_permit_root_login(&policy).unwrap();
    match remediate_result {
        linux::RemediateResult::Success(message) => {
            assert!(message.contains("remediated"));
        }
        linux::RemediateResult::Failed(message) => {
            panic!("Remediation should succeed: {}", message)
        }
    }

    // Save snapshot after remediation
    snapshot_provider
        .save_snapshot(&policy.id, "after", "PermitRootLogin no")
        .unwrap();

    // Final check - should pass (vulnerable pattern removed)
    let check_result_2 = linux::check_permit_root_login(&policy).unwrap();
    assert_eq!(check_result_2.policy_id, "B.1.a.i");
    assert!(
        check_result_2.passed,
        "Final check should pass (PermitRootLogin no)"
    );

    // Verify snapshots were called exactly twice
    assert_eq!(
        snapshot_provider.get_count(),
        2,
        "Snapshot provider should be called exactly twice"
    );
}
