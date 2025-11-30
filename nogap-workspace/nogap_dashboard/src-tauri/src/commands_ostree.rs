// NoGap Dashboard - OSTree-Lite Tauri Commands
// Tauri command wrappers for air-gapped USB repository operations

use nogap_core::ostree_lite::{
    discover_usb_repos, read_manifest, verify_manifest, pull_objects, 
    install_manifest, export_commit_to_target, OstreeError
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Preview information for a USB repository before import
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportPreview {
    /// Manifest version number
    pub version: u32,
    /// Number of objects in the manifest
    pub objects_count: usize,
    /// Total size of all objects in bytes
    pub total_size: u64,
    /// Path to the USB repository
    pub repo_path: String,
    /// Whether signature verification succeeded
    pub verified: bool,
    /// Verification status message
    pub verification_msg: String,
}

/// Result of an import or export operation
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    /// Whether the operation succeeded
    pub ok: bool,
    /// Human-readable status message
    pub message: String,
    /// Applied version/commit hash (if successful)
    pub applied_version: Option<String>,
}

/// Get the default local repository path: ~/.nogap/local_repo
fn default_local_repo() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Cannot determine home directory".to_string())?;
    
    let repo_path = home.join(".nogap").join("local_repo");
    
    // Ensure directory exists
    std::fs::create_dir_all(&repo_path)
        .map_err(|e| format!("Failed to create local repo directory: {}", e))?;
    
    Ok(repo_path)
}

/// Convert OstreeError to String for Tauri error handling
fn err_to_string(e: OstreeError) -> String {
    format!("{}", e)
}

/// Scan for USB repositories containing aegis_repo directories
///
/// Returns a list of USB repository paths as strings.
#[tauri::command]
pub async fn cmd_scan_usb_repos() -> Result<Vec<String>, String> {
    log::info!("Scanning for USB repositories...");
    
    let repos = discover_usb_repos()
        .map_err(err_to_string)?;
    
    let repo_strings: Vec<String> = repos
        .into_iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    
    log::info!("Found {} USB repositories", repo_strings.len());
    Ok(repo_strings)
}

/// Preview a USB repository before importing
///
/// Reads the manifest, verifies the signature, and returns preview information
/// without actually importing any objects.
///
/// # Arguments
/// * `repo_path` - Path to the USB repository root (containing aegis_repo/)
#[tauri::command]
pub async fn cmd_preview_repo(repo_path: String) -> Result<ImportPreview, String> {
    log::info!("Previewing repository: {}", repo_path);
    
    let repo_root = PathBuf::from(&repo_path);
    let aegis_repo = repo_root.join("aegis_repo");
    
    if !aegis_repo.exists() || !aegis_repo.is_dir() {
        return Err(format!("Invalid repository: aegis_repo/ not found at {}", repo_path));
    }
    
    // Read manifest and signature from USB
    let (manifest, manifest_bytes) = read_manifest(&aegis_repo)
        .map_err(err_to_string)?;
    
    // Read signature file
    let sig_path = aegis_repo.join("refs").join("heads").join("production.sig");
    let sig_bytes = std::fs::read(&sig_path)
        .map_err(|e| format!("Failed to read signature: {}", e))?;
    
    // Verify signature
    let (verified, verification_msg) = match verify_manifest(&manifest_bytes, &sig_bytes) {
        Ok(_) => (true, "Signature valid".to_string()),
        Err(e) => (false, format!("Signature verification failed: {}", err_to_string(e))),
    };
    
    // Calculate total size by reading object sizes from USB
    let mut total_size = 0u64;
    let usb_objects = aegis_repo.join("objects");
    
    for obj in &manifest.objects {
        let obj_hash = &obj.hash;
        if obj_hash.len() >= 2 {
            let obj_path = usb_objects
                .join(&obj_hash[0..2])
                .join(&obj_hash[2..]);
            
            if let Ok(metadata) = std::fs::metadata(&obj_path) {
                total_size += metadata.len();
            }
        }
    }
    
    log::info!("Preview complete: {} objects, {} bytes, verified={}", 
               manifest.objects.len(), total_size, verified);
    
    Ok(ImportPreview {
        version: manifest.version,
        objects_count: manifest.objects.len(),
        total_size,
        repo_path,
        verified,
        verification_msg,
    })
}

/// Import a USB repository into the local repository
///
/// Verifies the signature, pulls all objects into the local CAS,
/// and installs the manifest to update the production ref.
///
/// # Arguments
/// * `repo_path` - Path to the USB repository root (containing aegis_repo/)
#[tauri::command]
pub async fn cmd_import_repo(repo_path: String) -> Result<ImportResult, String> {
    log::info!("Importing repository: {}", repo_path);
    
    let repo_root = PathBuf::from(&repo_path);
    let aegis_repo = repo_root.join("aegis_repo");
    
    if !aegis_repo.exists() || !aegis_repo.is_dir() {
        return Err(format!("Invalid repository: aegis_repo/ not found at {}", repo_path));
    }
    
    // Read manifest and signature
    let (manifest, manifest_bytes) = read_manifest(&aegis_repo)
        .map_err(err_to_string)?;
    
    let sig_path = aegis_repo.join("refs").join("heads").join("production.sig");
    let sig_bytes = std::fs::read(&sig_path)
        .map_err(|e| format!("Failed to read signature: {}", e))?;
    
    // Verify signature before importing
    verify_manifest(&manifest_bytes, &sig_bytes)
        .map_err(|e| format!("Signature verification failed: {}", err_to_string(e)))?;
    
    log::info!("Signature verified, pulling {} objects...", manifest.objects.len());
    
    // Get local repository path
    let local_repo = default_local_repo()?;
    
    // Pull objects from USB to local CAS
    let pulled = pull_objects(&aegis_repo, &manifest, &local_repo, None)
        .map_err(err_to_string)?;
    
    log::info!("Pulled {} objects, installing manifest...", pulled.len());
    
    // Install manifest (updates production ref atomically)
    install_manifest(&manifest, &local_repo)
        .map_err(err_to_string)?;
    
    log::info!("Import complete: version {}, commit {}", 
               manifest.version, manifest.commit_hash);
    
    Ok(ImportResult {
        ok: true,
        message: format!("Imported successfully: {} objects, version {}", 
                        manifest.objects.len(), manifest.version),
        applied_version: Some(manifest.commit_hash),
    })
}

