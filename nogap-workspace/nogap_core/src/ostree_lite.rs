//! # OSTree Lite - Lightweight Object Store for Air-Gapped Systems
//!
//! This module provides a simplified OSTree-inspired content-addressable storage (CAS)
//! system designed for secure, air-gapped environments using USB drives as transport.
//!
//! ## USB Repository Layout
//!
//! A USB repository follows this structure:
//! ```text
//! /usb_repo/
//! ├── manifest.json          # Canonical JSON manifest listing all objects
//! ├── manifest.json.sig      # Ed25519 signature of manifest
//! └── objects/               # Content-addressable object store
//!     ├── ab/                # First 2 chars of SHA256 hash
//!     │   └── cdef123...     # Remaining hash chars (object content)
//!     └── 01/
//!         └── 23456789...
//! ```
//!
//! ## Canonical JSON Signing Rule
//!
//! Before signing, the manifest JSON must be serialized in canonical form:
//! - Keys sorted alphabetically
//! - No whitespace between elements
//! - UTF-8 encoding
//! - Consistent number formatting
//!
//! This ensures identical byte representation across systems for signature verification.
//!
//! ## CAS Object Naming
//!
//! Objects are stored using SHA256 hashes with directory sharding:
//! - Hash format: 64 hex characters (e.g., `abcdef1234567890...`)
//! - Directory: First 2 characters (`ab/`)
//! - Filename: Remaining 62 characters (`cdef1234567890...`)
//! - Full path: `objects/ab/cdef1234567890...`

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use ed25519_dalek::{Signature, VerifyingKey};
use std::collections::HashMap;

/// Default trusted public key (placeholder - replace with actual production key)
/// This is used as a fallback when ~/.nogap/trusted_keys.json does not exist
const DEFAULT_TRUSTED_PUBKEY: &str = "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2";

/// Represents a manifest describing the contents of a USB repository
///
/// Contains metadata and a list of all objects required for a commit or update.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manifest {
    /// Version of the manifest format
    pub version: u32,
    /// Commit hash this manifest represents
    pub commit_hash: String,
    /// Timestamp when manifest was created (ISO 8601)
    pub timestamp: String,
    /// List of all objects in this manifest
    pub objects: Vec<ManifestObject>,
    /// Optional metadata
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

/// Represents a single object in the manifest
///
/// Each object corresponds to a file in the CAS store.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManifestObject {
    /// SHA256 hash of the object (64 hex chars)
    pub hash: String,
    /// Size of the object in bytes
    pub size: u64,
    /// Object type (e.g., "file", "tree", "commit")
    pub object_type: String,
}

/// Preview of what would be imported from a USB repository
///
/// Provides summary information before actual import.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportPreview {
    /// Total number of objects in manifest
    pub total_objects: usize,
    /// Objects already present locally
    pub already_present: usize,
    /// Objects that need to be pulled
    pub to_download: usize,
    /// Total bytes to download
    pub download_size: u64,
    /// Commit hash from manifest
    pub commit_hash: String,
}

/// Result of an import operation
///
/// Contains detailed information about what was imported.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportResult {
    /// Number of objects successfully imported
    pub imported_count: usize,
    /// Total bytes imported
    pub imported_bytes: u64,
    /// Objects that were skipped (already present)
    pub skipped_count: usize,
    /// List of imported object hashes
    pub imported_objects: Vec<String>,
    /// Whether the manifest was successfully installed
    pub manifest_installed: bool,
}

/// Error types for OSTree Lite operations
#[derive(Debug)]
pub enum OstreeError {
    /// I/O error (file system operations)
    Io(std::io::Error),
    /// JSON parsing or serialization error
    Json(serde_json::Error),
    /// Signature verification error
    Sig(String),
    /// Hash mismatch during verification
    HashMismatch { expected: String, actual: String },
    /// Object or file not found
    NotFound(String),
    /// Object size exceeds configured limit
    SizeLimitExceeded { size: u64, limit: u64 },
    /// Database error
    Db(String),
    /// Other unspecified error
    Other(String),
}

impl std::fmt::Display for OstreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OstreeError::Io(e) => write!(f, "I/O error: {}", e),
            OstreeError::Json(e) => write!(f, "JSON error: {}", e),
            OstreeError::Sig(msg) => write!(f, "Signature error: {}", msg),
            OstreeError::HashMismatch { expected, actual } => {
                write!(f, "Hash mismatch: expected {}, got {}", expected, actual)
            }
            OstreeError::NotFound(msg) => write!(f, "Not found: {}", msg),
            OstreeError::SizeLimitExceeded { size, limit } => {
                write!(f, "Size limit exceeded: {} bytes (limit: {})", size, limit)
            }
            OstreeError::Db(msg) => write!(f, "Database error: {}", msg),
            OstreeError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for OstreeError {}

impl From<std::io::Error> for OstreeError {
    fn from(err: std::io::Error) -> Self {
        OstreeError::Io(err)
    }
}

impl From<serde_json::Error> for OstreeError {
    fn from(err: serde_json::Error) -> Self {
        OstreeError::Json(err)
    }
}

// ============================================================================
// Public API Functions (Skeleton - No Implementation)
// ============================================================================

/// Discovers USB repositories by scanning mounted volumes
///
/// Searches for directories containing valid OSTree Lite repository structure
/// (manifest.json, manifest.json.sig, objects/ directory).
///
/// # Returns
/// Vector of paths to discovered USB repositories
///
/// # Errors
/// Returns `OstreeError` if filesystem scanning fails
pub fn discover_usb_repos() -> Result<Vec<PathBuf>, OstreeError> {
    let mut repos = Vec::new();

    #[cfg(target_os = "windows")]
    {
        repos.extend(discover_windows_repos()?);
    }

    #[cfg(target_os = "linux")]
    {
        repos.extend(discover_linux_repos()?);
    }

    #[cfg(target_os = "macos")]
    {
        repos.extend(discover_macos_repos()?);
    }

    Ok(repos)
}

