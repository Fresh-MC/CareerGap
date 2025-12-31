//! Compliance Drift Detection Module
//!
//! Detects when previously compliant policies become non-compliant.
//! Uses existing SQLite snapshot storage for historical audit results.
//!
//! IMPORTANT: This module is READ-ONLY for drift detection.
//! - No autonomous remediation
//! - No automatic scheduling
//! - Drift events are surfaced for user review only

use crate::engine::AuditResult;
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents a compliance state transition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriftType {
    /// Policy was compliant, now non-compliant (ALERT)
    Regression,
    /// Policy was non-compliant, now compliant (IMPROVEMENT)
    Improvement,
    /// Policy status unchanged
    NoChange,
}

impl DriftType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DriftType::Regression => "REGRESSION",
            DriftType::Improvement => "IMPROVEMENT",
            DriftType::NoChange => "NO_CHANGE",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            DriftType::Regression => "🔴",
            DriftType::Improvement => "🟢",
            DriftType::NoChange => "⚪",
        }
    }
}

/// A single drift event for a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftEvent {
    pub policy_id: String,
    pub drift_type: DriftType,
    pub previous_state: bool,
    pub current_state: bool,
    pub previous_message: String,
    pub current_message: String,
    pub detected_at: u64,
}

/// Summary of all drift events from a comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    /// Timestamp when drift detection was performed
    pub timestamp: u64,
    /// ID of the previous audit snapshot used for comparison
    pub previous_audit_id: Option<i64>,
    /// Total policies compared
    pub total_compared: usize,
    /// Policies that regressed (compliant → non-compliant)
    pub regressions: Vec<DriftEvent>,
    /// Policies that improved (non-compliant → compliant)
    pub improvements: Vec<DriftEvent>,
    /// Policies with no change
    pub unchanged_count: usize,
    /// New policies not in previous audit
    pub new_policies: Vec<String>,
    /// Policies removed since previous audit
    pub removed_policies: Vec<String>,
}

impl DriftReport {
    /// Returns true if there are any regressions (security concern)
    pub fn has_regressions(&self) -> bool {
        !self.regressions.is_empty()
    }

    /// Returns the count of regressions
    pub fn regression_count(&self) -> usize {
        self.regressions.len()
    }

    /// Returns the count of improvements
    pub fn improvement_count(&self) -> usize {
        self.improvements.len()
    }
}

/// Stored audit result for historical comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAuditResult {
    pub policy_id: String,
    pub passed: bool,
    pub message: String,
}

/// Gets the path to the drift detection database
fn get_drift_db_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(data_dir) = dirs::data_local_dir() {
            let app_dir = data_dir.join("nogap");
            let _ = std::fs::create_dir_all(&app_dir);
            return app_dir.join("drift_history.db");
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(data_dir) = dirs::data_local_dir() {
            let app_dir = data_dir.join("nogap");
            let _ = std::fs::create_dir_all(&app_dir);
            return app_dir.join("drift_history.db");
        }
    }

    PathBuf::from("drift_history.db")
}

/// Initializes the drift detection database
pub fn init_drift_db() -> SqlResult<Connection> {
    let db_path = get_drift_db_path();
    let conn = Connection::open(db_path)?;

    // Create audit history table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS audit_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            description TEXT
        )",
        [],
    )?;

    // Create audit results table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS audit_results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            audit_id INTEGER NOT NULL,
            policy_id TEXT NOT NULL,
            passed INTEGER NOT NULL,
            message TEXT,
            FOREIGN KEY (audit_id) REFERENCES audit_history(id)
        )",
        [],
    )?;

    // Create drift events table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS drift_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            policy_id TEXT NOT NULL,
            drift_type TEXT NOT NULL,
            previous_state INTEGER NOT NULL,
            current_state INTEGER NOT NULL,
            previous_message TEXT,
            current_message TEXT
        )",
        [],
    )?;

    // Create index for faster lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_audit_results_policy 
         ON audit_results(audit_id, policy_id)",
        [],
    )?;

    Ok(conn)
}

