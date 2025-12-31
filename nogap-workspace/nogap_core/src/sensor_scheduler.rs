/// Autonomous Sensing Loop - Background audit scheduler
///
/// This module implements the "Sense" component of the agentic system.
/// It provides continuous or scheduled observation of system state WITHOUT
/// decision-making, remediation, or AI reasoning.
///
/// Key responsibilities:
/// - Run background audits on a configurable schedule
/// - Capture and store state snapshots
/// - Detect compliance drift (observation only)
/// - Provide structured logging for observability
///
/// IMPORTANT: This is SENSE-ONLY. No planning, remediation, or LLM calls.

use crate::engine::{audit, AuditResult};
use crate::snapshot::{init_db, save_snapshot};
use crate::types::Policy;
use rusqlite::Connection;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

/// Configuration for the autonomous sensor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorConfig {
    /// Whether sensing is enabled (default: false)
    pub enabled: bool,
    /// Audit interval in hours
    pub interval_hours: u64,
    /// Maximum number of stored sense events
    pub max_stored_events: usize,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_hours: 24,
            max_stored_events: 100,
        }
    }
}

/// Represents a single automated audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SenseEvent {
    pub timestamp: u64,
    pub audit_count: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub drift_detected: bool,
    pub drift_count: usize,
}

impl SenseEvent {
    fn new(results: &[AuditResult], drift_count: usize) -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = results.len() - passed;
        
        Self {
            timestamp: ts,
            audit_count: results.len(),
            passed_count: passed,
            failed_count: failed,
            drift_detected: drift_count > 0,
            drift_count,
        }
    }
}

/// Raw drift event (no prioritization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftEvent {
    pub timestamp: u64,
    pub policy_id: String,
    pub previous_state: bool,
    pub current_state: bool,
}

/// Background sensor scheduler
pub struct SensorScheduler {
    config: SensorConfig,
    running: Arc<AtomicBool>,
    events: Arc<Mutex<Vec<SenseEvent>>>,
    drift_events: Arc<Mutex<Vec<DriftEvent>>>,
}

