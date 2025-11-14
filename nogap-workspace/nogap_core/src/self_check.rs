use sha2::{Digest, Sha256};
use std::{env, fs};
use std::thread::{self, JoinHandle};
use crossbeam_channel::{bounded, Receiver};

/// Result type for integrity checks
#[derive(Debug, Clone, PartialEq)]
pub enum IntegrityStatus {
    Pending,
    Verified,
    Failed(String),
}

/// Handle for non-blocking integrity verification
pub struct IntegrityCheckHandle {
    thread_handle: Option<JoinHandle<IntegrityStatus>>,
    receiver: Receiver<IntegrityStatus>,
}

impl IntegrityCheckHandle {
    /// Non-blocking poll to check if integrity verification is complete
    pub fn poll_integrity_status(&self) -> IntegrityStatus {
        // Try to receive the latest status without blocking
        let mut latest = IntegrityStatus::Pending;
        while let Ok(status) = self.receiver.try_recv() {
            latest = status;
        }
        latest
    }

    /// Blocking wait for integrity verification result
    pub fn wait_for_result(self) -> IntegrityStatus {
        if let Some(handle) = self.thread_handle {
            match handle.join() {
                Ok(status) => status,
                Err(_) => IntegrityStatus::Failed("Thread panicked during verification".to_string()),
            }
        } else {
            // Get result from channel
            self.receiver
                .recv()
                .unwrap_or(IntegrityStatus::Failed("Channel error".to_string()))
        }
    }
}

/// Starts integrity check on a background thread (non-blocking)
/// 
/// Returns a handle that can be polled for results without blocking the main thread.
/// Use `poll_integrity_status()` for non-blocking checks or `wait_for_result()` to block.
pub fn start_integrity_check() -> IntegrityCheckHandle {
    let (sender, receiver) = bounded(1);
    
    let thread_handle = thread::spawn(move || {
        let status = match verify_integrity_internal() {
            Ok(_) => IntegrityStatus::Verified,
            Err(e) => IntegrityStatus::Failed(e),
        };
        
        // Send result through channel
        let _ = sender.send(status.clone());
        
        // Trigger critical alert on failure
        if let IntegrityStatus::Failed(ref msg) = status {
            critical_alert(msg);
        }
        
        status
    });
    
    IntegrityCheckHandle {
        thread_handle: Some(thread_handle),
        receiver,
    }
}

/// Internal integrity verification (extracted for reuse)
fn verify_integrity_internal() -> Result<(), String> {
    let exe = env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;
    let data = fs::read(&exe).map_err(|e| format!("Failed to read executable: {}", e))?;
    
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let current = hasher.finalize();

    let expected = include_bytes!("../expected_hash.bin");
    
    if &current[..] != &expected[..] {
        return Err("Binary integrity check failed: executable has been tampered with".into());
    }
    
    Ok(())
}

/// Critical alert callback for integrity failures
fn critical_alert(message: &str) {
    eprintln!("🚨 CRITICAL SECURITY ALERT: {}", message);
    eprintln!("🚨 Binary integrity compromised - execution should halt immediately");
    // In production, this would trigger logging, notifications, or system shutdown
}

/// Verifies the integrity of the current executable by comparing its SHA256 hash
/// against an embedded reference hash.
/// 
/// This is the blocking version - for non-blocking checks use `start_integrity_check()`
pub fn verify_self_integrity() -> Result<(), String> {
    verify_integrity_internal()?;
    println!("✅ Binary verified successfully.");
    Ok(())
}

/// Generates the hash of the current executable for embedding during build
pub fn generate_self_hash() -> Result<Vec<u8>, String> {
    let exe = env::current_exe().map_err(|e| format!("Failed to get executable path: {}", e))?;
    let data = fs::read(&exe).map_err(|e| format!("Failed to read executable: {}", e))?;
    
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(hasher.finalize().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_generate_hash() {
        let result = generate_self_hash();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32); // SHA256 produces 32 bytes
    }

    #[test]
    fn test_threaded_integrity_check() {
        let handle = start_integrity_check();
        
        // Initially should be pending
        match handle.poll_integrity_status() {
            IntegrityStatus::Pending => {
                // Give thread time to complete
                thread::sleep(Duration::from_millis(100));
            }
            _ => {}
        }
        
        // Wait for final result
        let status = handle.wait_for_result();
        
        // Should either verify or fail (depending on hash match)
        assert!(matches!(status, IntegrityStatus::Verified | IntegrityStatus::Failed(_)));
    }

    #[test]
    fn test_poll_integrity_status() {
        let handle = start_integrity_check();
        
        // Poll multiple times and wait for completion
        let mut final_status = IntegrityStatus::Pending;
        for _ in 0..100 {
            final_status = handle.poll_integrity_status();
            if !matches!(final_status, IntegrityStatus::Pending) {
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
        
        // Should eventually complete (either verify or fail)
        assert!(matches!(
            final_status,
            IntegrityStatus::Verified | IntegrityStatus::Failed(_)
        ));
    }
}
