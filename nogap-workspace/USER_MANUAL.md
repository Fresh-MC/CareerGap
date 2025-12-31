# USER_MANUAL.md

## Installation

**Dashboard (Desktop App)**

*Windows*: Download `nogap-dashboard_1.0.0_x64.msi` from releases. Run as Administrator and follow the installation wizard. Requires Windows 10 (1809+) and Microsoft Visual C++ Redistributable. Launch from Start Menu or run `C:\Program Files\NoGap Dashboard\nogap-dashboard.exe`.

*macOS*: Download `NoGap Dashboard_1.0.0_aarch64.dmg`. Open DMG, drag app to Applications folder. If security warning appears: System Preferences → Security & Privacy → Open Anyway. Alternatively, use `.app` bundle with `xattr -cr "/Applications/NoGap Dashboard.app"` to remove quarantine. App bundle is 8.7 MB.

*Linux*: For Debian/Ubuntu, `sudo dpkg -i nogap-dashboard_1.0.0_amd64.deb && sudo apt-get install -f`. For universal support, `chmod +x nogap-dashboard_1.0.0_amd64.AppImage && ./nogap-dashboard_1.0.0_amd64.AppImage`. Requires GTK 3.24+, WebKit2GTK 4.1+, systemd. Launch via application menu or terminal: `nogap-dashboard`.

**CLI (Terminal Application)**

Extract `nogap-cli-v1.0.0-x86_64-linux.tar.gz` (or Windows `.zip`, macOS `.tar.gz`) to preferred location. Add to PATH or invoke directly: `./nogap-cli tui`. For system-wide installation on Linux, use DEB package: `sudo dpkg -i nogap-cli_*.deb` (installs to `/usr/bin/nogap-cli`). Binary is self-contained (~5-6 MB) and requires no dependencies beyond system tools (`sc.exe`/`reg.exe` on Windows, `sysctl`/`systemctl` on Linux).

## First-Time Setup

**Policy File Configuration**

The dashboard auto-loads bundled `policies.yaml` (1600+ policies embedded in Tauri resources). For CLI, specify policy path: `nogap-cli tui --policies /etc/nogap/policies.yaml` (Linux) or `nogap-cli.exe tui --policies C:\ProgramData\NoGap\policies.yaml` (Windows). Custom policy YAMLs follow the schema with 30+ optional fields (`id`, `platform`, `check_type`, `target_path`, `value_name`, `expected_state`, etc.). Store at repository-relative path for dev or system paths for production (`/etc/nogap` on Linux, `C:\ProgramData\NoGap` on Windows).

**Trusted Keys Setup**

Create `~/.nogap/trusted_keys.json` with Ed25519 public keys for manifest verification:
```json
{
  "keys": [
    "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2"
  ]
}
```
Without this file, the system uses a hardcoded fallback key (insecure for production). Generate key pairs with `ssh-keygen -t ed25519` or `age-keygen`, then extract public key hex bytes.

## Running an Audit

**GUI (Dashboard)**

1. Launch dashboard to main interface (60/40 split: policy table left, details right).
2. Use filters: Platform dropdown (Windows/Linux/All), Severity checkboxes (Critical/High/Medium/Low), Search box for text matching.
3. Select policy from table (click row or use `j`/`k` keys if keyboard navigation enabled).
4. Click "Audit" button or press `a` key. A modal displays "AUDITING..." during execution (blocking operation, typically 100-500ms per policy).
5. Status updates in "✓" column: green checkmark (Pass), red X (Fail), yellow ⚠ (Warning), gray ? (Unknown/Not Run).
6. For bulk audits, click "Audit All" to process all platform-applicable policies sequentially.

**CLI (Terminal)**

Interactive TUI: `nogap-cli tui --policies policies.yaml`. Navigate dashboard with `j`/`k` (down/up), `/` (search), `f` (filter modal), `o` (sort by ID/Severity/Platform). Press `a` on selected policy to audit. Results update status column inline. Press `S` to browse snapshot history, `d` to view BEFORE/AFTER diffs.

Headless mode: `nogap-cli audit --policies policies.yaml --json > report.json`. Outputs structured JSON with `timestamp` (ISO 8601), `compliance_score` (percentage), `results` array (each with `policy_id`, `passed` boolean, `message`). Use `--filter` to narrow scope: `nogap-cli audit --filter B.2.a.i` audits only policy B.2.a.i (IP forwarding). Exit code 0 on success, 1 on failure.

Parse JSON with `jq`: `jq '.compliance_score' report.json` returns percentage. Extract failed policies: `jq '.results[] | select(.passed == false) | .policy_id' report.json`.

## Remediation

**GUI (Dashboard)**

