# OSTree-Lite Tauri Integration

## Overview

This document describes the Tauri command wrappers that expose the OSTree-lite functionality to the frontend application.

## Implementation

### Files Modified/Created

1. **Created**: `src-tauri/src/commands_ostree.rs` (~250 lines)
   - Four Tauri command wrappers for OSTree operations
   - Helper functions for path management and error conversion

2. **Modified**: `src-tauri/src/lib.rs`
   - Added `mod commands_ostree;` module declaration
   - Registered 4 new commands in invoke_handler

3. **Modified**: `src-tauri/Cargo.toml`
   - Added `dirs = "5.0"` dependency for home directory detection

## Available Commands

### 1. `cmd_scan_usb_repos()`

Scans for USB drives containing `aegis_repo/` directories.

**Frontend Usage:**
```typescript
const repos = await invoke<string[]>('cmd_scan_usb_repos');
// Returns: ["/Volumes/USB1", "/Volumes/USB2", ...]
```

**Returns:** Array of USB repository paths as strings

**Errors:** String describing any I/O or discovery errors

---

### 2. `cmd_preview_repo(repo_path: string)`

Previews a USB repository without importing it. Reads the manifest, verifies the signature, and returns metadata.

**Frontend Usage:**
```typescript
const preview = await invoke<ImportPreview>('cmd_preview_repo', {
  repoPath: '/Volumes/USB1'
});

interface ImportPreview {
  version: number;          // Manifest version number
  objects_count: number;    // Number of objects in manifest
  total_size: number;       // Total size in bytes
  repo_path: string;        // Path to USB repo
  verified: boolean;        // Signature verification result
  verification_msg: string; // Verification status message
}
```

**Parameters:**
- `repo_path`: Path to USB repository root (containing aegis_repo/)

**Returns:** ImportPreview object with repository metadata

**Errors:** String describing verification or I/O errors

---

### 3. `cmd_import_repo(repo_path: string)`

Imports a USB repository into the local repository. Performs full workflow:
1. Verifies manifest signature
2. Pulls all objects into local CAS
3. Installs manifest (updates production ref atomically)

**Frontend Usage:**
```typescript
const result = await invoke<ImportResult>('cmd_import_repo', {
  repoPath: '/Volumes/USB1'
});

interface ImportResult {
  ok: boolean;                      // Operation success
  message: string;                  // Human-readable status
  applied_version: string | null;   // Commit hash if successful
}
```

**Parameters:**
- `repo_path`: Path to USB repository root

**Returns:** ImportResult with operation outcome

**Errors:** String describing any failure during import

**Note:** Local repository is automatically created at `~/.nogap/local_repo` if it doesn't exist

---

### 4. `cmd_export_commit(commit_hash: string, target_usb: string, confirmed: boolean)`

Exports a commit from the local repository to a USB drive with RSA signing.

**Frontend Usage:**
```typescript
const result = await invoke<ImportResult>('cmd_export_commit', {
  commitHash: '3f84a9c2d8b1...',
  targetUsb: '/Volumes/USB1',
  confirmed: true  // Must be true
});
```

**Parameters:**
- `commit_hash`: Commit hash to export
- `target_usb`: Path to target USB drive root
- `confirmed`: User confirmation flag (must be true)

**Returns:** ImportResult with export outcome

**Errors:** 
- "User confirmation required" if confirmed=false
- "Target USB must contain aegis_repo/" if target invalid
- Other errors from export operations

**Security:** Currently generates a new RSA keypair for each export. In production, this should load from secure storage (e.g., system keychain).

---

## Local Repository Path

All commands use `~/.nogap/local_repo` as the local repository path by default.

The directory structure is:
```
~/.nogap/local_repo/
├── objects/
│   ├── 3f/
│   │   └── 84a9c2d8b1...
│   └── ab/
│       └── cdef123456...
└── refs/
    └── heads/
        └── production  (contains commit hash)
```

## Error Handling

All commands return `Result<T, String>` to match Tauri's error handling pattern. Internal `OstreeError` instances are converted to human-readable strings via the `err_to_string()` helper function.

## Testing

To test the integration:

1. **Build the application:**
   ```bash
   cd nogap_dashboard/src-tauri
   cargo build
   ```

2. **Run in development mode:**
   ```bash
   cd nogap_dashboard
   npm run tauri dev
   ```

3. **Test from frontend console:**
   ```javascript
   // Scan for repos
   const repos = await window.__TAURI__.invoke('cmd_scan_usb_repos');
   
   // Preview a repo
   const preview = await window.__TAURI__.invoke('cmd_preview_repo', {
     repoPath: repos[0]
   });
   
   // Import a repo
   const result = await window.__TAURI__.invoke('cmd_import_repo', {
     repoPath: repos[0]
   });
   ```

## Known Limitations

1. **RSA Key Management**: The `cmd_export_commit` command currently generates a new RSA keypair for each export. In production, this should:
   - Load a persistent signing key from secure storage
   - Use the system keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
   - Potentially require user authentication before signing

2. **No Progress Callbacks**: Import/export operations don't currently provide progress updates. Consider adding progress events for large transfers.

3. **No Concurrent Operations**: Commands should not be called concurrently. The frontend should ensure only one repository operation is active at a time.

## Future Enhancements

- [ ] Persistent RSA key storage in system keychain
- [ ] Progress callbacks for long-running operations
- [ ] Concurrent operation detection/locking
- [ ] Detailed error types instead of strings
- [ ] Commit history/diff viewing commands
- [ ] Repository garbage collection command
- [ ] Configuration for custom local repo paths

## Integration Complete ✅

All four OSTree-lite commands are now available to the frontend via Tauri IPC. The implementation passes compilation and follows the existing Tauri command patterns in the codebase.
