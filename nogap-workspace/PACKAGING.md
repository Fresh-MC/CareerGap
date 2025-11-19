# NoGap Packaging Guide

This document provides step-by-step instructions for packaging and distributing the NoGap Security Platform components.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [CLI Binary Distribution](#cli-binary-distribution)
3. [Tauri Desktop Application](#tauri-desktop-application)
4. [MCP Server (Optional)](#mcp-server-optional)
5. [Cross-Platform Build Matrix](#cross-platform-build-matrix)

---

## Prerequisites

### Required Tools

```bash
# Rust toolchain (1.70+ recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Cargo (bundled with Rust)
cargo --version

# For cross-compilation (optional)
cargo install cross

# For Tauri desktop builds
npm install -g @tauri-apps/cli
# Or use pnpm/yarn as preferred
```

### System Dependencies

**Linux:**
```bash
# Debian/Ubuntu
sudo apt-get update
sudo apt-get install -y build-essential libssl-dev pkg-config

# Tauri-specific dependencies
sudo apt-get install -y \
  libwebkit2gtk-4.0-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

**macOS:**
```bash
# Install Xcode Command Line Tools
xcode-select --install

# No additional dependencies needed for Tauri
```

**Windows:**
```powershell
# Install Visual Studio Build Tools
# Or Visual Studio Community with "Desktop development with C++" workload

# WebView2 runtime (usually pre-installed on Windows 10/11)
# Download from: https://developer.microsoft.com/en-us/microsoft-edge/webview2/
```

---

## CLI Binary Distribution

The `nogap-cli` is a standalone terminal UI application for security policy auditing and remediation.

### 1. Build Release Binary

```bash
cd nogap-workspace

# Build optimized release binary
cargo build --release --package nogap_cli

# Binary location:
# Linux/macOS: target/release/nogap-cli
# Windows: target\release\nogap-cli.exe

# Verify binary size (should be ~5-6 MB)
ls -lh target/release/nogap-cli
```

### 2. Test the Binary

```bash
# Run help
./target/release/nogap-cli --help

# Run version
./target/release/nogap-cli --version

# Run integration smoke test
./integration_tests/smoke_test.sh
```

### 3. Strip Debug Symbols (Optional)

To reduce binary size further:

```bash
# Linux/macOS
strip target/release/nogap-cli

# Windows (using MSVC toolchain)
# Debug symbols are in separate .pdb file, can be deleted
```

### 4. Package for Distribution

**Option A: Tarball (Linux/macOS)**

```bash
cd target/release
tar -czf nogap-cli-v1.0.0-x86_64-linux.tar.gz nogap-cli
# Or for macOS:
tar -czf nogap-cli-v1.0.0-x86_64-macos.tar.gz nogap-cli

# Move to distribution directory
mv nogap-cli-*.tar.gz ../../dist/
```

**Option B: Zip Archive (Windows)**

```powershell
cd target\release
Compress-Archive -Path nogap-cli.exe -DestinationPath nogap-cli-v1.0.0-x86_64-windows.zip

# Move to distribution directory
Move-Item nogap-cli-*.zip ..\..\dist\
```

**Option C: DEB Package (Debian/Ubuntu)**

```bash
# Install cargo-deb
cargo install cargo-deb

# Build .deb package
cd nogap_cli
cargo deb

# Package location: target/debian/nogap-cli_*.deb

# Install locally to test
sudo dpkg -i ../target/debian/nogap-cli_*.deb
```

**Option D: RPM Package (Fedora/RHEL)**

```bash
# Install cargo-rpm
cargo install cargo-rpm

# Initialize RPM spec
cd nogap_cli
cargo rpm init

# Build .rpm package
cargo rpm build

# Package location: target/release/rpmbuild/RPMS/x86_64/
```

### 5. Distribution Checklist

- [ ] Binary is built with `--release` flag
- [ ] Binary size is reasonable (~5-8 MB unstripped)
- [ ] `--help` and `--version` work correctly
- [ ] Smoke tests pass (`integration_tests/smoke_test.sh`)
- [ ] Include `policies.yaml` in distribution package
- [ ] Include README or INSTALL instructions
- [ ] Provide SHA256 checksums for all archives

```bash
# Generate checksums
cd dist
sha256sum nogap-cli-*.tar.gz > checksums.txt
# Or on macOS:
shasum -a 256 nogap-cli-*.tar.gz > checksums.txt
```

---

## Tauri Desktop Application

The `nogap_dashboard` is a cross-platform desktop application built with Tauri (Rust backend + web frontend).

### 1. Install Tauri CLI

```bash
# Using Cargo
cargo install tauri-cli --version "^2.0.0"

# Or use npm/pnpm
npm install -g @tauri-apps/cli@next
```

### 2. Build Desktop Bundles

```bash
cd nogap_dashboard

# Development build (faster, larger)
cargo tauri dev

# Production release build
cargo tauri build

# Build artifacts location:
# Linux: src-tauri/target/release/bundle/appimage/
#        src-tauri/target/release/bundle/deb/
# macOS: src-tauri/target/release/bundle/dmg/
#        src-tauri/target/release/bundle/macos/
# Windows: src-tauri\target\release\bundle\msi\
#          src-tauri\target\release\bundle\nsis\
```

### 3. Platform-Specific Bundles

**Linux (AppImage + DEB)**

```bash
cd nogap_dashboard
cargo tauri build

# Outputs:
# - .AppImage (universal Linux bundle, no installation needed)
# - .deb (Debian/Ubuntu package)

# Test AppImage
chmod +x src-tauri/target/release/bundle/appimage/*.AppImage
./src-tauri/target/release/bundle/appimage/*.AppImage

# Test DEB
sudo dpkg -i src-tauri/target/release/bundle/deb/*.deb
```

**macOS (DMG + APP)**

```bash
cd nogap_dashboard
cargo tauri build

# Outputs:
# - .dmg (macOS disk image for distribution)
# - .app (macOS application bundle)

# Code signing (required for distribution)
# 1. Obtain Apple Developer certificate
# 2. Configure in src-tauri/tauri.conf.json:
#    "bundle": {
#      "macOS": {
#        "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)"
#      }
#    }

# Notarization (required for Gatekeeper)
xcrun notarytool submit src-tauri/target/release/bundle/dmg/*.dmg \
  --apple-id your@email.com \
  --password YOUR_APP_SPECIFIC_PASSWORD \
  --team-id YOUR_TEAM_ID \
  --wait
```

**Windows (MSI + NSIS)**

```powershell
cd nogap_dashboard
cargo tauri build

# Outputs:
# - .msi (Windows Installer package)
# - .exe (NSIS installer)

# Code signing (recommended for distribution)
# 1. Obtain code signing certificate
# 2. Use signtool.exe:
signtool sign /f certificate.pfx /p PASSWORD /tr http://timestamp.digicert.com /td sha256 /fd sha256 installer.exe
```

### 4. Tauri Configuration

Edit `src-tauri/tauri.conf.json` for packaging customization:

```json
{
  "productName": "NoGap Security Platform",
  "version": "1.0.0",
  "identifier": "com.nogap.security",
  "bundle": {
    "active": true,
    "targets": ["deb", "appimage", "msi", "dmg"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "externalBin": [],
    "resources": ["assets/policies.yaml"],
    "shortDescription": "Security compliance audit and remediation",
    "longDescription": "NoGap Security Platform provides automated security policy auditing and remediation for Windows and Linux systems."
  }
}
```

### 5. Distribution Checklist

- [ ] Tauri build completes successfully
- [ ] Application launches without errors
- [ ] All UI features work correctly
- [ ] Bundled `policies.yaml` is included
- [ ] Icons are properly set for all platforms
- [ ] Code signing certificates applied (macOS/Windows)
- [ ] Notarization completed (macOS)
- [ ] Installer tested on clean VM/system
- [ ] Generate SHA256 checksums for all installers

---

## MCP Server (Optional)

The `nogap_mcp` is a Model Context Protocol server for AI agent integration (Claude Desktop, etc.).

### 1. Build MCP Server Binary

```bash
cd nogap-workspace

# Build release binary
cargo build --release --package nogap_mcp

# Binary location:
# target/release/nogap_mcp (Linux/macOS)
# target\release\nogap_mcp.exe (Windows)
```

### 2. Docker Image (Recommended)

Create `Dockerfile` in `nogap_mcp/`:

```dockerfile
# nogap_mcp/Dockerfile
FROM rust:1.75-slim as builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY nogap_core ./nogap_core
COPY nogap_mcp ./nogap_mcp

RUN cargo build --release --package nogap_mcp

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/nogap_mcp /usr/local/bin/nogap_mcp
COPY nogap_core/policies.yaml /etc/nogap/policies.yaml

EXPOSE 8080

CMD ["nogap_mcp"]
```

Build and push Docker image:

```bash
# Build image
docker build -t nogap/mcp-server:latest -f nogap_mcp/Dockerfile .

# Test locally
docker run -p 8080:8080 nogap/mcp-server:latest

# Tag for registry
docker tag nogap/mcp-server:latest ghcr.io/your-org/nogap-mcp:v1.0.0

# Push to GitHub Container Registry
echo $GITHUB_TOKEN | docker login ghcr.io -u your-username --password-stdin
docker push ghcr.io/your-org/nogap-mcp:v1.0.0
```

### 3. MCP Configuration

Users can add to their MCP settings (e.g., Claude Desktop config):

```json
{
  "mcpServers": {
    "nogap-security": {
      "command": "/path/to/nogap_mcp",
      "args": [],
      "env": {
        "NOGAP_POLICIES": "/path/to/policies.yaml"
      }
    }
  }
}
```

Or with Docker:

```json
{
  "mcpServers": {
    "nogap-security": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-v",
        "/path/to/policies.yaml:/etc/nogap/policies.yaml",
        "ghcr.io/your-org/nogap-mcp:v1.0.0"
      ]
    }
  }
}
```

---

## Cross-Platform Build Matrix

For automated CI/CD builds (GitHub Actions, GitLab CI, etc.):

### GitHub Actions Example

```yaml
# .github/workflows/release.yml
name: Release Build

on:
  push:
    tags:
      - 'v*'

jobs:
  build-cli:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: nogap-cli-linux-x86_64
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: nogap-cli-macos-x86_64
          - os: macos-latest
            target: aarch64-apple-darwin
            artifact: nogap-cli-macos-arm64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: nogap-cli-windows-x86_64.exe

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Build Release
        run: cargo build --release --target ${{ matrix.target }} --package nogap_cli

      - name: Upload Artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: target/${{ matrix.target }}/release/nogap-cli*

  build-tauri:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-node@v4
        with:
          node-version: 20

      - name: Install Tauri CLI
        run: cargo install tauri-cli

      - name: Build Tauri App
        working-directory: nogap_dashboard
        run: cargo tauri build

      - name: Upload Bundles
        uses: actions/upload-artifact@v4
        with:
          name: tauri-${{ matrix.os }}
          path: nogap_dashboard/src-tauri/target/release/bundle/
```

### Using `cross` for Linux ARM Targets

```bash
# Install cross
cargo install cross

# Build for ARM64 Linux
cross build --release --target aarch64-unknown-linux-gnu --package nogap_cli

# Build for ARMv7 Linux (Raspberry Pi)
cross build --release --target armv7-unknown-linux-gnueabihf --package nogap_cli
```

---

## Release Checklist

Before publishing a release:

- [ ] All tests pass (`cargo test`)
- [ ] Clippy shows zero warnings (`cargo clippy -- -D warnings`)
- [ ] Smoke tests pass (`integration_tests/smoke_test.sh`)
- [ ] Version numbers updated in `Cargo.toml` files
- [ ] CHANGELOG.md updated with release notes
- [ ] All binaries built for target platforms
- [ ] Code signing applied (macOS/Windows)
- [ ] Notarization completed (macOS)
- [ ] SHA256 checksums generated
- [ ] Release notes drafted
- [ ] Documentation updated (README.md, PACKAGING.md)
- [ ] Git tag created: `git tag -a v1.0.0 -m "Release v1.0.0"`
- [ ] Tag pushed: `git push origin v1.0.0`

---

## Distribution Channels

### GitHub Releases

1. Create new release from tag
2. Upload all binary artifacts (.tar.gz, .zip, .deb, .rpm, .dmg, .msi)
3. Include checksums.txt
4. Add release notes

### Package Registries

- **crates.io** (Rust): `cargo publish` (for libraries only, not binaries)
- **Flathub** (Linux): Submit Flatpak manifest
- **Snapcraft** (Linux): Submit snap package
- **Homebrew** (macOS/Linux): Create formula in tap repository
- **Chocolatey** (Windows): Submit package
- **winget** (Windows): Submit manifest to winget-pkgs repository

### Container Registries

- **Docker Hub**: `docker push nogap/mcp-server:latest`
- **GitHub Container Registry**: `docker push ghcr.io/org/nogap-mcp:v1.0.0`

---

## Troubleshooting

### Common Build Issues

**Issue: "linker 'cc' not found"**
```bash
# Linux
sudo apt-get install build-essential

# macOS
xcode-select --install
```

**Issue: "WebKit2GTK not found" (Tauri on Linux)**
```bash
sudo apt-get install libwebkit2gtk-4.0-dev
```

**Issue: "Failed to create app bundle" (Tauri on macOS)**
- Ensure Xcode Command Line Tools installed
- Check code signing identity in tauri.conf.json
- Verify entitlements file is present

**Issue: "Binary too large" (>50 MB)**
- Ensure building with `--release` flag
- Run `strip` on binary to remove debug symbols
- Check for accidentally bundled debug dependencies

---

## Support

For packaging issues or questions:
- Open issue: https://github.com/your-org/nogap/issues
- Documentation: https://nogap.security/docs
- Community: Discord/Slack channel

---

**Last Updated**: Stage 10 - December 2024  
**NoGap Version**: 1.0.0 (pre-release)
