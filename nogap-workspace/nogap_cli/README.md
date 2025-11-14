# NoGap CLI - Operator Cockpit

Terminal User Interface (TUI) for the NoGap security compliance platform.

## Overview

NoGap CLI provides an interactive operator cockpit for managing security policies, running audits, and performing remediation tasks. Built with Ratatui for a keyboard-driven, high-performance terminal experience.

## Features

- **Dashboard View**: Browse and manage security policies with real-time status
- **Audit Execution**: Run security audits against policies synchronously
- **Remediation**: Apply fixes to non-compliant policies with confirmation
- **Diff Viewer**: Compare BEFORE/AFTER snapshots side-by-side
- **Snapshot Browser** (Week 3): Browse historical system snapshots with timestamps
- **Snapshot Diff Viewer** (Week 3): Line-by-line colored diff viewer for snapshots
- **Multi-Select Mode** (Week 3): Select multiple policies for batch operations
- **Batch Operations** (Week 3): Run audit or remediate on multiple policies simultaneously
- **High-Contrast Mode**: Toggle theme for accessibility
- **Keyboard-Driven**: All operations accessible via keyboard shortcuts

## Installation

### From Source

```bash
cd nogap-workspace
cargo build --release --bin nogap-cli
```

The binary will be available at `target/release/nogap-cli`.

### Development Build

```bash
cargo build --bin nogap-cli
```

## Usage

### Interactive TUI Mode

Launch the operator cockpit with a policies file:

```bash
nogap-cli tui [--policies path/to/policies.yaml]
```

If no policies path is provided, defaults to `policies.yaml` in the current directory.

### Non-Interactive Audit

Run an audit without the TUI:

```bash
nogap-cli audit [--policies path/to/policies.yaml] [--filter POLICY_ID]
```

### Non-Interactive Remediation

Remediate a specific policy:

```bash
nogap-cli remediate --id POLICY_ID [--policies path/to/policies.yaml] [--yes]
```

The `--yes` flag skips the confirmation prompt.

## Keyboard Shortcuts

### Global Keys

| Key | Action |
|-----|--------|
| `?` | Show help overlay |
| `q` or `Esc` | Quit (or go back to previous screen) |
| `Ctrl-C` | Force quit |

### Dashboard Navigation

| Key | Action |
|-----|--------|
| `j` or `↓` | Move down in policy list |
| `k` or `↑` | Move up in policy list |
| `t` | Toggle high-contrast theme |
| `S` (Shift+s) | Open snapshot browser |
| `m` | Toggle multi-select mode |
| `b` | Open batch operations menu (when in multi-select mode) |

### Multi-Select Mode (Week 3)

| Key | Action |
|-----|--------|
| `Space` | Toggle selection for current policy |
| `A` (Shift+a) | Select all visible policies |
| `N` (Shift+n) | Clear all selections |
| `m` | Exit multi-select mode |
| `b` | Open batch operations menu |

### Batch Operations Menu (Week 3)

| Key | Action |
|-----|--------|
| `j` or `↓` | Move down in menu |
| `k` or `↑` | Move up in menu |
| `Enter` | Execute selected batch operation |
| `Esc` | Cancel and return to dashboard |

### Snapshot Browser (Week 3)

| Key | Action |
|-----|--------|
| `j` or `↓` | Move down in snapshot list |
| `k` or `↑` | Move up in snapshot list |
| `Enter` | Open snapshot preview |
| `Esc` | Return to dashboard |

### Snapshot Preview (Week 3)

| Key | Action |
|-----|--------|
| `j` or `↓` | Scroll down |
| `k` or `↑` | Scroll up |
| `d` | Open diff viewer for this snapshot |
| `Esc` | Return to snapshot browser |

### Snapshot Diff Viewer (Week 3)

| Key | Action |
|-----|--------|
| `j` or `↓` | Scroll down one line |
| `k` or `↑` | Scroll up one line |
| `PgDn` | Scroll down one page (20 lines) |
| `PgUp` | Scroll up one page (20 lines) |
| `Esc` | Return to previous screen (preview or details) |

### Policy Actions

| Key | Action |
|-----|--------|
| `a` | Run audit on selected policy |
| `r` | Remediate selected policy (prompts for confirmation) |
| `d` | View diff (BEFORE/AFTER snapshots) |

### Modal Dialogs

| Key | Action |
|-----|--------|
| `Enter` or `y` | Confirm action |
| `Esc` or `n` | Cancel action |

## Demo Workflow

1. **Launch the TUI**:
   ```bash
   cargo run --bin nogap-cli -- tui
   ```

2. **Navigate policies**: Use `j`/`k` or arrow keys to browse the policy list.

3. **Run an audit**: Press `a` on a selected policy to execute an audit. The status column updates with ✓ (pass), ✗ (fail), or ⚠ (warning).

4. **Remediate a policy**: Press `r` to remediate. You'll be prompted for confirmation. Press `Enter` to confirm or `Esc` to cancel.

5. **View diff**: Press `d` to see a side-by-side comparison of BEFORE and AFTER snapshots.

