# NoGap CLI User Guide

## Quick Start

### Running the TUI Dashboard

From the workspace root directory:

```bash
cd /Users/sachin/Downloads/Project/NoGap/nogap-workspace
cargo run --bin nogap-cli -- tui -p nogap_core/policies.yaml
```

Or using the default policies path:

```bash
cargo run --bin nogap-cli -- tui
```

**Note:** The default policies path is `policies.yaml` in the current directory. Use `-p` flag to specify a different path.

## TUI Keybindings

### Navigation
- **j** or **↓** - Move down through policies
- **k** or **↑** - Move up through policies

### Actions
- **a** - Audit selected policy (runs blocking audit with modal)
- **r** - Remediate selected policy (shows confirmation, then runs blocking remediate with modal)
- **d** - View diff/details for selected policy
- **f** - Open filter modal (filter by severity/platform)
- **o** - Cycle through sort modes
- **/** - Enter search mode
- **t** - Toggle high-contrast theme
- **?** - Show help screen

### Global
- **Esc** or **q** - Quit (or go back to dashboard from other screens)

## Search Mode

Press **/** to enter search mode:
- Type to filter policies by ID, title, or description
- **Esc** - Exit search and clear filter
- **Backspace** - Delete last character

## Filter Modal

Press **f** to open the filter modal:
- **j**/**k** or **↑**/**↓** - Navigate options
- **Space** - Toggle selected filter
- **Enter** - Apply filters
- **Esc** - Cancel and close modal

Filter options:
- Severity: High, Medium, Low
- Platform: Windows, Linux, macOS

## Sort Modes

Press **o** to cycle through sort modes:
1. **ID** - Sort by policy ID (default)
2. **Severity** - Sort by severity (High → Medium → Low)
3. **Platform** - Sort by platform (alphabetical)
4. **Status** - Sort by compliance status (Fail → Warning → Pass → Unknown)

## Audit & Remediate Operations

### Audit Flow
1. Navigate to a policy using **j**/**k**
2. Press **a** to audit
3. A blocking modal appears: "AUDITING - Please wait..."
4. The audit runs synchronously
5. Modal closes and dashboard updates with results

### Remediate Flow
1. Navigate to a policy using **j**/**k**
2. Press **r** to remediate
3. Confirmation modal appears
4. Press **Enter** or **y** to confirm (or **Esc**/**n** to cancel)
5. A blocking modal appears: "REMEDIATING - Please wait..."
6. The remediation runs synchronously
7. Modal closes and dashboard updates with results

**Important:** All audit and remediate operations are synchronous and blocking. No async, no threads, no animation loops.

## Policy Status Indicators

- 🟢 **Pass** (Green) - Policy is compliant
- 🔴 **Fail** (Red) - Policy is non-compliant
- 🟡 **Warning** (Yellow) - Policy has warnings
- ⚪ **Unknown** (Gray) - Policy has not been audited yet

## CLI Arguments

### TUI Mode

```bash
nogap-cli tui [OPTIONS]

Options:
  -p, --policies <POLICIES>  Path to policies YAML file [default: policies.yaml]
  -h, --help                 Print help
```

### Example Usage

```bash
# Use default policies.yaml in current directory
cargo run --bin nogap-cli -- tui

# Specify custom policies file
cargo run --bin nogap-cli -- tui -p /path/to/custom_policies.yaml

# Use policies from nogap_core directory
cargo run --bin nogap-cli -- tui -p nogap_core/policies.yaml
```

## Policy YAML Schema

The CLI supports two formats for `expected_state`:

### String Format (for service status, simple checks)

```yaml
- id: "A.1.a.i"
  check_type: "service_status"
  service_name: "telnet"
  expected_state: "stopped_disabled"  # String value
```

### Map Format (for numeric comparisons)

```yaml
- id: "A.1.a.ii"
  check_type: "registry_key"
  value_name: "MinimumPasswordLength"
  expected_state:              # Map with operator and value
    operator: "gte"
    value: 14
```

**Supported operators:** `eq`, `ne`, `gt`, `gte`, `lt`, `lte`

## Troubleshooting

### Error: "No such file or directory"

Make sure you're running from the workspace directory:
```bash
cd /Users/sachin/Downloads/Project/NoGap/nogap-workspace
cargo run --bin nogap-cli -- tui -p nogap_core/policies.yaml
```

### Error: "Failed to parse YAML"

Check that your `expected_state` matches the check type:
- **service_status** → String ("running", "stopped_disabled")
- **registry_key**, **sysctl** → Map with operator and value
- **file_permissions** → String (permission pattern)

### Error: "could not find Cargo.toml"

You're not in the workspace directory. Navigate to:
```bash
cd /Users/sachin/Downloads/Project/NoGap/nogap-workspace
```

## Development

### Build Release Version

```bash
cargo build --release --bin nogap-cli
```

The binary will be at: `target/release/nogap-cli`

### Run Tests

```bash
cargo test --all
```

### Check Code Quality

```bash
RUSTFLAGS="-D warnings" cargo build --workspace
```

## Architecture Notes

- **100% Synchronous** - No async/await, no tokio, no threads
- **Blocking Modal UI** - Operations show modal → block → complete → hide modal
- **Zero Warnings** - Builds with `RUSTFLAGS="-D warnings"`
- **Cross-Platform** - Supports Windows, Linux, macOS policies
