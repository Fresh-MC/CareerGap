# DASHBOARD.md

## Dashboard UI Flow

NoGap Dashboard uses a **two-stage entry flow**:

1. **USB Manager (`usb.html`)**: Entry point for USB repository scanning and preview
2. **Main Dashboard (`index.html`)**: Policy audit, remediation, rollback, and reporting interface

### USB Manager → Main Dashboard Flow

- **Entry**: Application starts at `usb.html`
- **Scanning**: Automatically calls `cmd_list_all_drives()` and `cmd_scan_usb_repos()` on page load
- **Selection**: User clicks a drive → calls `cmd_preview_repo(drivePath)` → displays preview with signature verification status
- **Transition**: "Continue to Dashboard →" button navigates to `index.html` (main dashboard)

The USB stage is **optional** — users can skip directly to the main dashboard without importing. Import operations are handled via the main dashboard's "🔌 USB Manager" button, which navigates back to `usb.html`.

---

## Tauri IPC Commands

### USB Operations (`commands_ostree.rs`)

| Command | Purpose | Returns |
|---------|---------|---------|
| `cmd_list_all_drives()` | Lists all mounted drives (Windows: D-Z:, Linux: /media, /run/media, macOS: /Volumes) | `Vec<String>` |
| `cmd_scan_usb_repos()` | Discovers drives with `aegis_repo/` directories using `discover_usb_repos()` | `Vec<String>` |
| `cmd_preview_repo(repo_path)` | Reads manifest, verifies Ed25519 signature, calculates object size — **no import** | `ImportPreview` |
| `cmd_import_repo(repo_path)` | Verifies signature → `pull_objects()` → `install_manifest()` → updates production ref | `ImportResult` |
| `cmd_export_commit(hash, target, confirmed)` | Exports commit from `~/.nogap/local_repo` to target USB with RSA-2048 signing | `ImportResult` |

### Policy Operations (`lib.rs`)

| Command | Purpose | Returns |
|---------|---------|---------|
| `load_policies()` | Loads simplified Policy structs (id, title, description, platform, severity, status) | `Vec<Policy>` |
| `audit_policy(policy_id)` | Audits single policy using platform-specific check (registry, service, file, sysctl, SSH) | `AuditResult` |
| `audit_all_policies()` | Iterates all policies, calls platform audit functions | `Vec<AuditResult>` |
| `remediate_policy(policy_id)` | Applies fix with privilege check, sets registry/service/file/sysctl/SSH config | `RemediateResult` |
| `remediate_all_policies()` | Remediates all non-compliant policies, returns reboot_required flags | `Vec<RemediateResult>` |
| `rollback_policy(policy_id)` | Restores previous state using `nogap_core::engine::rollback()` | `RollbackResult` |
| `rollback_all()` | Rolls back all modified policies, returns success/failure per policy | `Vec<RollbackResult>` |

### Privilege & Reporting

| Command | Purpose | Returns |
|---------|---------|---------|
| `cmd_check_elevation()` | Checks if process is elevated (Windows: TokenElevation, Linux: UID 0) | `bool` |
| `cmd_require_elevation()` | Enforces admin/root privilege, returns error if not elevated | `Result<(), String>` |
| `generate_html_report(...)` | Generates HTML compliance report with policy stats, platform scores, timestamp | `String` (file path) |
| `export_pdf(html_path)` | Returns HTML path for browser print-to-PDF dialog | `String` |
| `generate_csv_report(...)` | Exports audit results as CSV with policy_id, title, status, compliance stats | `String` (file path) |

---

## USB Scanning Logic

### Discovery (`discover_usb_repos()`)

**Platform-specific drive enumeration**:
- **Windows**: `GetDriveTypeW()` checks D-Z: for DRIVE_REMOVABLE (2) or DRIVE_FIXED (3)
- **Linux**: Scans `/media` and `/run/media/[user]/`
- **macOS**: Scans `/Volumes`

**Repository Detection**: Filters drives with `aegis_repo/` directory containing `refs/heads/production` and `objects/` structure.

### Preview Without Import (`cmd_preview_repo()`)

1. **Validate**: Check `aegis_repo/` exists at `repo_path`
2. **Read Manifest**: `read_manifest(aegis_repo)` → parses `refs/heads/production` JSON
3. **Read Signature**: Loads `refs/heads/production.sig`
4. **Verify**: `verify_manifest(manifest_bytes, sig_bytes)` → Ed25519 signature verification
5. **Calculate Size**: Iterates `manifest.objects`, reads file sizes from USB `objects/<first2>/<remaining62>`
6. **Return**: `ImportPreview` with version, object count, size, verified flag, and verification message

**Key Feature**: Signature verification occurs **before** any import, preventing tampered USB sticks from polluting the local repo.

---

## Repo Preview Flow

**UI States in `usb.html`**:

1. **Loading**: Displays spinner with "Scanning for drives..."
2. **Drive List**: Renders drive items with:
   - ✅ icon + "NoGap Repo Found" badge (green) if `aegis_repo/` present
   - 💾 icon + "empty" badge (red) if no repo
