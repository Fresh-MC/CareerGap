// Example Tauri command handlers for USB Authority Creator
// Add these to your Tauri project's src-tauri/src/main.rs

use std::fs;
use std::path::Path;
use std::process::Command;

#[tauri::command]
fn cmd_create_directory(path: String) -> Result<(), String> {
    fs::create_dir_all(&path)
        .map_err(|e| format!("Failed to create directory {}: {}", path, e))?;
    Ok(())
}

#[tauri::command]
fn cmd_write_file(path: String, content: Vec<u8>) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    fs::write(&path, content)
        .map_err(|e| format!("Failed to write file {}: {}", path, e))?;
    Ok(())
}

#[tauri::command]
fn cmd_write_text_file(path: String, content: String) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    fs::write(&path, content)
        .map_err(|e| format!("Failed to write file {}: {}", path, e))?;
    Ok(())
}

#[tauri::command]
fn cmd_sign_manifest(manifest_path: String, sig_path: String) -> Result<(), String> {
    // Path to nogap-signer binary
    // Adjust this path based on your installation
    let signer_path = if cfg!(target_os = "windows") {
        "nogap_signer.exe"
    } else {
        "nogap_signer"
    };

    // Look for private key in common locations
    let key_paths = vec![
        "private.key",
        "../private.key",
        "../../private.key",
        "/opt/nogap/private.key",
        "C:\\Program Files\\NoGap\\private.key",
    ];

    let key_path = key_paths.iter()
        .find(|p| Path::new(p).exists())
        .ok_or_else(|| "Private key not found. Generate one with: nogap-signer keygen".to_string())?;

    // Execute nogap-signer
    let output = Command::new(signer_path)
        .args(&[
            "sign",
            "--input", &manifest_path,
            "--output", &sig_path,
            "--key", key_path,
        ])
        .output()
        .map_err(|e| format!("Failed to execute nogap-signer: {}. Ensure it's in PATH.", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Signing failed: {}", stderr));
    }

    Ok(())
}

#[tauri::command]
fn cmd_read_file(path: String) -> Result<Vec<u8>, String> {
    fs::read(&path)
        .map_err(|e| format!("Failed to read file {}: {}", path, e))
}

#[tauri::command]
fn cmd_file_exists(path: String) -> bool {
    Path::new(&path).exists()
}

#[tauri::command]
fn cmd_list_directory(path: String) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(&path)
        .map_err(|e| format!("Failed to read directory {}: {}", path, e))?;

    let mut files = Vec::new();
    for entry in entries {
        if let Ok(entry) = entry {
            if let Some(name) = entry.file_name().to_str() {
                files.push(name.to_string());
            }
        }
    }

    Ok(files)
}

// Register these commands in your Tauri builder:
/*
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            cmd_create_directory,
            cmd_write_file,
            cmd_write_text_file,
            cmd_sign_manifest,
            cmd_read_file,
            cmd_file_exists,
            cmd_list_directory
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
*/
