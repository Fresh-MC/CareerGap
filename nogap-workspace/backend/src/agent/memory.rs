//! Agent Memory Module
//!
//! Implements persistent timeline storage for all agent actions.
//! Every major agent action (resume upload, plan generation, plan edit, reflection)
//! appends to this timeline for full explainability and auditability.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// ============================================================
// MEMORY EVENT TYPES
// ============================================================

/// Types of events that can be recorded in memory
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryEventType {
    ResumeUploaded,
    PlanGenerated,
    PlanModified,
    StepCompleted,
    StepSkipped,
    GoalSet,
    GoalUpdated,
    ReflectionGenerated,
    CheckpointCreated,
    AssessmentUpdated,
}

impl MemoryEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryEventType::ResumeUploaded => "resume_uploaded",
            MemoryEventType::PlanGenerated => "plan_generated",
            MemoryEventType::PlanModified => "plan_modified",
            MemoryEventType::StepCompleted => "step_completed",
            MemoryEventType::StepSkipped => "step_skipped",
            MemoryEventType::GoalSet => "goal_set",
            MemoryEventType::GoalUpdated => "goal_updated",
            MemoryEventType::ReflectionGenerated => "reflection_generated",
            MemoryEventType::CheckpointCreated => "checkpoint_created",
            MemoryEventType::AssessmentUpdated => "assessment_updated",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "resume_uploaded" => Some(MemoryEventType::ResumeUploaded),
            "plan_generated" => Some(MemoryEventType::PlanGenerated),
            "plan_modified" => Some(MemoryEventType::PlanModified),
            "step_completed" => Some(MemoryEventType::StepCompleted),
            "step_skipped" => Some(MemoryEventType::StepSkipped),
            "goal_set" => Some(MemoryEventType::GoalSet),
            "goal_updated" => Some(MemoryEventType::GoalUpdated),
            "reflection_generated" => Some(MemoryEventType::ReflectionGenerated),
            "checkpoint_created" => Some(MemoryEventType::CheckpointCreated),
            "assessment_updated" => Some(MemoryEventType::AssessmentUpdated),
            _ => None,
        }
    }
}

// ============================================================
// MEMORY EVENT
// ============================================================

/// A single event in the agent's memory timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvent {
    pub id: String,
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: MemoryEventType,
    pub description: String,
    /// Optional structured metadata (JSON)
    pub metadata: Option<serde_json::Value>,
}

impl MemoryEvent {
    pub fn new(user_id: &str, event_type: MemoryEventType, description: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            timestamp: Utc::now(),
            event_type,
            description: description.to_string(),
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

// ============================================================
// CAREER MEMORY
// ============================================================

/// The main memory structure for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerMemory {
    pub user_id: String,
    pub timeline: Vec<MemoryEvent>,
}

impl CareerMemory {
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            timeline: Vec::new(),
        }
    }

    pub fn add_event(&mut self, event: MemoryEvent) {
        self.timeline.push(event);
    }

    /// Get events from the last N days
    pub fn events_since(&self, days: i64) -> Vec<&MemoryEvent> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        self.timeline
            .iter()
            .filter(|e| e.timestamp >= cutoff)
            .collect()
    }

    /// Get events of a specific type
    pub fn events_of_type(&self, event_type: MemoryEventType) -> Vec<&MemoryEvent> {
        self.timeline
            .iter()
            .filter(|e| e.event_type == event_type)
            .collect()
    }
}

// ============================================================
// MEMORY STORE (SQLite-backed)
// ============================================================

/// SQLite-backed persistent memory store
pub struct MemoryStore {
    conn: Arc<Mutex<Connection>>,
}

impl MemoryStore {
    /// Create a new memory store with SQLite backend
    pub fn new(db_path: Option<PathBuf>) -> SqlResult<Self> {
        let path = db_path.unwrap_or_else(|| PathBuf::from("career_memory.db"));
        let conn = Connection::open(path)?;
        
        // Initialize schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_events (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                event_type TEXT NOT NULL,
                description TEXT NOT NULL,
                metadata TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memory_user_id ON memory_events(user_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_memory_timestamp ON memory_events(timestamp)",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create an in-memory store for testing
    pub fn in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memory_events (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                event_type TEXT NOT NULL,
                description TEXT NOT NULL,
                metadata TEXT
            )",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Record a new event
    pub fn record_event(&self, event: &MemoryEvent) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let metadata_json = event.metadata.as_ref().map(|m| m.to_string());
        
        conn.execute(
            "INSERT INTO memory_events (id, user_id, timestamp, event_type, description, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                event.id,
                event.user_id,
                event.timestamp.to_rfc3339(),
                event.event_type.as_str(),
                event.description,
                metadata_json,
            ],
        )?;
        
        Ok(())
    }