/// Export a commit from the local repository to a USB drive
///
/// Copies all objects for the specified commit to the target USB,
/// creates a signed manifest, and performs atomic writes with verification.
///
/// # Arguments
/// * `commit_hash` - The commit hash to export
/// * `target_usb` - Path to the target USB drive root
/// * `confirmed` - User confirmation flag (must be true)
#[tauri::command]
pub async fn cmd_export_commit(
    commit_hash: String,
    target_usb: String,
    confirmed: bool,
) -> Result<ImportResult, String> {
    log::info!("Exporting commit {} to {}, confirmed={}", commit_hash, target_usb, confirmed);
    
    // Check confirmation
    if !confirmed {
        return Err("User confirmation required for export operation".to_string());
    }
    
    let target_path = PathBuf::from(&target_usb);
    let target_aegis = target_path.join("aegis_repo");
    
    // Verify target has aegis_repo directory
    if !target_aegis.exists() || !target_aegis.is_dir() {
        return Err(format!("Target USB must contain aegis_repo/ directory at {}", target_usb));
    }
    
    // Get local repository path
    let local_repo = default_local_repo()?;
    
    // Load or generate signing key
    // For now, we'll generate a new keypair (in production, load from secure storage)
    let (private_key, _public_key) = nogap_core::signing::generate_keypair()
        .map_err(|e| format!("Failed to generate signing key: {}", e))?;
    
    log::info!("Generated RSA keypair for signing");
    
    // Export commit to target USB
    export_commit_to_target(
        &local_repo,
        &commit_hash,
        &target_path,
        confirmed,
        &private_key,
    )
    .map_err(err_to_string)?;
    
    log::info!("Export complete: commit {}", commit_hash);
    
    Ok(ImportResult {
        ok: true,
        message: format!("Export completed successfully: commit {}", commit_hash),
        applied_version: Some(commit_hash),
    })
}

/// List all mounted drives (for development/testing)
///
/// Returns all mounted drives/volumes without filtering for aegis_repo.
/// This is useful for development where test USBs may be empty.
///
/// Platform-specific behavior:
/// - Windows: Enumerates drive letters D: to Z:, includes DRIVE_REMOVABLE and DRIVE_FIXED
/// - Linux: Lists subdirectories of /media and /run/media
/// - macOS: Lists subdirectories of /Volumes
///
/// Returns only entries that exist and are directories.
/// Permission errors are silently ignored.
#[tauri::command]
pub async fn cmd_list_all_drives() -> Result<Vec<String>, String> {
    log::info!("Listing all mounted drives");
    
    let mut drives = Vec::new();
    
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStrExt;
        
        // Enumerate drive letters from D: to Z:
        for letter in b'D'..=b'Z' {
            let drive_path = format!("{}:\\", letter as char);
            let drive_os_string: Vec<u16> = OsString::from(&drive_path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            
            unsafe {
                let drive_type = windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(
                    drive_os_string.as_ptr()
                );
                
                // Include DRIVE_REMOVABLE (2) and DRIVE_FIXED (3)
                // DRIVE_UNKNOWN = 0, DRIVE_NO_ROOT_DIR = 1, DRIVE_CDROM = 5, etc.
                if drive_type == 2 || drive_type == 3 {
                    let path = PathBuf::from(&drive_path);
                    if path.exists() && path.is_dir() {
                        log::debug!("Found drive: {}", drive_path);
                        drives.push(drive_path);
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // Check /media
        if let Ok(entries) = std::fs::read_dir("/media") {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        if let Ok(path) = entry.path().canonicalize() {
                            drives.push(path.to_string_lossy().to_string());
                            log::debug!("Found drive: {}", path.display());
                        }
                    }
                }
            }
        }
        
        // Check /run/media
        if let Ok(entries) = std::fs::read_dir("/run/media") {
            for entry in entries.flatten() {
                // /run/media contains user directories, check subdirectories
                if let Ok(user_entries) = std::fs::read_dir(entry.path()) {
                    for user_entry in user_entries.flatten() {
                        if let Ok(metadata) = user_entry.metadata() {
                            if metadata.is_dir() {
                                if let Ok(path) = user_entry.path().canonicalize() {
                                    drives.push(path.to_string_lossy().to_string());
                                    log::debug!("Found drive: {}", path.display());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // Check /Volumes
        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        if let Ok(path) = entry.path().canonicalize() {
                            drives.push(path.to_string_lossy().to_string());
                            log::debug!("Found drive: {}", path.display());
                        }
                    }
                }
            }
        }
    }
    
    log::info!("Found {} drives", drives.len());
    Ok(drives)
}
