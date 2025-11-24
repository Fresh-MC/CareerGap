# NoGap Installer

Simple CLI-only installer for NoGap server environments.

## Purpose

The installer prepares a server environment by:

1. Creating the NoGap installation directory:
   - **Windows**: `C:\Program Files\NoGap`
   - **Linux**: `/opt/nogap`

2. Creating the local repository structure:
   - `local_repo/objects`
   - `local_repo/refs`

3. Copying required artifacts:
   - CLI binary (`nogap-cli`)
   - `policies.yaml`
   - `trusted_keys.json`

4. Creating a symlink on Linux:
   - `/usr/local/bin/nogap-cli` → `/opt/nogap/nogap-cli`

## Prerequisites

Before running the installer, build the NoGap CLI:

```bash
cd nogap_cli
cargo build --release
```

Ensure these files exist:
- `target/release/nogap-cli` (or `nogap-cli.exe` on Windows)
- `configs/policies.yaml`
- `configs/trusted_keys.json`

## Building the Installer

```bash
cd nogap_installer
cargo build --release
```

The installer binary will be at: `target/release/nogap_installer`

## Running the Installer

### Linux

Run with root privileges (required for `/opt` and `/usr/local/bin`):

```bash
sudo ./target/release/nogap_installer
```

### Windows

Run as Administrator (required for `C:\Program Files`):

```powershell
.\target\release\nogap_installer.exe
```

## Installation Paths

### Linux
- Installation directory: `/opt/nogap/`
- Symlink: `/usr/local/bin/nogap-cli`
- After installation, run: `nogap-cli`

### Windows
- Installation directory: `C:\Program Files\NoGap\`
- Add to PATH manually or run: `C:\Program Files\NoGap\nogap-cli.exe`

## Implementation Details

- **No dependencies**: Uses only Rust standard library
- **Cross-platform**: Detects OS with `cfg!()` macros
- **Error handling**: Returns `std::io::Result<()>`
- **Simple**: No GUI, logging, or async runtime

## Troubleshooting

If you see warnings about missing files:
- Build the CLI first: `cd nogap_cli && cargo build --release`
- Create the configs directory with required files
- Ensure you're running the installer from the project root

## Notes

- The installer expects to run from the workspace root during development
- For production, adjust source paths to match your distribution structure
- On Windows, no symlink is created; users must add to PATH manually