6. **Toggle theme**: Press `t` to switch between normal (teal accent) and high-contrast (white accent) modes.

7. **Get help**: Press `?` to see the full keybinding reference.

8. **Exit**: Press `q` or `Esc` from the dashboard to quit.

### Week 3 Workflows

#### Batch Operations Workflow

1. **Enter multi-select mode**: Press `m` from the dashboard. Policies show checkboxes.

2. **Select policies**: 
   - Press `Space` to toggle individual policies
   - Press `A` (Shift+a) to select all visible policies
   - Press `N` (Shift+n) to clear all selections

3. **Open batch menu**: Press `b` to see batch operations menu with "Batch Audit" and "Batch Remediate" options.

4. **Execute batch operation**: Navigate with `j`/`k`, press `Enter` to confirm. A blocking modal shows progress for each policy.

5. **Exit multi-select**: Press `m` or `Esc` to return to normal dashboard mode.

#### Snapshot Browsing Workflow

1. **Open snapshot browser**: Press `S` (Shift+s) from the dashboard to see a timestamped list of historical snapshots.

2. **Select a snapshot**: Navigate with `j`/`k`, press `Enter` to open the snapshot preview.

3. **View snapshot content**: The preview shows the full snapshot JSON. Scroll with `j`/`k`.

4. **Open diff viewer**: Press `d` to see a line-by-line colored diff comparing consecutive snapshots:
   - **Green lines**: Added content
   - **Red lines**: Removed content  
   - **White lines**: Unchanged content

5. **Navigate diff**: Scroll with `j`/`k` (one line) or `PgDn`/`PgUp` (one page, 20 lines).

6. **Return navigation**: Press `Esc` to go back to snapshot preview, then `Esc` again to return to snapshot browser, then `Esc` to return to dashboard.

**Alternative diff path**: From the policy details screen, press `d` to view BEFORE/AFTER snapshots. Press `Esc` to return to the details screen.

## UI Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│ NoGap Policies (60%)        │  Policy Details (40%)                 │
│ ─────────────────────────   │  ─────────────────────────           │
│ ID          Title       S ✓ │  Selected Policy Title                │
│ TEST-001    Example     L ✓ │                                       │
│ TEST-002    Another     L ✗ │  ID: TEST-001                         │
│ TEST-003    Third       W ⚠ │  Platform: Linux                      │
│                             │  Type: file                           │
│                             │                                       │
│                             │  Description:                         │
│                             │  Example policy description...        │
│                             │                                       │
│                             │  Actions:                             │
│                             │    [a] Run Audit                      │
│                             │    [r] Remediate                      │
│                             │    [d] View Diff                      │
├─────────────────────────────────────────────────────────────────────┤
│ Status: Ready             [?] Help  [q] Quit                        │
└─────────────────────────────────────────────────────────────────────┘
```

## Color Scheme

- **Accent**: Teal (#2DD4BF) - Used for highlights and borders (normal mode)
- **Pass**: Green - Policies that passed audit
- **Fail**: Red - Policies that failed audit
- **Warning**: Orange - Policies with warnings
- **Unknown**: Gray - Policies not yet audited
- **High-Contrast**: White accent for improved accessibility

## Architecture

- **Synchronous**: All operations run synchronously (no async runtime)
- **Engine Integration**: Calls `nogap_core` for policy parsing, auditing, and remediation
- **Framework**: Built with [Ratatui](https://ratatui.rs/) and [Crossterm](https://github.com/crossterm-rs/crossterm)
- **CLI Parsing**: Uses [Clap](https://docs.rs/clap/) v4 with derive macros

## Testing

Run the unit tests:

```bash
cargo test --package nogap_cli
```

Tests include:
- Dashboard state navigation
- Status updates
- UI rendering snapshots
- Policy status color/symbol mappings

## Development

### Project Structure

```
nogap_cli/
├── src/
│   ├── main.rs              # CLI entry point with clap subcommands
│   ├── ui.rs                # Top-level TUI event loop
│   ├── keymap.rs            # Centralized keybinding definitions
│   ├── components/          # Reusable UI widgets
│   │   ├── mod.rs
│   │   └── table.rs         # Table component with selection
│   └── screens/             # Full-screen views
│       ├── mod.rs
│       ├── dashboard.rs     # Main policy list + details
│       └── details.rs       # Diff viewer
└── tests/
    └── ui_smoke.rs          # Unit tests for UI components
```

### Adding New Keybindings

1. Add the key check method to `keymap.rs`:
   ```rust
   pub fn is_my_action(code: KeyCode) -> bool {
       matches!(code, KeyCode::Char('x'))
   }
   ```

2. Add to `help_text()` in `keymap.rs`
3. Handle the key in `ui.rs` event loop

### Adding New Screens

1. Create a new file in `src/screens/`
2. Define a state struct and widget struct
3. Implement `ratatui::widgets::Widget` trait
4. Add to `screens/mod.rs`
5. Update `Screen` enum in `ui.rs`

## License

Part of the NoGap security compliance platform.

## Support

For issues or questions, refer to the main NoGap workspace documentation.
