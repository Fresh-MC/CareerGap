# NoGap USB Authority Creator

A complete web application that prepares USB drives as valid AegisPack repositories for NoGap's OSTree-lite engine. This tool creates content-addressable storage (CAS) repositories with Ed25519-signed manifests for secure, offline policy distribution.

## 🎯 Overview

This application generates USB repositories that are **fully compatible** with NoGap's OSTree-lite engine, including:
- Proper directory structure (`aegis_repo/`)
- Content-addressable object storage (SHA256-sharded)
- Canonical JSON manifests
- Ed25519 digital signatures
- Automatic recognition by `discover_usb_repos()`

## 📁 Project Structure

```
usb_authority_creator/
├── index.html              # Main UI
├── style.css               # Styling
├── main.js                 # Core logic (manifest generation, hashing)
├── tauri_commands.rs       # Example Tauri backend handlers
└── README.md               # This file

nogap_signer/               # Standalone signing CLI
├── Cargo.toml
└── src/main.rs
```

## 🚀 Quick Start

### 1. Build the Signer CLI

First, build the Ed25519 signing tool:

```bash
cd nogap_signer
cargo build --release
```

The binary will be at: `target/release/nogap_signer`

### 2. Generate Keypair

Generate an Ed25519 keypair for signing:

```bash
./target/release/nogap_signer keygen --private private.key --public public.key
```

**Important:** Keep `private.key` secure. Distribute `public.key` to all NoGap installations via `configs/trusted_keys.json`.

### 3. Set Up Tauri Project (Optional)

If using Tauri for the web interface:

a. Create a new Tauri project or use an existing one:
```bash
npm create tauri-app@latest
```

b. Copy `tauri_commands.rs` handlers to your Tauri project's `src-tauri/src/main.rs`

c. Register the commands in your Tauri builder:
```rust
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
```

d. Copy `index.html`, `style.css`, and `main.js` to your Tauri `src/` directory

### 4. Run the Application

**Option A: With Tauri**
```bash
npm run tauri dev
```

**Option B: Standalone (without filesystem access)**
```bash
# Serve with any HTTP server
python3 -m http.server 8000
# Open http://localhost:8000
```

Note: Standalone mode won't have filesystem access. Use Tauri for full functionality.

## 📖 Usage Instructions

### Step 1: Select USB Drive

1. Enter the USB mount point manually:
   - **Linux**: `/media/username/USBDRIVE` or `/run/media/username/USBDRIVE`
   - **macOS**: `/Volumes/USBDRIVE`
   - **Windows**: `D:\` or `E:\`

2. Or click "Browse" to select the directory (Tauri only)

### Step 2: Upload Files

1. Click "Select Files" or drag-and-drop files into the upload zone
2. Upload policy files, binaries, or configuration files
3. Each file is automatically hashed (SHA256)
4. Files appear in the list with their hash and size

### Step 3: Configure Version

1. Enter the repository version (e.g., `1.0.0`, `2023-11-24`)
2. This version is stored in the manifest

### Step 4: Prepare USB

1. Click "🚀 Prepare USB AegisPack"
2. The application will:
   - Create `aegis_repo/` directory structure
   - Copy files to content-addressable storage (`objects/xx/hash`)
   - Generate canonical JSON manifest
   - Sign the manifest with Ed25519
   - Write signature to USB

3. Wait for the success modal: "✅ USB-A Ready!"

## 🗂️ Generated USB Structure

After preparation, your USB will have:

```
<USB_ROOT>/
└── aegis_repo/
    ├── objects/
    │   ├── 01/
    │   │   └── 23456789abcdef...  (sharded objects)
    │   ├── 02/
    │   │   └── 34567890abcdef...
    │   └── ...
    └── refs/
        └── heads/
            ├── production.manifest  (canonical JSON)
            └── production.sig       (Ed25519 signature, hex-encoded)
