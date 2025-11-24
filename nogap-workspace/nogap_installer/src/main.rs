use std::fs;
use std::path::Path;
use std::io::Result;

fn main() -> Result<()> {
    println!("NoGap Installer - Server Environment Setup");
    println!("===========================================\n");

    // Detect OS and set installation paths
    #[cfg(target_os = "windows")]
    let install_dir = r"C:\Program Files\NoGap";
    
    #[cfg(target_family = "unix")]
    let install_dir = "/opt/nogap";

    println!("Installing to: {}", install_dir);

    // Create installation directory structure
    println!("Creating directories...");
    fs::create_dir_all(install_dir)?;
    fs::create_dir_all(format!("{}/local_repo/objects", install_dir))?;
    fs::create_dir_all(format!("{}/local_repo/refs", install_dir))?;
    println!("[OK] Directories created");

    // Define source paths (relative to installer binary location during dev)
    let cli_binary_src = "target/release/nogap-cli";
    let policies_src = "configs/policies.yaml";
    let trusted_keys_src = "configs/trusted_keys.json";

    #[cfg(target_os = "windows")]
    let cli_binary_src = "target/release/nogap-cli.exe";

    // Define destination paths
    let cli_binary_dest = format!("{}/nogap-cli", install_dir);
    let policies_dest = format!("{}/policies.yaml", install_dir);
    let trusted_keys_dest = format!("{}/trusted_keys.json", install_dir);

    #[cfg(target_os = "windows")]
    let cli_binary_dest = format!("{}/nogap-cli.exe", install_dir);

    // Copy artifacts
    println!("\nCopying artifacts...");
    
    if Path::new(cli_binary_src).exists() {
        fs::copy(cli_binary_src, &cli_binary_dest)?;
        println!("[OK] Copied nogap-cli binary");
    } else {
        eprintln!("[WARN] CLI binary not found at {}", cli_binary_src);
        eprintln!("       Run 'cargo build --release' in nogap_cli first");
    }

    if Path::new(policies_src).exists() {
        fs::copy(policies_src, &policies_dest)?;
        println!("[OK] Copied policies.yaml");
    } else {
        eprintln!("[WARN] policies.yaml not found at {}", policies_src);
    }

    if Path::new(trusted_keys_src).exists() {
        fs::copy(trusted_keys_src, &trusted_keys_dest)?;
        println!("[OK] Copied trusted_keys.json");
    } else {
        eprintln!("[WARN] trusted_keys.json not found at {}", trusted_keys_src);
    }

    // Create symlink on Linux
    #[cfg(target_family = "unix")]
    {
        println!("\nCreating symlink...");
        let symlink_path = "/usr/local/bin/nogap-cli";
        let target_path = format!("{}/nogap-cli", install_dir);
        
        // Remove existing symlink if present
        if Path::new(symlink_path).exists() {
            let _ = fs::remove_file(symlink_path);
        }
        
        std::os::unix::fs::symlink(&target_path, symlink_path)?;
        println!("[OK] Symlink created: {} -> {}", symlink_path, target_path);
    }

    println!("\n[OK] NoGap installed successfully.");
    
    #[cfg(target_family = "unix")]
    println!("\nYou can now run 'nogap-cli' from anywhere.");
    
    #[cfg(target_os = "windows")]
    println!("\nAdd '{}' to your PATH to run nogap-cli from anywhere.", install_dir);

    Ok(())
}
