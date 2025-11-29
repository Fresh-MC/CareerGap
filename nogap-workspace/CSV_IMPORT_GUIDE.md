# CSV Import and Export Guide

This guide covers the complete CSV reporting functionality across the NoGap platform, including CLI export, kiosk backend collection, and dashboard import.

## Table of Contents

1. [Overview](#overview)
2. [CLI CSV Export](#cli-csv-export)
3. [Kiosk Backend](#kiosk-backend)
4. [Dashboard CSV Import](#dashboard-csv-import)
5. [CSV Format Specification](#csv-format-specification)
6. [USB-B Workflow](#usb-b-workflow)
7. [Validation and Logging](#validation-and-logging)
8. [Troubleshooting](#troubleshooting)

---

## Overview

The CSV reporting system provides a complete pipeline for:
- **CLI**: Export audit/remediation results to CSV files
- **Kiosk**: Collect CSV reports from remote hosts via WinRM/SSH
- **Dashboard**: Import and visualize CSV reports with filtering and pagination

### Architecture

```
┌─────────────┐       ┌─────────────┐       ┌─────────────┐
│   CLI       │       │   Kiosk     │       │  Dashboard  │
│  (Rust)     │──CSV──│  (Python)   │──CSV──│ (JS/Tauri)  │
│             │       │             │       │             │
│ • audit     │       │ • WinRM     │       │ • PapaParse │
│ • remediate │       │ • SSH/SCP   │       │ • Filtering │
│ • USB-B     │       │ • Summary   │       │ • Pagination│
└─────────────┘       └─────────────┘       └─────────────┘
```

---

## CLI CSV Export

### Usage

#### Basic Export (Default Path)
```bash
# Windows: C:\nogap\report.csv
# Linux: /opt/nogap/report.csv
nogap-cli audit --export-csv

nogap-cli remediate <policy-id> --export-csv
```

#### Custom Path
```bash
nogap-cli audit --export-csv /path/to/custom/report.csv

nogap-cli remediate WIN-FW-001 --export-csv C:\reports\firewall.csv
```

#### USB-B Mode (Offline)
```bash
# Automatically detects USB-B and exports to:
# USB-B/reports/<hostname>/report.csv
nogap-cli audit --export-csv
```

### Implementation Details

**File**: `nogap-workspace/nogap_cli/src/main.rs`

**Key Functions**:
- `export_csv_report()` - Writes CSV with header and data rows
- `resolve_csv_path()` - Determines output path (USB-B > default > custom)
- `get_default_csv_path()` - Platform-specific defaults
- `detect_usb_b_mount()` - Scans for USB-B marker file

**Dependencies**:
```toml
csv = "1.3"
hostname = "0.4"
```

**Platform Detection**:
- Windows: Checks drives D-Z for `nogap_usb_repo` marker
- Linux: Checks `/media`, `/mnt`, `/run/media` for marker file

---

## Kiosk Backend

### Installation

```bash
cd nogap-workspace/nogap_kiosk
pip install -r requirements.txt
```

**Requirements**:
- `pywinrm>=0.4.3` - Windows WinRM support
- `paramiko>=3.4.0` - Linux SSH/SCP support

### Usage

```python
from kiosk_backend import KioskBackend, RemoteHost

# Define hosts
hosts = [
    RemoteHost(
        hostname="win-server-01",
        platform="windows",
        address="192.168.1.100",
        username="Administrator",
        password="SecurePass123"
    ),
    RemoteHost(
        hostname="ubuntu-server-01",
        platform="linux",
        address="192.168.1.101",
        username="admin",
        key_path="/home/user/.ssh/id_rsa"
    )
]

# Initialize backend
kiosk = KioskBackend(reports_dir="./kiosk_reports")

# Collect CSVs from all hosts
kiosk.process_hosts(hosts)

# Generate summary CSV
kiosk.generate_summary_csv()

# Export to USB-B
usb_path = "/media/user/USB-B"
kiosk.export_to_usb(usb_path)
```

### Remote Collection Methods

#### Windows (WinRM)
- **Protocol**: HTTP/HTTPS on port 5985/5986
- **Authentication**: Basic, NTLM, Kerberos
- **Command**: `Get-Content C:\nogap\report.csv`
- **Setup**: Run `Enable-PSRemoting` on target

#### Linux (SSH/SCP)
- **Authentication**: SSH key or password
- **Command**: `scp user@host:/opt/nogap/report.csv ./`
- **Setup**: Ensure SSH server running and firewall allows port 22

### Summary CSV Format

**File**: `kiosk_reports/summary.csv`

**Columns**:
```csv
hostname,platform,timestamp,total_policies,passed,failed,compliance_score,high_severity,medium_severity,low_severity
```

**Example**:
```csv
hostname,platform,timestamp,total_policies,passed,failed,compliance_score,high_severity,medium_severity,low_severity
win-server-01,windows,2024-01-15T10:30:00Z,25,20,5,80.0,2,2,1
ubuntu-server-01,linux,2024-01-15T10:35:00Z,30,28,2,93.3,1,0,1
```

---

## Dashboard CSV Import

### Accessing CSV Import

1. Open NoGap Dashboard
2. Click **"📥 Import CSV"** button in header
3. Choose import method:
   - **Select CSV File**: Browse local filesystem
   - **Import from USB**: Auto-detect USB-B and select host

### Features

#### File Selection
- **Format**: Accepts `.csv` files only
- **Validation**: Checks required columns and data format
- **Feedback**: Shows success/error messages with details

#### USB-B Import
- **Auto-detection**: Scans for USB-B marker file
- **Multi-host**: Select from multiple host reports if available
- **Path**: Reads from `USB-B/reports/<hostname>/report.csv`

#### Visualization

**Report Metadata**:
- Hostname (extracted from path/filename)
- Latest timestamp from CSV data
- Total policies, passed/failed counts
- Compliance score percentage

**Severity Summary**:
- High severity count (red card)
- Medium severity count (orange card)
- Low severity count (green card)

**Data Table**:
- Columns: Policy ID | Description | Expected | Actual | Status | Severity | Timestamp
- Sortable headers
- Status color coding (green=PASS, red=FAIL)
- Severity badges with color indicators

#### Filtering
- **Search**: Filter by policy ID or description
- **Status**: Filter by PASS/FAIL
- **Severity**: Filter by high/medium/low

#### Pagination
- 50 rows per page
- Previous/Next navigation
- Page counter display

### Implementation Details

**Files**:
- `nogap_dashboard/src/csv_import.html` - UI structure
- `nogap_dashboard/src/csv_import.js` - Frontend logic with PapaParse
- `nogap_dashboard/src-tauri/src/lib.rs` - Backend commands

**Tauri Commands**:
```rust
// Detect USB-B devices and find CSV reports
#[tauri::command]
fn detect_usb_csv_reports() -> Result<Vec<String>, String>

// Read CSV file contents
#[tauri::command]
fn read_csv_file(path: String) -> Result<String, String>
```

**JavaScript Library**:
- **PapaParse**: CSV parsing with header detection
- **CDN**: `https://cdn.jsdelivr.net/npm/papaparse@5.4.1/papaparse.min.js`

---

## CSV Format Specification

### Required Columns

All CSV files **MUST** contain these exact columns (case-sensitive):

```csv
policy_id,description,expected,actual,status,severity,timestamp
```

### Column Descriptions

| Column | Type | Values | Description |
|--------|------|--------|-------------|
| `policy_id` | String | e.g. "WIN-FW-001" | Unique policy identifier |
| `description` | String | Any text | Human-readable policy description |
| `expected` | String | Any text | Expected state or value |
| `actual` | String | Any text | Actual observed state or value |
| `status` | String | "PASS" or "FAIL" | Compliance status (case-insensitive) |
| `severity` | String | "high", "medium", "low" | Risk severity level (case-insensitive) |
| `timestamp` | String | RFC3339 format | ISO 8601 timestamp (e.g. "2024-01-15T10:30:00Z") |

### Example CSV

```csv
policy_id,description,expected,actual,status,severity,timestamp
WIN-FW-001,Windows Firewall Enabled,enabled,enabled,PASS,high,2024-01-15T10:30:00Z
WIN-FW-002,Inbound Rules Configured,strict,permissive,FAIL,high,2024-01-15T10:30:01Z
WIN-UAC-001,User Account Control,enabled,enabled,PASS,medium,2024-01-15T10:30:02Z
LIN-SSH-001,SSH Root Login Disabled,no,yes,FAIL,high,2024-01-15T10:30:03Z
LIN-FW-001,UFW Firewall Active,active,inactive,FAIL,medium,2024-01-15T10:30:04Z
```

### Validation Rules

1. **Header**: Must contain all 7 required columns in any order
2. **Status**: Must be "PASS" or "FAIL" (case-insensitive)
3. **Severity**: Must be "high", "medium", or "low" (case-insensitive)
4. **Timestamp**: Must be valid RFC3339/ISO 8601 format
5. **Empty File**: CSV must contain at least 1 data row (excluding header)

---

## USB-B Workflow

### Setup USB-B Device

1. **Create Marker File**:
   ```bash
   # Windows
   echo. > E:\nogap_usb_repo
   
   # Linux
   touch /media/usb/nogap_usb_repo
   ```

2. **Create Reports Directory**:
   ```bash
   # Windows
   mkdir E:\reports
   
   # Linux
   mkdir /media/usb/reports
   ```

### CLI Offline Export

```bash
# Insert USB-B device
# CLI automatically detects and exports to:
# USB-B/reports/<hostname>/report.csv

nogap-cli audit --export-csv
```

**Output Path Example**:
- Windows: `E:\reports\DESKTOP-ABC123\report.csv`
- Linux: `/media/usb/reports/ubuntu-server-01/report.csv`

### Kiosk USB Export

```python
kiosk = KioskBackend(reports_dir="./kiosk_reports")
kiosk.process_hosts(hosts)
kiosk.generate_summary_csv()

# Export all reports to USB-B
kiosk.export_to_usb("/media/user/USB-B")
```

**Directory Structure on USB-B**:
```
USB-B/
├── nogap_usb_repo          (marker file)
└── reports/
    ├── win-server-01/
    │   └── report.csv
    ├── ubuntu-server-01/
    │   └── report.csv
    └── summary.csv
```

### Dashboard USB Import

1. Insert USB-B device
2. Open Dashboard → CSV Import
3. Click **"🔌 Import from USB"**
4. Select host from list (if multiple)
5. View imported report

---

## Validation and Logging

### CLI Validation

**File**: `nogap_cli/src/main.rs`

**Checks**:
- Directory creation succeeds
- CSV file writable
- Hostname detection succeeds
- USB-B marker file exists (if applicable)

**Logging**:
```rust
eprintln!("✓ CSV report exported to: {}", csv_path);
eprintln!("✗ Failed to export CSV: {}", error);
```

### Kiosk Validation

**File**: `nogap_kiosk/kiosk_backend.py`

**Checks**:
- WinRM connection successful
- SSH connection successful
- CSV file exists on remote host
- CSV file downloaded successfully
- Summary CSV generation succeeds

**Logging**:
```python
# Configured to both file and stdout
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler('kiosk.log'),
        logging.StreamHandler()
    ]
)

# Example logs
logger.info(f"Successfully fetched CSV from Windows host: {host.hostname}")
logger.error(f"Failed to fetch CSV from {host.hostname}: {error}")
```

### Dashboard Validation

**File**: `nogap_dashboard/src/csv_import.js`

**Checks**:
- All required columns present
- CSV not empty
- Severity values valid (high|medium|low)
- Timestamps parseable as dates

**Logging**:
```javascript
console.log("CSV validation successful");
console.warn(`Warning: ${invalidRows.length} rows have invalid severity values`);
console.error(`CSV Import Error: ${message}`);
```

**User Feedback**:
- ✓ Success: Green banner with record count
- ✗ Error: Red banner with specific error message

---

## Troubleshooting

### CLI Issues

#### "Failed to create directory"
**Cause**: Insufficient permissions  
**Solution**: Run with elevated privileges (Administrator/root)

#### "USB-B not detected"
**Cause**: Marker file missing  
**Solution**: Create `nogap_usb_repo` file at USB root

#### "Failed to get hostname"
**Cause**: Hostname crate error  
**Solution**: Check system hostname configuration

### Kiosk Issues

#### "WinRM connection failed"
**Cause**: WinRM not enabled on target  
**Solution**: Run `Enable-PSRemoting` on Windows host

#### "SSH connection refused"
**Cause**: SSH server not running  
**Solution**: Start SSH service: `sudo systemctl start ssh`

#### "Permission denied copying file"
**Cause**: User lacks read permissions  
**Solution**: Ensure user has read access to `/opt/nogap/report.csv`

#### "pywinrm not found"
**Cause**: Missing Python package  
**Solution**: `pip install pywinrm paramiko`

### Dashboard Issues

#### "No USB-B device detected"
**Cause**: Marker file missing or USB not mounted  
**Solution**: Check USB is mounted and marker file exists

#### "CSV validation failed"
**Cause**: Missing columns or invalid format  
**Solution**: Check CSV has all 7 required columns with correct headers

#### "Failed to read CSV file"
**Cause**: File permissions or path error  
**Solution**: Verify file exists and is readable by dashboard process

#### "Tauri command not found"
**Cause**: Backend not compiled with new commands  
**Solution**: Rebuild dashboard: `cd nogap_dashboard && cargo build`

### General Issues

#### Performance: Large CSV files slow
**Solution**: 
- Use pagination (50 rows per page)
- Filter data before export
- Split into multiple reports by severity

#### Encoding: Special characters corrupted
**Solution**: 
- Ensure UTF-8 encoding
- Escape special characters in CSV
- Use PapaParse with encoding option

---

## Security Considerations

### CLI
- CSV files may contain sensitive system information
- Store reports in secure locations with appropriate permissions
- Consider encrypting USB-B devices

### Kiosk
- Use SSH keys instead of passwords when possible
- Enable HTTPS for WinRM instead of HTTP
- Store credentials securely (environment variables, secrets manager)
- Restrict network access to management subnet

### Dashboard
- Validate all CSV input to prevent injection attacks
- Sanitize HTML output (PapaParse escapes by default)
- Limit file upload size
- Implement user authentication for production deployments

---

## Example Workflows

### Workflow 1: Single Host Audit

```bash
# On target host
nogap-cli audit --export-csv

# Copy CSV to management station
scp C:\nogap\report.csv admin@mgmt:/home/admin/reports/

# Import in dashboard
# Dashboard → Import CSV → Select file
```

### Workflow 2: Multi-Host Collection

```python
# On management station
from kiosk_backend import KioskBackend, RemoteHost

hosts = [
    RemoteHost("host1", "windows", "192.168.1.10", "admin", password="pass"),
    RemoteHost("host2", "linux", "192.168.1.11", "root", key_path="/root/.ssh/id_rsa")
]

kiosk = KioskBackend()
kiosk.process_hosts(hosts)
kiosk.generate_summary_csv()

# View summary.csv in dashboard
```

### Workflow 3: Offline Air-Gapped Environment

```bash
# 1. Prepare USB-B
echo. > E:\nogap_usb_repo
mkdir E:\reports

# 2. On each air-gapped host (insert USB)
nogap-cli audit --export-csv

# 3. Remove USB, connect to internet-connected workstation

# 4. Import in dashboard
# Dashboard → Import from USB → Select host
```

---

## Future Enhancements

- [ ] Automated scheduling for kiosk collections
- [ ] Email notifications for compliance failures
- [ ] Historical trend analysis in dashboard
- [ ] CSV diff comparison between audit runs
- [ ] Export dashboard view to Excel format
- [ ] Real-time dashboard updates via WebSocket
- [ ] Role-based access control for sensitive reports
- [ ] Integration with SIEM systems
- [ ] Custom policy templates for CSV export
- [ ] Bulk remediation from CSV import

---

## Support

For issues or questions:
- Check [Troubleshooting](#troubleshooting) section
- Review logs: `kiosk.log`, browser console, CLI stderr
- Validate CSV format against specification
- Ensure all dependencies installed and up-to-date
