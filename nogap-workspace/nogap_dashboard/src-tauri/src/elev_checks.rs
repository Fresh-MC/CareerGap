//! Real admin/elevated privilege enforcement for NoGap.
//! If ensure_admin() fails, privileged commands MUST NOT run.

use std::io;

pub type PrivResult<T> = Result<T, PrivilegeError>;

#[derive(Debug, thiserror::Error)]
pub enum PrivilegeError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Not running with administrative privileges")]
    NotElevated,
    #[error("Platform error: {0}")]
    Other(String),
}

#[cfg(target_os = "windows")]
pub fn is_elevated() -> PrivResult<bool> {
    use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use std::mem;

    unsafe {
        let mut token_handle = std::ptr::null_mut();
        
        // Open the current process token
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle) == 0 {
            return Err(PrivilegeError::Io(io::Error::last_os_error()));
        }

        // Query token elevation status
        let mut elevation: TOKEN_ELEVATION = mem::zeroed();
        let mut return_length = 0u32;

        let result = GetTokenInformation(
            token_handle,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut return_length,
        );

        // Close token handle
        winapi::um::handleapi::CloseHandle(token_handle);

        if result == 0 {
            return Err(PrivilegeError::Io(io::Error::last_os_error()));
        }

        Ok(elevation.TokenIsElevated != 0)
    }
}

#[cfg(unix)]
pub fn is_elevated() -> PrivResult<bool> {
    use nix::unistd::Uid;
    
    // Check if effective user ID is root (0)
    Ok(Uid::effective().is_root())
}

/// Ensures the process is running with administrative privileges.
/// Returns Ok(()) if elevated, or Err(PrivilegeError::NotElevated) if not.
pub fn ensure_admin() -> PrivResult<()> {
    if is_elevated()? {
        Ok(())
    } else {
        Err(PrivilegeError::NotElevated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_elevated_returns_result() {
        // This test just ensures the function can be called without panicking
        let result = is_elevated();
        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_admin_returns_result() {
        // This test ensures ensure_admin returns a result
        let result = ensure_admin();
        // We can't guarantee the test environment is elevated, so just check it returns
        assert!(result.is_ok() || matches!(result, Err(PrivilegeError::NotElevated)));
    }
}
