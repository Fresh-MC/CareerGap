# NoGap Dashboard

A native desktop application for auditing and remediating Windows and Linux security policies. Built with Tauri 2.x for maximum performance and security.

![Version](https://img.shields.io/badge/version-1.0.0-blue)
![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)

---

## 🚀 Features

### Stage 6 - Complete Audit & Remediation System
- ✅ **1600+ Security Policies** - Comprehensive Windows and Linux policy coverage
- ✅ **Platform-Specific Auditing** - Registry, local policies, services, file permissions, sysctl, SSH config
- ✅ **One-Click Remediation** - Automated security policy enforcement
- ✅ **Smart Filtering** - Filter by platform, severity, and search
- ✅ **Native Performance** - Rust backend with HTML/CSS/JS frontend
- ✅ **Cross-Platform** - Single codebase for macOS, Windows, and Linux

### Audit Capabilities
- **Windows**: Registry keys, local policies, service status
- **Linux**: File permissions, sysctl parameters, SSH configuration, systemd services
- **Both**: Service monitoring, configuration validation

### Remediation Capabilities
- **Windows**: Registry modifications, local policy enforcement, service control
- **Linux**: File permission fixes, sysctl updates, SSH hardening, service management
- **Both**: Automated compliance enforcement with rollback support

---

## 📦 Installation

### macOS (Apple Silicon/Intel)

**Option 1: DMG Installer** (Recommended)
1. Download `NoGap Dashboard_1.0.0_aarch64.dmg` from releases
2. Open the DMG file
3. Drag "NoGap Dashboard" to Applications folder
4. Launch from Applications
5. If security warning appears: System Preferences → Security & Privacy → Open Anyway

**Option 2: .app Bundle**
1. Copy `NoGap Dashboard.app` to Applications folder
2. Run: `xattr -cr "/Applications/NoGap Dashboard.app"` to remove quarantine
3. Launch from Applications

**Size**: ~8.7 MB (app bundle), ~3 MB (DMG)

### Windows

**MSI Installer** (Recommended)
1. Download `nogap-dashboard_1.0.0_x64.msi` from releases
2. Run installer as Administrator
3. Follow installation wizard
4. Launch from Start Menu

**Portable EXE**
1. Download `nogap-dashboard_1.0.0_x64-setup.exe`
2. Run as Administrator for full functionality
3. No installation required

**Requirements**: Windows 10 (1809+), Microsoft Visual C++ Redistributable

### Linux

**Debian/Ubuntu (.deb)**
```bash
sudo dpkg -i nogap-dashboard_1.0.0_amd64.deb
sudo apt-get install -f
nogap-dashboard
```

**AppImage** (Universal)
```bash
chmod +x nogap-dashboard_1.0.0_amd64.AppImage
./nogap-dashboard_1.0.0_amd64.AppImage
```

**Red Hat/Fedora (.rpm)**
```bash
sudo rpm -i nogap-dashboard-1.0.0.x86_64.rpm
nogap-dashboard
```

**Requirements**: GTK 3.24+, WebKit2GTK 4.1+, systemd

---

## 🛠️ Usage

### Dashboard Interface

1. **Load Policies**: Automatically loads 1600+ policies on startup from bundled `policies.yaml`
2. **Filter by Platform**: Windows, Linux, or All
3. **Filter by Severity**: Critical, High, Medium, Low
4. **Search**: Real-time search across titles and descriptions
5. **Audit Policies**: 
   - Individual: Click "Audit" button on any policy
   - Bulk: Click "Audit All" to scan all platform-applicable policies
6. **Remediate Policies**:
   - Individual: Click "Remediate" on non-compliant policies
   - Bulk: Click "Remediate All" to fix all failures

### Privilege Requirements

#### ⚠️ Administrator/Root Required for Remediation

**Windows**: Right-click → "Run as administrator"
**Linux**: `sudo nogap-dashboard` or `pkexec nogap-dashboard`
**macOS**: Launch normally, will prompt for admin password

#### Why Elevated Privileges?
- Registry modifications (Windows)
- Local policy changes (Windows/Linux)
- Service control (all platforms)
- File permission changes (Linux)
- System configuration (sysctl, SSH, etc.)

---

## 📋 Policy Categories

### Windows (Annexure A)
- User Rights Assignment
- Security Options
- Advanced Audit Policy
- Registry Security
- Local Policy Configuration
- Service Control

### Linux (Annexure B)
- File System Permissions
- Kernel Parameters (sysctl)
- SSH Daemon Configuration
- Service Management (systemd)
- PAM Configuration
- Audit Rules

---

## 🏗️ Building from Source

### Prerequisites
- **Node.js** 18+
- **Rust** 1.70+
- **Platform Tools**:
  - macOS: Xcode Command Line Tools
  - Windows: Visual Studio 2019+ with C++ tools
  - Linux: build-essential, GTK/WebKit2GTK dev packages

### Development Build
```bash
cd nogap-workspace/nogap_dashboard
npm install
npm run dev
```

### Production Build
```bash
# Current platform
npm run build

# Platform-specific
npm run build:macos    # Universal binary (Intel + Apple Silicon)
npm run build:windows  # x86_64 (requires MinGW for cross-compile)
npm run build:linux    # x86_64
```

### Output Location
```
target/release/bundle/
├── macos/
│   └── NoGap Dashboard.app
├── dmg/
│   └── NoGap Dashboard_1.0.0_aarch64.dmg
├── msi/
│   └── nogap-dashboard_1.0.0_x64.msi
├── nsis/
│   └── nogap-dashboard_1.0.0_x64-setup.exe
├── deb/
│   └── nogap-dashboard_1.0.0_amd64.deb
└── appimage/
    └── nogap-dashboard_1.0.0_amd64.AppImage
```

---

## 🧪 Development

### Project Structure
```
nogap_dashboard/
├── src/                    # Frontend (HTML/CSS/JS)
│   ├── index.html         # Main UI
│   ├── styles.css         # Styling
│   └── assets/            # Static assets
├── src-tauri/             # Rust backend
│   ├── src/
│   │   └── lib.rs         # IPC commands, audit/remediation logic
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── dist/                  # Built frontend
│   └── app.js             # Compiled JavaScript
├── package.json           # Node dependencies
├── RELEASE.md             # Release documentation
└── README.md              # This file
```

### Key Files
- **lib.rs**: Rust backend with audit/remediation implementations
- **app.js**: Frontend state management and UI rendering
- **policies.yaml**: 1600+ security policy definitions (in nogap_core/)
- **tauri.conf.json**: App configuration, bundle settings, resources

### Tech Stack
- **Frontend**: Vanilla JavaScript, HTML5, CSS3
- **Backend**: Rust with Tauri 2.x
- **IPC**: `window.__TAURI__.core.invoke()`
- **Policy Format**: YAML with serde deserialization
- **Build**: Tauri CLI with cargo

---

## 🐛 Troubleshooting

### "Policy file not found"
- Development: Ensure `nogap-workspace/nogap_core/policies.yaml` exists
- Production: File is automatically bundled via `tauri.conf.json` resources

### "Permission denied" during remediation
- **Windows**: Restart as Administrator
- **Linux**: Use `sudo` or `pkexec`
- **macOS**: Approve admin password prompts

### Audit returns "Error" status
- Verify system tools are available:
  - Windows: `sc.exe`, `reg.exe`
  - Linux: `systemctl`, `sysctl`, `chmod`, `grep`
- Check application logs for detailed errors
- Ensure policy YAML syntax is valid

### Build failures
```bash
# Clear cache and rebuild
cd nogap_dashboard/src-tauri
cargo clean
cd ..
npm run build
```

---

## 📚 Documentation

- **RELEASE.md** - Detailed release notes, build instructions, installation guide
- **BUILD_GUIDE.md** - Platform-specific build instructions (if available)
- **nogap_core/policies.yaml** - Policy definitions with schema documentation

---

## 🔒 Security Considerations

1. **Backup First**: Always backup system config before mass remediation
2. **Test Environment**: Test policies on non-production systems
3. **Audit Before Remediate**: Review compliance status before applying fixes
4. **Check Reversibility**: Review "Reversible" column before changes
5. **Elevated Privileges**: Use admin/root access responsibly
6. **Policy Review**: Understand policy impact before enforcement

---

## 📈 Project Status

### Stage 7 - Release (COMPLETE) ✅
- ✅ Production build configuration
- ✅ Bundle generation (DMG, MSI, DEB, AppImage)
- ✅ Resource embedding (policies.yaml)
- ✅ Release documentation
- ✅ Installation guides
- ✅ Version 1.0.0 published

### Previous Stages
- ✅ Stage 0: Project initialization
- ✅ Stage 1: Tauri 2.x setup
- ✅ Stage 2: IPC commands and API connector
- ✅ Stage 3: Cross-platform build targets
- ✅ Stage 4: Dashboard UI implementation
- ✅ Stage 5: YAML policy loading and validation
- ✅ Stage 6: Complete audit and remediation system

---

## 🛣️ Roadmap

### Future Enhancements
- 🔄 Windows Registry API (replace CLI tools)
- 🔄 Windows Local Policy API (native implementation)
- 🔄 Detailed audit reports (PDF/HTML export)
- 🔄 Custom policy creation UI
- 🔄 Scheduled audit tasks
- 🔄 Email/webhook notifications
- 🔄 Policy compliance trending
- 🔄 Multi-system management

---

## 📄 License

See main project LICENSE file

## 🤝 Contributing

Contributions welcome! Please submit issues and pull requests to the main repository.

---

**Version**: 1.0.0  
**Build Date**: November 2025  
**Platforms**: macOS (Apple Silicon + Intel), Windows x64, Linux x64  
**Bundle Sizes**: 3-9 MB (depending on platform)

**Ready for production deployment!** 🎉

