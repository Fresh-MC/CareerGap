use std::fs;
use std::path::Path;
use std::io::Result;

fn main() -> Result<()> {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║     NoGap System Installer - Full Environment         ║");
    println!("║     Dashboard + CLI + Configs + Local Repository      ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // Detect OS and set installation paths
    #[cfg(target_os = "windows")]
    let install_base = r"C:\Program Files\NoGap";
    
    #[cfg(target_family = "unix")]
    let install_base = "/opt/nogap";

    println!("Installation directory: {}\n", install_base);

    // Create directory structure
    println!("▶ Creating directory structure...");
    create_directories(install_base)?;

    // Copy binaries
    println!("\n▶ Copying binaries...");
    copy_binaries(install_base)?;

    // Copy configuration files
    println!("\n▶ Copying configuration files...");
    copy_configs(install_base)?;

    // Copy local repository template
    println!("\n▶ Setting up local repository...");
    copy_local_repo_template(install_base)?;

    // Set permissions on Linux
    #[cfg(target_family = "unix")]
    {
        println!("\n▶ Setting permissions...");
        set_linux_permissions(install_base)?;
    }

    // Create symlinks on Linux
    #[cfg(target_family = "unix")]
    {
        println!("\n▶ Creating symlinks...");
        create_linux_symlinks(install_base)?;
    }

    println!("\n╔════════════════════════════════════════════════════════╗");
    println!("║              [OK] Installation complete!              ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    print_post_install_instructions(install_base);

    Ok(())
}

fn create_directories(base: &str) -> Result<()> {
    let directories = vec![
        format!("{}/dashboard", base),
        format!("{}/cli", base),
        format!("{}/local_repo/objects", base),
        format!("{}/local_repo/refs", base),
        format!("{}/configs", base),
    ];

    for dir in directories {
        fs::create_dir_all(&dir)?;
        println!("  [OK] Created directory: {}", dir);
    }

    Ok(())
}

fn copy_binaries(base: &str) -> Result<()> {
    // Dashboard binary
    #[cfg(target_os = "windows")]
    let dashboard_src = "target/release/nogap-dashboard.exe";
    #[cfg(target_family = "unix")]
    let dashboard_src = "target/release/nogap-dashboard";

    let dashboard_dest = format!("{}/dashboard/nogap-dashboard", base);
    #[cfg(target_os = "windows")]
    let dashboard_dest = format!("{}/dashboard/nogap-dashboard.exe", base);

    if Path::new(dashboard_src).exists() {
        fs::copy(dashboard_src, &dashboard_dest)?;
        println!("  [OK] Copied nogap-dashboard");
    } else {
        eprintln!("  [WARN] Dashboard binary not found at {}", dashboard_src);
        eprintln!("         Build it first: cd nogap_dashboard && cargo build --release");
    }

    // CLI binary
    #[cfg(target_os = "windows")]
    let cli_src = "target/release/nogap-cli.exe";
    #[cfg(target_family = "unix")]
    let cli_src = "target/release/nogap-cli";

    let cli_dest = format!("{}/cli/nogap-cli", base);
    #[cfg(target_os = "windows")]
    let cli_dest = format!("{}/cli/nogap-cli.exe", base);

    if Path::new(cli_src).exists() {
        fs::copy(cli_src, &cli_dest)?;
        println!("  [OK] Copied nogap-cli");
    } else {
        eprintln!("  [WARN] CLI binary not found at {}", cli_src);
        eprintln!("         Build it first: cd nogap_cli && cargo build --release");
    }

    Ok(())
}

fn copy_configs(base: &str) -> Result<()> {
    let configs = vec![
        ("configs/policies.yaml", format!("{}/configs/policies.yaml", base)),
        ("configs/trusted_keys.json", format!("{}/configs/trusted_keys.json", base)),
    ];

    for (src, dest) in configs {
        if Path::new(src).exists() {
            fs::copy(src, &dest)?;
            println!("  [OK] Copied {}", src);
        } else {
            eprintln!("  [WARN] Config file not found: {}", src);
            eprintln!("         Ensure configs/ directory exists with required files");
        }
    }

    Ok(())
}

fn copy_local_repo_template(base: &str) -> Result<()> {
    let template_path = "assets/local_repo_template";
    
    if !Path::new(template_path).exists() {
        eprintln!("  [WARN] Local repo template not found at {}", template_path);
        eprintln!("         Creating empty local_repo structure");
        return Ok(());
    }

    // Copy all files from template to local_repo
    let local_repo_dest = format!("{}/local_repo", base);
    
    match copy_dir_all(template_path, &local_repo_dest) {
        Ok(_) => println!("  [OK] Copied local repository template"),
        Err(e) => {
            eprintln!("  [WARN] Failed to copy local_repo_template: {}", e);
            eprintln!("         Continuing with empty local_repo structure");
        }
    }

    Ok(())
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(&dst)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.as_ref().join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}

#[cfg(target_family = "unix")]
fn set_linux_permissions(base: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // Set permissions on local_repo directory (700 = rwx------)
    let local_repo_path = format!("{}/local_repo", base);
    let mut perms = fs::metadata(&local_repo_path)?.permissions();
    perms.set_mode(0o700);
    fs::set_permissions(&local_repo_path, perms)?;
    println!("  [OK] Set permissions 700 on local_repo");

    // Set permissions on configs (600 = rw-------)
    let configs = vec![
        format!("{}/configs/policies.yaml", base),
        format!("{}/configs/trusted_keys.json", base),
    ];

    for config_path in configs {
        if Path::new(&config_path).exists() {
            let mut perms = fs::metadata(&config_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&config_path, perms)?;
            println!("  [OK] Set permissions 600 on {}", config_path);
        }
    }

    Ok(())
}

#[cfg(target_family = "unix")]
fn create_linux_symlinks(base: &str) -> Result<()> {
    let symlinks = vec![
        (
            format!("{}/cli/nogap-cli", base),
            "/usr/local/bin/nogap-cli".to_string(),
        ),
        (
            format!("{}/dashboard/nogap-dashboard", base),
            "/usr/local/bin/nogap-dashboard".to_string(),
        ),
    ];

    for (target, link) in symlinks {
        // Remove existing symlink if present
        if Path::new(&link).exists() {
            let _ = fs::remove_file(&link);
        }

        std::os::unix::fs::symlink(&target, &link)?;
        println!("  [OK] Created symlink: {} -> {}", link, target);
    }

    Ok(())
}

fn print_post_install_instructions(base: &str) {
    #[cfg(target_family = "unix")]
    {
        println!("Post-Installation:");
        println!("  • Run 'nogap-cli' from anywhere");
        println!("  • Run 'nogap-dashboard' to launch the GUI");
        println!("  • Configuration: {}/configs/", base);
        println!("  • Local repository: {}/local_repo/", base);
    }

    #[cfg(target_os = "windows")]
    {
        println!("Post-Installation:");
        println!("  • Add '{}\\cli' to your PATH to run nogap-cli", base);
        println!("  • Add '{}\\dashboard' to your PATH to run nogap-dashboard", base);
        println!("  • Or run directly:");
        println!("    - {}\\cli\\nogap-cli.exe", base);
        println!("    - {}\\dashboard\\nogap-dashboard.exe", base);
        println!("  • Configuration: {}\\configs\\", base);
        println!("  • Local repository: {}\\local_repo\\", base);
    }
}
