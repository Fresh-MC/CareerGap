//! OS privilege checking for remediation operations.
//! Ensures appropriate elevated privileges before making system changes.

/// Check if the current process has administrator privileges on Windows.
#[cfg(target_os = "windows")]
pub fn ensure_admin() -> Result<(), String> {
    use windows_sys::Win32::UI::Shell::IsUserAnAdmin;

    unsafe {
        if IsUserAnAdmin() == 0 {
            return Err("Admin privileges required".to_string());
        }
    }
    Ok(())
}

/// Check if the current process has root privileges on Linux.
#[cfg(target_os = "linux")]
pub fn ensure_root() -> Result<(), String> {
    unsafe {
        if libc::geteuid() != 0 {
            return Err("Root privileges required".to_string());
        }
    }
    Ok(())
}

/// Privilege check for unsupported operating systems.
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
#[allow(dead_code)]
pub fn ensure_privs() -> Result<(), String> {
    Err("Privilege checks not supported on this OS".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privilege_check_exists() {
        #[cfg(target_os = "windows")]
        {
            // Just verify the function exists and returns a Result
            let _ = ensure_admin();
        }

        #[cfg(target_os = "linux")]
        {
            // Just verify the function exists and returns a Result
            let _ = ensure_root();
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            // Should always return error on unsupported OS
            assert!(ensure_privs().is_err());
        }
    }
}