3. **Drive Selection**: Click drive → calls `cmd_preview_repo()` → displays:
   - **Success**: Repository path, version, object count, size, signature status (✅ Verified or ❌ Failed)
   - **Error**: Red error box with failure reason (e.g., "No NoGap repository found")
4. **Continue**: Navigation to `index.html` (import not required)

**Preview Display** (success case):
```
Repository: D:\aegis_repo
Version: 42
Objects: 1200
Total Size: 256 MB
Signature: ✅ Verified
```

**Signature failure** shows red "❌ Signature verification failed: [error details]" — user can still view preview but import will fail.

---

## Import Workflow

**Triggered by**: Future feature or manual invoke from main dashboard (not yet wired in UI)

**Steps (`cmd_import_repo()`)**:

1. **Validate**: Verify `aegis_repo/` exists at `repo_path`
2. **Read & Verify**: Load manifest + signature → `verify_manifest()` (blocks on signature failure)
3. **Pull Objects**: `pull_objects(usb_repo, manifest, local_repo, progress_callback)`
   - Streams objects from USB `aegis_repo/objects/<hash>` to `~/.nogap/local_repo/objects/<hash>`
   - SHA256 verification for each object
   - Atomic writes with temporary `.tmp` files
4. **Install Manifest**: `install_manifest(manifest, local_repo)`
   - Updates `refs/heads/production` with new commit hash
   - Backup: renames existing `production` to `production.prev`
   - Atomic: write to `production.tmp` → rename to `production`
5. **Return**: `ImportResult` with success flag, message, and applied version

**Critical**: Step 2 signature verification **must pass** before any objects are pulled. This prevents supply-chain attacks where malicious USBs inject unsigned commits.

---

## UI States

### Loading State
- **Trigger**: Any async Tauri command (audit, remediate, import)
- **Display**: Centered spinner overlay with "Processing..." message
- **Visibility**: `showLoading(true)` sets `#loading` to `display: flex`, `showLoading(false)` hides

### Error Handling
- **Tauri Error**: Caught in `catch` blocks → `showNotification(message, "error")`
- **Notification Display**: Red toast with 3-second auto-dismiss
- **Preview Errors**: Inline red error box in `#preview` div (e.g., "Failed to preview repository: [error]")

### Signature Failure State
- **USB Preview**: Shows "❌ Signature verification failed: [reason]" in red
- **Import Blocked**: `cmd_import_repo()` returns `Err(...)` if verification fails, preventing object pull
- **User Feedback**: Error notification + preview area displays failure details

### Drive Detection States
- **No Drives**: Empty state with 💾 icon and "No drives detected"
- **No Repos**: All drives show "empty" badge — preview area shows "No NoGap repository found" when clicked
- **Mixed**: Drives with repos show green "NoGap Repo Found", others show red "empty"

---

## Privilege Enforcement UI

### Elevation Check (`elev_checks.rs`)

**Windows**: `GetTokenInformation(TokenElevation)` → checks `TokenIsElevated` flag  
**Linux**: `Uid::effective().is_root()` → checks UID == 0

### UI Integration

1. **Pre-Action Check**: Remediation/rollback buttons call `cmd_check_elevation()` before operation
2. **Error Display**: If not elevated, shows notification: "Administrative privileges required for this operation"
3. **Backend Enforcement**: `cmd_require_elevation()` called in `remediate_policy()`, `remediate_all_policies()`, `rollback_policy()`, `rollback_all()`
4. **Fail-Fast**: If `ensure_admin()` returns `Err(PrivilegeError::NotElevated)`, operation aborts immediately with error message

**No UI Prompt**: Dashboard does **not** auto-elevate. User must manually restart with admin/sudo.

---

## Strengths/Weaknesses of UX

### Strengths

1. **Air-Gapped Preview**: Signature verification occurs **before** import, preventing malicious USB sticks from entering the system
2. **Clear Visual Feedback**: Drive badges (✅ repo found, 💾 empty), signature status (✅/❌), loading spinners, color-coded notifications
3. **Separation of Concerns**: USB manager isolated from policy dashboard, reducing cognitive load
4. **Non-Blocking Navigation**: "Continue to Dashboard" works without import, allowing users to work with existing local repo
5. **Rollback Safety**: Individual and bulk rollback with confirmation dialogs, detailed result modals

### Weaknesses

1. **No Auto-Import**: USB preview does not trigger import — requires future "Import" button or manual command
2. **No Elevation Prompt**: Dashboard does not auto-request admin privileges, requiring manual restart (Windows: "Run as Administrator", Linux: `sudo`)
3. **Limited USB Write Support**: Export commit feature (`cmd_export_commit()`) not wired in UI, only accessible via IPC
4. **Single-Threaded Import**: No progress bar during `pull_objects()` — UI freezes on large imports (1000+ objects)
5. **No Diff Preview**: USB preview shows object count/size but not which policies changed vs. current local repo
6. **Error Recovery**: Signature failure blocks import but provides no mitigation steps (e.g., "Verify USB source" guidance)

---

**Stage 2 complete.**