```

### Manifest Format

`production.manifest` contains:
```json
{
  "objects": [
    {
      "hash": "0123456789abcdef...",
      "size": 12345
    }
  ],
  "version": "1.0.0"
}
```

**Important:** 
- Keys are sorted alphabetically
- Objects array is sorted by hash
- No extra whitespace (canonical JSON)
- This ensures consistent signature verification

### Signature Format

`production.sig` contains:
- Raw Ed25519 signature (64 bytes)
- Hex-encoded (128 characters)
- Verifiable with the public key

## 🔐 Security Model

### Signing Process

1. **Canonical JSON**: Manifest is generated with sorted keys
2. **Ed25519 Signing**: 
   ```bash
   nogap-signer sign --input production.manifest --output production.sig --key private.key
   ```
3. **Signature Storage**: Raw 64-byte signature, hex-encoded

### Verification (by NoGap)

NoGap's OSTree-lite engine verifies:
1. Discovers USB via `discover_usb_repos()`
2. Reads manifest via `read_manifest()`
3. Loads signature from `production.sig`
4. Verifies with `ed25519_dalek` using trusted public keys
5. Checks each object's SHA256 hash during `pull_objects()`

### Key Distribution

Add the public key to all NoGap installations:

**`configs/trusted_keys.json`:**
```json
{
  "keys": [
    {
      "id": "authority-2024",
      "key": "<base64-encoded-32-bytes>"
    }
  ]
}
```

## 🛠️ NoGap Signer CLI

### Commands

**Generate Keypair:**
```bash
nogap-signer keygen --private private.key --public public.key
```

**Sign Manifest:**
```bash
nogap-signer sign --input production.manifest --output production.sig --key private.key
```

**Verify Signature:**
```bash
nogap-signer verify --manifest production.manifest --signature production.sig --key public.key
```

## 🔗 Integration with NoGap OSTree-lite

This tool creates repositories that are **automatically recognized** by your Rust codebase:

### Discovery
```rust
// From ostree_lite.rs
let usb_repos = discover_usb_repos()?;
// Finds: /Volumes/USBDRIVE/aegis_repo/
```

### Manifest Reading
```rust
let manifest_bytes = read_manifest(&repo_path)?;
let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;
```

### Signature Verification
```rust
let signature = fs::read(repo_path.join("refs/heads/production.sig"))?;
verify_manifest(&manifest_bytes, &signature, &trusted_keys)?;
```

### Object Pulling
```rust
for obj in manifest.objects {
    let object_path = object_path_for_hash(&obj.hash);
    pull_objects(&repo_path, &local_cas, &[obj])?;
}
```

## 📋 Requirements

### Runtime Requirements
- **Tauri**: For filesystem access and native dialogs
- **nogap_signer**: For Ed25519 signing (must be in PATH)
- **Private key**: `private.key` for signing operations

### Browser Support
- Modern browsers with Web Crypto API (for SHA256)
- File API support (for drag-and-drop)

## 🚨 Troubleshooting

### "Tauri API not available"
**Solution:** Run the app within a Tauri context, not as a standalone HTML file.

### "Private key not found"
**Solution:** 
```bash
cd nogap_signer
cargo run --release -- keygen
```
Place `private.key` in the working directory or common locations.

### "Signing failed"
**Solution:** Ensure `nogap_signer` is in your PATH:
```bash
export PATH=$PATH:/path/to/nogap_signer/target/release
```

### NoGap doesn't recognize the USB
**Checklist:**
- Directory is named exactly `aegis_repo`
- Manifest is at `refs/heads/production.manifest`
- Signature is at `refs/heads/production.sig`
- Objects are in `objects/xx/<remaining-hash>`
- Public key is in NoGap's `trusted_keys.json`

## 🎨 Customization

### Change Manifest Structure

Edit `main.js`:
```javascript
function generateCanonicalManifest() {
    const manifest = {
        objects: objects,
        version: state.version,
        // Add custom fields here
        timestamp: Date.now(),
        author: "MyOrg"
    };
    return JSON.stringify(manifest, Object.keys(manifest).sort(), 2);
}
```

### Change Shard Size

Objects are sharded by first 2 hex characters. To change:

Edit `main.js`:
```javascript
const shardDir = hash.substring(0, 3); // Use first 3 chars
const remaining = hash.substring(3);
```

Update your Rust code accordingly in `ostree_lite.rs`.

## 📦 Distribution

### For End Users

1. Build a Tauri app:
   ```bash
   npm run tauri build
   ```

2. Distribute the installer with:
   - The Tauri app binary
   - `nogap_signer` binary
   - `private.key` (secure delivery)
   - Instructions for generating AegisPacks

### For Developers

1. Clone the NoGap repository
2. Navigate to `usb_authority_creator/`
3. Follow the Quick Start guide above

## 🔒 Security Best Practices

1. **Private Key Storage**:
   - Store `private.key` in a secure location
   - Never commit to version control
   - Use hardware security modules (HSM) for production

2. **Key Rotation**:
   - Generate new keypairs periodically
   - Distribute new public keys to all installations
   - Maintain backward compatibility with old keys

3. **Access Control**:
   - Limit who can create AegisPacks
   - Audit all USB repository creations
   - Use multi-signature schemes for critical updates

4. **Verification**:
   - Always verify created USBs before distribution
   - Test with NoGap's verification tools
   - Check hash integrity of all objects

## 📝 License

Part of the NoGap project.

## 🤝 Contributing

This tool is designed to integrate seamlessly with NoGap's OSTree-lite engine. When modifying:

1. Maintain canonical JSON formatting
2. Preserve Ed25519 signature compatibility
3. Keep directory structure consistent
4. Update both frontend and backend together

## 📚 Additional Resources

- **NoGap Core Documentation**: See `CORE.md` for OSTree-lite details
- **Ed25519 Specification**: [RFC 8032](https://tools.ietf.org/html/rfc8032)
- **Tauri Documentation**: [tauri.app](https://tauri.app)
- **Web Crypto API**: [MDN Web Docs](https://developer.mozilla.org/en-US/docs/Web/API/Web_Crypto_API)

## ✅ Verification Checklist

Before distributing a USB:

- [ ] Manifest contains all objects
- [ ] All objects exist in `objects/` with correct hashes
- [ ] Manifest is canonical JSON (sorted keys)
- [ ] Signature file exists and is 128 hex characters
- [ ] NoGap recognizes the repository with `discover_usb_repos()`
- [ ] Signature verifies with `verify_manifest()`
- [ ] Objects can be pulled with `pull_objects()`

---

**Ready to create your first AegisPack? Follow the Quick Start guide above!**