impl SensorScheduler {
    /// Create a new sensor scheduler with given configuration
    pub fn new(config: SensorConfig) -> Self {
        log::info!("[SENSOR] Initializing scheduler with config: {:?}", config);
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            events: Arc::new(Mutex::new(Vec::new())),
            drift_events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Check if the sensor is currently running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Start the background sensing loop
    ///
    /// This spawns a background thread that runs audits at the configured interval.
    /// The loop will continue until `stop()` is called.
    ///
    /// # Arguments
    /// * `policies` - The policies to audit
    ///
    /// # Returns
    /// * `Ok(())` if the loop started successfully
    /// * `Err(...)` if the sensor is already running or disabled
    pub fn start(&self, policies: Vec<Policy>) -> Result<(), Box<dyn Error>> {
        if !self.config.enabled {
            return Err("Sensor is disabled in configuration".into());
        }

        if self.running.load(Ordering::Relaxed) {
            return Err("Sensor is already running".into());
        }

        log::info!("[SENSOR] Starting autonomous sensing loop (interval: {}h)", self.config.interval_hours);
        println!("🔍 Autonomous sensor starting (interval: {}h)", self.config.interval_hours);

        self.running.store(true, Ordering::Relaxed);

        let running = Arc::clone(&self.running);
        let events = Arc::clone(&self.events);
        let _drift_events = Arc::clone(&self.drift_events);
        let interval = Duration::from_secs(self.config.interval_hours * 3600);
        let max_stored = self.config.max_stored_events;

        // Spawn background thread
        thread::spawn(move || {
            log::info!("[SENSOR] Background thread started");
            
            while running.load(Ordering::Relaxed) {
                log::info!("[SENSOR] Starting scheduled audit");
                println!("🔍 [SENSOR] Audit started at {}", format_timestamp());

                // Run audit (reuse existing engine)
                match run_sense_audit(&policies) {
                    Ok((results, drift_count)) => {
                        // Create sense event
                        let event = SenseEvent::new(&results, drift_count);
                        log::info!("[SENSOR] Audit completed: {} total, {} passed, {} failed, {} drifted",
                            event.audit_count, event.passed_count, event.failed_count, event.drift_count);
                        println!("✅ [SENSOR] Audit completed: {} policies, {} drifted", 
                            event.audit_count, event.drift_count);

                        // Store event (bounded)
                        {
                            let mut events_guard = events.lock().unwrap();
                            events_guard.push(event);
                            if events_guard.len() > max_stored {
                                events_guard.remove(0); // Remove oldest
                            }
                        }

                        log::info!("[SENSOR] Snapshot stored with source=agent_sense");
                        println!("📸 [SENSOR] Snapshot stored");
                    }
                    Err(e) => {
                        log::error!("[SENSOR] Audit failed: {}", e);
                        eprintln!("❌ [SENSOR] Audit failed: {}", e);
                    }
                }

                // Sleep until next interval
                log::info!("[SENSOR] Sleeping for {} hours", interval.as_secs() / 3600);
                println!("😴 [SENSOR] Next audit in {} hours", interval.as_secs() / 3600);
                
                // Break sleep into smaller chunks to allow responsive shutdown
                let sleep_chunks = 60; // Check every minute
                for _ in 0..sleep_chunks {
                    if !running.load(Ordering::Relaxed) {
                        break;
                    }
                    thread::sleep(Duration::from_secs(interval.as_secs() / sleep_chunks));
                }
            }

            log::info!("[SENSOR] Background thread stopped");
            println!("🛑 [SENSOR] Sensing loop stopped");
        });

        Ok(())
    }

    /// Stop the background sensing loop
    pub fn stop(&self) {
        if self.running.load(Ordering::Relaxed) {
            log::info!("[SENSOR] Stopping sensing loop");
            self.running.store(false, Ordering::Relaxed);
        }
    }

    /// Get all stored sense events
    pub fn get_events(&self) -> Vec<SenseEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Get all stored drift events
    pub fn get_drift_events(&self) -> Vec<DriftEvent> {
        self.drift_events.lock().unwrap().clone()
    }

    /// Clear all stored events (for testing)
    pub fn clear_events(&self) {
        self.events.lock().unwrap().clear();
        self.drift_events.lock().unwrap().clear();
    }
}

/// Run a single sense audit (internal helper)
///
/// This function:
/// 1. Runs the audit using the existing engine
/// 2. Stores the results in a snapshot tagged with "agent_sense"
/// 3. Compares against previous snapshot to detect drift
/// 4. Returns results and drift count
///
/// # Safety
/// - Fails safely if audit requires elevated privileges
/// - Does not modify system state
/// - Does not perform remediation
fn run_sense_audit(policies: &[Policy]) -> Result<(Vec<AuditResult>, usize), Box<dyn Error>> {
    // Run audit (reuse existing engine - no duplication)
    let results = match audit(policies) {
        Ok(r) => r,
        Err(e) => {
            log::warn!("[SENSOR] Audit failed (possibly due to insufficient privileges): {}", e);
            // Fail safely - return empty results rather than crashing
            return Ok((Vec::new(), 0));
        }
    };

    // Store snapshot with agent_sense tag
    let drift_count = store_sense_snapshot(&results)?;

    Ok((results, drift_count))
}

/// Store a sense audit snapshot with proper tagging
///
/// This stores the audit results in the existing snapshot database,
/// tagged with source="agent_sense" to distinguish from manual audits.
///
/// # Returns
/// Number of policies that drifted since last sense audit
fn store_sense_snapshot(results: &[AuditResult]) -> Result<usize, Box<dyn Error>> {
    // Initialize DB connection
    let conn = init_db()?;

    // Serialize results
    let results_json = serde_json::to_string(results)?;
    
    // Get previous sense snapshot for drift detection
    let previous_results = load_previous_sense_snapshot(&conn)?;
    
    // Detect drift (simple comparison)
    let drift_count = if let Some(prev) = previous_results {
        detect_simple_drift(&prev, results)
    } else {
        0 // No previous snapshot, no drift
    };

    // Save new snapshot with agent_sense tag
    save_snapshot(
        &conn,
        Some("agent_sense"),
        "Automated audit by sensor scheduler",
        &results_json,
        &results_json,
    )?;

    Ok(drift_count)
}

/// Load the previous sense snapshot from database
fn load_previous_sense_snapshot(conn: &Connection) -> Result<Option<Vec<AuditResult>>, Box<dyn Error>> {
    let mut stmt = conn.prepare(
        "SELECT after_state FROM snapshots 
         WHERE policy_id = 'agent_sense' 
         ORDER BY timestamp DESC 
         LIMIT 1"
    )?;

    let result = stmt.query_row([], |row| {
        let json_str: String = row.get(0)?;
        Ok(json_str)
    });

    match result {
        Ok(json_str) => {
            let results: Vec<AuditResult> = serde_json::from_str(&json_str)?;
            Ok(Some(results))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Detect drift between two audit result sets
///
/// This is a simple comparison that counts how many policies changed state.
/// This is SENSE-ONLY - no prioritization, scoring, or reasoning.
fn detect_simple_drift(previous: &[AuditResult], current: &[AuditResult]) -> usize {
    let mut drift_count = 0;

    for curr in current {
        if let Some(prev) = previous.iter().find(|p| p.policy_id == curr.policy_id) {
            if prev.passed != curr.passed {
                drift_count += 1;
                log::info!("[SENSOR] Drift detected: {} changed from {} to {}", 
                    curr.policy_id, 
                    if prev.passed { "PASS" } else { "FAIL" },
                    if curr.passed { "PASS" } else { "FAIL" }
                );
            }
        }
    }

    drift_count
}

/// Format current timestamp for display
fn format_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    format!("{}", now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_config_default() {
        let config = SensorConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.interval_hours, 24);
        assert_eq!(config.max_stored_events, 100);
    }

    #[test]
    fn test_sensor_creation() {
        let config = SensorConfig {
            enabled: true,
            interval_hours: 1,
            max_stored_events: 50,
        };
        let scheduler = SensorScheduler::new(config);
        assert!(!scheduler.is_running());
    }

    #[test]
    fn test_sensor_disabled() {
        let config = SensorConfig::default(); // disabled by default
        let scheduler = SensorScheduler::new(config);
        let policies = vec![];
        
        let result = scheduler.start(policies);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("disabled"));
    }

    #[test]
    fn test_drift_detection() {
        let prev = vec![
            AuditResult {
                policy_id: "test1".to_string(),
                passed: true,
                message: "ok".to_string(),
            },
            AuditResult {
                policy_id: "test2".to_string(),
                passed: false,
                message: "fail".to_string(),
            },
        ];

        let curr = vec![
            AuditResult {
                policy_id: "test1".to_string(),
                passed: false, // Changed
                message: "fail".to_string(),
            },
            AuditResult {
                policy_id: "test2".to_string(),
                passed: false, // Same
                message: "fail".to_string(),
            },
        ];

        let drift = detect_simple_drift(&prev, &curr);
        assert_eq!(drift, 1);
    }

    #[test]
    fn test_sense_event_creation() {
        let results = vec![
            AuditResult {
                policy_id: "test1".to_string(),
                passed: true,
                message: "ok".to_string(),
            },
            AuditResult {
                policy_id: "test2".to_string(),
                passed: false,
                message: "fail".to_string(),
            },
        ];

        let event = SenseEvent::new(&results, 1);
        assert_eq!(event.audit_count, 2);
        assert_eq!(event.passed_count, 1);
        assert_eq!(event.failed_count, 1);
        assert_eq!(event.drift_count, 1);
        assert!(event.drift_detected);
    }
}
