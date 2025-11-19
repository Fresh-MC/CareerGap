# NoGap Dashboard - Release Guide

## Version 1.0.0

### Overview
NoGap Dashboard is a native desktop application for auditing and remediating Windows and Linux security policies. Built with Tauri 2.x, it provides a fast, secure, and cross-platform solution for security policy management.

---

## System Requirements

### macOS
- **OS Version**: macOS 10.15 (Catalina) or later
- **Architecture**: Intel (x86_64) or Apple Silicon (arm64)
- **Permissions**: Administrator privileges for remediation operations

### Windows
- **OS Version**: Windows 10 (1809) or later
- **Architecture**: x86_64
- **Permissions**: Administrator privileges for remediation operations
- **Dependencies**: Microsoft Visual C++ Redistributable

### Linux
- **Distributions**: Ubuntu 20.04+, Debian 11+, Fedora 35+, or equivalent
- **Architecture**: x86_64
- **Permissions**: Root privileges for remediation operations
- **Dependencies**: 
  - GTK 3.24+
  - WebKit2GTK 4.1+
  - systemd (for service management)

---

## Building from Source

### Prerequisites
1. **Node.js**: Version 18 or later
2. **Rust**: Latest stable version (1.70+)
3. **Tauri CLI**: Installed via npm
4. **Platform-specific tools**:
   - macOS: Xcode Command Line Tools
   - Windows: Visual Studio 2019+ with C++ tools
   - Linux: Build essentials, GTK/WebKit2GTK development packages

### Build Commands

#### Development Build
```bash
cd nogap-workspace/nogap_dashboard
npm install
npm run dev
```

#### Production Build (Current Platform)
```bash
npm run build
```

#### Platform-Specific Builds
```bash
# macOS Universal Binary
npm run build:macos

# Windows (cross-compile from Linux/macOS requires MinGW)
npm run build:windows

# Linux
npm run build:linux
```

### Build Outputs
Bundles are created in: `src-tauri/target/release/bundle/`

**macOS**:
- `.dmg` - Disk image installer
- `.app` - Application bundle

**Windows**:
- `.msi` - Windows Installer package
- `.exe` - Portable executable (NSIS installer)

**Linux**:
- `.deb` - Debian/Ubuntu package
- `.AppImage` - Universal Linux executable
- `.rpm` - Red Hat/Fedora package (if configured)

---

## Installation

### macOS
1. Open the `.dmg` file
2. Drag "NoGap Dashboard" to Applications
3. Launch from Applications folder
4. If security warning appears, go to System Preferences → Security & Privacy → Open Anyway

### Windows
1. Run the `.msi` or `.exe` installer
2. Follow installation wizard
3. Launch from Start Menu or Desktop shortcut
4. Run as Administrator for remediation features

### Linux

#### Debian/Ubuntu (.deb)
```bash
sudo dpkg -i nogap-dashboard_1.0.0_amd64.deb
sudo apt-get install -f  # Fix dependencies if needed
```

#### AppImage
```bash
chmod +x nogap-dashboard_1.0.0_amd64.AppImage
./nogap-dashboard_1.0.0_amd64.AppImage
```

---

## Feature Overview

### Stage 6 Implementation (Current)
✅ **Full Audit System**:
- Registry key checks (Windows)
- Local policy checks (Windows/Linux)
- Service status monitoring (Windows/Linux)
- File permission auditing (Linux)
- sysctl parameter validation (Linux)
- SSH configuration auditing (Linux)

✅ **Complete Remediation System**:
- Registry key modifications (Windows)
- Local policy enforcement (Windows/Linux)
- Service control (start/stop/disable)
- File permission corrections (Linux)
- sysctl parameter updates (Linux)
- SSH configuration hardening (Linux)

✅ **Platform Intelligence**:
- Automatic OS detection
- Platform-specific policy filtering
- Cross-platform compatibility checks

✅ **Policy Management**:
- 1600+ Windows and Linux security policies
- YAML-based policy definitions
- Severity-based filtering (Critical, High, Medium, Low)
- Reversibility indicators

---

## Usage

### Dashboard Interface
1. **Load Policies**: Automatically loads on startup from bundled `policies.yaml`
2. **Filter Policies**: Use dropdowns to filter by platform and severity
3. **Search**: Real-time search across policy titles and descriptions
4. **Audit**: Click "Audit" on individual policies or "Audit All" for complete scan
5. **Remediate**: Click "Remediate" to fix non-compliant policies (requires elevated privileges)

### Privilege Requirements

#### Windows
- Run as Administrator: Right-click → "Run as administrator"
- Required for: Registry edits, service control, local policy changes

#### Linux
- Run with sudo: `sudo ./nogap-dashboard`
- Or use pkexec: `pkexec ./nogap-dashboard`
- Required for: File permission changes, sysctl modifications, SSH config updates, service control

#### macOS
- Run as Administrator: Launch normally, will prompt for admin password when needed
- Required for: System configuration changes, service control

---

## Troubleshooting

### "Policy file not found"
- Ensure `policies.yaml` is in the correct resource location
- For development: `nogap-workspace/nogap_core/policies.yaml`
- For production: Bundled automatically via `tauri.conf.json` resources

### "Permission denied" during remediation
- Windows: Restart application as Administrator
- Linux: Run with `sudo` or `pkexec`
- macOS: Ensure you approve admin password prompts

### Audit shows "Error" status
- Check that required system tools are available:
  - Windows: `sc.exe`, `reg.exe`
  - Linux: `systemctl`, `sysctl`, `chmod`
- Verify policy YAML syntax is correct
- Check application logs for detailed error messages

### Build failures
- Ensure all dependencies are installed: `npm install`
- Update Rust: `rustup update stable`
- Clear build cache: `cargo clean` in `src-tauri/`
- Check Tauri CLI version: `npm list @tauri-apps/cli`

---

## Security Considerations

1. **Elevated Privileges**: Remediation requires admin/root access - use responsibly
2. **Backup**: Always backup system configuration before mass remediation
3. **Testing**: Test policies on non-production systems first
4. **Reversibility**: Check "Reversible" column before applying changes
5. **Audit First**: Always audit before remediating to understand impact

---

## Development Roadmap

### Completed (v1.0.0)
- ✅ Tauri 2.x native application
- ✅ Full YAML policy loading and validation
- ✅ Platform-specific audit implementations
- ✅ Platform-specific remediation implementations
- ✅ Dashboard UI with filtering and search
- ✅ Cross-platform build configuration

### Future Enhancements
- 🔄 Windows Registry API integration (replace sc.exe)
- 🔄 Windows Local Policy API (replace PowerShell calls)
- 🔄 Detailed audit logs and reporting
- 🔄 Policy export/import functionality
- 🔄 Custom policy creation
- 🔄 Scheduled audit tasks
- 🔄 Email notifications for critical findings

---

## License
See main project LICENSE file

## Support
For issues, feature requests, or contributions, visit the project repository.

---

**Build Date**: November 2025  
**Version**: 1.0.0  
**Platform**: macOS, Windows, Linux
