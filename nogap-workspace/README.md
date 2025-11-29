# nogap-workspace/README.md

# NoGap Workspace

This repository contains the NoGap workspace, which includes a Rust library (`nogap_core`), a Rust CLI application (`nogap_cli`), a GUI application (`nogap_dashboard`) built with Tauri, and a Python kiosk backend for remote CSV collection.

## Projects

### nogap_core
- A Rust library that provides core functionality for the NoGap project, including the snapshot engine, YAML parser, and policy manager.

### nogap_cli
- A command-line interface for NoGap that wraps the functionality of `nogap_core`. It provides commands such as:
  - `audit`: Audit the current state with optional CSV export
  - `remediate`: Apply policy remediations with optional CSV export
  - `--export-csv <path>`: Export audit/remediation results to CSV format

### nogap_dashboard
- A GUI application built with Tauri that provides a user-friendly interface for interacting with NoGap functionalities. Features include:
  - Policy audit and remediation
  - HTML/PDF report generation
  - **CSV export and import** with filtering and visualization
  - USB-B device management for offline operations
  - Playwright tests for UI automation

### nogap_kiosk
- A Python backend for collecting CSV reports from remote hosts via:
  - **WinRM** (Windows hosts)
  - **SSH/SCP** (Linux hosts)
  - Summary CSV generation
  - USB-B export for air-gapped environments

## CSV Reporting Features

The NoGap platform provides comprehensive CSV reporting capabilities across all components. For complete documentation, see [CSV_IMPORT_GUIDE.md](./CSV_IMPORT_GUIDE.md).

### Quick Start

**CLI CSV Export**:
```bash
# Export audit results to default location
nogap-cli audit --export-csv

# Export to custom path
nogap-cli audit --export-csv /path/to/report.csv

# Export with USB-B auto-detection
nogap-cli audit --export-csv  # Automatically detects USB-B
```

**Kiosk Remote Collection**:
```python
from kiosk_backend import KioskBackend, RemoteHost

hosts = [
    RemoteHost("host1", "windows", "192.168.1.10", "admin", password="pass"),
    RemoteHost("host2", "linux", "192.168.1.11", "root", key_path="/root/.ssh/id_rsa")
]

kiosk = KioskBackend()
kiosk.process_hosts(hosts)
kiosk.generate_summary_csv()
```

**Dashboard CSV Import**:
1. Open NoGap Dashboard
2. Click **"📥 Import CSV"** button
3. Select CSV file or import from USB-B
4. View results with filtering and pagination

## Getting Started

To get started with the NoGap workspace, clone the repository and run the following commands:

```bash
# Navigate to the workspace directory
cd nogap-workspace

# Build the projects
cargo build

# Run the CLI application
cargo run --bin nogap_cli
```

## Testing

To run the Playwright tests for the GUI application, navigate to the `nogap_dashboard` directory and run:

```bash
# Navigate to the dashboard directory
cd nogap_dashboard

# Install dependencies
npm install

# Run Playwright tests
npx playwright test
```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any improvements or features you'd like to add.