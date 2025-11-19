use crate::signing::{generate_keypair, sign_file};
use crossbeam_channel::{bounded, Receiver};
use rsa::{RsaPrivateKey, RsaPublicKey};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Auto-signing job status
#[derive(Debug, Clone, PartialEq)]
pub enum SigningStatus {
    Pending,
    InProgress,
    Completed { signature: Vec<u8> },
    Failed(String),
}

/// Auto-signing job configuration
#[derive(Debug, Clone)]
pub struct SigningJob {
    pub file_path: PathBuf,
    pub output_path: PathBuf,
    pub job_id: usize,
}

/// Handle for auto-signing operations
pub struct AutoSignerHandle {
    job_id: usize,
    receiver: Receiver<SigningStatus>,
}

impl AutoSignerHandle {
    /// Non-blocking poll for signing status
    pub fn poll_status(&self) -> SigningStatus {
        // Try to receive the latest status
        let mut latest = SigningStatus::Pending;
        while let Ok(status) = self.receiver.try_recv() {
            latest = status;
        }
        latest
    }

    /// Blocking wait for signing completion
    pub fn wait_for_completion(self) -> SigningStatus {
        // Receive all messages until channel closes, return the last one
        let mut final_status = SigningStatus::Pending;
        while let Ok(status) = self.receiver.recv() {
            final_status = status.clone();
            // If we got a terminal state, we're done
            if matches!(
                final_status,
                SigningStatus::Completed { .. } | SigningStatus::Failed(_)
            ) {
                break;
            }
        }
        final_status
    }

    /// Get job ID
    pub fn job_id(&self) -> usize {
        self.job_id
    }
}

/// Background auto-signer that processes signing jobs asynchronously
pub struct AutoSigner {
    private_key: Arc<RsaPrivateKey>,
    public_key: Arc<RsaPublicKey>,
    job_counter: Arc<Mutex<usize>>,
}

impl AutoSigner {
    /// Creates a new auto-signer with generated RSA keypair
    pub fn new() -> Result<Self, String> {
        let (private_key, public_key) = generate_keypair()?;

        println!("🔐 AutoSigner initialized with RSA-2048 keypair");

        Ok(Self {
            private_key: Arc::new(private_key),
            public_key: Arc::new(public_key),
            job_counter: Arc::new(Mutex::new(0)),
        })
    }

    /// Creates an auto-signer with provided keys
    pub fn with_keys(private_key: RsaPrivateKey, public_key: RsaPublicKey) -> Self {
        Self {
            private_key: Arc::new(private_key),
            public_key: Arc::new(public_key),
            job_counter: Arc::new(Mutex::new(0)),
        }
    }

    /// Gets the public key for verification
    pub fn public_key(&self) -> &RsaPublicKey {
        &self.public_key
    }

    /// Queues a file for automatic signing (non-blocking)
    ///
    /// Spawns a background thread to sign the file and saves signature to output path.
    ///
    /// # Arguments
    /// * `file_path` - Path to file to sign
    /// * `output_path` - Path where signature will be saved
    ///
    /// # Returns
    /// Handle to poll or wait for completion
    pub fn sign_file_async<P: AsRef<Path>>(
        &self,
        file_path: P,
        output_path: P,
    ) -> AutoSignerHandle {
        let file_path = file_path.as_ref().to_path_buf();
        let output_path = output_path.as_ref().to_path_buf();

        // Get unique job ID
        let job_id = {
            let mut counter = self.job_counter.lock().unwrap();
            *counter += 1;
            *counter
        };

        let (sender, receiver) = bounded(1);

        let private_key = Arc::clone(&self.private_key);

        // Spawn signing thread
        thread::spawn(move || {
            let _ = sender.send(SigningStatus::InProgress);

            let status = match Self::sign_and_save(&file_path, &output_path, &private_key) {
                Ok(signature) => {
                    println!("✅ Auto-signed [Job {}]: {:?}", job_id, file_path);
                    SigningStatus::Completed { signature }
                }
                Err(e) => {
                    eprintln!("❌ Auto-sign failed [Job {}]: {}", job_id, e);
                    SigningStatus::Failed(e)
                }
            };

            let _ = sender.send(status);
        });

        AutoSignerHandle { job_id, receiver }
    }

    /// Internal: Signs file and saves signature
    fn sign_and_save(
        file_path: &Path,
        output_path: &Path,
        private_key: &RsaPrivateKey,
    ) -> Result<Vec<u8>, String> {
        // Sign the file
        let signature = sign_file(file_path.to_str().ok_or("Invalid file path")?, private_key)?;

        // Save signature to output path
        fs::write(output_path, &signature)
            .map_err(|e| format!("Failed to write signature: {}", e))?;

        Ok(signature)
    }

