use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::time::{SystemTime, UNIX_EPOCH};

/// Structured diff between two snapshots
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<(String, String, String)>, // (key, old_value, new_value)
}

impl SnapshotDiff {
    /// Creates an empty diff
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            changed: Vec::new(),
        }
    }

    /// Returns true if there are no differences
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }

    /// Pretty-prints the diff
    pub fn display(&self) {
        if self.is_empty() {
            println!("📊 No differences found");
            return;
        }

        println!("📊 Snapshot Diff:");

        if !self.added.is_empty() {
            println!("  ➕ Added ({}):", self.added.len());
            for key in &self.added {
                println!("     • {}", key);
            }
        }

        if !self.removed.is_empty() {
            println!("  ➖ Removed ({}):", self.removed.len());
            for key in &self.removed {
                println!("     • {}", key);
            }
        }

        if !self.changed.is_empty() {
            println!("  🔄 Modified ({}):", self.changed.len());
            for (key, old, new) in &self.changed {
                println!("     • {}: '{}' → '{}'", key, old, new);
            }
        }
    }
}

impl Default for SnapshotDiff {
    fn default() -> Self {
        Self::new()
    }
}

/// Rollback state for a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackState {
    pub policy_id: String,
    pub value: Value,
}

/// Initializes the snapshot database with required schema
pub fn init_db() -> Result<Connection> {
    let conn = Connection::open("snapshots.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS snapshots (
            id INTEGER PRIMARY KEY,
            policy_id TEXT,
            timestamp INTEGER,
            description TEXT,
            before_state TEXT,
            after_state TEXT
        )",
        [],
    )?;
    println!("✅ Snapshot engine initialized");
    Ok(conn)
}

/// Saves a snapshot of system state before and after an operation
///
/// # Arguments
/// * `conn` - Database connection
/// * `policy_id` - Policy identifier (optional for backwards compatibility)
/// * `desc` - Description of the operation
/// * `before` - State before the operation
/// * `after` - State after the operation
pub fn save_snapshot(conn: &Connection, policy_id: Option<&str>, desc: &str, before: &str, after: &str) -> Result<()> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    conn.execute(
        "INSERT INTO snapshots (policy_id, timestamp, description, before_state, after_state)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![policy_id, ts, desc, before, after],
    )?;

    println!("📸 Snapshot saved: {}", desc);
    Ok(())
}

/// Retrieves a snapshot by ID
pub fn get_snapshot(conn: &Connection, id: i64) -> Result<(i64, String, String, String)> {
    let mut stmt = conn.prepare(
        "SELECT timestamp, description, before_state, after_state 
         FROM snapshots WHERE id = ?1",
    )?;

    stmt.query_row(params![id], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
    })
}

/// Lists all snapshots
pub fn list_snapshots(conn: &Connection) -> Result<Vec<(i64, i64, String)>> {
    let mut stmt =
        conn.prepare("SELECT id, timestamp, description FROM snapshots ORDER BY id DESC")?;

    let snapshots = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;

    snapshots.collect()
}

/// Simulates rollback to a previous snapshot
///
/// In a full implementation, this would restore the system state
/// from the snapshot's before_state
pub fn rollback_snapshot(conn: &Connection, id: i64) -> Result<()> {
    let (timestamp, desc, before_state, _) = get_snapshot(conn, id)?;
    println!("🔁 Rolling back snapshot {}: {}", id, desc);
    println!("   Timestamp: {}", timestamp);
    println!("   Restoring state: {}", before_state);
    Ok(())
}

/// Saves a rollback state for a specific policy
///
/// # Arguments
/// * `conn` - Database connection
/// * `policy_id` - Policy identifier
/// * `before_state_json` - JSON representation of state before remediation
pub fn save_rollback(conn: &Connection, policy_id: &str, before_state_json: &str) -> Result<()> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    conn.execute(
        "INSERT INTO snapshots (policy_id, timestamp, description, before_state, after_state)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![policy_id, ts, format!("Rollback point for {}", policy_id), before_state_json, ""],
    )?;

    Ok(())
}

/// Loads the last rollback snapshot for a specific policy
///
/// # Arguments
/// * `conn` - Database connection
/// * `policy_id` - Policy identifier
///
/// # Returns
/// `Some(RollbackState)` if found, `None` otherwise
pub fn load_last_snapshot(conn: &Connection, policy_id: &str) -> Option<RollbackState> {
    let mut stmt = conn.prepare(
        "SELECT policy_id, before_state FROM snapshots 
         WHERE policy_id = ?1 AND before_state != '' 
         ORDER BY timestamp DESC LIMIT 1"
    ).ok()?;

    stmt.query_row(params![policy_id], |row| {
        let pid: String = row.get(0)?;
        let before_json: String = row.get(1)?;
        let value: Value = serde_json::from_str(&before_json).unwrap_or(Value::Null);
        
        Ok(RollbackState {
            policy_id: pid,
            value,
        })
    }).ok()
}

