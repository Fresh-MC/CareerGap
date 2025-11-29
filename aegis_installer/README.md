# Aegis USB Authority Installer

A Tauri 2.x desktop application for preparing USB drives as signed **AegisPack repositories** for air-gapped NoGap deployments.

---

## Overview

**Aegis Installer** transforms USB drives into cryptographically signed repositories containing:
- NoGap binaries and dependencies
- Content-addressable object storage (SHA256)
- Ed25519-signed manifest for integrity verification
- Version metadata and timestamps

**Technology Stack:**
- **Tauri 2.x**: Rust backend + vanilla HTML/CSS/JS frontend
- **Ed25519**: Digital signatures for tamper-proof manifests
- **Web Crypto API**: Client-side SHA256 hashing
- **Cross-platform**: Windows, macOS, Linux support

---

## Requirements

### Development
- **Node.js** 18+ and npm
- **Rust** stable (1.70+)
- **Tauri CLI** 2.x

### Runtime
- **USB drive** with write permissions
- **Private Ed25519 key** (32 bytes, raw or hex-encoded)

---

## Installation & Build

### 1. Clone/Navigate to Project

```bash
cd aegis_installer
```

### 2. Install Dependencies

```bash
npm install
```

This installs:
- `@tauri-apps/api` ^2.0.0
- `@tauri-apps/cli` ^2.0.0

### 3. Development Mode

Run with hot-reload:

```bash
npm run tauri dev
```

Or using the shorthand:

```bash
npm run dev
```

### 4. Production Build

Generate platform-specific executables:

```bash
npm run tauri build
```

Or:

```bash
npm run build
```

**Build Outputs:**
- **Windows**: `src-tauri/target/release/aegis-installer.exe`
- **macOS**: `src-tauri/target/release/bundle/dmg/Aegis Installer_1.0.0_universal.dmg`
- **Linux**: `src-tauri/target/release/bundle/deb/aegis-installer_1.0.0_amd64.deb`

---

## Key Generation

Generate an Ed25519 keypair for signing:

### Using OpenSSL

```bash
# Generate private key (32 bytes)
openssl genpkey -algorithm Ed25519 -out private.key

# Extract public key
openssl pkey -in private.key -pubout -out public.key
```

### Using Rust (nogap_signer tool)

```bash
cd ../nogap_signer
cargo run -- keygen --output keys/
```

**Output:**
- `private.key` - Keep secure on authority machine (never commit to version control)
- `public.key` - Distribute to target systems for verification

---

## Usage

### Workflow

1. **Launch Application**
   ```bash
   npm run tauri dev
   ```

