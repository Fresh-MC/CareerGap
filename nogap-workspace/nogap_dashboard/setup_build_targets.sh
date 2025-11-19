#!/bin/bash
# NoGap Dashboard - Cross-Compilation Setup
# Sets up Windows and Linux build targets on macOS

set -e

echo "===== NoGap Dashboard Cross-Compilation Setup ====="
echo ""

# Check if Homebrew is installed
if ! command -v brew &> /dev/null; then
    echo "Error: Homebrew not found. Install from https://brew.sh"
    exit 1
fi

# Check if Rust is installed
if ! command -v rustup &> /dev/null; then
    echo "Error: Rust not found. Install from https://rustup.rs"
    exit 1
fi

echo "[1/4] Installing MinGW-w64 for Windows cross-compilation..."
if brew list mingw-w64 &> /dev/null; then
    echo "  ✓ MinGW-w64 already installed"
else
    brew install mingw-w64
    echo "  ✓ MinGW-w64 installed"
fi

echo ""
echo "[2/4] Adding Rust Windows target (x86_64-pc-windows-gnu)..."
rustup target add x86_64-pc-windows-gnu
echo "  ✓ Windows target added"

echo ""
echo "[3/4] Adding Rust Linux target (x86_64-unknown-linux-gnu)..."
rustup target add x86_64-unknown-linux-gnu
echo "  ✓ Linux target added"

echo ""
echo "[4/4] Creating Cargo config for cross-compilation..."
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-ar"

[target.x86_64-unknown-linux-gnu]
linker = "x86_64-linux-gnu-gcc"
EOF
echo "  ✓ Cargo config created"

echo ""
echo "===== Setup Complete! ====="
echo ""
echo "Build commands:"
echo "  macOS:   npm run tauri build"
echo "  Windows: npm run tauri build -- --target x86_64-pc-windows-gnu"
echo "  Linux:   npm run tauri build -- --target x86_64-unknown-linux-gnu"
echo ""
echo "Note: Linux cross-compilation may require additional setup."
echo "See BUILD_GUIDE.md for details."