1. Select non-compliant policy (red X in status column).
2. Click "Remediate" button or press `r` key.
3. Confirmation modal appears with policy title, ID, and warning: "This will create a snapshot and apply changes."
4. Press `Enter`/`y` to confirm or `Esc`/`n` to cancel.
5. On confirmation, modal displays "REMEDIATING..." (blocking operation, 500ms-5s depending on policy complexity).
6. Remediation executes: saves BEFORE snapshot → applies changes (registry write, secedit INF generation, sysctl write, file edit, service control) → saves AFTER snapshot.
7. Status updates to green checkmark (Pass) or red X (Fail) with error message in logs.
8. For bulk remediation, use multi-select mode: press `m` to enable, `Space` to toggle selections, `b` to open batch menu, choose "Batch Remediate".

**CLI (Terminal)**

Interactive: Press `r` on selected policy in TUI, confirm with `Enter`. Status updates inline.

Non-interactive: `sudo nogap-cli remediate --policies policies.yaml --id A.1.a.i --yes` (Linux) or `nogap-cli.exe remediate --policies policies.yaml --id A.1.a.i --yes` (Windows, run as Administrator). The `--yes` flag skips confirmation, critical for automation. Without `--yes`, prompts "Proceed? (y/N):" on stdin.

**Privilege Requirements**: Remediation requires admin (Windows) or root (Linux). Dashboard prompts for password on macOS. CLI must be launched with `sudo` (Linux/macOS) or from elevated command prompt (Windows). Without privileges, returns error: "Privileges required".

## Rollback

**GUI (Dashboard)**

Rollback UI is not yet implemented in v1.0.0. Snapshots are created automatically during remediation but must be restored manually via CLI.

**CLI (Terminal)**

1. Browse snapshots: Press `S` in TUI to open snapshot browser (displays ID, timestamp, description sorted by ID descending).
2. Select snapshot with `j`/`k`, press `Enter` to preview BEFORE/AFTER states in scrollable viewer.
3. Press `d` to view diff (side-by-side comparison with added/removed/changed highlighting).
4. To rollback, exit TUI and run (not yet exposed in v1.0.0): `sudo nogap-cli rollback --policy-id A.1.a.i` (hypothetical command—requires implementation).

**Manual Rollback**: Query snapshot database directly:
```bash
sqlite3 snapshots.db "SELECT id, policy_id, timestamp, description FROM snapshots ORDER BY id DESC LIMIT 10;"
sqlite3 snapshots.db "SELECT before_state FROM snapshots WHERE policy_id='A.1.a.i' ORDER BY id DESC LIMIT 1;" > state.json
# Manually parse state.json and apply via platform tools (reg.exe, sysctl, etc.)
```

Automatic rollback via `engine::rollback()` supports 6 check types: `registry_key`, `local_policy`, `service_status`, `sysctl_key`, `file_content`, `file_permissions`. Unsupported types require manual intervention.

## Automation with --json

**CI/CD Integration**

```bash
# Run audit in headless mode
sudo nogap-cli audit --policies /etc/nogap/policies.yaml --json > audit_$(date +%Y%m%d_%H%M%S).json

# Check compliance score
SCORE=$(jq '.compliance_score' audit_*.json)
if (( $(echo "$SCORE < 80" | bc -l) )); then
  echo "Compliance below 80%: $SCORE"
  exit 1
fi

# List failed policies
jq -r '.results[] | select(.passed == false) | "\(.policy_id): \(.message)"' audit_*.json

# Auto-remediate specific policy
sudo nogap-cli remediate --policies /etc/nogap/policies.yaml --id B.2.a.i --yes
```

**JSON Output Schema**:
```json
{
  "timestamp": "2025-11-23T10:30:45Z",
  "compliance_score": 87.5,
  "results": [
    {
      "policy_id": "A.1.a.i",
      "passed": true,
      "message": "Password history set to 24"
    },
    {
      "policy_id": "B.2.a.i",
      "passed": false,
      "message": "IP forwarding enabled (expected: disabled)"
    }
  ]
}
```

**Error Handling**: On failure (policy load error, audit crash, JSON serialization error), outputs `{"error": "description"}` and exits with code 1. Monitoring systems should check both exit code and parse JSON for `error` field.

**Scheduled Audits**: Integrate with cron (Linux) or Task Scheduler (Windows):
```bash
# Linux crontab: Daily audit at 2 AM
0 2 * * * /usr/bin/nogap-cli audit --policies /etc/nogap/policies.yaml --json >> /var/log/nogap/audit.log 2>&1

# Windows Task Scheduler: Weekly audit every Sunday 3 AM
schtasks /create /tn "NoGap Audit" /tr "C:\Program Files\NoGap\nogap-cli.exe audit --policies C:\ProgramData\NoGap\policies.yaml --json > C:\Logs\nogap_audit.json" /sc weekly /d SUN /st 03:00
```

**Stage 5 documentation complete.**