/// Stores audit results for future drift comparison
pub fn store_audit_results(
    conn: &Connection,
    results: &[AuditResult],
    description: Option<&str>,
) -> SqlResult<i64> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Insert audit history record
    conn.execute(
        "INSERT INTO audit_history (timestamp, description) VALUES (?1, ?2)",
        params![timestamp, description],
    )?;

    let audit_id = conn.last_insert_rowid();

    // Insert each audit result
    for result in results {
        conn.execute(
            "INSERT INTO audit_results (audit_id, policy_id, passed, message) 
             VALUES (?1, ?2, ?3, ?4)",
            params![audit_id, result.policy_id, result.passed as i32, result.message],
        )?;
    }

    Ok(audit_id)
}

/// Retrieves the most recent audit results for comparison
pub fn get_latest_audit(conn: &Connection) -> SqlResult<Option<(i64, Vec<StoredAuditResult>)>> {
    let mut stmt = conn.prepare(
        "SELECT id FROM audit_history ORDER BY timestamp DESC LIMIT 1",
    )?;

    let audit_id: Option<i64> = stmt.query_row([], |row| row.get(0)).ok();

    match audit_id {
        Some(id) => {
            let results = get_audit_results(conn, id)?;
            Ok(Some((id, results)))
        }
        None => Ok(None),
    }
}

/// Retrieves audit results for a specific audit ID
pub fn get_audit_results(conn: &Connection, audit_id: i64) -> SqlResult<Vec<StoredAuditResult>> {
    let mut stmt = conn.prepare(
        "SELECT policy_id, passed, message FROM audit_results WHERE audit_id = ?1",
    )?;

    let results = stmt.query_map(params![audit_id], |row| {
        Ok(StoredAuditResult {
            policy_id: row.get(0)?,
            passed: row.get::<_, i32>(1)? != 0,
            message: row.get(2)?,
        })
    })?;

    results.collect()
}

/// Compares current audit results with previous audit to detect drift
pub fn detect_drift(
    conn: &Connection,
    current_results: &[AuditResult],
) -> SqlResult<DriftReport> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut report = DriftReport {
        timestamp,
        previous_audit_id: None,
        total_compared: 0,
        regressions: Vec::new(),
        improvements: Vec::new(),
        unchanged_count: 0,
        new_policies: Vec::new(),
        removed_policies: Vec::new(),
    };

    // Get previous audit results
    let previous = get_latest_audit(conn)?;

    match previous {
        None => {
            // No previous audit - all policies are "new"
            report.new_policies = current_results.iter().map(|r| r.policy_id.clone()).collect();
            report.total_compared = 0;
        }
        Some((prev_id, prev_results)) => {
            report.previous_audit_id = Some(prev_id);

            // Build maps for comparison
            let prev_map: HashMap<&str, &StoredAuditResult> = prev_results
                .iter()
                .map(|r| (r.policy_id.as_str(), r))
                .collect();

            let curr_map: HashMap<&str, &AuditResult> = current_results
                .iter()
                .map(|r| (r.policy_id.as_str(), r))
                .collect();

            // Compare each current result with previous
            for result in current_results {
                match prev_map.get(result.policy_id.as_str()) {
                    Some(prev) => {
                        report.total_compared += 1;

                        let drift_type = match (prev.passed, result.passed) {
                            (true, false) => DriftType::Regression,
                            (false, true) => DriftType::Improvement,
                            _ => DriftType::NoChange,
                        };

                        match drift_type {
                            DriftType::Regression => {
                                report.regressions.push(DriftEvent {
                                    policy_id: result.policy_id.clone(),
                                    drift_type: DriftType::Regression,
                                    previous_state: prev.passed,
                                    current_state: result.passed,
                                    previous_message: prev.message.clone(),
                                    current_message: result.message.clone(),
                                    detected_at: timestamp,
                                });
                            }
                            DriftType::Improvement => {
                                report.improvements.push(DriftEvent {
                                    policy_id: result.policy_id.clone(),
                                    drift_type: DriftType::Improvement,
                                    previous_state: prev.passed,
                                    current_state: result.passed,
                                    previous_message: prev.message.clone(),
                                    current_message: result.message.clone(),
                                    detected_at: timestamp,
                                });
                            }
                            DriftType::NoChange => {
                                report.unchanged_count += 1;
                            }
                        }
                    }
                    None => {
                        // New policy not in previous audit
                        report.new_policies.push(result.policy_id.clone());
                    }
                }
            }

            // Find removed policies (in previous but not current)
            for prev in &prev_results {
                if !curr_map.contains_key(prev.policy_id.as_str()) {
                    report.removed_policies.push(prev.policy_id.clone());
                }
            }
        }
    }

    // Store drift events for historical tracking
    for event in &report.regressions {
        store_drift_event(conn, event)?;
    }
    for event in &report.improvements {
        store_drift_event(conn, event)?;
    }

    Ok(report)
}

