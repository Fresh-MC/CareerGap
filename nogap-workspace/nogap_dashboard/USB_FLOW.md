# USB-First Startup Flow

## Overview

The NoGap Dashboard now implements a **USB-first flow** where the application starts with the USB repository manager (`usb.html`) instead of the main dashboard (`index.html`). This provides a streamlined workflow for air-gapped environments where USB repositories are the primary distribution method.

## Architecture

### Startup Sequence

1. **Application Launch** → Tauri loads `usb.html` (configured in `tauri.conf.json`)
2. **Auto-Scan** → Automatically scans for USB repositories on load (500ms delay)
3. **Auto-Preview** → If repositories found, automatically previews the first one
4. **Conditional Redirect**:
   - **Valid Signature**: Shows 3-second countdown → redirects to `index.html`
   - **Invalid Signature**: Shows error, provides Retry/Skip buttons
   - **No USB**: Shows "No USB detected" message with Retry/Skip buttons

### Navigation Flow

```
usb.html (USB Manager)
  ↓ (auto-scan + auto-preview)
  ↓ (if valid signature)
  ↓ (3-second countdown)
  → index.html (Dashboard)
  ← (🔌 USB Manager button)
```

## Configuration

### Tauri Configuration (`tauri.conf.json`)

```json
{
  "app": {
    "windows": [
      {
        "url": "usb.html"  // ← Startup page
      }
    ]
  }
}
```

### Frontend Files

1. **usb.html** - USB repository manager (entry point)
   - Auto-scan functionality
   - Repository preview with signature verification
   - Import workflow
   - Navigation to dashboard

2. **index.html** - Main policy dashboard
   - Policy management UI
   - "🔌 USB Manager" button to return to USB flow

3. **main.js** - Dashboard event handlers
   - USB Manager button click handler

## Features

### Automatic Flow

- **Auto-Scan**: Executes `cmd_scan_usb_repos()` on DOMContentLoaded
- **Auto-Preview**: Calls `cmd_preview_repo()` on first discovered repository
- **Auto-Redirect**: 3-second countdown if signature is valid

### Manual Controls

- **Retry Scan**: Re-runs USB repository scan
- **Skip to Dashboard**: Navigate directly to main dashboard
- **Import Now**: Manual import trigger (stops countdown)

### Error Handling

- No USB detected → Shows retry/skip options
- Invalid signature → Shows error message with retry/skip
- Import failure → Displays error with retry option

## User Experience

### Success Path (Valid USB)
1. App opens → shows "Scanning for USB repositories..."
2. USB found → shows preview with green checkmark
3. Countdown: "Redirecting to dashboard in 3 seconds..."
4. Automatic redirect to main dashboard

### Error Path (No USB or Invalid)
1. App opens → shows "Scanning for USB repositories..."
2. Error detected → shows error message
3. User options:
   - Click "Retry Scan" to search again
   - Click "Skip to Dashboard" to proceed without import

### Manual Import
1. Valid USB detected → preview shown
2. User clicks "Import Now" before countdown completes
3. Import executes immediately → shows success/error message
4. On success → proceeds to dashboard

## Technical Implementation

### Tauri Commands Used

```rust
cmd_scan_usb_repos()           // Discover USB repositories
cmd_preview_repo(repo_path)    // Preview and verify manifest
cmd_import_repo(repo_path)     // Import repository to local storage
```

### JavaScript Navigation

```javascript
// Dashboard → USB Manager
document.getElementById("usb-manager-btn").addEventListener("click", () => {
  window.location.href = "usb.html";
});

// USB Manager → Dashboard (auto-redirect)
function navigateToDashboard() {
  window.location.href = "index.html";
}
```

## Testing

### Development Mode

```bash
cd nogap_dashboard
npm run tauri dev
```

### Expected Behavior

1. **No USB Connected**:
   - Shows "No USB Repository Detected"
   - Retry/Skip buttons visible
   - No countdown

2. **Valid USB Connected**:
   - Shows repository preview
   - Green checkmark for signature
   - 3-second countdown
   - Auto-redirect to dashboard

3. **Invalid Signature**:
   - Shows repository preview
   - Red X for signature
   - Error message displayed
   - Retry/Skip buttons (no countdown)

4. **Dashboard Navigation**:
   - Dashboard has "🔌 USB Manager" button
   - Clicking returns to usb.html
   - Auto-scan executes again

## Design Decisions

### Why USB-First?

Air-gapped environments require USB repositories for updates and distribution. Starting with USB management ensures users can immediately:
- Check for new repository updates
- Import security policy updates
- Verify cryptographic signatures
- Proceed to dashboard with latest data

### Why Auto-Redirect?

Reduces friction for standard workflow:
- Most users will insert USB, wait for import, then use dashboard
- Auto-redirect eliminates unnecessary click
- 3-second countdown provides time to cancel if needed
- Manual import option available for those who want control

### Why Keep Dashboard Separate?

- `index.html` remains full-featured policy management UI
- USB manager is optional - users can skip to dashboard
- Separation of concerns: USB import vs. policy management
- Allows future enhancements to either page independently

## Files Modified

1. **nogap_dashboard/src-tauri/tauri.conf.json**
   - Added `"url": "usb.html"` to window configuration

2. **nogap_dashboard/src/usb.html** (NEW)
   - Complete USB-first flow implementation
   - Auto-scan, auto-preview, auto-redirect logic

3. **nogap_dashboard/src/index.html**
   - Added "🔌 USB Manager" button to controls

4. **nogap_dashboard/src/main.js**
   - Added event handler for USB Manager button

## Future Enhancements

- [ ] Settings to disable auto-redirect (for testing/debugging)
- [ ] Multiple repository selection (currently auto-previews first)
- [ ] Repository comparison (diff between local and USB)
- [ ] Import history log
- [ ] Background import progress indicator
- [ ] Notification when new USB inserted while dashboard open
