# NoGap Kiosk Backend

Python backend for NoGap kiosk operations, including remote CSV report collection and summary generation.

## Features

- **Remote CSV Collection**: Fetch CSV reports from Windows (WinRM) and Linux (SSH) hosts
- **Platform-Specific Paths**: 
  - Windows: `C:\nogap\report.csv`
  - Linux: `/opt/nogap/report.csv`
- **Summary Generation**: Auto-generate compliance summary CSV
- **USB Export**: Export all reports to USB-B drive
- **Comprehensive Logging**: Track all operations with detailed logs

## Installation

```bash
# Install required Python packages
pip install pywinrm paramiko sshpass

# Or use requirements.txt
pip install -r requirements.txt
```

## Usage

### Basic Example

```python
from kiosk_backend import KioskBackend, RemoteHost

# Initialize kiosk
kiosk = KioskBackend(reports_dir="kiosk_reports")

# Define hosts
hosts = [
    RemoteHost(
        hostname="server01",
        platform="linux",
        address="192.168.1.10",
        username="admin",
        key_path="/path/to/key.pem"
    ),
    RemoteHost(
        hostname="workstation01",
        platform="windows",
        address="192.168.1.20",
        username="administrator",
        password="SecurePassword123"
    )
]

# Fetch CSV reports from all hosts
results = kiosk.process_hosts(hosts)

# Generate summary CSV
summary_path = kiosk.generate_summary_csv()

# Export to USB-B
kiosk.export_to_usb("/media/usb")
```

### Configuration File

Create `hosts.json`:

```json
{
  "hosts": [
    {
      "hostname": "server01",
      "platform": "linux",
      "address": "192.168.1.10",
      "username": "admin",
      "key_path": "/home/user/.ssh/id_rsa"
    },
    {
      "hostname": "workstation01",
      "platform": "windows",
      "address": "192.168.1.20",
      "username": "administrator",
      "password": "SecurePassword123"
    }
  ]
}
```

## Report Structure

### Per-Host Reports
```
kiosk_reports/
├── server01/
│   └── report.csv
├── workstation01/
│   └── report.csv
└── summary.csv
```

### Individual Report Format (report.csv)
```csv
policy_id,description,expected,actual,status,severity,timestamp
A.1.a.i,Password complexity enabled,Enabled,Disabled,FAIL,high,2025-11-29T12:34:56Z
A.1.a.ii,Password length minimum,14,12,FAIL,medium,2025-11-29T12:34:56Z
```

### Summary Report Format (summary.csv)
```csv
hostname,compliance_score,passed,failed,high,medium,low,timestamp
server01,87.5,14,2,1,1,0,2025-11-29T12:34:56Z
workstation01,92.3,24,2,0,2,0,2025-11-29T12:34:56Z
```

## API Reference

### KioskBackend

#### `__init__(reports_dir: str = "kiosk_reports")`
Initialize kiosk backend with reports directory.

#### `fetch_csv_from_windows(host: RemoteHost) -> bool`
Fetch CSV report from Windows host via WinRM.

#### `fetch_csv_from_linux(host: RemoteHost) -> bool`
Fetch CSV report from Linux host via SSH.

#### `process_hosts(hosts: List[RemoteHost]) -> Dict[str, bool]`
Process multiple hosts and fetch all CSV reports.

#### `generate_summary_csv() -> Path`
Generate summary CSV from all collected reports.

#### `export_to_usb(usb_path: str) -> bool`
Export all reports to USB-B drive.

### RemoteHost

```python
RemoteHost(
    hostname: str,           # Host identifier
    platform: str,           # 'windows' or 'linux'
    address: str,            # IP address or hostname
    username: str,           # SSH/WinRM username
    password: Optional[str], # Password (if not using key)
    key_path: Optional[str]  # SSH key path (Linux only)
)
```

## Windows (WinRM) Setup

Enable WinRM on remote Windows hosts:

```powershell
# Enable WinRM
Enable-PSRemoting -Force

# Configure WinRM
winrm quickconfig

# Allow unencrypted traffic (for testing only)
winrm set winrm/config/service '@{AllowUnencrypted="true"}'

# Set authentication
winrm set winrm/config/service/auth '@{Basic="true"}'
```

## Linux (SSH) Setup

Ensure SSH is enabled and accessible:

```bash
# Enable SSH service
sudo systemctl enable ssh
sudo systemctl start ssh

# Copy SSH key (if using key authentication)
ssh-copy-id admin@192.168.1.10
```

## Logging

All operations are logged to:
- **File**: `kiosk.log`
- **Console**: stdout

Log format:
```
2025-11-29 12:34:56 - nogap-kiosk - INFO - Fetching CSV from Windows host: workstation01
2025-11-29 12:34:57 - nogap-kiosk - INFO - ✓ CSV fetched from workstation01: kiosk_reports/workstation01/report.csv
```

## Error Handling

The backend provides comprehensive error handling:
- Connection timeouts
- Authentication failures
- Missing files
- Invalid CSV formats
- USB export errors

All errors are logged with detailed context.

## Security Considerations

- **Passwords**: Avoid hardcoding passwords; use environment variables or secure vaults
- **SSH Keys**: Use key-based authentication for Linux hosts
- **WinRM**: Use HTTPS and encrypted connections in production
- **Credentials**: Store credentials securely, never in source code

## Dependencies

- Python 3.7+
- `pywinrm`: WinRM support for Windows hosts
- `paramiko`: SSH support (alternative to command-line tools)
- `sshpass`: Password-based SSH (if not using keys)

## Troubleshooting

### WinRM Connection Failed
```bash
# Check WinRM service
winrm enumerate winrm/config/listener

# Test connectivity
Test-NetConnection -ComputerName <host> -Port 5985
```

### SSH Connection Failed
```bash
# Test SSH connectivity
ssh -v admin@192.168.1.10

# Check SSH key permissions
chmod 600 /path/to/key.pem
```

### CSV Not Found
Ensure nogap-cli has been run on remote host with `--export-csv` flag:
```bash
# On remote host
nogap-cli audit --export-csv default
```
