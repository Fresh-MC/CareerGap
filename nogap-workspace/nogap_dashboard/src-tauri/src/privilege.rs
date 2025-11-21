//! OS privilege checking for remediation operations.
//! Ensures appropriate elevated privileges before making system changes.

/// Check if the current process has administrator privileges on Windows.
/// Uses winreg to attempt write access to HKLM as a privilege test.
#[cfg(target_os = "windows")]
pub fn ensure_admin() -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    
    // Try to open a system registry key with write access
    match hklm.open_subkey_with_flags(
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run",
        KEY_WRITE,
    ) {
        Ok(_) => Ok(()),
        Err(_) => Err("Admin privileges required. Run as Administrator.".to_string()),
    }
}

/// Check if the current process has root privileges on Linux.
#[cfg(target_os = "linux")]
pub fn ensure_root() -> Result<(), String> {
    unsafe {
        if libc::geteuid() != 0 {
            return Err("Root privileges required. Run with sudo.".to_string());
        }
    }
    Ok(())
}

/// Privilege check for unsupported operating systems (no-op).
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
#[allow(dead_code)]
pub fn ensure_privs() -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privilege_check_exists() {
        #[cfg(target_os = "windows")]
        {
            let _ = ensure_admin();
        }

        #[cfg(target_os = "linux")]
        {
            let _ = ensure_root();
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            assert!(ensure_privs().is_ok());
        }
    }
}
