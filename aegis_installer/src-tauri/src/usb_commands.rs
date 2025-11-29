use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[tauri::command]
fn cmd_list_drives() -> Result<Vec<String>, String> {
    let mut drives = Vec::new();
    
    #[cfg(target_os = "windows")]
    {
        for letter in b'D'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            if Path::new(&drive).exists() {
                drives.push(drive);
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        if let Ok(entries) = fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.file_name().unwrap() != "Macintosh HD" {
                    if let Some(path_str) = path.to_str() {
                        drives.push(path_str.to_string());
                    }
                }
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // Check /media/$USER
        if let Ok(user) = std::env::var("USER") {
            let media_path = format!("/media/{}", user);
            if let Ok(entries) = fs::read_dir(&media_path) {
                for entry in entries.flatten() {
                    if let Some(path_str) = entry.path().to_str() {
                        drives.push(path_str.to_string());
                    }
                }
            }
        }
        
        // Check /run/media/$USER
        if let Ok(user) = std::env::var("USER") {
            let run_media_path = format!("/run/media/{}", user);
            if let Ok(entries) = fs::read_dir(&run_media_path) {
                for entry in entries.flatten() {
                    if let Some(path_str) = entry.path().to_str() {
                        drives.push(path_str.to_string());
                    }
                }
            }
        }
    }
    
    Ok(drives)
}

#[tauri::command]
fn cmd_create_repo_structure(usb_path: String) -> Result<(), String> {
    let repo_path = Path::new(&usb_path).join("aegis_repo");
    
    // Create aegis_repo/objects
    fs::create_dir_all(repo_path.join("objects"))
        .map_err(|e| format!("Failed to create objects directory: {}", e))?;
    
    // Create aegis_repo/refs/heads
    fs::create_dir_all(repo_path.join("refs/heads"))
        .map_err(|e| format!("Failed to create refs/heads directory: {}", e))?;
    
    Ok(())
}

#[tauri::command]
fn cmd_write_object(usb_path: String, hash: String, bytes: Vec<u8>) -> Result<(), String> {
    if hash.len() < 2 {
        return Err("Hash too short for sharding".to_string());
    }
    
    let repo_path = Path::new(&usb_path).join("aegis_repo");
    let shard_dir = &hash[0..2];
    let object_dir = repo_path.join("objects").join(shard_dir);
    
    // Create shard directory
    fs::create_dir_all(&object_dir)
        .map_err(|e| format!("Failed to create object directory: {}", e))?;
    
    let object_path = object_dir.join(&hash);
    let temp_path = object_dir.join(format!("{}.tmp", hash));
    
    // Write to temp file
    fs::write(&temp_path, &bytes)
        .map_err(|e| format!("Failed to write object temp file: {}", e))?;
    
    // Atomic rename
    fs::rename(&temp_path, &object_path)
        .map_err(|e| format!("Failed to rename object file: {}", e))?;
    
    Ok(())
}

#[tauri::command]
fn cmd_write_manifest(usb_path: String, manifest: String) -> Result<(), String> {
    let repo_path = Path::new(&usb_path).join("aegis_repo");
    let manifest_path = repo_path.join("refs/heads/production.manifest");
    let temp_path = repo_path.join("refs/heads/production.manifest.tmp");
    
    // Write to temp file
    fs::write(&temp_path, manifest.as_bytes())
        .map_err(|e| format!("Failed to write manifest temp file: {}", e))?;
    
    // Atomic rename
    fs::rename(&temp_path, &manifest_path)
        .map_err(|e| format!("Failed to rename manifest file: {}", e))?;
    
    Ok(())
}

#[tauri::command]
fn cmd_sign_manifest(manifest_path: String, key_path: String) -> Result<String, String> {
    // Read manifest bytes
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    
    // Load private key
    let key_bytes = load_private_key(&key_path)?;
    
    // Create signing key
    let signing_key = SigningKey::from_bytes(&key_bytes);
    
    // Sign the manifest
    let signature: Signature = signing_key.sign(&manifest_bytes);
    
    // Convert signature to hex
    let signature_hex = hex::encode(signature.to_bytes());
    
    Ok(signature_hex)
}

#[tauri::command]
fn cmd_write_signature(usb_path: String, signature_hex: String) -> Result<(), String> {
    let repo_path = Path::new(&usb_path).join("aegis_repo");
    let sig_path = repo_path.join("refs/heads/production.sig");
    let temp_path = repo_path.join("refs/heads/production.sig.tmp");
    
    // Write to temp file
    fs::write(&temp_path, signature_hex.as_bytes())
        .map_err(|e| format!("Failed to write signature temp file: {}", e))?;
    
    // Atomic rename
    fs::rename(&temp_path, &sig_path)
        .map_err(|e| format!("Failed to rename signature file: {}", e))?;
    
    Ok(())
}

// Helper: Load private key (supports raw binary or hex-encoded)
fn load_private_key(path: &str) -> Result<[u8; 32], String> {
    let mut file = File::open(path)
        .map_err(|e| format!("Failed to open private key: {}", e))?;
    
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)
        .map_err(|e| format!("Failed to read private key: {}", e))?;
    
    // Try raw binary (32 bytes)
    if contents.len() == 32 {
        let mut key = [0u8; 32];
        key.copy_from_slice(&contents);
        return Ok(key);
    }
    
    // Try hex-encoded (64 hex chars = 32 bytes)
    let contents_str = String::from_utf8_lossy(&contents);
    let trimmed = contents_str.trim();
    
    if trimmed.len() == 64 {
        let decoded = hex::decode(trimmed)
            .map_err(|e| format!("Failed to decode hex key: {}", e))?;
        
        if decoded.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&decoded);
            return Ok(key);
        }
    }
    
    Err(format!("Invalid private key format. Expected 32 bytes raw or 64 hex characters, got {} bytes", contents.len()))
}