    /// Get all events for a user
    pub fn get_user_memory(&self, user_id: &str) -> SqlResult<CareerMemory> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, user_id, timestamp, event_type, description, metadata
             FROM memory_events
             WHERE user_id = ?1
             ORDER BY timestamp ASC"
        )?;

        let events = stmt.query_map([user_id], |row| {
            let event_type_str: String = row.get(3)?;
            let metadata_str: Option<String> = row.get(5)?;
            let timestamp_str: String = row.get(2)?;
            
            Ok(MemoryEvent {
                id: row.get(0)?,
                user_id: row.get(1)?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                event_type: MemoryEventType::from_str(&event_type_str)
                    .unwrap_or(MemoryEventType::CheckpointCreated),
                description: row.get(4)?,
                metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
            })
        })?;

        let timeline: Vec<MemoryEvent> = events.filter_map(|e| e.ok()).collect();
        
        Ok(CareerMemory {
            user_id: user_id.to_string(),
            timeline,
        })
    }

    /// Get recent events (last N days)
    pub fn get_recent_events(&self, user_id: &str, days: i64) -> SqlResult<Vec<MemoryEvent>> {
        let cutoff = (Utc::now() - chrono::Duration::days(days)).to_rfc3339();
        let conn = self.conn.lock().unwrap();
        
        let mut stmt = conn.prepare(
            "SELECT id, user_id, timestamp, event_type, description, metadata
             FROM memory_events
             WHERE user_id = ?1 AND timestamp >= ?2
             ORDER BY timestamp ASC"
        )?;

        let events = stmt.query_map(params![user_id, cutoff], |row| {
            let event_type_str: String = row.get(3)?;
            let metadata_str: Option<String> = row.get(5)?;
            let timestamp_str: String = row.get(2)?;
            
            Ok(MemoryEvent {
                id: row.get(0)?,
                user_id: row.get(1)?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                event_type: MemoryEventType::from_str(&event_type_str)
                    .unwrap_or(MemoryEventType::CheckpointCreated),
                description: row.get(4)?,
                metadata: metadata_str.and_then(|s| serde_json::from_str(&s).ok()),
            })
        })?;

        Ok(events.filter_map(|e| e.ok()).collect())
    }
}

// ============================================================
// HELPER FUNCTIONS
// ============================================================

/// Record a resume upload event
pub fn record_resume_upload(store: &MemoryStore, user_id: &str, filename: &str) -> SqlResult<()> {
    let event = MemoryEvent::new(
        user_id,
        MemoryEventType::ResumeUploaded,
        &format!("Uploaded resume: {}", filename),
    );
    store.record_event(&event)
}

/// Record a plan generation event
pub fn record_plan_generated(store: &MemoryStore, user_id: &str, step_count: usize) -> SqlResult<()> {
    let event = MemoryEvent::new(
        user_id,
        MemoryEventType::PlanGenerated,
        &format!("Generated career roadmap with {} steps", step_count),
    );
    store.record_event(&event)
}

/// Record a plan modification event
pub fn record_plan_modified(store: &MemoryStore, user_id: &str, change_description: &str) -> SqlResult<()> {
    let event = MemoryEvent::new(
        user_id,
        MemoryEventType::PlanModified,
        &format!("Modified roadmap: {}", change_description),
    );
    store.record_event(&event)
}

/// Record a step completion event
pub fn record_step_completed(store: &MemoryStore, user_id: &str, step_title: &str) -> SqlResult<()> {
    let event = MemoryEvent::new(
        user_id,
        MemoryEventType::StepCompleted,
        &format!("Completed: {}", step_title),
    );
    store.record_event(&event)
}

/// Record a reflection generation event
pub fn record_reflection(store: &MemoryStore, user_id: &str, reflection_summary: &str) -> SqlResult<()> {
    let event = MemoryEvent::new(
        user_id,
        MemoryEventType::ReflectionGenerated,
        reflection_summary,
    );
    store.record_event(&event)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_store() {
        let store = MemoryStore::in_memory().unwrap();
        let user_id = "test_user";

        // Record events
        record_resume_upload(&store, user_id, "resume.pdf").unwrap();
        record_plan_generated(&store, user_id, 5).unwrap();
        record_step_completed(&store, user_id, "Learn Rust").unwrap();

        // Retrieve memory
        let memory = store.get_user_memory(user_id).unwrap();
        assert_eq!(memory.timeline.len(), 3);
        assert_eq!(memory.timeline[0].event_type, MemoryEventType::ResumeUploaded);
        assert_eq!(memory.timeline[1].event_type, MemoryEventType::PlanGenerated);
        assert_eq!(memory.timeline[2].event_type, MemoryEventType::StepCompleted);
    }
}
