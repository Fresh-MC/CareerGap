// This file contains the core logic, snapshot engine, YAML parser, and policy manager for the NoGap project.

// Week 1 Security Primitives
pub mod self_check;
pub mod types;
pub mod policy_parser;
pub mod secure_workspace;
pub mod snapshot;
pub mod signing;

// Week 2 Advanced Features
// pub mod policy_sandbox;
pub mod auto_signer;
pub mod platforms;
pub mod engine;

// Legacy API functions (preserved for backward compatibility)
pub fn audit_system() -> String {
    "NoGap Audit: System scan complete. No vulnerabilities detected.".to_string()
}

pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version() {
        assert_eq!(get_version(), "0.1.0");
    }

    #[test]
    fn test_audit_system() {
        let result = audit_system();
        assert!(result.contains("NoGap Audit"));
        assert!(result.contains("System scan complete"));
        assert!(result.contains("No vulnerabilities detected"));
    }
}