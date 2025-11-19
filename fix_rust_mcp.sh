#!/bin/bash
# Quick-fix: force Copilot's Rust MCP to use the correct working directory

ROOT="/Users/sachin/Downloads/Project/NoGap"
MCP_DIR="$ROOT/.mcp"
VSCODE_DIR="$ROOT/.vscode"

mkdir -p "$MCP_DIR" "$VSCODE_DIR"

# write correct rust MCP config
cat > "$MCP_DIR/rust.json" <<EOF
{
  "id": "rust",
  "name": "Rust SDK MCP",
  "command": "cargo",
  "args": ["check"],
  "workingDirectory": "$ROOT/nogap-workspace/nogap_core",
  "description": "Checks nogap_core crate"
}
EOF

# minimal manifest so Copilot can see it
cat > "$VSCODE_DIR/mcp.json" <<EOF
{
  "mcpServers": [
    { "id": "rust", "path": ".mcp/rust.json" }
  ]
}
EOF

echo "✅ Rust MCP configuration written."
echo "➡️  In VS Code: run 'Developer: Reload Window' from the Command Palette, then 'MCP: List Servers' → Start Server (rust)."