    /// Signs multiple files in parallel
    ///
    /// Returns handles for all signing jobs that can be polled independently.
    pub fn sign_batch<P: AsRef<Path>>(
        &self,
        files: &[(P, P)], // (file_path, output_path) pairs
    ) -> Vec<AutoSignerHandle> {
        files
            .iter()
            .map(|(file, output)| self.sign_file_async(file, output))
            .collect()
    }

    /// Monitors a directory and auto-signs new files matching a pattern
    ///
    /// This is a demonstration of continuous auto-signing.
    /// In production, use a proper file watcher crate.
    pub fn watch_directory<P: AsRef<Path>>(
        &self,
        dir_path: P,
        pattern: &str,
        output_dir: P,
        interval: Duration,
    ) -> Result<(), String> {
        let dir_path = dir_path.as_ref();
        let output_dir = output_dir.as_ref();
        let pattern = pattern.to_string();

        if !dir_path.is_dir() {
            return Err(format!("{:?} is not a directory", dir_path));
        }

        fs::create_dir_all(output_dir)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;

        println!(
            "👀 Watching {:?} for files matching '{}'",
            dir_path, pattern
        );

        loop {
            let entries =
                fs::read_dir(dir_path).map_err(|e| format!("Failed to read directory: {}", e))?;

            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        if name_str.contains(&pattern) {
                            let sig_name = format!("{}.sig", name_str);
                            let output_path = output_dir.join(sig_name);

                            if !output_path.exists() {
                                let _handle = self.sign_file_async(&path, &output_path);
                            }
                        }
                    }
                }
            }

            thread::sleep(interval);
        }
    }
}

impl Default for AutoSigner {
    fn default() -> Self {
        Self::new().expect("Failed to create AutoSigner")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_auto_signer_creation() {
        let signer = AutoSigner::new();
        assert!(signer.is_ok());
    }

    #[test]
    fn test_sign_file_async() {
        let signer = AutoSigner::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let file_path = temp_dir.path().join("test.txt");
        let sig_path = temp_dir.path().join("test.sig");

        fs::write(&file_path, b"Test content").unwrap();

        let handle = signer.sign_file_async(&file_path, &sig_path);

        // Poll until complete
        let mut status = SigningStatus::Pending;
        for _ in 0..50 {
            status = handle.poll_status();
            if matches!(
                status,
                SigningStatus::Completed { .. } | SigningStatus::Failed(_)
            ) {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(matches!(status, SigningStatus::Completed { .. }));
        assert!(sig_path.exists());
    }

    #[test]
    fn test_sign_file_async_wait() {
        let signer = AutoSigner::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let file_path = temp_dir.path().join("test2.txt");
        let sig_path = temp_dir.path().join("test2.sig");

        fs::write(&file_path, b"Test content 2").unwrap();

        let handle = signer.sign_file_async(&file_path, &sig_path);
        let status = handle.wait_for_completion();

        assert!(matches!(status, SigningStatus::Completed { .. }));
        assert!(sig_path.exists());
    }

    #[test]
    fn test_sign_batch() {
        let signer = AutoSigner::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let files: Vec<_> = (0..3)
            .map(|i| {
                let file = temp_dir.path().join(format!("file{}.txt", i));
                let sig = temp_dir.path().join(format!("file{}.sig", i));
                fs::write(&file, format!("Content {}", i).as_bytes()).unwrap();
                (file, sig)
            })
            .collect();

        let file_refs: Vec<_> = files.iter().map(|(f, s)| (f, s)).collect();
        let handles = signer.sign_batch(&file_refs);

        assert_eq!(handles.len(), 3);

        // Wait for all to complete
        for handle in handles {
            let status = handle.wait_for_completion();
            assert!(matches!(status, SigningStatus::Completed { .. }));
        }

        // Verify all signature files exist
        for (_, sig_path) in &files {
            assert!(sig_path.exists());
        }
    }

    #[test]
    fn test_sign_file_not_found() {
        let signer = AutoSigner::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let file_path = temp_dir.path().join("nonexistent.txt");
        let sig_path = temp_dir.path().join("output.sig");

        let handle = signer.sign_file_async(&file_path, &sig_path);
        let status = handle.wait_for_completion();

        assert!(matches!(status, SigningStatus::Failed(_)));
    }

    #[test]
    fn test_auto_signer_job_ids() {
        let signer = AutoSigner::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let file_path = temp_dir.path().join("test.txt");
        let sig_path1 = temp_dir.path().join("test1.sig");
        let sig_path2 = temp_dir.path().join("test2.sig");

        fs::write(&file_path, b"Test").unwrap();

        let handle1 = signer.sign_file_async(&file_path, &sig_path1);
        let handle2 = signer.sign_file_async(&file_path, &sig_path2);

        // Job IDs should be unique and sequential
        assert_ne!(handle1.job_id(), handle2.job_id());
        assert_eq!(handle2.job_id(), handle1.job_id() + 1);
    }
}
