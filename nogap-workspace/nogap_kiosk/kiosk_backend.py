#!/usr/bin/env python3
"""
NoGap Kiosk Backend
Manages remote policy audits and CSV report collection via WinRM and SSH
"""

import os
import sys
import json
import logging
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Optional
import subprocess
import shutil

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler('kiosk.log'),
        logging.StreamHandler(sys.stdout)
    ]
)
logger = logging.getLogger('nogap-kiosk')


class RemoteHost:
    """Represents a remote host configuration"""
    
    def __init__(self, hostname: str, platform: str, address: str, 
                 username: str, password: Optional[str] = None,
                 key_path: Optional[str] = None):
        self.hostname = hostname
        self.platform = platform  # 'windows' or 'linux'
        self.address = address
        self.username = username
        self.password = password
        self.key_path = key_path


class KioskBackend:
    """Main kiosk backend for CSV report collection"""
    
    def __init__(self, reports_dir: str = "kiosk_reports"):
        self.reports_dir = Path(reports_dir)
        self.reports_dir.mkdir(exist_ok=True)
        logger.info(f"Kiosk initialized with reports directory: {self.reports_dir}")
    
    def fetch_csv_from_windows(self, host: RemoteHost) -> bool:
        """
        Fetch CSV report from Windows host via WinRM
        Downloads C:\\nogap\\report.csv
        """
        logger.info(f"Fetching CSV from Windows host: {host.hostname}")
        
        try:
            # Create host-specific directory
            host_dir = self.reports_dir / host.hostname
            host_dir.mkdir(exist_ok=True)
            
            local_csv = host_dir / "report.csv"
            remote_csv = "C:\\nogap\\report.csv"
            
            # Use pywinrm for WinRM connection
            try:
                import winrm
                
                session = winrm.Session(
                    f'http://{host.address}:5985/wsman',
                    auth=(host.username, host.password),
                    transport='ntlm'
                )
                
                # Read remote file content
                script = f'Get-Content -Path "{remote_csv}" -Raw'
                result = session.run_ps(script)
                
                if result.status_code == 0:
                    # Write content to local file
                    local_csv.write_text(result.std_out.decode('utf-8'))
                    logger.info(f"✓ CSV fetched from {host.hostname}: {local_csv}")
                    return True
                else:
                    error_msg = result.std_err.decode('utf-8')
                    logger.error(f"WinRM command failed: {error_msg}")
                    return False
                    
            except ImportError:
                logger.warning("pywinrm not installed, falling back to command-line tools")
                # Fallback: use command-line WinRS or other tools
                return self._fetch_windows_fallback(host, remote_csv, local_csv)
                
        except Exception as e:
            logger.error(f"Failed to fetch CSV from {host.hostname}: {e}")
            return False
    
    def _fetch_windows_fallback(self, host: RemoteHost, remote_csv: str, 
                                 local_csv: Path) -> bool:
        """Fallback method for Windows CSV fetch using command-line tools"""
        try:
            # Try using smbclient or net use
            logger.info(f"Attempting fallback fetch from {host.hostname}")
            
            # This is a placeholder - actual implementation depends on available tools
            # Could use: smbclient, net use, or custom PowerShell remoting
            
            logger.warning("Fallback fetch not implemented - install pywinrm for WinRM support")
            return False
            
        except Exception as e:
            logger.error(f"Fallback fetch failed: {e}")
            return False
    
    def fetch_csv_from_linux(self, host: RemoteHost) -> bool:
        """
        Fetch CSV report from Linux host via SSH
        Downloads /opt/nogap/report.csv
        """
        logger.info(f"Fetching CSV from Linux host: {host.hostname}")
        
        try:
            # Create host-specific directory
            host_dir = self.reports_dir / host.hostname
            host_dir.mkdir(exist_ok=True)
            
            local_csv = host_dir / "report.csv"
            remote_csv = "/opt/nogap/report.csv"
            
            # Build SCP command
            if host.key_path:
                # Use SSH key authentication
                cmd = [
                    'scp',
                    '-i', host.key_path,
                    '-o', 'StrictHostKeyChecking=no',
                    f'{host.username}@{host.address}:{remote_csv}',
                    str(local_csv)
                ]
            else:
                # Use password authentication (requires sshpass)
                cmd = [
                    'sshpass', '-p', host.password,
                    'scp',
                    '-o', 'StrictHostKeyChecking=no',
                    f'{host.username}@{host.address}:{remote_csv}',
                    str(local_csv)
                ]
            
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=30
            )
            
            if result.returncode == 0 and local_csv.exists():
                logger.info(f"✓ CSV fetched from {host.hostname}: {local_csv}")
                return True
            else:
                logger.error(f"SCP failed: {result.stderr}")
                return False
                
        except subprocess.TimeoutExpired:
            logger.error(f"Timeout fetching CSV from {host.hostname}")
            return False
        except Exception as e:
            logger.error(f"Failed to fetch CSV from {host.hostname}: {e}")
            return False
    
    def fetch_csv_from_host(self, host: RemoteHost) -> bool:
        """Fetch CSV from host based on platform"""
        if host.platform.lower() == 'windows':
            return self.fetch_csv_from_windows(host)
        elif host.platform.lower() == 'linux':
            return self.fetch_csv_from_linux(host)
        else:
            logger.error(f"Unknown platform: {host.platform}")
            return False
    
    def process_hosts(self, hosts: List[RemoteHost]) -> Dict[str, bool]:
        """Process multiple hosts and fetch CSV reports"""
        results = {}
        
        for host in hosts:
            logger.info(f"Processing host: {host.hostname}")
            success = self.fetch_csv_from_host(host)
            results[host.hostname] = success
        
        return results
    
    def generate_summary_csv(self) -> Path:
        """
        Generate summary CSV from all collected reports
        Columns: hostname,compliance_score,passed,failed,high,medium,low,timestamp
        """
        logger.info("Generating summary CSV...")
        
        summary_csv = self.reports_dir / "summary.csv"
        timestamp = datetime.utcnow().isoformat() + 'Z'
        
        try:
            import csv
            
            with open(summary_csv, 'w', newline='') as f:
                writer = csv.writer(f)
                writer.writerow([
                    'hostname',
                    'compliance_score',
                    'passed',
                    'failed',
                    'high',
                    'medium',
                    'low',
                    'timestamp'
                ])
                
                # Process each host directory
                for host_dir in self.reports_dir.iterdir():
                    if host_dir.is_dir():
                        report_csv = host_dir / "report.csv"
                        if report_csv.exists():
                            stats = self._analyze_report(report_csv)
                            writer.writerow([
                                host_dir.name,
                                stats['compliance_score'],
                                stats['passed'],
                                stats['failed'],
                                stats['severity']['high'],
                                stats['severity']['medium'],
                                stats['severity']['low'],
                                timestamp
                            ])
            
            logger.info(f"✓ Summary CSV generated: {summary_csv}")
            return summary_csv
            
        except Exception as e:
            logger.error(f"Failed to generate summary CSV: {e}")
            raise
    
    def _analyze_report(self, csv_path: Path) -> Dict:
        """Analyze individual report CSV and extract statistics"""
        import csv
        
        stats = {
            'passed': 0,
            'failed': 0,
            'total': 0,
            'compliance_score': 0.0,
            'severity': {
                'high': 0,
                'medium': 0,
                'low': 0
            }
        }
        
        try:
            with open(csv_path, 'r') as f:
                reader = csv.DictReader(f)
                for row in reader:
                    stats['total'] += 1
                    
                    # Count pass/fail
                    if row.get('status', '').upper() == 'PASS':
                        stats['passed'] += 1
                    else:
                        stats['failed'] += 1
                        
                        # Count severity for failures
                        severity = row.get('severity', 'medium').lower()
                        if severity in stats['severity']:
                            stats['severity'][severity] += 1
            
            # Calculate compliance score
            if stats['total'] > 0:
                stats['compliance_score'] = round(
                    (stats['passed'] / stats['total']) * 100, 2
                )
            
        except Exception as e:
            logger.warning(f"Error analyzing {csv_path}: {e}")
        
        return stats
    
    def export_to_usb(self, usb_path: str) -> bool:
        """
        Export all CSVs to USB-B drive
        Copies to USB-B/reports/
        """
        logger.info(f"Exporting reports to USB: {usb_path}")
        
        try:
            usb_reports = Path(usb_path) / "reports"
            usb_reports.mkdir(parents=True, exist_ok=True)
            
            # Copy all host directories
            for host_dir in self.reports_dir.iterdir():
                if host_dir.is_dir():
                    dest_dir = usb_reports / host_dir.name
                    if dest_dir.exists():
                        shutil.rmtree(dest_dir)
                    shutil.copytree(host_dir, dest_dir)
                    logger.info(f"✓ Copied {host_dir.name} to USB")
            
            # Copy summary CSV
            summary_csv = self.reports_dir / "summary.csv"
            if summary_csv.exists():
                shutil.copy(summary_csv, usb_reports / "summary.csv")
                logger.info("✓ Copied summary.csv to USB")
            
            logger.info(f"✓ All reports exported to: {usb_reports}")
            return True
            
        except Exception as e:
            logger.error(f"Failed to export to USB: {e}")
            return False


def main():
    """Example usage"""
    # Initialize kiosk
    kiosk = KioskBackend()
    
    # Define hosts (would typically come from config file)
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
            password="password123"
        ),
    ]
    
    # Process all hosts
    results = kiosk.process_hosts(hosts)
    
    # Print results
    for hostname, success in results.items():
        status = "✓" if success else "✗"
        print(f"{status} {hostname}")
    
    # Generate summary
    summary_path = kiosk.generate_summary_csv()
    print(f"\nSummary generated: {summary_path}")
    
    # Optional: Export to USB
    # kiosk.export_to_usb("/media/usb")


if __name__ == "__main__":
    main()