2. **Select USB Drive**
   - Click "🔄 Refresh Drives" to scan for USB devices
   - Select target drive from dropdown (e.g., `/Volumes/USB`, `E:\`)

3. **Add Files**
   - Drag-and-drop files onto the upload zone, or
   - Click "Browse Files" to select manually
   - Files are hashed using SHA256 (Web Crypto API)

4. **Configure**
   - **Version**: Enter semantic version (e.g., `1.0.0`)
   - **Private Key**: Browse to `private.key` location

5. **Prepare USB**
   - Click "🚀 Prepare USB Drive"
   - Monitor progress in status log
   - Wait for "USB-A Ready!" confirmation

### Repository Structure

After preparation, the USB contains:

```
<USB_ROOT>/
└── aegis_repo/
    ├── objects/              # Content-addressable storage
    │   ├── ab/
    │   │   └── cdef1234...   # Object files (sharded by first 2 chars)
    │   └── 12/
    │       └── 3456abcd...
    └── refs/
        └── heads/
            ├── production.manifest    # JSON manifest
            └── production.sig         # Ed25519 signature (hex)
```

### Manifest Format

`production.manifest` contains:

```json
{
  "file_count": 3,
  "objects": [
    {
      "hash": "abcdef1234567890...",
      "path": "nogap-cli",
      "size": 1024000
    }
  ],
  "timestamp": "2025-11-24T10:30:00Z",
  "version": "1.0.0"
}
```

**Note:** Keys are sorted alphabetically, objects sorted by hash (canonical JSON).

---

## Architecture

### Tauri Commands (Rust Backend)

Implemented in `src-tauri/src/usb_commands.rs`:

| Command | Purpose | Platform-Specific |
|---------|---------|-------------------|
| `cmd_list_drives` | Detect USB drives | ✅ (Windows D:-Z:, macOS /Volumes, Linux /media) |
| `cmd_create_repo_structure` | Create `aegis_repo/` directories | ❌ |
| `cmd_write_object` | Write object with sharding (`objects/XX/...`) | ❌ |
| `cmd_write_manifest` | Write manifest JSON | ❌ |
| `cmd_sign_manifest` | Ed25519 sign manifest, return hex signature | ❌ |
| `cmd_write_signature` | Write `.sig` file | ❌ |

**Key Features:**
- Atomic writes using `.tmp` + `rename()` pattern
- Sharded storage for efficient object retrieval
- Ed25519-dalek for cryptographic operations
- Cross-platform drive detection

### Frontend (Vanilla JS)

Implemented in `src/main.js`:

**Key Functions:**
- `loadDrives()` - Invoke `cmd_list_drives`, populate dropdown
- `addFiles()` - Compute SHA256 using Web Crypto API
- `prepareUSB()` - Orchestrate 6-step USB preparation workflow
- `generateManifest()` - Create canonical JSON (sorted keys/objects)

**No frameworks used** - Pure HTML/CSS/JS with Tauri API bindings.

---

## Security Best Practices

### ⚠️ CRITICAL REQUIREMENTS

1. **Private Key Management**
   - Store `private.key` on **secure, offline machine**
   - Never commit to version control (check `.gitignore`)
   - Use encrypted storage or hardware security module (HSM)
   - Delete from build machine after signing

2. **Air-Gapped Operation**
   - **DO NOT host this application online**
   - Run only on isolated, secure workstations
   - Transfer USB via trusted physical courier

3. **Key Rotation**
   - If private key is compromised:
     1. Generate new keypair immediately
     2. Re-sign all USB repositories
     3. Distribute new public key out-of-band
     4. Revoke old key via secure channel

4. **Verification**
   - Embed `public.key` in target NoGap installations
   - Verify signature before extracting objects:
     ```bash
     # Manual verification (if needed)
     cat production.manifest | \
       openssl dgst -sha512 -verify public.key -signature production.sig
     ```

5. **USB Handling**
   - Format USB drives before use
   - Use tamper-evident seals for transport
   - Maintain chain-of-custody logs

---

## Troubleshooting

### Build Errors

**Issue:** `error: failed to run custom build command for tauri`  
**Fix:** Update Rust toolchain
```bash
rustup update stable
```

**Issue:** `npm ERR! Missing script: "tauri"`  
**Fix:** Install Tauri CLI
```bash
npm install -g @tauri-apps/cli@next
```

### Runtime Errors

**Issue:** "No USB drives detected"  
**Fix:**
- macOS: Check `/Volumes/` contains mounted drives
- Linux: Check `/media/$USER/` or `/run/media/$USER/`
- Windows: Verify drive letters appear in File Explorer

**Issue:** "Failed to load private key"  
**Fix:**
- Verify file exists and is readable
- Check format: 32 bytes raw binary or 64 hex characters
- Ensure no PEM headers (strip `-----BEGIN/END-----`)

**Issue:** "Failed to write object"  
**Fix:**
- Check USB drive has sufficient space
- Verify write permissions on USB mount point
- Try reformatting USB as exFAT or FAT32

**Issue:** "Signature verification failed"  
**Fix:**
- Ensure correct `public.key` matches signing `private.key`
- Check manifest was not modified after signing
- Regenerate keys if corruption suspected

---

## Development Notes

### Adding New Commands

1. **Define in Rust** (`src-tauri/src/usb_commands.rs`):
   ```rust
   #[tauri::command]
   fn cmd_new_feature(param: String) -> Result<String, String> {
       // Implementation
       Ok("success".to_string())
   }
   ```

2. **Register in main.rs**:
   ```rust
   .invoke_handler(tauri::generate_handler![
       // ... existing commands
       usb_commands::cmd_new_feature
   ])
   ```

3. **Call from JavaScript**:
   ```javascript
   import { invoke } from '@tauri-apps/api/core';
   
   const result = await invoke('cmd_new_feature', { param: 'value' });
   ```

### Testing

**Rust Unit Tests:**
```bash
cd src-tauri
cargo test
```

**Manual Integration Test:**
1. Prepare test USB with known files
2. Verify signature externally:
   ```bash
   # Extract and verify
   openssl dgst -sha512 -verify public.key \
     -signature aegis_repo/refs/heads/production.sig \
     aegis_repo/refs/heads/production.manifest
   ```

---

## Project Structure

```
aegis_installer/
├── package.json              # npm dependencies
├── .gitignore                # Excludes *.key, *.pem, private.*
├── src/
│   ├── index.html            # Main UI
│   ├── style.css             # Flat design styles
│   └── main.js               # Frontend logic (Web Crypto + Tauri invokes)
├── src-tauri/
│   ├── tauri.conf.json       # Tauri 2.x config (valid schema)
│   ├── Cargo.toml            # Rust dependencies
│   ├── build.rs              # Build script
│   └── src/
│       ├── main.rs           # Entry point, command registration
│       └── usb_commands.rs   # 6 Tauri commands implementation
└── README.md                 # This file
```

---

## License

Copyright © 2025 NoGap Project. All rights reserved.

**Security Notice:** This software is provided "as-is" for air-gapped deployment scenarios. The authors assume no liability for key management, physical security of USB devices, or cryptographic implementation vulnerabilities. Users are responsible for:
- Securing private keys
- Establishing physical chain-of-custody protocols
- Following organizational security policies
- Performing independent security audits

---

## Support

For issues or questions:
1. Review [Troubleshooting](#troubleshooting) section
2. Check Tauri 2.x documentation: https://v2.tauri.app/
3. Consult NoGap project maintainers

**⚠️ WARNING: Do not submit issues containing private keys, signatures, or other sensitive cryptographic material.**