/// Compares two snapshots and produces a structured diff
///
/// Parses the `after_state` of both snapshots as JSON and compares them.
///
/// # Arguments
/// * `conn` - Database connection
/// * `id1` - ID of first snapshot (older)
/// * `id2` - ID of second snapshot (newer)
///
/// # Returns
/// `SnapshotDiff` showing additions, deletions, and modifications
pub fn compare_snapshots(conn: &Connection, id1: i64, id2: i64) -> Result<SnapshotDiff> {
    let (_, _, _, after1) = get_snapshot(conn, id1)?;
    let (_, _, _, after2) = get_snapshot(conn, id2)?;

    // Parse JSON states
    let state1: Map<String, Value> = serde_json::from_str(&after1).unwrap_or_default();
    let state2: Map<String, Value> = serde_json::from_str(&after2).unwrap_or_default();

    let mut diff = SnapshotDiff::new();

    // Find added and changed keys
    for (key, value2) in &state2 {
        match state1.get(key) {
            None => diff.added.push(key.clone()),
            Some(value1) if value1 != value2 => {
                diff.changed
                    .push((key.clone(), value1.to_string(), value2.to_string()));
            }
            _ => {}
        }
    }

    // Find removed keys
    for key in state1.keys() {
        if !state2.contains_key(key) {
            diff.removed.push(key.clone());
        }
    }

    Ok(diff)
}

/// Compares a snapshot's before and after states
///
/// Useful for seeing what changed within a single operation
pub fn diff_snapshot_states(conn: &Connection, id: i64) -> Result<SnapshotDiff> {
    let (_, _, before, after) = get_snapshot(conn, id)?;

    let state_before: Map<String, Value> = serde_json::from_str(&before).unwrap_or_default();
    let state_after: Map<String, Value> = serde_json::from_str(&after).unwrap_or_default();

    let mut diff = SnapshotDiff::new();

    // Find added and changed keys
    for (key, value_after) in &state_after {
        match state_before.get(key) {
            None => diff.added.push(key.clone()),
            Some(value_before) if value_before != value_after => {
                diff.changed.push((
                    key.clone(),
                    value_before.to_string(),
                    value_after.to_string(),
                ));
            }
            _ => {}
        }
    }

    // Find removed keys
    for key in state_before.keys() {
        if !state_after.contains_key(key) {
            diff.removed.push(key.clone());
        }
    }

    Ok(diff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_db() {
        let conn = init_db();
        assert!(conn.is_ok());
    }

    #[test]
    fn test_save_and_retrieve_snapshot() {
        let conn = init_db().unwrap();

        let result = save_snapshot(&conn, Some("TEST.1"), "Test operation", "state_before", "state_after");
        assert!(result.is_ok());

        let snapshots = list_snapshots(&conn).unwrap();
        assert!(!snapshots.is_empty());

        let (id, _, _) = snapshots[0];
        let snapshot = get_snapshot(&conn, id);
        assert!(snapshot.is_ok());

        let (_, desc, before, after) = snapshot.unwrap();
        assert_eq!(desc, "Test operation");
        assert_eq!(before, "state_before");
        assert_eq!(after, "state_after");
    }

    #[test]
    fn test_rollback_snapshot() {
        let conn = init_db().unwrap();
        save_snapshot(&conn, Some("TEST.1"), "Test", "before", "after").unwrap();

        let snapshots = list_snapshots(&conn).unwrap();
        let (id, _, _) = snapshots[0];

        let result = rollback_snapshot(&conn, id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compare_snapshots() {
        let conn = init_db().unwrap();

        // Create two snapshots with JSON states
        let state1 = r#"{"key1": "value1", "key2": "value2"}"#;
        let state2 = r#"{"key1": "modified", "key3": "new"}"#;

        save_snapshot(&conn, Some("TEST.1"), "Snapshot 1", "{}", state1).unwrap();
        save_snapshot(&conn, Some("TEST.2"), "Snapshot 2", "{}", state2).unwrap();

        let snapshots = list_snapshots(&conn).unwrap();
        let id1 = snapshots[1].0; // older
        let id2 = snapshots[0].0; // newer

        let diff = compare_snapshots(&conn, id1, id2).unwrap();

        assert_eq!(diff.added.len(), 1);
        assert!(diff.added.contains(&"key3".to_string()));

        assert_eq!(diff.removed.len(), 1);
        assert!(diff.removed.contains(&"key2".to_string()));

        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].0, "key1");
    }

    #[test]
    fn test_diff_snapshot_states() {
        let conn = init_db().unwrap();

        let before = r#"{"config": "old", "setting": "value"}"#;
        let after = r#"{"config": "new", "feature": "enabled"}"#;

        save_snapshot(&conn, Some("TEST.1"), "Config change", before, after).unwrap();

        let snapshots = list_snapshots(&conn).unwrap();
        let id = snapshots[0].0;

        let diff = diff_snapshot_states(&conn, id).unwrap();

        assert_eq!(diff.added.len(), 1);
        assert!(diff.added.contains(&"feature".to_string()));

        assert_eq!(diff.removed.len(), 1);
        assert!(diff.removed.contains(&"setting".to_string()));

        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].0, "config");
    }

    #[test]
    fn test_snapshot_diff_is_empty() {
        let diff = SnapshotDiff::new();
        assert!(diff.is_empty());

        let mut diff2 = SnapshotDiff::new();
        diff2.added.push("key".to_string());
        assert!(!diff2.is_empty());
    }

    #[test]
    fn test_snapshot_diff_display() {
        let mut diff = SnapshotDiff::new();
        diff.added.push("new_key".to_string());
        diff.removed.push("old_key".to_string());
        diff.changed
            .push(("modified".to_string(), "old".to_string(), "new".to_string()));

        // Should print without panicking
        diff.display();
    }
}