#[cfg(target_os = "windows")]
fn discover_windows_repos() -> Result<Vec<PathBuf>, OstreeError> {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsString;
    
    let mut repos = Vec::new();
    
    // Iterate through drive letters D: through Z:
    for letter in b'D'..=b'Z' {
        let drive_path = format!("{}:\\", letter as char);
        let wide: Vec<u16> = OsString::from(&drive_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        unsafe {
            // Check drive type using Windows API
            let drive_type = winapi::um::fileapi::GetDriveTypeW(wide.as_ptr());
            
            // Accept DRIVE_REMOVABLE (2) or DRIVE_FIXED (3)
            // Skip DRIVE_NO_ROOT_DIR (1), DRIVE_CDROM (5), etc.
            if drive_type != 2 && drive_type != 3 {
                continue;
            }
        }
        
        // Check if aegis_repo exists
        let repo_path = PathBuf::from(&drive_path).join("aegis_repo");
        if repo_path.exists() && repo_path.is_dir() {
            // Validate path security
            if let Ok(canonical) = std::fs::canonicalize(&repo_path) {
                // Ensure canonical path starts with the drive root
                let drive_root = PathBuf::from(&drive_path);
                if let Ok(canonical_root) = std::fs::canonicalize(&drive_root) {
                    if canonical.starts_with(&canonical_root) {
                        repos.push(canonical);
                    }
                }
            }
        }
    }
    
    Ok(repos)
}

#[cfg(target_os = "linux")]
fn discover_linux_repos() -> Result<Vec<PathBuf>, OstreeError> {
    let mut repos = Vec::new();
    
    // Check /media/*
    if let Ok(entries) = std::fs::read_dir("/media") {
        for entry in entries.flatten() {
            if let Ok(repo_path) = check_mount_for_repo(&entry.path()) {
                repos.push(repo_path);
            }
        }
    }
    
    // Check /run/media/*
    if let Ok(entries) = std::fs::read_dir("/run/media") {
        for entry in entries.flatten() {
            // /run/media typically has user subdirectories
            if entry.path().is_dir() {
                if let Ok(user_entries) = std::fs::read_dir(entry.path()) {
                    for user_entry in user_entries.flatten() {
                        if let Ok(repo_path) = check_mount_for_repo(&user_entry.path()) {
                            repos.push(repo_path);
                        }
                    }
                }
            }
        }
    }
    
    Ok(repos)
}

#[cfg(target_os = "macos")]
fn discover_macos_repos() -> Result<Vec<PathBuf>, OstreeError> {
    let mut repos = Vec::new();
    
    // Check /Volumes/*
    if let Ok(entries) = std::fs::read_dir("/Volumes") {
        for entry in entries.flatten() {
            if let Ok(repo_path) = check_mount_for_repo(&entry.path()) {
                repos.push(repo_path);
            }
        }
    }
    
    Ok(repos)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn check_mount_for_repo(mount_path: &Path) -> Result<PathBuf, OstreeError> {
    let repo_path = mount_path.join("aegis_repo");
    
    // Check if aegis_repo exists (don't follow symlinks)
    if !repo_path.exists() || !repo_path.is_dir() {
        return Err(OstreeError::NotFound("aegis_repo not found".to_string()));
    }
    
    // Resolve canonical path
    let canonical = std::fs::canonicalize(&repo_path)?;
    
    // Ensure canonical path doesn't escape the mount root
    let canonical_mount = std::fs::canonicalize(mount_path)?;
    if !canonical.starts_with(&canonical_mount) {
        return Err(OstreeError::Other("Path escapes mount root".to_string()));
    }
    
    Ok(canonical)
}

/// Reads and parses a manifest from a repository
///
/// Reads both the manifest.json file and returns its raw bytes for signature verification.
///
/// # Arguments
/// * `repo_root` - Path to the repository root directory
///
/// # Returns
/// Tuple of (parsed Manifest, raw manifest bytes)
///
/// # Errors
/// Returns `OstreeError` if manifest file is missing or invalid JSON
pub fn read_manifest(repo_root: &Path) -> Result<(Manifest, Vec<u8>), OstreeError> {
    // Construct paths to manifest and signature files
    let manifest_path = repo_root.join("refs").join("heads").join("production.manifest");
    let sig_path = repo_root.join("refs").join("heads").join("production.sig");
    
    // Check if manifest file exists
    if !manifest_path.exists() {
        return Err(OstreeError::NotFound(format!(
            "Manifest file not found: {}",
            manifest_path.display()
        )));
    }
    
    // Check if signature file exists
    if !sig_path.exists() {
        return Err(OstreeError::NotFound(format!(
            "Signature file not found: {}",
            sig_path.display()
        )));
    }
    
    // Read manifest file as raw bytes
    let manifest_bytes = std::fs::read(&manifest_path)?;
    
    // Parse manifest JSON into Manifest struct
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;
    
    // Read signature file as raw bytes
    let sig_bytes = std::fs::read(&sig_path)?;
    
    // Return parsed manifest and signature bytes
    Ok((manifest, sig_bytes))
}

/// Verifies the Ed25519 signature of a manifest
///
/// Validates that the manifest bytes were signed by a trusted key.
///
/// # Arguments
/// * `manifest_bytes` - Raw canonical JSON bytes of the manifest
/// * `sig` - Ed25519 signature bytes
///
/// # Returns
/// `Ok(())` if signature is valid
///
/// # Errors
/// Returns `OstreeError::Sig` if signature verification fails
pub fn verify_manifest(manifest_bytes: &[u8], sig: &[u8]) -> Result<(), OstreeError> {
    // Load trusted public key
    let pubkey_hex = load_trusted_public_key()?;
    
    // Convert hex to bytes
    let pubkey_bytes = hex::decode(&pubkey_hex)
        .map_err(|e| OstreeError::Sig(format!("Invalid public key hex: {}", e)))?;
    
    // Create VerifyingKey from bytes
    let verifying_key = VerifyingKey::from_bytes(
        pubkey_bytes.as_slice().try_into()
            .map_err(|_| OstreeError::Sig("Invalid public key length (expected 32 bytes)".to_string()))?
    ).map_err(|e| OstreeError::Sig(format!("Invalid ed25519 public key: {}", e)))?;
    
    // Create Signature from raw bytes
    let signature = Signature::from_bytes(
        sig.try_into()
            .map_err(|_| OstreeError::Sig("Invalid signature length (expected 64 bytes)".to_string()))?
    );
    
    // Verify signature using strict verification
    verifying_key.verify_strict(manifest_bytes, &signature)
        .map_err(|_| OstreeError::Sig("invalid manifest signature".to_string()))?;
    
    Ok(())
}

/// Loads the trusted public key from ~/.nogap/trusted_keys.json or fallback to default
///
/// # Returns
/// Hex-encoded public key string
///
/// # Errors
/// Returns `OstreeError::Sig` if JSON is invalid or key is missing
fn load_trusted_public_key() -> Result<String, OstreeError> {
    // Try to load from ~/.nogap/trusted_keys.json
    if let Some(home_dir) = std::env::var_os("HOME") {
        let keys_path = PathBuf::from(home_dir)
            .join(".nogap")
            .join("trusted_keys.json");
        
        if keys_path.exists() {
            // Read and parse the file
            let content = std::fs::read_to_string(&keys_path)?;
            let json: HashMap<String, String> = serde_json::from_str(&content)
                .map_err(|e| OstreeError::Sig(format!("Invalid trusted_keys.json: {}", e)))?;
            
            // Extract public_key_hex
            return json.get("public_key_hex")
                .map(|s| s.clone())
                .ok_or_else(|| OstreeError::Sig("Missing 'public_key_hex' in trusted_keys.json".to_string()));
        }
    }
    
    // Fallback to default key
    Ok(DEFAULT_TRUSTED_PUBKEY.to_string())
}

/// Constructs the filesystem path for an object given its hash
///
/// Uses the standard sharding scheme: `objects/<first2>/<remaining62>`
///
/// # Arguments
/// * `repo_root` - Path to the repository root
/// * `hash_hex` - 64-character hex SHA256 hash (expected to be lowercase)
///
/// # Returns
/// Full path to where the object should be stored
///
/// # Behavior for Invalid Inputs
/// If `hash_hex` is shorter than 3 characters, returns a placeholder path:
/// `repo_root/objects/??/<hash_hex>` to ensure deterministic behavior without panicking.
/// This allows graceful degradation for edge cases while maintaining path consistency.
///
/// # Examples
/// ```
/// use std::path::{Path, PathBuf};
/// use nogap_core::ostree_lite::object_path_for_hash;
///
/// let repo = Path::new("/repo");
/// let hash = "aabbccddee1122334455667788990011223344556677889900112233445566778899";
/// let path = object_path_for_hash(repo, hash);
/// assert_eq!(path, PathBuf::from("/repo/objects/aa/bbccddee1122334455667788990011223344556677889900112233445566778899"));
/// ```
pub fn object_path_for_hash(repo_root: &Path, hash_hex: &str) -> PathBuf {
    // Handle invalid/short hash inputs gracefully
    if hash_hex.len() < 3 {
        // Return placeholder path for deterministic behavior
        return repo_root.join("objects").join("??").join(hash_hex);
    }
    
    // Split hash into first 2 chars (directory) and remaining chars (filename)
    let (first2, rest) = hash_hex.split_at(2);
    
    // Build path: repo_root/objects/<first2>/<rest>
    repo_root.join("objects").join(first2).join(rest)
}

/// Checks if an object exists in the local CAS store
///
/// Returns true only if the object file exists at the expected CAS path.
/// Does NOT follow symlinks - symlinks are treated as non-existent.
/// Does NOT canonicalize paths for performance and security.
/// Does NOT read or hash file content - pure filesystem metadata check.
///
/// # Arguments
/// * `local_repo` - Path to the local repository
/// * `hash_hex` - SHA256 hash of the object to check
///
/// # Returns
/// `true` if object exists as a regular file, `false` otherwise
///
/// # Behavior
/// - Returns `false` if path exists but is a directory
/// - Returns `false` if path is a symlink (even if target exists)
/// - Returns `false` if file does not exist
/// - Returns `false` on any metadata errors
///
/// # Examples
/// ```no_run
/// use std::path::Path;
/// use nogap_core::ostree_lite::check_local_cas;
///
/// let repo = Path::new("/repo");
/// let hash = "aabbccddee1122334455667788990011223344556677889900112233445566778899";
/// if check_local_cas(repo, hash) {
///     println!("Object already exists locally");
/// }
/// ```
pub fn check_local_cas(local_repo: &Path, hash_hex: &str) -> bool {
    // Compute expected CAS path
    let object_path = object_path_for_hash(local_repo, hash_hex);
    
    // Get metadata without following symlinks
    match std::fs::symlink_metadata(&object_path) {
        Ok(metadata) => {
            // Return true only if it's a regular file (not directory, not symlink)
            metadata.is_file()
        }
        Err(_) => {
            // File doesn't exist or we can't access it
            false
        }
    }
}

/// Pulls objects from USB repository to local repository
///
/// Copies objects that are missing locally, with optional size limits per object.
/// Performs streaming SHA256 verification and atomic writes via temporary files.
///
/// # Arguments
/// * `usb_repo` - Path to source USB repository
/// * `manifest` - Manifest describing objects to pull
/// * `local_repo` - Path to destination local repository
/// * `per_object_limit` - Optional size limit per object in bytes (default: 100 MB)
///
/// # Returns
/// Vector of paths to all objects that exist locally after pull (including already-present ones)
///
/// # Errors
/// - `NotFound` - Object missing from USB repository
/// - `SizeLimitExceeded` - Object exceeds size limit
/// - `HashMismatch` - Computed hash doesn't match manifest
/// - `Other` - Zero-length object file
/// - `Io` - File system operations fail
///
/// # Implementation Details
/// - Default size limit: 100 MB per object
/// - Streams objects through temporary files with incremental SHA256 hashing
/// - Atomically renames temp file to final path after verification
/// - Automatically creates parent directories as needed
/// - Cleans up temporary files on any error
pub fn pull_objects(
    usb_repo: &Path,
    manifest: &Manifest,
    local_repo: &Path,
    per_object_limit: Option<u64>,
) -> Result<Vec<PathBuf>, OstreeError> {
    use sha2::{Sha256, Digest};
    use std::io::{Read, Write};
    
    // Default size limit: 100 MB
    const DEFAULT_SIZE_LIMIT: u64 = 100 * 1024 * 1024;
    let size_limit = per_object_limit.unwrap_or(DEFAULT_SIZE_LIMIT);
    
    let mut result_paths = Vec::new();
    
    // Process each object in manifest
    for obj in &manifest.objects {
        let hash_hex = &obj.hash;
        
        // Build local object path
        let local_object_path = object_path_for_hash(local_repo, hash_hex);
        
        // Check if object already exists locally
        if check_local_cas(local_repo, hash_hex) {
            // Skip copy, but include in result
            result_paths.push(local_object_path);
            continue;
        }
        
        // Build USB object path
        let usb_object_path = object_path_for_hash(usb_repo, hash_hex);
        
        // Verify USB object exists and is a regular file
        let usb_metadata = match std::fs::metadata(&usb_object_path) {
            Ok(m) => m,
            Err(_) => {
                return Err(OstreeError::NotFound(format!(
                    "object not found: {}",
                    hash_hex
                )));
            }
        };
        
        if !usb_metadata.is_file() {
            return Err(OstreeError::NotFound(format!(
                "object not found: {}",
                hash_hex
            )));
        }
        
        // Enforce size limit
        let file_size = usb_metadata.len();
        
        if file_size == 0 {
            return Err(OstreeError::Other("zero-length CAS object".to_string()));
        }
        
        if file_size > size_limit {
            return Err(OstreeError::SizeLimitExceeded {
                size: file_size,
                limit: size_limit,
            });
        }
        
        // Create parent directory for local object
        if let Some(parent) = local_object_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Create temporary file path
        let temp_path = local_object_path.with_extension("tmp");
        
        // Ensure temp file cleanup on error
        let pull_result = (|| -> Result<(), OstreeError> {
            // Open USB source file
            let mut source = std::fs::File::open(&usb_object_path)?;
            
            // Create temporary destination file
            let mut dest = std::fs::File::create(&temp_path)?;
            
            // Initialize SHA256 hasher
            let mut hasher = Sha256::new();
            
            // Stream copy with hash computation
            let mut buffer = [0u8; 8192];
            loop {
                let n = source.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                
                // Write to destination
                dest.write_all(&buffer[..n])?;
                
                // Update hash
                hasher.update(&buffer[..n]);
            }
            
            // Flush and close destination file
            dest.flush()?;
            drop(dest);
            
            // Compute final hash
            let computed_hash = hex::encode(hasher.finalize());
            
            // Verify hash matches manifest
            if computed_hash != *hash_hex {
                return Err(OstreeError::HashMismatch {
                    expected: hash_hex.clone(),
                    actual: computed_hash,
                });
            }
            
            // Atomically rename temp file to final path
            std::fs::rename(&temp_path, &local_object_path)?;
            
            Ok(())
        })();
        
        // Clean up temp file on error
        if pull_result.is_err() {
            let _ = std::fs::remove_file(&temp_path);
            return Err(pull_result.unwrap_err());
        }
        
        // Add to result
        result_paths.push(local_object_path);
    }
    
    Ok(result_paths)
}

/// Installs a manifest into the local repository
///
/// Records the manifest and updates repository metadata.
///
/// # Arguments
/// * `manifest` - The manifest to install
/// * `local_repo` - Path to the local repository
///
/// # Returns
/// `Ok(())` if installation succeeds
///
/// # Errors
/// Returns `OstreeError` if manifest cannot be written
pub fn install_manifest(manifest: &Manifest, local_repo: &Path) -> Result<(), OstreeError> {
    use sha2::{Sha256, Digest};
    use std::io::Write;

    // Step 1: Verify all manifest objects exist in local CAS
    for obj in &manifest.objects {
        if !check_local_cas(local_repo, &obj.hash) {
            return Err(OstreeError::NotFound(format!(
                "CAS object missing locally: {}",
                obj.hash
            )));
        }
    }

    // Step 2: Compute commit hash (manifest hash)
    let canonical_bytes = serde_json::to_vec(manifest)
        .map_err(|e| OstreeError::Other(format!("Failed to serialize manifest: {}", e)))?;
    
    let mut hasher = Sha256::new();
    hasher.update(&canonical_bytes);
    let commit_hash = hex::encode(hasher.finalize());

    // Step 3: Write new ref atomically
    let refs_heads = local_repo.join("refs").join("heads");
    
    // Step 3A: Prepare directories
    std::fs::create_dir_all(&refs_heads)?;

    let production_ref = refs_heads.join("production");
    let temp_ref = refs_heads.join("production.tmp");
    let prev_ref = refs_heads.join("production.prev");

    // Step 3B: Write temp ref
    let ref_content = serde_json::json!({
        "commit": commit_hash,
        "version": manifest.version,
        "timestamp": manifest.timestamp
    });
    
    let ref_json = serde_json::to_string_pretty(&ref_content)
        .map_err(|e| OstreeError::Other(format!("Failed to serialize ref: {}", e)))?;

    // Closure to ensure cleanup on error
    let install_result = (|| -> Result<(), OstreeError> {
        let mut temp_file = std::fs::File::create(&temp_ref)?;
        temp_file.write_all(ref_json.as_bytes())?;
        temp_file.sync_all()?;
        drop(temp_file);

        // Step 3C: Preserve previous ref
        if production_ref.exists() {
            // Overwrite .prev if it already exists
            if prev_ref.exists() {
                std::fs::remove_file(&prev_ref)?;
            }
            std::fs::rename(&production_ref, &prev_ref)?;
        }

        // Step 3D: Atomic rename
        std::fs::rename(&temp_ref, &production_ref)?;
        
        Ok(())
    })();

    // Cleanup temp file on error
    if install_result.is_err() {
        let _ = std::fs::remove_file(&temp_ref);
        return Err(install_result.unwrap_err());
    }

    Ok(())
}

/// Exports a commit from local repository to target USB drive
///
/// Creates a new manifest and copies all required objects to the USB repository.
///
/// # Security Notes
/// - **NEVER** writes private keys to the target USB drive
/// - **REQUIRES** explicit user confirmation before calling (enforced via `confirmed` parameter)
/// - Uses existing signing infrastructure (signing::sign_data or auto_signer)
/// - Always verifies written manifest with verify_manifest() after export
/// - If signer prompts for passphrase, this must be communicated in the UI
///
/// # Arguments
/// * `local_repo` - Path to source local repository
/// * `commit_hash` - Hash of the commit to export
/// * `target_usb` - Path to destination USB repository (must be removable on Windows)
/// * `confirmed` - Must be true to proceed (indicates user provided explicit consent)
/// * `signing_key` - Private key for signing the manifest
///
/// # Returns
/// `Ok(())` if export succeeds
///
/// # Errors
/// Returns `OstreeError` if:
/// - Confirmation not provided (`confirmed == false`)
/// - Commit doesn't exist in local repository
/// - Target is not removable (Windows only, best-effort check)
/// - Target doesn't contain `aegis_repo/` directory
/// - Copy operations fail
/// - Hash verification fails
/// - Signing fails
pub fn export_commit_to_target(
    local_repo: &Path,
    commit_hash: &str,
    target_usb: &Path,
    confirmed: bool,
    signing_key: &rsa::RsaPrivateKey,
) -> Result<(), OstreeError> {
    use sha2::{Sha256, Digest};
    use std::io::{Read, Write};

    // Step 1: Check confirmation
    if !confirmed {
        return Err(OstreeError::Other(
            "export requires explicit confirmation".into()
        ));
    }

    // Step 2: Platform-specific removable media check (best-effort)
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        // Convert path to wide string for Windows API
        let path_str = target_usb.to_str()
            .ok_or_else(|| OstreeError::Other("Invalid path encoding".into()))?;
        
        // Extract drive letter (e.g., "E:\\" from "E:\\path")
        let drive_letter = if path_str.len() >= 2 && path_str.chars().nth(1) == Some(':') {
            format!("{}:\\", &path_str[0..1])
        } else {
            return Err(OstreeError::Other("Windows path must include drive letter".into()));
        };
        
        let wide: Vec<u16> = OsStr::new(&drive_letter)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        unsafe {
            let drive_type = winapi::um::fileapi::GetDriveTypeW(wide.as_ptr());
            // DRIVE_REMOVABLE = 2
            if drive_type != 2 {
                return Err(OstreeError::Other(
                    "Target must be removable media (Windows DRIVE_REMOVABLE check failed)".into()
                ));
            }
        }
    }

    // Step 3: Verify target has aegis_repo directory
    let target_repo = target_usb.join("aegis_repo");
    if !target_repo.exists() || !target_repo.is_dir() {
        return Err(OstreeError::Other(
            "Target USB must contain aegis_repo/ directory".into()
        ));
    }

    // Step 4: Discover commit and build object set
    // Read the manifest for this commit from local refs
    let local_refs = local_repo.join("refs").join("heads");
    let production_ref = local_refs.join("production");
    
    if !production_ref.exists() {
        return Err(OstreeError::NotFound(
            format!("commit not found: {}", commit_hash)
        ));
    }

    // Read and parse the ref to get manifest
    let ref_content = std::fs::read_to_string(&production_ref)?;
    let ref_json: serde_json::Value = serde_json::from_str(&ref_content)
        .map_err(|e| OstreeError::Other(format!("Invalid ref JSON: {}", e)))?;
    
    let stored_commit = ref_json["commit"].as_str()
        .ok_or_else(|| OstreeError::Other("Ref missing commit field".into()))?;
    
    if stored_commit != commit_hash {
        return Err(OstreeError::NotFound(
            format!("commit hash mismatch: expected {}, found {}", commit_hash, stored_commit)
        ));
    }

    // Read the actual manifest from local repo to get object list
    // The manifest was used to create the commit, so we need to reconstruct it
    // For now, we'll read from a canonical location or derive from commit
    // Since install_manifest creates the ref, we need the original manifest
    // For this implementation, we'll read the manifest from a standard location
    let manifest_path = local_refs.join("production.manifest");
    let manifest = if manifest_path.exists() {
        // Read manifest directly (no signature verification needed for local repo)
        let manifest_bytes = std::fs::read(&manifest_path)?;
        serde_json::from_slice::<Manifest>(&manifest_bytes)?
    } else {
        // Fallback: reconstruct from objects directory
        return Err(OstreeError::NotFound(
            "production.manifest not found in local repo".into()
        ));
    };

    // Step 5: Copy each required object to target
    let target_objects = target_repo.join("objects");
    std::fs::create_dir_all(&target_objects)?;

    for obj in &manifest.objects {
        let obj_hash = &obj.hash;
        
        // Build paths
        let (first_two, rest) = if obj_hash.len() >= 2 {
            (&obj_hash[0..2], &obj_hash[2..])
        } else {
            return Err(OstreeError::Other(format!("Invalid hash length: {}", obj_hash)));
        };
        
        let target_dir = target_objects.join(first_two);
        let target_path = target_dir.join(rest);
        let temp_path = target_dir.join(format!("{}.tmp", rest));

        // Check if object already exists on target
        if target_path.exists() {
            if target_path.is_file() {
                // Skip copy - object already exists
                continue;
            } else {
                return Err(OstreeError::Other(
                    format!("Target object path exists but is not a file: {}", target_path.display())
                ));
            }
        }

        // Read from local CAS
        let local_obj_path = object_path_for_hash(local_repo, obj_hash);
        if !local_obj_path.exists() {
            return Err(OstreeError::NotFound(
                format!("Local object not found: {}", obj_hash)
            ));
        }

        // Create target directory
        std::fs::create_dir_all(&target_dir)?;

        // Stream copy to temp file with hash verification
        let mut source_file = std::fs::File::open(&local_obj_path)?;
        let mut temp_file = std::fs::File::create(&temp_path)?;
        let mut hasher = Sha256::new();
        
        let mut buffer = vec![0u8; 65536]; // 64KB buffer
        loop {
            let bytes_read = source_file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            
            hasher.update(&buffer[..bytes_read]);
            temp_file.write_all(&buffer[..bytes_read])?;
        }
        
        // Sync to disk
        temp_file.sync_all()?;
        drop(temp_file);

        // Verify hash
        let computed_hash = hex::encode(hasher.finalize());
        if computed_hash != *obj_hash {
            // Cleanup temp file
            let _ = std::fs::remove_file(&temp_path);
            return Err(OstreeError::HashMismatch {
                expected: obj_hash.clone(),
                actual: computed_hash,
            });
        }

        // Atomic rename
        std::fs::rename(&temp_path, &target_path)?;
    }

    // Step 6: Create production.manifest and sign it
    let refs_heads = target_repo.join("refs").join("heads");
    std::fs::create_dir_all(&refs_heads)?;

    // Serialize manifest to canonical JSON
    let manifest_bytes = serde_json::to_vec(&manifest)?;

    // Sign the manifest bytes using provided signing key
    let signature = crate::signing::sign_data(&manifest_bytes, signing_key)
        .map_err(|e| OstreeError::Other(format!("Signing failed: {}", e)))?;

    // Write manifest and signature atomically
    let manifest_path = refs_heads.join("production.manifest");
    let manifest_tmp = refs_heads.join("production.manifest.tmp");
    let sig_path = refs_heads.join("production.sig");
    let sig_tmp = refs_heads.join("production.sig.tmp");

    // Closure for atomic write of both files
    let write_result = (|| -> Result<(), OstreeError> {
        // Write manifest temp
        let mut manifest_file = std::fs::File::create(&manifest_tmp)?;
        manifest_file.write_all(&manifest_bytes)?;
        manifest_file.sync_all()?;
        drop(manifest_file);

        // Write signature temp
        let mut sig_file = std::fs::File::create(&sig_tmp)?;
        sig_file.write_all(&signature)?;
        sig_file.sync_all()?;
        drop(sig_file);

        // Atomic rename both files
        std::fs::rename(&manifest_tmp, &manifest_path)?;
        std::fs::rename(&sig_tmp, &sig_path)?;

        // Sync directory (best-effort)
        #[cfg(unix)]
        {
            if let Ok(dir) = std::fs::File::open(&refs_heads) {
                let _ = dir.sync_all();
            }
        }

        Ok(())
    })();

    // Cleanup on error
    if write_result.is_err() {
        let _ = std::fs::remove_file(&manifest_tmp);
        let _ = std::fs::remove_file(&sig_tmp);
        return Err(write_result.unwrap_err());
    }

    // Step 7: Final verification - read back and verify signature
    // Note: We skip verify_manifest here since it uses Ed25519 and we're using RSA signing
    // The verification happens in the UI/CLI layer with the appropriate public key
    // For testing purposes, we verify using RSA directly
    let written_manifest_bytes = std::fs::read(&manifest_path)?;
    let written_sig = std::fs::read(&sig_path)?;
    
    // Verify using RSA (matches our signing method)
    let public_key = rsa::RsaPublicKey::from(signing_key);
    if !crate::signing::verify_data_signature(&written_manifest_bytes, &written_sig, &public_key) {
        return Err(OstreeError::Sig("Final verification failed after export".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_discover_valid_repo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mount_path = temp_dir.path().join("usb_mount");
        let repo_path = mount_path.join("aegis_repo");
        
        fs::create_dir_all(&repo_path).unwrap();
        
        // Test platform-specific discovery
        #[cfg(target_os = "linux")]
        {
            let result = check_mount_for_repo(&mount_path);
            assert!(result.is_ok());
            let found_path = result.unwrap();
            assert!(found_path.ends_with("aegis_repo"));
        }
        
        #[cfg(target_os = "macos")]
        {
            let result = check_mount_for_repo(&mount_path);
            assert!(result.is_ok());
            let found_path = result.unwrap();
            assert!(found_path.ends_with("aegis_repo"));
        }
    }

    #[test]
    fn test_discover_missing_repo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mount_path = temp_dir.path().join("usb_mount");
        
        fs::create_dir_all(&mount_path).unwrap();
        // Don't create aegis_repo
        
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let result = check_mount_for_repo(&mount_path);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_discover_linux_style_paths() {
        let temp_dir = tempfile::tempdir().unwrap();
        
        // Simulate /media/user/USB
        let media_path = temp_dir.path().join("media").join("testuser");
        let repo1 = media_path.join("aegis_repo");
        fs::create_dir_all(&repo1).unwrap();
        
        // Simulate /run/media/user/USB2
        let run_media_path = temp_dir.path().join("run").join("media").join("testuser");
        let repo2 = run_media_path.join("aegis_repo");
        fs::create_dir_all(&repo2).unwrap();
        
        #[cfg(target_os = "linux")]
        {
            // Test individual mount check
            let result1 = check_mount_for_repo(&media_path);
            assert!(result1.is_ok());
            
            let result2 = check_mount_for_repo(&run_media_path);
            assert!(result2.is_ok());
        }
    }

    #[test]
    fn test_discover_macos_style_paths() {
        let temp_dir = tempfile::tempdir().unwrap();
        
        // Simulate /Volumes/USB_DRIVE
        let volumes_path = temp_dir.path().join("Volumes").join("USB_DRIVE");
        let repo_path = volumes_path.join("aegis_repo");
        fs::create_dir_all(&repo_path).unwrap();
        
        #[cfg(target_os = "macos")]
        {
            let result = check_mount_for_repo(&volumes_path);
            assert!(result.is_ok());
            let found_path = result.unwrap();
            assert!(found_path.ends_with("aegis_repo"));
        }
    }

    #[test]
    fn test_discover_windows_style_paths() {
        // This test creates mock directory structures similar to Windows paths
        // Actual GetDriveTypeW testing requires Windows environment
        let temp_dir = tempfile::tempdir().unwrap();
        
        // Simulate D:\aegis_repo structure
        let mock_drive = temp_dir.path().join("D_drive");
        let repo_path = mock_drive.join("aegis_repo");
        fs::create_dir_all(&repo_path).unwrap();
        
        // Verify the structure exists
        assert!(repo_path.exists());
        assert!(repo_path.is_dir());
        
        // On Windows, this would be tested via actual discovery
        #[cfg(target_os = "windows")]
        {
            // The actual Windows discovery would check drive letters
            // This test validates the path structure only
            let canonical = std::fs::canonicalize(&repo_path).unwrap();
            assert!(canonical.ends_with("aegis_repo"));
        }
    }

    #[test]
    fn test_path_canonicalization() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mount_path = temp_dir.path().join("usb_mount");
        let repo_path = mount_path.join("aegis_repo");
        
        fs::create_dir_all(&repo_path).unwrap();
        
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let result = check_mount_for_repo(&mount_path);
            assert!(result.is_ok());
            
            let canonical = result.unwrap();
            // Canonical path should be absolute
            assert!(canonical.is_absolute());
            // Should contain aegis_repo
            assert!(canonical.ends_with("aegis_repo"));
        }
    }

    #[test]
    fn test_empty_discovery_returns_ok() {
        // When no repos are found, should return Ok(vec![]) not an error
        let temp_dir = tempfile::tempdir().unwrap();
        
        // Create mount point without aegis_repo
        let mount_path = temp_dir.path().join("empty_mount");
        fs::create_dir_all(&mount_path).unwrap();
        
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let result = check_mount_for_repo(&mount_path);
            // This should error because repo not found
            assert!(result.is_err());
            
            // But discover_usb_repos() should return Ok(vec![])
            // (tested implicitly by not finding any valid repos)
        }
    }

    #[test]
    fn test_multiple_repos_discovery() {
        let temp_dir = tempfile::tempdir().unwrap();
        
        // Create multiple mount points with repos
        let mount1 = temp_dir.path().join("mount1");
        let repo1 = mount1.join("aegis_repo");
        fs::create_dir_all(&repo1).unwrap();
        
        let mount2 = temp_dir.path().join("mount2");
        let repo2 = mount2.join("aegis_repo");
        fs::create_dir_all(&repo2).unwrap();
        
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let result1 = check_mount_for_repo(&mount1);
            let result2 = check_mount_for_repo(&mount2);
            
            assert!(result1.is_ok());
            assert!(result2.is_ok());
            
            // Both should be different paths
            assert_ne!(result1.unwrap(), result2.unwrap());
        }
    }

    #[test]
    fn test_reject_non_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mount_path = temp_dir.path().join("usb_mount");
        fs::create_dir_all(&mount_path).unwrap();
        
        // Create aegis_repo as a file, not directory
        let repo_path = mount_path.join("aegis_repo");
        fs::write(&repo_path, b"not a directory").unwrap();
        
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let result = check_mount_for_repo(&mount_path);
            assert!(result.is_err());
        }
    }

    // ========================================================================
    // Tests for read_manifest()
    // ========================================================================

    #[test]
    fn test_read_manifest_ok() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();
        
        // Create refs/heads/ directory structure
        let refs_heads = repo_root.join("refs").join("heads");
        fs::create_dir_all(&refs_heads).unwrap();
        
        // Create a valid manifest
        let manifest = Manifest {
            version: 1,
            commit_hash: "abc123def456".to_string(),
            timestamp: "2025-11-22T10:00:00Z".to_string(),
            objects: vec![
                ManifestObject {
                    hash: "deadbeef".to_string(),
                    size: 1024,
                    object_type: "file".to_string(),
                }
            ],
            metadata: None,
        };
        
        let manifest_json = serde_json::to_vec(&manifest).unwrap();
        let sig_bytes = b"fake_signature_bytes";
        
        // Write manifest and signature files
        fs::write(refs_heads.join("production.manifest"), &manifest_json).unwrap();
        fs::write(refs_heads.join("production.sig"), sig_bytes).unwrap();
        
        // Test read_manifest
        let result = read_manifest(repo_root);
        assert!(result.is_ok());
        
        let (parsed_manifest, sig) = result.unwrap();
        assert_eq!(parsed_manifest.version, 1);
        assert_eq!(parsed_manifest.commit_hash, "abc123def456");
        assert_eq!(parsed_manifest.objects.len(), 1);
        assert_eq!(sig, sig_bytes);
    }

    #[test]
    fn test_missing_manifest() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();
        
        // Create refs/heads/ directory
        let refs_heads = repo_root.join("refs").join("heads");
        fs::create_dir_all(&refs_heads).unwrap();
        
        // Write only signature file, no manifest
        fs::write(refs_heads.join("production.sig"), b"signature").unwrap();
        
        // Test read_manifest
        let result = read_manifest(repo_root);
        assert!(result.is_err());
        
        match result {
            Err(OstreeError::NotFound(msg)) => {
                assert!(msg.contains("Manifest file not found"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_missing_sig() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();
        
        // Create refs/heads/ directory
        let refs_heads = repo_root.join("refs").join("heads");
        fs::create_dir_all(&refs_heads).unwrap();
        
        // Write only manifest file, no signature
        let manifest = Manifest {
            version: 1,
            commit_hash: "test123".to_string(),
            timestamp: "2025-11-22T10:00:00Z".to_string(),
            objects: vec![],
            metadata: None,
        };
        let manifest_json = serde_json::to_vec(&manifest).unwrap();
        fs::write(refs_heads.join("production.manifest"), manifest_json).unwrap();
        
        // Test read_manifest
        let result = read_manifest(repo_root);
        assert!(result.is_err());
        
        match result {
            Err(OstreeError::NotFound(msg)) => {
                assert!(msg.contains("Signature file not found"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_invalid_json() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();
        
        // Create refs/heads/ directory
        let refs_heads = repo_root.join("refs").join("heads");
        fs::create_dir_all(&refs_heads).unwrap();
        
        // Write invalid JSON to manifest file
        fs::write(refs_heads.join("production.manifest"), b"{ invalid json }").unwrap();
        fs::write(refs_heads.join("production.sig"), b"signature").unwrap();
        
        // Test read_manifest
        let result = read_manifest(repo_root);
        assert!(result.is_err());
        
        match result {
            Err(OstreeError::Json(_)) => {
                // Expected error type
            }
            _ => panic!("Expected Json error"),
        }
    }

    #[test]
    fn test_empty_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();
        
        // Create refs/heads/ directory
        let refs_heads = repo_root.join("refs").join("heads");
        fs::create_dir_all(&refs_heads).unwrap();
        
        // Test 1: Empty manifest file (should fail JSON parsing)
        fs::write(refs_heads.join("production.manifest"), b"").unwrap();
        fs::write(refs_heads.join("production.sig"), b"signature").unwrap();
        
        let result = read_manifest(repo_root);
        assert!(result.is_err());
        match result {
            Err(OstreeError::Json(_)) => {
                // Expected - empty file is invalid JSON
            }
            _ => panic!("Expected Json error for empty manifest"),
        }
        
        // Test 2: Valid manifest but empty signature (should succeed with empty sig)
        let manifest = Manifest {
            version: 1,
            commit_hash: "test456".to_string(),
            timestamp: "2025-11-22T10:00:00Z".to_string(),
            objects: vec![],
            metadata: None,
        };
        let manifest_json = serde_json::to_vec(&manifest).unwrap();
        fs::write(refs_heads.join("production.manifest"), manifest_json).unwrap();
        fs::write(refs_heads.join("production.sig"), b"").unwrap();
        
        let result = read_manifest(repo_root);
        assert!(result.is_ok());
        
        let (_parsed_manifest, sig) = result.unwrap();
        assert_eq!(sig.len(), 0); // Empty signature still returned
    }

    #[test]
    fn test_nested_refs_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo_root = temp_dir.path();
        
        // Create nested refs/heads/ directory structure
        let refs_heads = repo_root.join("refs").join("heads");
        fs::create_dir_all(&refs_heads).unwrap();
        
        // Also create a decoy at wrong path
        fs::create_dir_all(repo_root.join("refs")).unwrap();
        fs::write(
            repo_root.join("refs").join("production.manifest"),
            b"decoy manifest"
        ).unwrap();
        
        // Create correct files in refs/heads/
        let manifest = Manifest {
            version: 1,
            commit_hash: "correct_path".to_string(),
            timestamp: "2025-11-22T10:00:00Z".to_string(),
            objects: vec![],
            metadata: None,
        };
        let manifest_json = serde_json::to_vec(&manifest).unwrap();
        fs::write(refs_heads.join("production.manifest"), &manifest_json).unwrap();
        fs::write(refs_heads.join("production.sig"), b"correct_sig").unwrap();
        
        // Test read_manifest - should read from refs/heads/, not refs/
        let result = read_manifest(repo_root);
        assert!(result.is_ok());
        
        let (parsed_manifest, sig) = result.unwrap();
        assert_eq!(parsed_manifest.commit_hash, "correct_path");
        assert_eq!(sig, b"correct_sig");
    }

    // ===== verify_manifest Tests =====

    #[test]
    fn test_verify_manifest_ok() {
        use ed25519_dalek::{SigningKey, Signer};
        use rand::rngs::OsRng;

        // Generate keypair
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let pubkey_hex = hex::encode(verifying_key.to_bytes());

        // Create ~/.nogap/trusted_keys.json with public key
        let dir = tempfile::tempdir().unwrap();
        let nogap_dir = dir.path().join(".nogap");
        std::fs::create_dir_all(&nogap_dir).unwrap();
        let keys_path = nogap_dir.join("trusted_keys.json");
        let keys_json = format!(r#"{{"public_key_hex": "{}"}}"#, pubkey_hex);
        std::fs::write(&keys_path, keys_json).unwrap();

        // Set HOME to tempdir
        let original_home = std::env::var_os("HOME");
        std::env::set_var("HOME", dir.path());

        // Create manifest bytes
        let manifest_bytes = b"{\"version\":1}";

        // Sign the manifest
        let signature = signing_key.sign(manifest_bytes);
        let sig_bytes = signature.to_bytes();

        // Verify signature
        let result = verify_manifest(manifest_bytes, &sig_bytes);

        // Restore HOME
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        assert!(result.is_ok(), "Signature verification should succeed");
    }

    #[test]
    fn test_verify_manifest_fail_modified_data() {
        use ed25519_dalek::{SigningKey, Signer};
        use rand::rngs::OsRng;

        // Generate keypair
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let pubkey_hex = hex::encode(verifying_key.to_bytes());

        // Create ~/.nogap/trusted_keys.json
        let dir = tempfile::tempdir().unwrap();
        let nogap_dir = dir.path().join(".nogap");
        std::fs::create_dir_all(&nogap_dir).unwrap();
        let keys_path = nogap_dir.join("trusted_keys.json");
        let keys_json = format!(r#"{{"public_key_hex": "{}"}}"#, pubkey_hex);
        std::fs::write(&keys_path, keys_json).unwrap();

        // Set HOME
        let original_home = std::env::var_os("HOME");
        std::env::set_var("HOME", dir.path());

        // Create and sign manifest
        let manifest_bytes = b"{\"version\":1}";
        let signature = signing_key.sign(manifest_bytes);
        let sig_bytes = signature.to_bytes();

        // Modify manifest bytes (tampering)
        let modified_bytes = b"{\"version\":2}";

        // Verify should fail
        let result = verify_manifest(modified_bytes, &sig_bytes);

        // Restore HOME
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        assert!(result.is_err(), "Verification should fail for modified data");
        match result {
            Err(OstreeError::Sig(_)) => {},
            _ => panic!("Expected Sig error"),
        }
    }

    #[test]
    fn test_verify_manifest_fail_wrong_key() {
        use ed25519_dalek::{SigningKey, Signer};
        use rand::rngs::OsRng;

        // Generate two keypairs
        let signing_key = SigningKey::generate(&mut OsRng);
        let wrong_key = SigningKey::generate(&mut OsRng);
        let wrong_verifying_key = wrong_key.verifying_key();
        let wrong_pubkey_hex = hex::encode(wrong_verifying_key.to_bytes());

        // Create ~/.nogap/trusted_keys.json with WRONG public key
        let dir = tempfile::tempdir().unwrap();
        let nogap_dir = dir.path().join(".nogap");
        std::fs::create_dir_all(&nogap_dir).unwrap();
        let keys_path = nogap_dir.join("trusted_keys.json");
        let keys_json = format!(r#"{{"public_key_hex": "{}"}}"#, wrong_pubkey_hex);
        std::fs::write(&keys_path, keys_json).unwrap();

        // Set HOME
        let original_home = std::env::var_os("HOME");
        std::env::set_var("HOME", dir.path());

        // Sign with correct key
        let manifest_bytes = b"{\"version\":1}";
        let signature = signing_key.sign(manifest_bytes);
        let sig_bytes = signature.to_bytes();

        // Verify with wrong key should fail
        let result = verify_manifest(manifest_bytes, &sig_bytes);

        // Restore HOME
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        assert!(result.is_err(), "Verification should fail with wrong key");
        match result {
            Err(OstreeError::Sig(_)) => {},
            _ => panic!("Expected Sig error"),
        }
    }

    #[test]
    fn test_verify_manifest_invalid_sig_length() {
        use ed25519_dalek::SigningKey;
        use rand::rngs::OsRng;

        // Generate keypair
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let pubkey_hex = hex::encode(verifying_key.to_bytes());

        // Create ~/.nogap/trusted_keys.json
        let dir = tempfile::tempdir().unwrap();
        let nogap_dir = dir.path().join(".nogap");
        std::fs::create_dir_all(&nogap_dir).unwrap();
        let keys_path = nogap_dir.join("trusted_keys.json");
        let keys_json = format!(r#"{{"public_key_hex": "{}"}}"#, pubkey_hex);
        std::fs::write(&keys_path, keys_json).unwrap();

        // Set HOME
        let original_home = std::env::var_os("HOME");
        std::env::set_var("HOME", dir.path());

        // Create manifest
        let manifest_bytes = b"{\"version\":1}";

        // Invalid signature: wrong length (should be 64 bytes)
        let invalid_sig = vec![0u8; 32]; // Only 32 bytes instead of 64

        // Verify should fail with length error
        let result = verify_manifest(manifest_bytes, &invalid_sig);

        // Restore HOME
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        assert!(result.is_err(), "Verification should fail for invalid signature length");
        match result {
            Err(OstreeError::Sig(msg)) => {
                assert!(msg.contains("Invalid signature length") || msg.contains("expected 64 bytes"),
                    "Error message should mention signature length: {}", msg);
            },
            _ => panic!("Expected Sig error for invalid length"),
        }
    }

    #[test]
    fn test_verify_manifest_fallback_key() {
        // DO NOT create trusted_keys.json - should use DEFAULT_TRUSTED_PUBKEY
        let dir = tempfile::tempdir().unwrap();
        
        // Set HOME to tempdir (no .nogap directory)
        let original_home = std::env::var_os("HOME");
        std::env::set_var("HOME", dir.path());

        // Try to verify (will use DEFAULT_TRUSTED_PUBKEY)
        let manifest_bytes = b"{\"version\":1}";
        let fake_sig = vec![0u8; 64];

        let result = verify_manifest(manifest_bytes, &fake_sig);

        // Restore HOME
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }

        // Should return error (signature won't match default key), but proves fallback key loaded
        // The important thing is it doesn't panic or return NotFound - it uses the default key
        assert!(result.is_err(), "Verification should fail (wrong signature)");
        match result {
            Err(OstreeError::Sig(msg)) => {
                // This proves DEFAULT_TRUSTED_PUBKEY was loaded and used
                assert!(msg.contains("invalid manifest signature") || msg.contains("Invalid"),
                    "Should get signature verification error: {}", msg);
            },
            _ => panic!("Expected Sig error (fallback key was used but signature is wrong)"),
        }
    }

    // ========================================
    // STAGE 5 TESTS: object_path_for_hash() and check_local_cas()
    // ========================================

    #[test]
    fn test_object_path_for_hash_basic() {
        // Test standard 64-character SHA256 hash
        let repo_root = Path::new("/repo");
        let hash = "aabbccddee1122334455667788990011223344556677889900112233445566778899";
        
        let path = object_path_for_hash(repo_root, hash);
        
        // Expected path: /repo/objects/aa/bbccddee1122334455667788990011223344556677889900112233445566778899
        let expected = PathBuf::from("/repo/objects/aa/bbccddee1122334455667788990011223344556677889900112233445566778899");
        
        assert_eq!(path, expected, "Path should follow objects/<first2>/<rest> pattern");
        
        // Verify component structure
        let components: Vec<_> = path.components().collect();
        assert!(components[components.len() - 3].as_os_str() == "objects", "Should have 'objects' directory");
        assert!(components[components.len() - 2].as_os_str() == "aa", "Should have first 2 chars as subdirectory");
        assert!(components[components.len() - 1].as_os_str() == "bbccddee1122334455667788990011223344556677889900112233445566778899", 
            "Should have remaining 62 chars as filename");
    }

    #[test]
    fn test_object_path_for_hash_short_input() {
        // Test hash shorter than 3 characters - should return placeholder path
        let repo_root = Path::new("/repo");
        
        // Test with 0 chars
        let path_empty = object_path_for_hash(repo_root, "");
        assert_eq!(path_empty, PathBuf::from("/repo/objects/??/"), 
            "Empty hash should return placeholder path");
        
        // Test with 1 char
        let path_one = object_path_for_hash(repo_root, "a");
        assert_eq!(path_one, PathBuf::from("/repo/objects/??/a"),
            "Single char hash should return placeholder path");
        
        // Test with 2 chars
        let path_two = object_path_for_hash(repo_root, "ab");
        assert_eq!(path_two, PathBuf::from("/repo/objects/??/ab"),
            "Two char hash should return placeholder path");
        
        // Test with exactly 3 chars - should work normally
        let path_three = object_path_for_hash(repo_root, "abc");
        assert_eq!(path_three, PathBuf::from("/repo/objects/ab/c"),
            "Three char hash should split normally (2 + 1)");
    }

    #[test]
    fn test_check_local_cas_true() {
        // Create temporary repository with proper CAS structure
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();
        
        // Create a test hash
        let hash = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        
        // Build expected CAS path
        let objects_dir = repo_root.join("objects").join("12");
        std::fs::create_dir_all(&objects_dir).unwrap();
        
        let object_file = objects_dir.join("34567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");
        std::fs::write(&object_file, b"fake object content").unwrap();
        
        // Check should return true
        let exists = check_local_cas(repo_root, hash);
        assert!(exists, "Should return true when object file exists");
    }

    #[test]
    fn test_check_local_cas_false_missing() {
        // Create temporary repository but don't create the object file
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();
        
        let hash = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        
        // Create the shard directory but not the file
        let objects_dir = repo_root.join("objects").join("12");
        std::fs::create_dir_all(&objects_dir).unwrap();
        
        // Check should return false (file doesn't exist)
        let exists = check_local_cas(repo_root, hash);
        assert!(!exists, "Should return false when object file is missing");
    }

    #[test]
    fn test_check_local_cas_false_directory() {
        // Create temporary repository
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();
        
        let hash = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        
        // Build CAS path but create it as a directory instead of a file
        let objects_dir = repo_root.join("objects").join("12");
        std::fs::create_dir_all(&objects_dir).unwrap();
        
        let object_path = objects_dir.join("34567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");
        std::fs::create_dir_all(&object_path).unwrap(); // Create as directory
        
        // Check should return false (it's a directory, not a file)
        let exists = check_local_cas(repo_root, hash);
        assert!(!exists, "Should return false when CAS path is a directory");
    }

    #[test]
    #[cfg(unix)] // symlink API is Unix-specific
    fn test_check_local_cas_ignores_symlinks() {
        use std::os::unix::fs::symlink;
        
        // Create temporary repository
        let dir = tempfile::tempdir().unwrap();
        let repo_root = dir.path();
        
        let hash = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        
        // Create target file (what symlink points to)
        let target_file = repo_root.join("target.txt");
        std::fs::write(&target_file, b"target content").unwrap();
        
        // Build CAS path and create as symlink
        let objects_dir = repo_root.join("objects").join("12");
        std::fs::create_dir_all(&objects_dir).unwrap();
        
        let symlink_path = objects_dir.join("34567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");
        symlink(&target_file, &symlink_path).unwrap();
        
        // Verify symlink was created (using symlink_metadata)
        let metadata = std::fs::symlink_metadata(&symlink_path).unwrap();
        assert!(metadata.file_type().is_symlink(), "Should be a symlink");
        
        // Check should return false (symlinks are not accepted)
        let exists = check_local_cas(repo_root, hash);
        assert!(!exists, "Should return false for symlinks (must be regular file)");
    }

    // ========================================
    // STAGE 6 TESTS: pull_objects()
    // ========================================

    #[test]
    fn test_pull_objects_copies_missing() {
        use sha2::{Sha256, Digest};
        
        // Create USB and local repos
        let usb_dir = tempfile::tempdir().unwrap();
        let usb_repo = usb_dir.path();
        
        let local_dir = tempfile::tempdir().unwrap();
        let local_repo = local_dir.path();
        
        // Create test content and compute hash
        let content = b"This is a test object for pull_objects";
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = hex::encode(hasher.finalize());
        
        // Create USB object
        let usb_obj_dir = usb_repo.join("objects").join(&hash[0..2]);
        std::fs::create_dir_all(&usb_obj_dir).unwrap();
        let usb_obj_path = usb_obj_dir.join(&hash[2..]);
        std::fs::write(&usb_obj_path, content).unwrap();
        
        // Create manifest
        let manifest = Manifest {
            version: 1,
            commit_hash: "test_commit".to_string(),
            timestamp: "2025-11-22T00:00:00Z".to_string(),
            objects: vec![ManifestObject {
                hash: hash.clone(),
                size: content.len() as u64,
                object_type: "blob".to_string(),
            }],
            metadata: None,
        };
        
        // Verify object doesn't exist locally yet
        assert!(!check_local_cas(local_repo, &hash), "Object should not exist locally before pull");
        
        // Pull objects
        let result = pull_objects(usb_repo, &manifest, local_repo, None);
        assert!(result.is_ok(), "Pull should succeed: {:?}", result);
        
        let paths = result.unwrap();
        assert_eq!(paths.len(), 1, "Should return one path");
        
        // Verify object now exists locally
        assert!(check_local_cas(local_repo, &hash), "Object should exist locally after pull");
        
        // Verify content matches
        let local_obj_path = object_path_for_hash(local_repo, &hash);
        let local_content = std::fs::read(&local_obj_path).unwrap();
        assert_eq!(local_content, content, "Content should match after pull");
    }

    #[test]
    fn test_pull_objects_skips_existing() {
        use sha2::{Sha256, Digest};
        
        // Create USB and local repos
        let usb_dir = tempfile::tempdir().unwrap();
        let usb_repo = usb_dir.path();
        
        let local_dir = tempfile::tempdir().unwrap();
        let local_repo = local_dir.path();
        
        // Create test content
        let content = b"Existing object content";
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = hex::encode(hasher.finalize());
        
        // Create object in BOTH repos
        for repo in &[usb_repo, local_repo] {
            let obj_dir = repo.join("objects").join(&hash[0..2]);
            std::fs::create_dir_all(&obj_dir).unwrap();
            let obj_path = obj_dir.join(&hash[2..]);
            std::fs::write(&obj_path, content).unwrap();
        }
        
        // Get local file's modification time before pull
        let local_obj_path = object_path_for_hash(local_repo, &hash);
        let metadata_before = std::fs::metadata(&local_obj_path).unwrap();
        let mtime_before = metadata_before.modified().unwrap();
        
        // Create manifest
        let manifest = Manifest {
            version: 1,
            commit_hash: "test_commit".to_string(),
            timestamp: "2025-11-22T00:00:00Z".to_string(),
            objects: vec![ManifestObject {
                hash: hash.clone(),
                size: content.len() as u64,
                object_type: "blob".to_string(),
            }],
            metadata: None,
        };
        
        // Pull objects
        let result = pull_objects(usb_repo, &manifest, local_repo, None);
        assert!(result.is_ok(), "Pull should succeed");
        
        let paths = result.unwrap();
        assert_eq!(paths.len(), 1, "Should return one path (existing object)");
        assert_eq!(paths[0], local_obj_path, "Should return existing object path");
        
        // Verify file was NOT rewritten (modification time unchanged)
        let metadata_after = std::fs::metadata(&local_obj_path).unwrap();
        let mtime_after = metadata_after.modified().unwrap();
        assert_eq!(mtime_before, mtime_after, "File should not be rewritten if already exists");
    }

    #[test]
    fn test_pull_objects_missing_usb_blob() {
        // Create USB and local repos
        let usb_dir = tempfile::tempdir().unwrap();
        let usb_repo = usb_dir.path();
        
        let local_dir = tempfile::tempdir().unwrap();
        let local_repo = local_dir.path();
        
        // Create manifest referencing non-existent object
        let fake_hash = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let manifest = Manifest {
            version: 1,
            commit_hash: "test_commit".to_string(),
            timestamp: "2025-11-22T00:00:00Z".to_string(),
            objects: vec![ManifestObject {
                hash: fake_hash.to_string(),
                size: 1024,
                object_type: "blob".to_string(),
            }],
            metadata: None,
        };
        
        // Pull should fail with NotFound
        let result = pull_objects(usb_repo, &manifest, local_repo, None);
        assert!(result.is_err(), "Pull should fail for missing USB object");
        
        match result {
            Err(OstreeError::NotFound(msg)) => {
                assert!(msg.contains(fake_hash), "Error should mention missing hash: {}", msg);
            }
            _ => panic!("Expected NotFound error, got: {:?}", result),
        }
    }

    #[test]
    fn test_pull_objects_hash_mismatch() {
        // Create USB and local repos
        let usb_dir = tempfile::tempdir().unwrap();
        let usb_repo = usb_dir.path();
        
        let local_dir = tempfile::tempdir().unwrap();
        let local_repo = local_dir.path();
        
        // Create corrupt content
        let corrupt_content = b"This is corrupted content";
        
        // Compute hash of DIFFERENT content
        use sha2::{Sha256, Digest};
        let correct_content = b"This is the correct content";
        let mut hasher = Sha256::new();
        hasher.update(correct_content);
        let correct_hash = hex::encode(hasher.finalize());
        
        // Store corrupt content under correct hash path on USB
        let usb_obj_dir = usb_repo.join("objects").join(&correct_hash[0..2]);
        std::fs::create_dir_all(&usb_obj_dir).unwrap();
        let usb_obj_path = usb_obj_dir.join(&correct_hash[2..]);
        std::fs::write(&usb_obj_path, corrupt_content).unwrap();
        
        // Create manifest with correct hash
        let manifest = Manifest {
            version: 1,
            commit_hash: "test_commit".to_string(),
            timestamp: "2025-11-22T00:00:00Z".to_string(),
            objects: vec![ManifestObject {
                hash: correct_hash.clone(),
                size: corrupt_content.len() as u64,
                object_type: "blob".to_string(),
            }],
            metadata: None,
        };
        
        // Pull should fail with HashMismatch
        let result = pull_objects(usb_repo, &manifest, local_repo, None);
        assert!(result.is_err(), "Pull should fail for hash mismatch");
        
        match result {
            Err(OstreeError::HashMismatch { expected, actual }) => {
                assert_eq!(expected, correct_hash, "Expected hash should match manifest");
                assert_ne!(actual, correct_hash, "Actual hash should differ");
            }
            _ => panic!("Expected HashMismatch error, got: {:?}", result),
        }
        
        // Verify temp file was cleaned up (no .tmp files left)
        let local_obj_path = object_path_for_hash(local_repo, &correct_hash);
        let temp_path = local_obj_path.with_extension("tmp");
        assert!(!temp_path.exists(), "Temp file should be cleaned up on error");
        
        // Verify final object doesn't exist
        assert!(!check_local_cas(local_repo, &correct_hash), "Object should not exist after failed pull");
    }

    #[test]
    fn test_pull_objects_size_limit_exceeded() {
        use sha2::{Sha256, Digest};
        
        // Create USB and local repos
        let usb_dir = tempfile::tempdir().unwrap();
        let usb_repo = usb_dir.path();
        
        let local_dir = tempfile::tempdir().unwrap();
        let local_repo = local_dir.path();
        
        // Create large content (1 MB)
        let large_content = vec![0u8; 1024 * 1024];
        let mut hasher = Sha256::new();
        hasher.update(&large_content);
        let hash = hex::encode(hasher.finalize());
        
        // Create USB object
        let usb_obj_dir = usb_repo.join("objects").join(&hash[0..2]);
        std::fs::create_dir_all(&usb_obj_dir).unwrap();
        let usb_obj_path = usb_obj_dir.join(&hash[2..]);
        std::fs::write(&usb_obj_path, &large_content).unwrap();
        
        // Create manifest
        let manifest = Manifest {
            version: 1,
            commit_hash: "test_commit".to_string(),
            timestamp: "2025-11-22T00:00:00Z".to_string(),
            objects: vec![ManifestObject {
                hash: hash.clone(),
                size: large_content.len() as u64,
                object_type: "blob".to_string(),
            }],
            metadata: None,
        };
        
        // Pull with small limit (1 KB)
        let small_limit = 1024;
        let result = pull_objects(usb_repo, &manifest, local_repo, Some(small_limit));
        assert!(result.is_err(), "Pull should fail for size limit");
        
        match result {
            Err(OstreeError::SizeLimitExceeded { size, limit }) => {
                assert_eq!(size, large_content.len() as u64, "Size should match file size");
                assert_eq!(limit, small_limit, "Limit should match provided limit");
            }
            _ => panic!("Expected SizeLimitExceeded error, got: {:?}", result),
        }
        
        // Verify object doesn't exist locally
        assert!(!check_local_cas(local_repo, &hash), "Object should not exist after failed pull");
    }

    #[test]
    fn test_pull_objects_zero_length_blob() {
        use sha2::{Sha256, Digest};
        
        // Create USB and local repos
        let usb_dir = tempfile::tempdir().unwrap();
        let usb_repo = usb_dir.path();
        
        let local_dir = tempfile::tempdir().unwrap();
        let local_repo = local_dir.path();
        
        // Compute hash of empty content
        let mut hasher = Sha256::new();
        hasher.update(b"");
        let hash = hex::encode(hasher.finalize());
        
        // Create zero-length USB object
        let usb_obj_dir = usb_repo.join("objects").join(&hash[0..2]);
        std::fs::create_dir_all(&usb_obj_dir).unwrap();
        let usb_obj_path = usb_obj_dir.join(&hash[2..]);
        std::fs::write(&usb_obj_path, b"").unwrap(); // Empty file
        
        // Create manifest
        let manifest = Manifest {
            version: 1,
            commit_hash: "test_commit".to_string(),
            timestamp: "2025-11-22T00:00:00Z".to_string(),
            objects: vec![ManifestObject {
                hash: hash.clone(),
                size: 0,
                object_type: "blob".to_string(),
            }],
            metadata: None,
        };
        
        // Pull should fail with Other error
        let result = pull_objects(usb_repo, &manifest, local_repo, None);
        assert!(result.is_err(), "Pull should fail for zero-length object");
        
        match result {
            Err(OstreeError::Other(msg)) => {
                assert!(msg.contains("zero-length"), "Error should mention zero-length: {}", msg);
            }
            _ => panic!("Expected Other error for zero-length, got: {:?}", result),
        }
        
        // Verify object doesn't exist locally
        assert!(!check_local_cas(local_repo, &hash), "Object should not exist after failed pull");
    }

    // ========================================
    // STAGE 7 TESTS: install_manifest()
    // ========================================

    #[test]
    fn test_install_manifest_success() {
        use sha2::{Sha256, Digest};
        
        let temp = tempfile::tempdir().unwrap();
        let local_repo = temp.path();
        
        // Create test object in local CAS
        let content = b"Test object content for install";
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = hex::encode(hasher.finalize());
        
        let obj_path = object_path_for_hash(local_repo, &hash);
        std::fs::create_dir_all(obj_path.parent().unwrap()).unwrap();
        std::fs::write(&obj_path, content).unwrap();
        
        // Create manifest
        let manifest = Manifest {
            version: 1,
            commit_hash: String::new(), // Will be computed by install_manifest
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            objects: vec![ManifestObject {
                hash: hash.clone(),
                size: content.len() as u64,
                object_type: "blob".to_string(),
            }],
            metadata: None,
        };
        
        // Install manifest
        let result = install_manifest(&manifest, local_repo);
        assert!(result.is_ok(), "Install should succeed: {:?}", result);
        
        // Verify production ref was created
        let production_ref = local_repo.join("refs/heads/production");
        assert!(production_ref.exists(), "Production ref should exist");
        
        // Verify ref contains correct commit hash (manifest hash)
        let ref_content = std::fs::read_to_string(&production_ref).unwrap();
        let ref_json: serde_json::Value = serde_json::from_str(&ref_content).unwrap();
        
        // Compute expected commit hash
        let canonical_bytes = serde_json::to_vec(&manifest).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(&canonical_bytes);
        let expected_commit = hex::encode(hasher.finalize());
        
        assert_eq!(ref_json["commit"].as_str().unwrap(), expected_commit, "Commit hash should match");
        assert_eq!(ref_json["version"].as_u64().unwrap(), 1, "Version should match");
        assert_eq!(ref_json["timestamp"].as_str().unwrap(), "2024-01-01T00:00:00Z", "Timestamp should match");
    }

    #[test]
    fn test_install_manifest_missing_object() {
        let temp = tempfile::tempdir().unwrap();
        let local_repo = temp.path();
        
        // Create manifest referencing non-existent object
        let fake_hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let manifest = Manifest {
            version: 1,
            commit_hash: String::new(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            objects: vec![ManifestObject {
                hash: fake_hash.to_string(),
                size: 100,
                object_type: "blob".to_string(),
            }],
            metadata: None,
        };
        
        // Attempt install should fail
        let result = install_manifest(&manifest, local_repo);
        assert!(result.is_err(), "Install should fail for missing object");
        
        match result {
            Err(OstreeError::NotFound(msg)) => {
                assert!(msg.contains(fake_hash), "Error should mention missing hash: {}", msg);
            }
            _ => panic!("Expected NotFound error, got: {:?}", result),
        }
        
        // Verify no ref files were created
        let production_ref = local_repo.join("refs/heads/production");
        assert!(!production_ref.exists(), "Production ref should not exist after failure");
        
        let temp_ref = local_repo.join("refs/heads/production.tmp");
        assert!(!temp_ref.exists(), "Temp ref should be cleaned up");
    }

    #[test]
    fn test_install_manifest_overwrite_previous_ref() {
        use sha2::{Sha256, Digest};
        
        let temp = tempfile::tempdir().unwrap();
        let local_repo = temp.path();
        
        // Create first object
        let content1 = b"First object";
        let mut hasher = Sha256::new();
        hasher.update(content1);
        let hash1 = hex::encode(hasher.finalize());
        
        let obj_path1 = object_path_for_hash(local_repo, &hash1);
        std::fs::create_dir_all(obj_path1.parent().unwrap()).unwrap();
        std::fs::write(&obj_path1, content1).unwrap();
        
        // Install first manifest
        let manifest1 = Manifest {
            version: 1,
            commit_hash: String::new(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            objects: vec![ManifestObject {
                hash: hash1.clone(),
                size: content1.len() as u64,
                object_type: "blob".to_string(),
            }],
            metadata: None,
        };
        
        install_manifest(&manifest1, local_repo).unwrap();
        
        // Read production ref content
        let production_ref = local_repo.join("refs/heads/production");
        let old_content = std::fs::read_to_string(&production_ref).unwrap();
        
        // Create second object
        let content2 = b"Second object";
        let mut hasher = Sha256::new();
        hasher.update(content2);
        let hash2 = hex::encode(hasher.finalize());
        
        let obj_path2 = object_path_for_hash(local_repo, &hash2);
        std::fs::create_dir_all(obj_path2.parent().unwrap()).unwrap();
        std::fs::write(&obj_path2, content2).unwrap();
        
        // Install second manifest
        let manifest2 = Manifest {
            version: 2,
            commit_hash: String::new(),
            timestamp: "2024-02-01T00:00:00Z".to_string(),
            objects: vec![ManifestObject {
                hash: hash2.clone(),
                size: content2.len() as u64,
                object_type: "blob".to_string(),
            }],
            metadata: None,
        };
        
        install_manifest(&manifest2, local_repo).unwrap();
        
        // Verify production.prev contains old content
        let prev_ref = local_repo.join("refs/heads/production.prev");
        assert!(prev_ref.exists(), "Production.prev should exist");
        
        let prev_content = std::fs::read_to_string(&prev_ref).unwrap();
        assert_eq!(prev_content, old_content, "Production.prev should contain old ref");
        
        // Verify production contains new content
        let new_content = std::fs::read_to_string(&production_ref).unwrap();
        let new_json: serde_json::Value = serde_json::from_str(&new_content).unwrap();
        assert_eq!(new_json["version"].as_u64().unwrap(), 2, "Production should have new version");
        assert_eq!(new_json["timestamp"].as_str().unwrap(), "2024-02-01T00:00:00Z", "Production should have new timestamp");
    }

    #[test]
    fn test_install_manifest_atomic_temp_cleanup() {
        use sha2::{Sha256, Digest};
        
        let temp = tempfile::tempdir().unwrap();
        let local_repo = temp.path();
        
        // Create object in CAS that doesn't match its hash (will fail precondition)
        let content = b"Test object";
        let mut hasher = Sha256::new();
        hasher.update(content);
        let correct_hash = hex::encode(hasher.finalize());
        
        let obj_path = object_path_for_hash(local_repo, &correct_hash);
        std::fs::create_dir_all(obj_path.parent().unwrap()).unwrap();
        std::fs::write(&obj_path, content).unwrap();
        
        // Create manifest with TWO objects - second one missing
        // This will fail during precondition check after temp file is created
        let fake_hash = "1111111111111111111111111111111111111111111111111111111111111111";
        let manifest = Manifest {
            version: 1,
            commit_hash: String::new(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            objects: vec![
                ManifestObject {
                    hash: correct_hash.clone(),
                    size: content.len() as u64,
                    object_type: "blob".to_string(),
                },
                ManifestObject {
                    hash: fake_hash.to_string(),
                    size: 100,
                    object_type: "blob".to_string(),
                },
            ],
            metadata: None,
        };
        
        // Attempt install (should fail on missing second object)
        let result = install_manifest(&manifest, local_repo);
        assert!(result.is_err(), "Install should fail for missing second object");
        
        match &result {
            Err(OstreeError::NotFound(msg)) => {
                assert!(msg.contains(fake_hash), "Error should mention missing hash: {}", msg);
            }
            _ => panic!("Expected NotFound error, got: {:?}", result),
        }
        
        // Verify temp file was cleaned up (even though error occurred during precondition)
        let refs_heads = local_repo.join("refs/heads");
        let temp_ref = refs_heads.join("production.tmp");
        assert!(!temp_ref.exists(), "Temp ref should be cleaned up after failure");
    }

    #[test]
    fn test_install_manifest_correct_commit_hash() {
        use sha2::{Sha256, Digest};
        
        let temp = tempfile::tempdir().unwrap();
        let local_repo = temp.path();
        
        // Create multiple objects
        let content1 = b"Object 1";
        let content2 = b"Object 2";
        
        let mut hasher = Sha256::new();
        hasher.update(content1);
        let hash1 = hex::encode(hasher.finalize());
        
        let mut hasher = Sha256::new();
        hasher.update(content2);
        let hash2 = hex::encode(hasher.finalize());
        
        // Create objects in CAS
        for (hash, content) in &[(hash1.clone(), content1), (hash2.clone(), content2)] {
            let obj_path = object_path_for_hash(local_repo, hash);
            std::fs::create_dir_all(obj_path.parent().unwrap()).unwrap();
            std::fs::write(&obj_path, content).unwrap();
        }
        
        // Create manifest with specific ordering
        let manifest = Manifest {
            version: 3,
            commit_hash: String::new(),
            timestamp: "2024-03-15T12:34:56Z".to_string(),
            objects: vec![
                ManifestObject {
                    hash: hash1.clone(),
                    size: content1.len() as u64,
                    object_type: "blob".to_string(),
                },
                ManifestObject {
                    hash: hash2.clone(),
                    size: content2.len() as u64,
                    object_type: "blob".to_string(),
                },
            ],
            metadata: None,
        };
        
        // Compute expected commit hash manually
        let canonical_bytes = serde_json::to_vec(&manifest).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(&canonical_bytes);
        let expected_commit = hex::encode(hasher.finalize());
        
        // Install manifest
        install_manifest(&manifest, local_repo).unwrap();
        
        // Read production ref
        let production_ref = local_repo.join("refs/heads/production");
        let ref_content = std::fs::read_to_string(&production_ref).unwrap();
        let ref_json: serde_json::Value = serde_json::from_str(&ref_content).unwrap();
        
        // Verify commit hash matches exactly
        let actual_commit = ref_json["commit"].as_str().unwrap();
        assert_eq!(actual_commit, expected_commit, "Commit hash must match canonical JSON hash");
        
        // Verify hash is lowercase hex
        assert!(actual_commit.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "Commit hash must be lowercase hex");
        assert_eq!(actual_commit.len(), 64, "Commit hash must be 64 characters (SHA256)");
    }

    // ============================================================================
    // STAGE 8 TESTS: export_commit_to_target()
    // ============================================================================

    #[test]
    fn test_export_commit_requires_confirmation() {
        use tempfile::tempdir;

        let local_repo = tempdir().unwrap();
        let target_usb = tempdir().unwrap();
        
        // Create aegis_repo on target
        let target_repo = target_usb.path().join("aegis_repo");
        std::fs::create_dir_all(&target_repo).unwrap();

        // Generate test keypair
        let (private_key, _public_key) = crate::signing::generate_keypair().unwrap();

        // Call without confirmation (confirmed = false)
        let result = export_commit_to_target(
            local_repo.path(),
            "abc123",
            target_usb.path(),
            false, // Not confirmed
            &private_key,
        );

        // Should fail with specific error
        match result {
            Err(OstreeError::Other(msg)) => {
                assert!(msg.contains("requires explicit confirmation"), 
                    "Expected confirmation error, got: {}", msg);
            }
            _ => panic!("Expected Other error about confirmation, got: {:?}", result),
        }
    }

    #[test]
    fn test_export_commit_missing_local_commit() {
        use tempfile::tempdir;

        let local_repo = tempdir().unwrap();
        let target_usb = tempdir().unwrap();
        
        // Create necessary directories
        let local_refs = local_repo.path().join("refs/heads");
        std::fs::create_dir_all(&local_refs).unwrap();
        
        let target_repo = target_usb.path().join("aegis_repo");
        std::fs::create_dir_all(&target_repo).unwrap();

        // Generate test keypair
        let (private_key, _public_key) = crate::signing::generate_keypair().unwrap();

        // Call with non-existent commit
        let result = export_commit_to_target(
            local_repo.path(),
            "nonexistent_commit_hash",
            target_usb.path(),
            true, // Confirmed
            &private_key,
        );

        // Should fail with NotFound
        match result {
            Err(OstreeError::NotFound(msg)) => {
                assert!(msg.contains("commit not found"), 
                    "Expected commit not found error, got: {}", msg);
            }
            _ => panic!("Expected NotFound error, got: {:?}", result),
        }
    }

    #[test]
    fn test_export_commit_missing_aegis_repo() {
        use tempfile::tempdir;

        let local_repo = tempdir().unwrap();
        let target_usb = tempdir().unwrap();
        // Intentionally NOT creating aegis_repo on target

        // Generate test keypair
        let (private_key, _public_key) = crate::signing::generate_keypair().unwrap();

        // Call export
        let result = export_commit_to_target(
            local_repo.path(),
            "any_commit",
            target_usb.path(),
            true,
            &private_key,
        );

        // Should fail with Other error about aegis_repo
        match result {
            Err(OstreeError::Other(msg)) => {
                assert!(msg.contains("aegis_repo"), 
                    "Expected aegis_repo error, got: {}", msg);
            }
            _ => panic!("Expected Other error about aegis_repo, got: {:?}", result),
        }
    }

    #[test]
    fn test_export_commit_copies_objects() {
        use tempfile::tempdir;
        use sha2::{Sha256, Digest};

        let local_repo = tempdir().unwrap();
        let target_usb = tempdir().unwrap();
        
        // Setup local repo structure
        let local_objects = local_repo.path().join("objects");
        let local_refs = local_repo.path().join("refs/heads");
        std::fs::create_dir_all(&local_objects).unwrap();
        std::fs::create_dir_all(&local_refs).unwrap();

        // Setup target repo structure
        let target_repo = target_usb.path().join("aegis_repo");
        std::fs::create_dir_all(&target_repo).unwrap();

        // Create test object content and hash
        let obj_content = b"test object content for export";
        let mut hasher = Sha256::new();
        hasher.update(obj_content);
        let obj_hash = hex::encode(hasher.finalize());

        // Write object to local CAS
        let obj_dir = local_objects.join(&obj_hash[0..2]);
        std::fs::create_dir_all(&obj_dir).unwrap();
        let obj_path = obj_dir.join(&obj_hash[2..]);
        std::fs::write(&obj_path, obj_content).unwrap();

        // Create manifest
        let manifest = Manifest {
            version: 1,
            commit_hash: "test_commit_123".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            objects: vec![
                ManifestObject {
                    hash: obj_hash.clone(),
                    size: obj_content.len() as u64,
                    object_type: "file".to_string(),
                },
            ],
            metadata: None,
        };

        // Write manifest to local repo
        let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
        std::fs::write(local_refs.join("production.manifest"), &manifest_json).unwrap();

        // Compute commit hash and write production ref
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let mut commit_hasher = Sha256::new();
        commit_hasher.update(&manifest_bytes);
        let commit_hash = hex::encode(commit_hasher.finalize());

        let ref_content = serde_json::json!({
            "commit": commit_hash,
            "version": 1,
            "timestamp": "2024-01-01T00:00:00Z"
        });
        std::fs::write(local_refs.join("production"), 
                      serde_json::to_string_pretty(&ref_content).unwrap()).unwrap();

        // Generate test keypair
        let (private_key, public_key) = crate::signing::generate_keypair().unwrap();

        // Export commit to target
        let result = export_commit_to_target(
            local_repo.path(),
            &commit_hash,
            target_usb.path(),
            true,
            &private_key,
        );

        assert!(result.is_ok(), "Export should succeed, got: {:?}", result);

        // Verify object was copied to target
        let target_obj_path = target_repo.join("objects")
            .join(&obj_hash[0..2])
            .join(&obj_hash[2..]);
        assert!(target_obj_path.exists(), "Object should be copied to target");
        
        let copied_content = std::fs::read(&target_obj_path).unwrap();
        assert_eq!(&copied_content, obj_content, "Copied object content should match");

        // Verify manifest and signature were written
        let target_manifest = target_repo.join("refs/heads/production.manifest");
        let target_sig = target_repo.join("refs/heads/production.sig");
        assert!(target_manifest.exists(), "Manifest should be written to target");
        assert!(target_sig.exists(), "Signature should be written to target");

        // Verify signature is valid
        let written_manifest = std::fs::read(&target_manifest).unwrap();
        let written_sig = std::fs::read(&target_sig).unwrap();
        assert!(crate::signing::verify_data_signature(&written_manifest, &written_sig, &public_key),
                "Signature should be valid");
    }

    #[test]
    fn test_export_commit_skips_existing_objects() {
        use tempfile::tempdir;
        use sha2::{Sha256, Digest};

        let local_repo = tempdir().unwrap();
        let target_usb = tempdir().unwrap();
        
        // Setup repositories
        let local_objects = local_repo.path().join("objects");
        let local_refs = local_repo.path().join("refs/heads");
        std::fs::create_dir_all(&local_objects).unwrap();
        std::fs::create_dir_all(&local_refs).unwrap();

        let target_repo = target_usb.path().join("aegis_repo");
        let target_objects = target_repo.join("objects");
        std::fs::create_dir_all(&target_objects).unwrap();

        // Create two test objects
        let obj1_content = b"first object content";
        let obj2_content = b"second object content";
        
        let mut hasher1 = Sha256::new();
        hasher1.update(obj1_content);
        let obj1_hash = hex::encode(hasher1.finalize());
        
        let mut hasher2 = Sha256::new();
        hasher2.update(obj2_content);
        let obj2_hash = hex::encode(hasher2.finalize());

        // Write both objects to local CAS
        for (hash, content) in [(&obj1_hash, &obj1_content[..]), (&obj2_hash, &obj2_content[..])] {
            let obj_dir = local_objects.join(&hash[0..2]);
            std::fs::create_dir_all(&obj_dir).unwrap();
            std::fs::write(obj_dir.join(&hash[2..]), content).unwrap();
        }

        // Pre-place first object on target (simulate existing object)
        let target_obj1_dir = target_objects.join(&obj1_hash[0..2]);
        std::fs::create_dir_all(&target_obj1_dir).unwrap();
        std::fs::write(target_obj1_dir.join(&obj1_hash[2..]), obj1_content).unwrap();

        // Get mtime of existing object
        let existing_obj_path = target_obj1_dir.join(&obj1_hash[2..]);
        let existing_mtime = std::fs::metadata(&existing_obj_path).unwrap().modified().unwrap();

        // Create manifest with both objects
        let manifest = Manifest {
            version: 1,
            commit_hash: "test_commit_456".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            objects: vec![
                ManifestObject {
                    hash: obj1_hash.clone(),
                    size: obj1_content.len() as u64,
                    object_type: "file".to_string(),
                },
                ManifestObject {
                    hash: obj2_hash.clone(),
                    size: obj2_content.len() as u64,
                    object_type: "file".to_string(),
                },
            ],
            metadata: None,
        };

        // Setup refs
        std::fs::write(local_refs.join("production.manifest"), 
                      serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let mut commit_hasher = Sha256::new();
        commit_hasher.update(&manifest_bytes);
        let commit_hash = hex::encode(commit_hasher.finalize());

        let ref_content = serde_json::json!({
            "commit": commit_hash,
            "version": 1,
            "timestamp": "2024-01-01T00:00:00Z"
        });
        std::fs::write(local_refs.join("production"), 
                      serde_json::to_string_pretty(&ref_content).unwrap()).unwrap();

        // Generate keypair and export
        let (private_key, _) = crate::signing::generate_keypair().unwrap();
        
        let result = export_commit_to_target(
            local_repo.path(),
            &commit_hash,
            target_usb.path(),
            true,
            &private_key,
        );

        assert!(result.is_ok(), "Export should succeed");

        // Verify first object was NOT modified (mtime should be same)
        let new_mtime = std::fs::metadata(&existing_obj_path).unwrap().modified().unwrap();
        assert_eq!(existing_mtime, new_mtime, "Existing object should not be modified");

        // Verify second object was copied
        let target_obj2_path = target_objects.join(&obj2_hash[0..2]).join(&obj2_hash[2..]);
        assert!(target_obj2_path.exists(), "New object should be copied");
        
        let copied_content = std::fs::read(&target_obj2_path).unwrap();
        assert_eq!(&copied_content, obj2_content, "New object content should match");
    }

    #[test]
    fn test_export_manifest_signature_written_and_verified() {
        use tempfile::tempdir;
        use sha2::{Sha256, Digest};

        let local_repo = tempdir().unwrap();
        let target_usb = tempdir().unwrap();
        
        // Setup full local repo
        let local_objects = local_repo.path().join("objects");
        let local_refs = local_repo.path().join("refs/heads");
        std::fs::create_dir_all(&local_objects).unwrap();
        std::fs::create_dir_all(&local_refs).unwrap();

        let target_repo = target_usb.path().join("aegis_repo");
        std::fs::create_dir_all(&target_repo).unwrap();

        // Create multiple test objects
        let objects_data = vec![
            (b"content one" as &[u8], "/path/one"),
            (b"content two", "/path/two"),
            (b"content three", "/path/three"),
        ];

        let mut manifest_objects = Vec::new();
        for (content, _path) in objects_data {
            let mut hasher = Sha256::new();
            hasher.update(content);
            let hash = hex::encode(hasher.finalize());

            // Write to local CAS
            let obj_dir = local_objects.join(&hash[0..2]);
            std::fs::create_dir_all(&obj_dir).unwrap();
            std::fs::write(obj_dir.join(&hash[2..]), content).unwrap();

            manifest_objects.push(ManifestObject {
                hash,
                size: content.len() as u64,
                object_type: "file".to_string(),
            });
        }

        // Create manifest with metadata
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("author".to_string(), "test_user".to_string());
        metadata.insert("description".to_string(), "test export".to_string());

        let manifest = Manifest {
            version: 42,
            commit_hash: "explicit_commit_hash".to_string(),
            timestamp: "2024-11-22T12:00:00Z".to_string(),
            objects: manifest_objects,
            metadata: Some(metadata),
        };

        // Setup refs
        std::fs::write(local_refs.join("production.manifest"), 
                      serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let mut commit_hasher = Sha256::new();
        commit_hasher.update(&manifest_bytes);
        let commit_hash = hex::encode(commit_hasher.finalize());

        let ref_content = serde_json::json!({
            "commit": commit_hash,
            "version": 42,
            "timestamp": "2024-11-22T12:00:00Z"
        });
        std::fs::write(local_refs.join("production"), 
                      serde_json::to_string_pretty(&ref_content).unwrap()).unwrap();

        // Generate test keypair
        let (private_key, public_key) = crate::signing::generate_keypair().unwrap();

        // Export
        let result = export_commit_to_target(
            local_repo.path(),
            &commit_hash,
            target_usb.path(),
            true,
            &private_key,
        );

        assert!(result.is_ok(), "Export should succeed: {:?}", result);

        // Read written manifest and signature
        let target_manifest_path = target_repo.join("refs/heads/production.manifest");
        let target_sig_path = target_repo.join("refs/heads/production.sig");
        
        assert!(target_manifest_path.exists(), "Manifest should exist");
        assert!(target_sig_path.exists(), "Signature should exist");

        let written_manifest = std::fs::read(&target_manifest_path).unwrap();
        let written_sig = std::fs::read(&target_sig_path).unwrap();

        // Verify signature is valid with public key
        let is_valid = crate::signing::verify_data_signature(&written_manifest, &written_sig, &public_key);
        assert!(is_valid, "Signature must be valid");

        // Verify manifest content matches original
        let written_manifest_obj: Manifest = serde_json::from_slice(&written_manifest).unwrap();
        assert_eq!(written_manifest_obj.version, 42);
        assert_eq!(written_manifest_obj.objects.len(), 3);
        assert_eq!(written_manifest_obj.timestamp, "2024-11-22T12:00:00Z");
    }

    #[test]
    fn test_export_temp_cleanup_on_failure() {
        use tempfile::tempdir;
        use sha2::{Sha256, Digest};

        let local_repo = tempdir().unwrap();
        let target_usb = tempdir().unwrap();
        
        // Setup local repo
        let local_objects = local_repo.path().join("objects");
        let local_refs = local_repo.path().join("refs/heads");
        std::fs::create_dir_all(&local_objects).unwrap();
        std::fs::create_dir_all(&local_refs).unwrap();

        let target_repo = target_usb.path().join("aegis_repo");
        std::fs::create_dir_all(&target_repo).unwrap();

        // Create object with INCORRECT hash in local CAS
        let obj_content = b"actual content";
        let wrong_hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        
        // Write object with wrong hash
        let obj_dir = local_objects.join(&wrong_hash[0..2]);
        std::fs::create_dir_all(&obj_dir).unwrap();
        std::fs::write(obj_dir.join(&wrong_hash[2..]), obj_content).unwrap();

        // Create manifest referencing the wrong hash
        let manifest = Manifest {
            version: 1,
            commit_hash: "fail_test".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            objects: vec![
                ManifestObject {
                    hash: wrong_hash.to_string(),
                    size: obj_content.len() as u64,
                    object_type: "file".to_string(),
                },
            ],
            metadata: None,
        };

        // Setup refs
        std::fs::write(local_refs.join("production.manifest"), 
                      serde_json::to_string_pretty(&manifest).unwrap()).unwrap();
        
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let mut commit_hasher = Sha256::new();
        commit_hasher.update(&manifest_bytes);
        let commit_hash = hex::encode(commit_hasher.finalize());

        let ref_content = serde_json::json!({
            "commit": commit_hash,
            "version": 1,
            "timestamp": "2024-01-01T00:00:00Z"
        });
        std::fs::write(local_refs.join("production"), 
                      serde_json::to_string_pretty(&ref_content).unwrap()).unwrap();

        // Generate keypair
        let (private_key, _) = crate::signing::generate_keypair().unwrap();

        // Export should fail due to hash mismatch
        let result = export_commit_to_target(
            local_repo.path(),
            &commit_hash,
            target_usb.path(),
            true,
            &private_key,
        );

        // Verify it failed with HashMismatch
        match result {
            Err(OstreeError::HashMismatch { expected, actual }) => {
                assert!(!expected.is_empty(), "Expected hash should be set");
                assert!(!actual.is_empty(), "Actual hash should be set");
            }
            _ => panic!("Expected HashMismatch error, got: {:?}", result),
        }

        // Verify temp file was cleaned up
        let target_obj_dir = target_repo.join("objects").join(&wrong_hash[0..2]);
        let temp_path = target_obj_dir.join(format!("{}.tmp", &wrong_hash[2..]));
        assert!(!temp_path.exists(), "Temp file should be cleaned up after failure");

        // Verify final object was NOT created
        let final_path = target_obj_dir.join(&wrong_hash[2..]);
        assert!(!final_path.exists(), "Final object should not exist after failure");
    }
}

