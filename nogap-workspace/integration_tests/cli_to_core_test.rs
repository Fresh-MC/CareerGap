//! Integration test for CLI calling nogap_core functions

use nogap_core;

#[test]
fn test_core_version() {
    let version = nogap_core::get_version();
    assert_eq!(version, "0.1.0");
}

#[test]
fn test_core_audit() {
    let result = nogap_core::audit_system();
    assert!(result.contains("NoGap Audit"));
    assert!(result.contains("System scan complete"));
}
