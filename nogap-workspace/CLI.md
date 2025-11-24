# CLI.md

## Command Structure and Subcommands

The `nogap-cli` binary provides three subcommands using `clap` for argument parsing. The `tui` subcommand launches the interactive terminal UI with an optional `--policies` flag (defaults to `policies.yaml`). The `audit` subcommand runs non-interactive compliance checks with `--policies`, `--filter` for policy ID filtering, and `--json` for machine-readable output. The `remediate` subcommand applies fixes with `--policies`, `--id` to target a specific policy, and `--yes` to skip confirmation prompts. All commands use `policy_parser::load_policy()` to load YAML policy definitions via a three-tier path resolution (repository-relative → current directory → system paths). The CLI invokes `nogap_core::engine` functions (`audit()`, `remediate()`) directly, passing policy vectors and snapshot providers for state management.

## TUI Flow (Navigation, Layout, Keybindings)

The TUI implements a multi-screen architecture using `ratatui` and `crossterm`. The main `Dashboard` screen displays a two-column layout: left panel shows a scrollable policy table with ID, title, severity (C/H/M/L), and compliance status (✓/✗/⚠/?), while the right panel shows details for the selected policy (title, ID, platform, check_type, description, and available actions). Navigation uses vim-style keys (`j`/`k` or `↓`/`↑` for movement, `/` for search, `f` for filter modal, `o` for sort cycling, `s` for snapshot browser). The `Details` screen splits horizontally to show BEFORE/AFTER snapshots with scrollable content. The `Help` screen (`?` key) lists all keybindings. Modal overlays support confirmation dialogs (remediation warnings), filter selection (severity/platform checkboxes with `Space` to toggle), and blocking operation indicators (audit/remediate progress). The TUI maintains an `AppState` struct tracking current screen, dashboard state (policies, statuses, sort mode, filters), multi-select state (for batch operations), and modal stack. The event loop polls keyboard input every 100ms, routing key codes to screen-specific handlers via the `KeyMap` abstraction.

## Automation Mode (--json)

The `audit --json` flag enables headless operation for CI/CD integration. The `run_headless_audit()` function serializes results into a structured `CliReport` JSON object containing `timestamp` (ISO 8601), `compliance_score` (percentage), and `results` array (each with `policy_id`, `passed` boolean, `message`). Errors are returned as JSON objects with `error` fields, ensuring parseable output for automation scripts. The function exits with code 1 on failure (policy load errors, audit failures, JSON serialization errors). This mode bypasses the TUI entirely, printing raw JSON to stdout for consumption by monitoring systems, CI pipelines, or configuration management tools. The compliance score is computed as `(passed / total) * 100` across all audited policies.

## Integration with nogap_core

The CLI integrates with `nogap_core` through direct function calls to `engine::audit()` and `engine::remediate()`. The `run_audit()` function clones the selected policy, wraps it in a vector, and calls `engine::audit(&policies)`, receiving `Vec<AuditResult>` with `policy_id`, `passed`, and `message` fields. Results update the dashboard's `statuses` vector (Pass/Fail/Warning). The `run_remediate()` function follows the same pattern but passes a `RealSnapshotProvider` to enable BEFORE/AFTER snapshot capture via `nogap_core::snapshot` SQLite integration. Batch operations iterate through multi-selected policy indices, invoking engine functions synchronously with `std::slice::from_ref()` for single-policy slices. The CLI uses modal overlays to display "AUDITING" and "REMEDIATING" messages during blocking operations. Error handling converts `Result<T, anyhow::Error>` returns into dashboard status updates or error modals.

## Snapshot Usage via CLI

Snapshot management integrates `nogap_core::snapshot` SQLite functions into the TUI. Pressing `S` calls `snapshot::init_db()` to open `snapshots.db`, then `snapshot::list_snapshots(&conn)` to retrieve `Vec<(id, timestamp, description)>` tuples sorted by ID descending. The `SnapshotBrowser` screen displays this list with `j`/`k` navigation and `Enter` to preview. Pressing `Enter` invokes `snapshot::get_snapshot(&conn, id)` to load BEFORE/AFTER state JSON strings, rendering them in the `SnapshotPreview` screen with scrollable content (`j`/`k` for line-by-line, `PageUp`/`PageDown` for chunks). The `d` key opens the `DiffViewer` to compare snapshot states side-by-side using `DiffViewerState::new(&before, &after)`. The remediation engine automatically calls `snapshot::save_snapshot()` before and after changes via `RealSnapshotProvider`, creating audit trails for rollback operations. The CLI does not yet expose direct rollback commands but provides full read access to snapshot history.

## Error Handling and Exit Codes

The CLI implements hierarchical error handling using `anyhow::Result` for propagation. The `main()` function returns `Result<()>`, with `?` operators bubbling errors to top-level handlers. Policy loading failures exit with code 1 via `std::process::exit(1)` after printing JSON error messages in headless mode. Audit and remediation errors are caught within TUI loops, converting `Err(e)` into dashboard status updates with `set_last_action(format!("Audit failed: {}", e))` to avoid crashing the interactive session. Modal-based confirmation screens handle user cancellations gracefully (pressing `Esc` or `n` returns to dashboard without error). Snapshot database initialization failures display error messages in the status bar but do not terminate the session. The TUI uses `crossterm::terminal::disable_raw_mode()` and `LeaveAlternateScreen` in a cleanup phase to restore terminal state even after errors. Exit codes: 0 for success, 1 for policy load/audit/remediation failures in headless mode, 130 for Ctrl+C interrupts (handled by `KeyModifiers::CONTROL` check on 'c' key).

## Example Usage for Windows and Linux

**Windows Example (Interactive TUI)**:
```cmd
nogap-cli.exe tui --policies C:\ProgramData\NoGap\policies.yaml
```
Opens the dashboard showing Windows policies (filtered by default). Press `a` to audit the selected policy (e.g., A.1.a.i password history check), which invokes `platforms::windows::audit_policy()` to query the registry or run secedit. Press `r` to remediate (prompts confirmation), which calls `platforms::windows::remediate_policy()` to apply registry writes or secedit INF changes. Press `S` to browse snapshots, `d` to view diffs, `f` to filter by severity (High/Medium/Low), `o` to sort by ID/Severity/Platform.

**Linux Example (Headless Audit)**:
```bash
sudo nogap-cli audit --policies /etc/nogap/policies.yaml --json > audit_report.json
```
Audits all Linux policies without TUI, outputting JSON with `compliance_score` and policy results. Use `jq` to parse: `jq '.compliance_score' audit_report.json` returns percentage. The `--filter` flag narrows scope: `nogap-cli audit --filter B.2.a.i` audits only IP forwarding policy. Remediation example:
```bash
sudo nogap-cli remediate --policies /etc/nogap/policies.yaml --id B.2.a.i --yes
```
Disables IP forwarding via `sysctl -w net.ipv4.ip_forward=0` without confirmation, creating BEFORE/AFTER snapshots in `snapshots.db`. The `--yes` flag is critical for automation scripts to avoid blocking on stdin reads.

**Stage 4 complete.**
