# NoGap Dashboard - Cross-Platform Build Guide

## Prerequisites

### macOS (Development Platform)
- Rust toolchain: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Node.js and npm
- Xcode Command Line Tools: `xcode-select --install`

### Cross-Compilation Setup

#### Windows Target (x86_64-pc-windows-gnu)
```bash
# Install MinGW-w64 toolchain
brew install mingw-w64

# Add Rust target
rustup target add x86_64-pc-windows-gnu

# Create cargo config for Windows linking
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-ar"
EOF
```

#### Linux Target (x86_64-unknown-linux-gnu)
```bash
# Install cross-compilation toolchain
brew install FiloSottile/musl-cross/musl-cross

# Add Rust target
rustup target add x86_64-unknown-linux-gnu

# Alternative: Use cross-compilation container
cargo install cross
```

## Build Commands

### Development (macOS)
```bash
cd /Users/sachin/Downloads/Project/NoGap/nogap-workspace/nogap_dashboard
npm run tauri dev
```

### Production Builds

#### macOS (Native)
```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/macos/
```

#### Windows (Cross-compile from macOS)
```bash
npm run tauri build -- --target x86_64-pc-windows-gnu
# Output: src-tauri/target/x86_64-pc-windows-gnu/release/
```

#### Linux (Using cross)
```bash
# Method 1: Using cross tool (recommended)
cross build --release --target x86_64-unknown-linux-gnu
cd src-tauri && cargo tauri build --target x86_64-unknown-linux-gnu

# Method 2: Native Linux build (run on Linux machine)
npm run tauri build
# Output: src-tauri/target/release/bundle/deb/ and .AppImage
```

## Bundle Outputs

### macOS
- `.app` bundle: `src-tauri/target/release/bundle/macos/NoGap Dashboard.app`
- `.dmg` installer: `src-tauri/target/release/bundle/dmg/`

### Windows
- `.exe` executable: `src-tauri/target/x86_64-pc-windows-gnu/release/nogap_dashboard.exe`
- `.msi` installer: `src-tauri/target/x86_64-pc-windows-gnu/release/bundle/msi/`

### Linux
- `.deb` package: `src-tauri/target/release/bundle/deb/`
- `.AppImage`: `src-tauri/target/release/bundle/appimage/`
- `.rpm` package: `src-tauri/target/release/bundle/rpm/`

## Troubleshooting

### Windows Cross-Compilation Issues
If you encounter linking errors:
```bash
# Verify MinGW installation
which x86_64-w64-mingw32-gcc

# Check Rust target
rustup target list | grep windows-gnu
```

### Linux Cross-Compilation Issues
If cross-compilation fails, build natively on a Linux machine or use Docker:
```bash
docker run --rm -v $(pwd):/app -w /app rust:latest cargo build --release
```

## Quick Setup Script
```bash
#!/bin/bash
# setup_build_targets.sh

echo "Installing cross-compilation toolchains..."

# Windows
brew install mingw-w64
rustup target add x86_64-pc-windows-gnu

# Linux
brew install FiloSottile/musl-cross/musl-cross
rustup target add x86_64-unknown-linux-gnu

# Cross tool (alternative for Linux)
cargo install cross

echo "Cross-compilation setup complete!"
echo "Run 'npm run tauri build -- --target <target>' to build"
```

Save as `setup_build_targets.sh` and run: `chmod +x setup_build_targets.sh && ./setup_build_targets.sh`