/// Stores a drift event in the database
fn store_drift_event(conn: &Connection, event: &DriftEvent) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO drift_events (timestamp, policy_id, drift_type, previous_state, 
         current_state, previous_message, current_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            event.detected_at,
            event.policy_id,
            event.drift_type.as_str(),
            event.previous_state as i32,
            event.current_state as i32,
            event.previous_message,
            event.current_message
        ],
    )?;
    Ok(())
}

/// Retrieves recent drift events from the database
pub fn get_recent_drift_events(conn: &Connection, limit: usize) -> SqlResult<Vec<DriftEvent>> {
    let mut stmt = conn.prepare(
        "SELECT timestamp, policy_id, drift_type, previous_state, current_state,
                previous_message, current_message
         FROM drift_events 
         ORDER BY timestamp DESC 
         LIMIT ?1",
    )?;

    let events = stmt.query_map(params![limit as i64], |row| {
        let drift_type_str: String = row.get(2)?;
        let drift_type = match drift_type_str.as_str() {
            "REGRESSION" => DriftType::Regression,
            "IMPROVEMENT" => DriftType::Improvement,
            _ => DriftType::NoChange,
        };

        Ok(DriftEvent {
            detected_at: row.get(0)?,
            policy_id: row.get(1)?,
            drift_type,
            previous_state: row.get::<_, i32>(3)? != 0,
            current_state: row.get::<_, i32>(4)? != 0,
            previous_message: row.get(5)?,
            current_message: row.get(6)?,
        })
    })?;

    events.collect()
}

/// Lists all audit history entries
pub fn list_audit_history(conn: &Connection, limit: usize) -> SqlResult<Vec<(i64, u64, Option<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, description FROM audit_history 
         ORDER BY timestamp DESC LIMIT ?1",
    )?;

    let history = stmt.query_map(params![limit as i64], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;

    history.collect()
}

