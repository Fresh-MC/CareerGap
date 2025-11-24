# NoGap System Installer

Complete installation utility for the full NoGap environment on fresh workstations.

## Overview

This installer prepares a clean machine with the complete NoGap desktop experience:
- **Dashboard** (GUI application)
- **CLI** (Command-line interface)
- **Local repository** (Content-addressable storage)
- **Configuration files** (Policies and trusted keys)

## Installation Targets

### Linux
```
/opt/nogap/
├── dashboard/
│   └── nogap-dashboard
├── cli/
│   └── nogap-cli
├── local_repo/
│   ├── objects/
│   └── refs/
└── configs/
    ├── policies.yaml
    └── trusted_keys.json
```

**Symlinks created:**
- `/usr/local/bin/nogap-cli` → `/opt/nogap/cli/nogap-cli`
- `/usr/local/bin/nogap-dashboard` → `/opt/nogap/dashboard/nogap-dashboard`

**Permissions set:**
- `local_repo/`: 700 (rwx------)
- `configs/*.yaml`: 600 (rw-------)
- `configs/*.json`: 600 (rw-------)

### Windows
```
C:\Program Files\NoGap\
├── dashboard\
│   └── nogap-dashboard.exe
├── cli\
│   └── nogap-cli.exe
├── local_repo\
│   ├── objects\
│   └── refs\
└── configs\
    ├── policies.yaml
    └── trusted_keys.json
```

**Note:** On Windows, no symlinks are created. Add directories to PATH manually.

## Prerequisites

Before running the installer, build all components:

### 1. Build the Dashboard
```bash
cd nogap_dashboard
cargo build --release
```

### 2. Build the CLI
```bash
cd nogap_cli
cargo build --release
```

### 3. Prepare Configuration Files

Ensure these files exist in the repo:
```
configs/policies.yaml
configs/trusted_keys.json
```

### 4. (Optional) Prepare Local Repository Template

Create template structure:
```
assets/local_repo_template/
├── objects/
└── refs/
```

## Building the Installer

```bash
cd nogap_system_installer
cargo build --release
```

The installer binary will be at: `target/release/nogap_system_installer`

## Running the Installer

### Linux

**Requires root privileges** (for `/opt` and `/usr/local/bin`):

```bash
sudo ./target/release/nogap_system_installer
```

### Windows

**Run as Administrator** (required for `C:\Program Files`):

```powershell
# Right-click -> Run as Administrator, or:
.\target\release\nogap_system_installer.exe
```

## Expected Output

```
╔════════════════════════════════════════════════════════╗
║     NoGap System Installer - Full Environment         ║
║     Dashboard + CLI + Configs + Local Repository      ║
╚════════════════════════════════════════════════════════╝

Installation directory: /opt/nogap

▶ Creating directory structure...
  [OK] Created directory: /opt/nogap/dashboard
  [OK] Created directory: /opt/nogap/cli
  [OK] Created directory: /opt/nogap/local_repo/objects
  [OK] Created directory: /opt/nogap/local_repo/refs
  [OK] Created directory: /opt/nogap/configs

▶ Copying binaries...
  [OK] Copied nogap-dashboard
  [OK] Copied nogap-cli

▶ Copying configuration files...
  [OK] Copied configs/policies.yaml
  [OK] Copied configs/trusted_keys.json

▶ Setting up local repository...
  [OK] Copied local repository template

▶ Setting permissions...
  [OK] Set permissions 700 on local_repo
  [OK] Set permissions 600 on /opt/nogap/configs/policies.yaml
  [OK] Set permissions 600 on /opt/nogap/configs/trusted_keys.json

▶ Creating symlinks...
  [OK] Created symlink: /usr/local/bin/nogap-cli -> /opt/nogap/cli/nogap-cli
  [OK] Created symlink: /usr/local/bin/nogap-dashboard -> /opt/nogap/dashboard/nogap-dashboard

╔════════════════════════════════════════════════════════╗
║              [OK] Installation complete!              ║
╚════════════════════════════════════════════════════════╝

Post-Installation:
  • Run 'nogap-cli' from anywhere
  • Run 'nogap-dashboard' to launch the GUI
  • Configuration: /opt/nogap/configs/
  • Local repository: /opt/nogap/local_repo/
```

## Post-Installation Usage

### Linux
```bash
# Run CLI from anywhere
nogap-cli --help

# Launch Dashboard
nogap-dashboard

# Access configs
cat /opt/nogap/configs/policies.yaml
```

### Windows
```powershell
# Add to PATH (as Administrator):
setx /M PATH "%PATH%;C:\Program Files\NoGap\cli;C:\Program Files\NoGap\dashboard"

# Or run directly:
"C:\Program Files\NoGap\cli\nogap-cli.exe" --help
"C:\Program Files\NoGap\dashboard\nogap-dashboard.exe"
```

## Implementation Details

- **Zero dependencies**: Uses only Rust standard library
- **Cross-platform**: Automatic OS detection with `cfg!()` macros
- **Safe error handling**: Returns `std::io::Result<()>` with clear error messages
- **Atomic operations**: Creates directories before copying files
- **Recursive copying**: Supports nested directory structures

## Troubleshooting

### Missing binaries
```
[WARN] Dashboard binary not found at target/release/nogap-dashboard
```
**Solution:** Build the dashboard first:
```bash
cd nogap_dashboard
cargo build --release
```

### Missing configs
```
[WARN] Config file not found: configs/policies.yaml
```
**Solution:** Create the configs directory:
```bash
mkdir -p configs
# Add your policies.yaml and trusted_keys.json
```

### Permission denied (Linux)
```
Error: Permission denied (os error 13)
```
**Solution:** Run with sudo:
```bash
sudo ./target/release/nogap_system_installer
```

### Permission denied (Windows)
```
Access is denied. (os error 5)
```
**Solution:** Run as Administrator (right-click → Run as Administrator)

## File Permissions (Linux)

The installer automatically sets secure permissions:

| Path | Permissions | Reason |
|------|-------------|--------|
| `local_repo/` | 700 | Protect CAS objects |
| `configs/*.yaml` | 600 | Protect policy definitions |
| `configs/*.json` | 600 | Protect cryptographic keys |

## Development Notes

- The installer assumes it runs from the workspace root during development
- Source paths are relative to the current directory
- For production distribution, adjust paths or bundle files with the installer
- The `assets/local_repo_template/` directory is optional; installer continues if missing

## Uninstallation

### Linux
```bash
sudo rm -rf /opt/nogap
sudo rm /usr/local/bin/nogap-cli
sudo rm /usr/local/bin/nogap-dashboard
```

### Windows
```powershell
# Run as Administrator
Remove-Item "C:\Program Files\NoGap" -Recurse -Force
```

## License

Part of the NoGap project.
