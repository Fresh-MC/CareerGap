use crate::platforms::linux;
use crate::platforms::windows;
use crate::types::Policy;
use std::error::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct AuditResult {
    pub policy_id: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RemediateResult {
    Success { policy_id: String, message: String },
    Failed { policy_id: String, message: String },
}

pub trait SnapshotProvider {
    fn save_snapshot(
        &self,
        policy_id: &str,
        stage: &str,
        context: &str,
    ) -> Result<(), Box<dyn Error>>;
}

pub struct RealSnapshotProvider;

impl SnapshotProvider for RealSnapshotProvider {
    fn save_snapshot(
        &self,
        policy_id: &str,
        stage: &str,
        context: &str,
    ) -> Result<(), Box<dyn Error>> {
        // For real implementation, we would use snapshot::init_db() and save_snapshot()
        // For now, just log the snapshot
        println!("📸 Snapshot [{}] - {}: {}", stage, policy_id, context);
        Ok(())
    }
}

pub struct MockSnapshotProvider {
    pub snapshot_count: std::sync::Mutex<usize>,
}

impl MockSnapshotProvider {
    pub fn new() -> Self {
        Self {
            snapshot_count: std::sync::Mutex::new(0),
        }
    }

    pub fn get_count(&self) -> usize {
        *self.snapshot_count.lock().unwrap()
    }
}

impl SnapshotProvider for MockSnapshotProvider {
    fn save_snapshot(
        &self,
        _policy_id: &str,
        _stage: &str,
        _context: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut count = self.snapshot_count.lock().unwrap();
        *count += 1;
        Ok(())
    }
}

pub fn audit(policies: &[Policy]) -> Result<Vec<AuditResult>, Box<dyn Error>> {
    let mut results = Vec::new();

    for policy in policies {
        let result = match (policy.platform.as_str(), policy.check_type.as_str(), policy.id.as_str()) {
            // Windows registry-based checks (Stage 5)
            ("windows", "local_policy", "A.1.a.i") => {
                // A.1.a.i: Password complexity (implemented as registry check)
                let registry = windows::RealRegistry;
                let platform_result = windows::check_password_complexity(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "registry_key", "A.1.a.ii") => {
                // A.1.a.ii: Minimum password length
                let registry = windows::RealRegistry;
                let platform_result = windows::check_min_password_length(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "registry_key", "A.1.a.iii") => {
                // A.1.a.iii: Maximum password age
                let registry = windows::RealRegistry;
                let platform_result = windows::check_max_password_age(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "registry_key", "A.2.a.ii") => {
                // A.2.a.ii: Administrator account renamed
                let registry = windows::RealRegistry;
                let platform_result = windows::check_admin_account_renamed(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "registry_key", "A.2.b.i") => {
                // A.2.b.i: Account lockout threshold
                let registry = windows::RealRegistry;
                let platform_result = windows::check_lockout_threshold(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "registry_key", "A.2.b.ii") => {
                // A.2.b.ii: Account lockout duration
                let registry = windows::RealRegistry;
                let platform_result = windows::check_lockout_duration(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "registry_key", "A.3.a.i") => {
                // A.3.a.i: SMBv1 disabled
                let registry = windows::RealRegistry;
                let platform_result = windows::check_smb1_disabled(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "registry_key", "A.3.a.ii") => {
                // A.3.a.ii: Windows Firewall Domain profile
                let registry = windows::RealRegistry;
                let platform_result = windows::check_firewall_domain_profile(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "registry_key", "A.3.a.iv") => {
                // A.3.a.iv: Windows Firewall Public profile
                let registry = windows::RealRegistry;
                let platform_result = windows::check_firewall_public_profile(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "service_status", "A.4.a.i") => {
                // A.4.a.i: Remote Desktop (TermService) disabled
                let service_manager = windows::RealServiceManager;
                let platform_result = windows::check_termservice_disabled(policy, &service_manager)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "service_status", "A.4.a.ii") => {
                // A.4.a.ii: Print Spooler disabled
                let service_manager = windows::RealServiceManager;
                let platform_result = windows::check_spooler_disabled(policy, &service_manager)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "service_status", "A.4.b.i") => {
                // A.4.b.i: Windows Time (W32Time) enabled
                let service_manager = windows::RealServiceManager;
                let platform_result = windows::check_w32time_enabled(policy, &service_manager)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "registry_key", "A.5.a.i") => {
                // A.5.a.i: Restrict anonymous SAM enumeration
                let registry = windows::RealRegistry;
                let platform_result = windows::check_restrict_anonymous_sam(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "registry_key", "A.5.a.ii") => {
                // A.5.a.ii: Restrict anonymous
                let registry = windows::RealRegistry;
                let platform_result = windows::check_restrict_anonymous(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "registry_key", "A.5.b.i") => {
                // A.5.b.i: LAN Manager authentication level (NTLMv2)
                let registry = windows::RealRegistry;
                let platform_result = windows::check_lm_compatibility(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "service_status", "A.7.a.i") => {
                // A.7.a.i: Remote Registry disabled
                let service_manager = windows::RealServiceManager;
                let platform_result = windows::check_remote_registry_disabled(policy, &service_manager)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "registry_key", "A.7.b.i") => {
                // A.7.b.i: UAC elevation prompt
                let registry = windows::RealRegistry;
                let platform_result = windows::check_uac_elevation(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("windows", "registry_key", "A.8.a.i") => {
                // A.8.a.i: AutoPlay disabled for all drives
                let registry = windows::RealRegistry;
                let platform_result = windows::check_autoplay_disabled(policy, &registry)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            // Legacy Stage 3/4 fallback for generic service_status
            ("windows", "service_status", _) => {
                let service_manager = windows::RealServiceManager;
                let platform_result = windows::check_service_status(policy, &service_manager)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            // Linux file_regex checks (Stage 5)
            ("linux", "file_regex", "B.1.a.i") => {
                // B.1.a.i: SSH PermitRootLogin
                let platform_result = linux::check_permit_root_login(policy)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "file_regex", "B.1.a.ii") => {
                // B.1.a.ii: SSH Protocol 2
                let platform_result = linux::check_ssh_protocol_2(policy)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "file_regex", "B.1.a.iii") => {
                // B.1.a.iii: SSH MaxAuthTries
                let platform_result = linux::check_ssh_max_auth_tries(policy)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "file_regex", "B.1.b.i") => {
                // B.1.b.i: SSH X11Forwarding
                let platform_result = linux::check_ssh_x11_forwarding(policy)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "file_regex", "B.5.a.i") => {
                // B.5.a.i: Core dumps disabled
                let platform_result = linux::check_core_dumps(policy)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "file_regex", "B.7.a.i") => {
                // B.7.a.i: PASS_MAX_DAYS
                let platform_result = linux::check_pass_max_days(policy)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "file_regex", "B.7.a.ii") => {
                // B.7.a.ii: PASS_MIN_DAYS
                let platform_result = linux::check_pass_min_days(policy)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "file_regex", "B.7.b.i") => {
                // B.7.b.i: PASS_WARN_AGE
                let platform_result = linux::check_pass_warn_age(policy)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "file_regex", "B.8.a.i") => {
                // B.8.a.i: Default umask
                let platform_result = linux::check_default_umask(policy)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            // Linux sysctl_check (Stage 5)
            ("linux", "sysctl_check", "B.2.a.i") => {
                // B.2.a.i: IP forwarding disabled
                let sysctl = linux::RealSysctlProvider;
                let platform_result = linux::check_ip_forward(policy, &sysctl)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "sysctl_check", "B.2.a.ii") => {
                // B.2.a.ii: ICMP redirects disabled
                let sysctl = linux::RealSysctlProvider;
                let platform_result = linux::check_icmp_redirects(policy, &sysctl)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "sysctl_check", "B.2.a.iii") => {
                // B.2.a.iii: Secure ICMP redirects disabled
                let sysctl = linux::RealSysctlProvider;
                let platform_result = linux::check_secure_redirects(policy, &sysctl)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "sysctl_check", "B.2.b.i") => {
                // B.2.b.i: TCP SYN cookies enabled
                let sysctl = linux::RealSysctlProvider;
                let platform_result = linux::check_tcp_syn_cookies(policy, &sysctl)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "sysctl_check", "B.6.b.i") => {
                // B.6.b.i: ASLR enabled
                let sysctl = linux::RealSysctlProvider;
                let platform_result = linux::check_sysctl_aslr(policy, &sysctl)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "sysctl_check", "B.8.b.i") => {
                // B.8.b.i: IPv6 router ads disabled
                let sysctl = linux::RealSysctlProvider;
                let platform_result = linux::check_ipv6_router_ads(policy, &sysctl)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "sysctl_check", "B.9.a.i") => {
                // B.9.a.i: Source route disabled
                let sysctl = linux::RealSysctlProvider;
                let platform_result = linux::check_source_route(policy, &sysctl)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "sysctl_check", "B.9.b.i") => {
                // B.9.b.i: Martian packets logged
                let sysctl = linux::RealSysctlProvider;
                let platform_result = linux::check_log_martians(policy, &sysctl)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "sysctl_check", "B.9.c.i") => {
                // B.9.c.i: ICMP broadcast ignored
                let sysctl = linux::RealSysctlProvider;
                let platform_result = linux::check_icmp_echo_ignore(policy, &sysctl)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            // Linux file_permissions_glob (Stage 5)
            ("linux", "file_permissions_glob", "B.4.a.i") => {
                // B.4.a.i: /etc/passwd permissions
                let platform_result = linux::check_passwd_perms(policy)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "file_permissions_glob", "B.4.a.ii") => {
                // B.4.a.ii: /etc/shadow permissions
                let platform_result = linux::check_shadow_perms(policy)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "file_permissions_glob", "B.4.a.iii") => {
                // B.4.a.iii: /etc/group permissions
                let platform_result = linux::check_group_perms(policy)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "file_permissions_glob", "B.4.b.i") => {
                // B.4.b.i: /etc/ssh/sshd_config permissions
                let platform_result = linux::check_sshd_config_perms(policy)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            // Linux package_absence (Stage 5)
            ("linux", "package_absence", "B.3.a.i") => {
                // B.3.a.i: telnet not installed
                let pkg_provider = linux::RealPackageProvider;
                let platform_result = linux::check_telnet_absent(policy, &pkg_provider)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "package_absence", "B.3.a.ii") => {
                // B.3.a.ii: rsh-client not installed
                let pkg_provider = linux::RealPackageProvider;
                let platform_result = linux::check_rsh_absent(policy, &pkg_provider)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            // Linux service_status (Stage 5)
            ("linux", "service_status", "B.3.b.i") => {
                // B.3.b.i: avahi-daemon disabled
                let svc_mgr = linux::RealServiceManager;
                let platform_result = linux::check_avahi_disabled(policy, &svc_mgr)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            ("linux", "service_status", "B.6.a.i") => {
                // B.6.a.i: rsyslog enabled
                let svc_mgr = linux::RealServiceManager;
                let platform_result = linux::check_rsyslog_enabled(policy, &svc_mgr)?;
                AuditResult {
                    policy_id: platform_result.policy_id,
                    passed: platform_result.passed,
                    message: platform_result.message,
                }
            }
            _ => {
                return Err(format!(
                    "Unsupported platform/check_type combination: {}/{} (policy: {})",
                    policy.platform, policy.check_type, policy.id
                )
                .into())
            }
        };

        results.push(result);
    }

    Ok(results)
}

pub fn remediate(
    policies: &[Policy],
    snapshot_provider: &dyn SnapshotProvider,
) -> Result<Vec<RemediateResult>, Box<dyn Error>> {
    let mut results = Vec::new();

    for policy in policies {
        let context = format!(
            "Remediating policy {} on platform {}",
            policy.id, policy.platform
        );

        snapshot_provider.save_snapshot(&policy.id, "BEFORE", &context)?;

        let remediate_result = match (policy.platform.as_str(), policy.id.as_str()) {
            // Windows registry-based remediations (Stage 5)
            ("windows", "A.1.a.i") => {
                // A.1.a.i: Password complexity
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_password_complexity(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.1.a.ii") => {
                // A.1.a.ii: Minimum password length
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_min_password_length(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.1.a.iii") => {
                // A.1.a.iii: Maximum password age
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_max_password_age(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.2.a.ii") => {
                // A.2.a.ii: Administrator account renamed
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_admin_account_renamed(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.2.b.i") => {
                // A.2.b.i: Account lockout threshold
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_lockout_threshold(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.2.b.ii") => {
                // A.2.b.ii: Account lockout duration
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_lockout_duration(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.3.a.i") => {
                // A.3.a.i: SMBv1 disabled
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_smb1_disabled(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.3.a.ii") => {
                // A.3.a.ii: Windows Firewall Domain profile
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_firewall_domain_profile(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.3.a.iv") => {
                // A.3.a.iv: Windows Firewall Public profile
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_firewall_public_profile(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.4.a.i") => {
                // A.4.a.i: Remote Desktop (TermService) disabled
                let service_manager = windows::RealServiceManager;
                let platform_result = windows::remediate_termservice_disabled(policy, &service_manager)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.4.a.ii") => {
                // A.4.a.ii: Print Spooler disabled
                let service_manager = windows::RealServiceManager;
                let platform_result = windows::remediate_spooler_disabled(policy, &service_manager)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.4.b.i") => {
                // A.4.b.i: Windows Time (W32Time) enabled
                let service_manager = windows::RealServiceManager;
                let platform_result = windows::remediate_w32time_enabled(policy, &service_manager)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.5.a.i") => {
                // A.5.a.i: Restrict anonymous SAM enumeration
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_restrict_anonymous_sam(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.5.a.ii") => {
                // A.5.a.ii: Restrict anonymous
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_restrict_anonymous(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.5.b.i") => {
                // A.5.b.i: LAN Manager authentication level (NTLMv2)
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_lm_compatibility(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.7.a.i") => {
                // A.7.a.i: Remote Registry disabled
                let service_manager = windows::RealServiceManager;
                let platform_result = windows::remediate_remote_registry_disabled(policy, &service_manager)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.7.b.i") => {
                // A.7.b.i: UAC elevation prompt
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_uac_elevation(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("windows", "A.8.a.i") => {
                // A.8.a.i: AutoPlay disabled for all drives
                let registry = windows::RealRegistry;
                let platform_result = windows::remediate_autoplay_disabled(policy, &registry)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            // Linux Stage 5 remediations
            ("linux", "B.1.a.i") => {
                let platform_result = linux::remediate_permit_root_login(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.1.a.ii") => {
                let platform_result = linux::remediate_ssh_protocol_2(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.1.a.iii") => {
                let platform_result = linux::remediate_ssh_max_auth_tries(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.1.b.i") => {
                let platform_result = linux::remediate_ssh_x11_forwarding(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.2.a.i") => {
                let mut sysctl = linux::RealSysctlProvider;
                let platform_result = linux::remediate_ip_forward(policy, &mut sysctl)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.2.a.ii") => {
                let mut sysctl = linux::RealSysctlProvider;
                let platform_result = linux::remediate_icmp_redirects(policy, &mut sysctl)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.2.a.iii") => {
                let mut sysctl = linux::RealSysctlProvider;
                let platform_result = linux::remediate_secure_redirects(policy, &mut sysctl)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.2.b.i") => {
                let mut sysctl = linux::RealSysctlProvider;
                let platform_result = linux::remediate_tcp_syn_cookies(policy, &mut sysctl)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.3.a.i") => {
                let pkg = linux::RealPackageProvider;
                let platform_result = linux::remediate_telnet_absent(policy, &pkg)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.3.a.ii") => {
                let pkg = linux::RealPackageProvider;
                let platform_result = linux::remediate_rsh_absent(policy, &pkg)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.3.b.i") => {
                let mut svc = linux::RealServiceManager;
                let platform_result = linux::remediate_avahi_disabled(policy, &mut svc)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.4.a.i") => {
                let platform_result = linux::remediate_passwd_perms(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.4.a.ii") => {
                let platform_result = linux::remediate_shadow_perms(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.4.a.iii") => {
                let platform_result = linux::remediate_group_perms(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.4.b.i") => {
                let platform_result = linux::remediate_sshd_config_perms(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.5.a.i") => {
                let platform_result = linux::remediate_core_dumps(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.6.a.i") => {
                let mut svc = linux::RealServiceManager;
                let platform_result = linux::remediate_rsyslog_enabled(policy, &mut svc)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.6.b.i") => {
                let mut sysctl = linux::RealSysctlProvider;
                let platform_result = linux::remediate_sysctl_aslr(policy, &mut sysctl)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.7.a.i") => {
                let platform_result = linux::remediate_pass_max_days(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.7.a.ii") => {
                let platform_result = linux::remediate_pass_min_days(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.7.b.i") => {
                let platform_result = linux::remediate_pass_warn_age(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.8.a.i") => {
                let platform_result = linux::remediate_default_umask(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.8.b.i") => {
                let mut sysctl = linux::RealSysctlProvider;
                let platform_result = linux::remediate_ipv6_router_ads(policy, &mut sysctl)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.9.a.i") => {
                let mut sysctl = linux::RealSysctlProvider;
                let platform_result = linux::remediate_source_route(policy, &mut sysctl)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.9.b.i") => {
                let mut sysctl = linux::RealSysctlProvider;
                let platform_result = linux::remediate_log_martians(policy, &mut sysctl)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", "B.9.c.i") => {
                let mut sysctl = linux::RealSysctlProvider;
                let platform_result = linux::remediate_icmp_echo_ignore(policy, &mut sysctl)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            // Legacy Stage 3/4 fallback patterns
            ("windows", _) if policy.remediate_type.as_deref() == Some("service_disable") => {
                let service_manager = windows::RealServiceManager;
                let platform_result =
                    windows::remediate_service_disable(policy, &service_manager)?;
                match platform_result {
                    windows::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    windows::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            ("linux", _) if policy.remediate_type.as_deref() == Some("file_replace") => {
                let platform_result = linux::remediate_file_replace(policy)?;
                match platform_result {
                    linux::RemediateResult::Success(msg) => RemediateResult::Success {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                    linux::RemediateResult::Failed(msg) => RemediateResult::Failed {
                        policy_id: policy.id.clone(),
                        message: msg,
                    },
                }
            }
            _ => {
                return Err(format!(
                    "Unsupported platform/remediate_type combination: {}/{} (policy: {})",
                    policy.platform,
                    policy.remediate_type.as_deref().unwrap_or("none"),
                    policy.id
                )
                .into())
            }
        };

        snapshot_provider.save_snapshot(&policy.id, "AFTER", &context)?;

        results.push(remediate_result);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Policy;

    #[test]
    fn test_audit_unsupported_platform() {
        let policy = Policy {
            id: "TEST.1".to_string(),
            platform: "macos".to_string(),
            check_type: "unknown".to_string(),
            ..Default::default()
        };

        let result = audit(&[policy]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unsupported platform"));
    }

    #[test]
    fn test_remediate_unsupported_type() {
        let policy = Policy {
            id: "TEST.1".to_string(),
            platform: "linux".to_string(),
            remediate_type: Some("unknown".to_string()),
            ..Default::default()
        };

        let snapshot_provider = MockSnapshotProvider::new();
        let result = remediate(&[policy], &snapshot_provider);
        assert!(result.is_err());
    }
}