/// Formats drift report for CLI display
pub fn format_drift_report(report: &DriftReport) -> String {
    let mut output = String::new();

    output.push_str("\n╔══════════════════════════════════════════════════════════════╗\n");
    output.push_str("║              COMPLIANCE DRIFT DETECTION REPORT               ║\n");
    output.push_str("╠══════════════════════════════════════════════════════════════╣\n");

    if report.has_regressions() {
        output.push_str(&format!(
            "║  🔴 ALERT: {} REGRESSION(S) DETECTED                          \n",
            report.regression_count()
        ));
    } else {
        output.push_str("║  ✅ No regressions detected                                  ║\n");
    }

    output.push_str(&format!(
        "║  Policies compared: {}                                        \n",
        report.total_compared
    ));
    output.push_str(&format!(
        "║  Improvements: {}  |  Unchanged: {}                           \n",
        report.improvement_count(),
        report.unchanged_count
    ));

    if !report.new_policies.is_empty() {
        output.push_str(&format!(
            "║  New policies: {}                                             \n",
            report.new_policies.len()
        ));
    }

    if report.has_regressions() {
        output.push_str("╠══════════════════════════════════════════════════════════════╣\n");
        output.push_str("║  REGRESSIONS (compliant → non-compliant):                    ║\n");
        for event in &report.regressions {
            output.push_str(&format!(
                "║  🔴 {} - {}                \n",
                event.policy_id,
                truncate_string(&event.current_message, 40)
            ));
        }
    }

    if !report.improvements.is_empty() {
        output.push_str("╠══════════════════════════════════════════════════════════════╣\n");
        output.push_str("║  IMPROVEMENTS (non-compliant → compliant):                   ║\n");
        for event in &report.improvements {
            output.push_str(&format!(
                "║  🟢 {} - Now compliant                    \n",
                event.policy_id
            ));
        }
    }

    output.push_str("╚══════════════════════════════════════════════════════════════╝\n");

    output
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        
        conn.execute(
            "CREATE TABLE audit_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                description TEXT
            )",
            [],
        ).unwrap();

        conn.execute(
            "CREATE TABLE audit_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                audit_id INTEGER NOT NULL,
                policy_id TEXT NOT NULL,
                passed INTEGER NOT NULL,
                message TEXT
            )",
            [],
        ).unwrap();

        conn.execute(
            "CREATE TABLE drift_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                policy_id TEXT NOT NULL,
                drift_type TEXT NOT NULL,
                previous_state INTEGER NOT NULL,
                current_state INTEGER NOT NULL,
                previous_message TEXT,
                current_message TEXT
            )",
            [],
        ).unwrap();

        conn
    }

    #[test]
    fn test_store_and_retrieve_audit() {
        let conn = create_test_db();
        
        let results = vec![
            AuditResult {
                policy_id: "A.1".to_string(),
                passed: true,
                message: "Compliant".to_string(),
            },
            AuditResult {
                policy_id: "A.2".to_string(),
                passed: false,
                message: "Non-compliant".to_string(),
            },
        ];

        let audit_id = store_audit_results(&conn, &results, Some("Test audit")).unwrap();
        assert!(audit_id > 0);

        let (id, stored) = get_latest_audit(&conn).unwrap().unwrap();
        assert_eq!(id, audit_id);
        assert_eq!(stored.len(), 2);
    }

    #[test]
    fn test_detect_regression() {
        let conn = create_test_db();
        
        // Store initial audit (A.1 compliant)
        let initial = vec![
            AuditResult {
                policy_id: "A.1".to_string(),
                passed: true,
                message: "Compliant".to_string(),
            },
        ];
        store_audit_results(&conn, &initial, None).unwrap();

        // Current audit (A.1 non-compliant - REGRESSION)
        let current = vec![
            AuditResult {
                policy_id: "A.1".to_string(),
                passed: false,
                message: "Non-compliant".to_string(),
            },
        ];

        let report = detect_drift(&conn, &current).unwrap();

        assert!(report.has_regressions());
        assert_eq!(report.regression_count(), 1);
        assert_eq!(report.regressions[0].policy_id, "A.1");
    }

    #[test]
    fn test_detect_improvement() {
        let conn = create_test_db();
        
        // Store initial audit (A.1 non-compliant)
        let initial = vec![
            AuditResult {
                policy_id: "A.1".to_string(),
                passed: false,
                message: "Non-compliant".to_string(),
            },
        ];
        store_audit_results(&conn, &initial, None).unwrap();

        // Current audit (A.1 compliant - IMPROVEMENT)
        let current = vec![
            AuditResult {
                policy_id: "A.1".to_string(),
                passed: true,
                message: "Compliant".to_string(),
            },
        ];

        let report = detect_drift(&conn, &current).unwrap();

        assert!(!report.has_regressions());
        assert_eq!(report.improvement_count(), 1);
    }

    #[test]
    fn test_no_previous_audit() {
        let conn = create_test_db();
        
        let current = vec![
            AuditResult {
                policy_id: "A.1".to_string(),
                passed: true,
                message: "Compliant".to_string(),
            },
        ];

        let report = detect_drift(&conn, &current).unwrap();

        assert!(!report.has_regressions());
        assert_eq!(report.new_policies.len(), 1);
        assert_eq!(report.total_compared, 0);
    }

    #[test]
    fn test_drift_type_display() {
        assert_eq!(DriftType::Regression.as_str(), "REGRESSION");
        assert_eq!(DriftType::Improvement.as_str(), "IMPROVEMENT");
        assert_eq!(DriftType::Regression.icon(), "🔴");
        assert_eq!(DriftType::Improvement.icon(), "🟢");
    }
}
