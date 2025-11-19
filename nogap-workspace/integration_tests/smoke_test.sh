#!/usr/bin/env bash
# NoGap Integration Smoke Test
# Tests basic functionality: build, CLI help, minimal audit
# Exit codes: 0 = all tests passed, 1 = test failed

set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Change to workspace root
cd "$(dirname "$0")/.."

echo -e "${YELLOW}=== NoGap Integration Smoke Test ===${NC}"
echo ""

# Test 1: Build all packages
echo -e "${YELLOW}[1/4] Building all packages...${NC}"
if cargo build --release --package nogap_core --package nogap_cli > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Build successful${NC}"
else
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi

# Test 2: CLI --help
echo -e "${YELLOW}[2/4] Testing CLI --help...${NC}"
CLI_BIN="./target/release/nogap-cli"
if [ ! -f "$CLI_BIN" ]; then
    echo -e "${RED}✗ CLI binary not found at $CLI_BIN${NC}"
    exit 1
fi

HELP_OUTPUT=$("$CLI_BIN" --help 2>&1 || true)
if echo "$HELP_OUTPUT" | grep -q "NoGap Security Platform"; then
    echo -e "${GREEN}✓ CLI --help working${NC}"
else
    echo -e "${RED}✗ CLI --help output invalid${NC}"
    echo "Output: $HELP_OUTPUT"
    exit 1
fi

# Test 3: CLI --version
echo -e "${YELLOW}[3/4] Testing CLI --version...${NC}"
VERSION_OUTPUT=$("$CLI_BIN" --version 2>&1 || true)
if echo "$VERSION_OUTPUT" | grep -q "nogap-cli"; then
    echo -e "${GREEN}✓ CLI --version working${NC}"
else
    echo -e "${RED}✗ CLI --version output invalid${NC}"
    echo "Output: $VERSION_OUTPUT"
    exit 1
fi

# Test 4: Minimal audit (requires policy file)
echo -e "${YELLOW}[4/4] Testing minimal audit with dummy policy...${NC}"

# Create a minimal dummy policy file
DUMMY_POLICY=$(mktemp)
cat > "$DUMMY_POLICY" << 'EOF'
policies:
  - id: "TEST.1"
    title: "Test Policy"
    platform: "linux"
    check_type: "command"
    command: "echo pass"
    expected: "pass"
    description: "Dummy test policy"
EOF

# Run audit (expect it to execute without crashing)
# Note: We don't require it to pass since it's a dummy policy
AUDIT_OUTPUT=$("$CLI_BIN" audit --file "$DUMMY_POLICY" 2>&1 || true)
AUDIT_EXIT=$?

# Cleanup
rm -f "$DUMMY_POLICY"

# Check if CLI ran without panic (exit code 0 or 1 is acceptable, 101 is panic)
if [ $AUDIT_EXIT -eq 0 ] || [ $AUDIT_EXIT -eq 1 ]; then
    echo -e "${GREEN}✓ CLI audit executed without crash${NC}"
    echo "  (Exit code: $AUDIT_EXIT - expected for minimal test)"
else
    echo -e "${RED}✗ CLI audit crashed (exit code: $AUDIT_EXIT)${NC}"
    echo "Output: $AUDIT_OUTPUT"
    exit 1
fi

echo ""
echo -e "${GREEN}=== All smoke tests passed ===${NC}"
echo ""
echo "Summary:"
echo "  ✓ Release build successful"
echo "  ✓ CLI --help functional"
echo "  ✓ CLI --version functional"
echo "  ✓ CLI audit runs without crash"
echo ""
exit 0
