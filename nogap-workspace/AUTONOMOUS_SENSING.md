# Autonomous Sensing Loop

## Overview

The **Autonomous Sensing Loop** is the "Sense" component of NoGap's agentic architecture. It provides continuous, scheduled observation of system security state **without decision-making, remediation, or AI reasoning**.

This is pure observation—detecting what's happening on your system without taking action.

## Key Characteristics

### What It Does ✅
- **Background Audits**: Runs security audits automatically on a configurable schedule
- **State Capture**: Records audit results in timestamped snapshots
- **Drift Detection**: Identifies when compliance state changes over time
- **Structured Logging**: Provides observability into system state changes
- **Headless Operation**: Can run without UI for server environments

### What It Does NOT Do ❌
- **No Decision-Making**: Sensing does not evaluate or prioritize findings
- **No Remediation**: Sensing does not modify system configuration
- **No AI/LLM Calls**: Purely deterministic observation logic
- **No Planning**: Sensing does not recommend actions

## Architecture

```
┌─────────────────────────────────────────────────────┐
│           Autonomous Sensing Loop (SENSE)           │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌──────────────┐      ┌──────────────┐           │
│  │  Scheduler   │─────▶│ Audit Engine │           │
│  │ (Background) │      │  (Existing)  │           │
│  └──────────────┘      └──────┬───────┘           │
│         │                     │                    │
│         │                     ▼                    │
│         │              ┌──────────────┐            │
│         │              │  Snapshot DB │            │
│         │              │  (Tagged)    │            │
│         │              └──────┬───────┘            │
│         │                     │                    │
│         │                     ▼                    │
│         │              ┌──────────────┐            │
│         └─────────────▶│ Drift Events │            │
│                        │  (Raw Count) │            │
│                        └──────────────┘            │
│                                                     │
└─────────────────────────────────────────────────────┘
```

## Usage

### Starting the Sensing Loop

```bash
# Start with default 24-hour interval
nogap-cli sense-start

# Custom interval (12 hours)
nogap-cli sense-start --interval 12

# Use custom policy file
nogap-cli sense-start --policies /path/to/policies.yaml --interval 6
```

The sensing loop will:
1. Prompt for confirmation
2. Load policies from the specified file
3. Start background thread running audits every N hours
4. Log all activity with structured output
5. Run indefinitely until Ctrl+C

### Viewing Status

```bash
# Text output
nogap-cli sense-status

# JSON output for automation
nogap-cli sense-status --json
```

### Stopping the Loop

Press `Ctrl+C` to gracefully stop the sensing loop.

## Configuration

The sensor is configured via `SensorConfig`:

```rust
use nogap_core::sensor_scheduler::SensorConfig;

let config = SensorConfig {
    enabled: true,              // Must be true to start
    interval_hours: 24,         // Audit every 24 hours
    max_stored_events: 100,     // Keep last 100 events in memory
};
```

### Default Values
- **enabled**: `false` (sensing disabled by default)
- **interval_hours**: `24` (daily audits)
- **max_stored_events**: `100` (bounded memory)

## Data Storage

### Snapshot Database

All sensor audits are stored in the existing snapshot database with a special tag:

```sql
INSERT INTO snapshots (
    policy_id,        -- "agent_sense" for sensor audits
    timestamp,        -- Unix timestamp
    description,      -- "Automated audit by sensor scheduler"
    before_state,     -- JSON of audit results
    after_state       -- JSON of audit results (same as before)
)
```

**Location**: 
- Windows: `%LOCALAPPDATA%\nogap\snapshots.db`
- Linux/macOS: `~/.local/share/nogap/snapshots.db`

### Event History

The scheduler maintains in-memory event history:

```rust
pub struct SenseEvent {
    pub timestamp: u64,           // When audit ran
    pub audit_count: usize,       // Total policies checked
    pub passed_count: usize,      // Policies that passed
    pub failed_count: usize,      // Policies that failed
    pub drift_detected: bool,     // Did state change?
    pub drift_count: usize,       // How many changed
}
```

## Drift Detection

The sensor performs simple drift detection by comparing consecutive audit runs:

### Detection Logic
```
For each policy in current audit:
    Find same policy in previous audit
    If compliance state changed (pass↔fail):
        Increment drift count
        Log drift event
```

### What Counts as Drift
- Policy passes → fails
- Policy fails → passes

### What Does NOT Count
- Policy remains passing
- Policy remains failing
- New policies added
- Policies removed

### Example Output
```
[SENSOR] Drift detected: A.1.a.i changed from PASS to FAIL
[SENSOR] Drift detected: B.2.c.iv changed from FAIL to PASS
[SENSOR] Audit completed: 434 policies, 2 drifted
```

## Safety and Privileges

### Fail-Safe Behavior

If an audit requires elevated privileges and fails:
```rust
// Sensor catches error and continues
[SENSOR] Audit failed (possibly due to insufficient privileges): Permission denied
```

The sensor will:
- Log the error
- Return empty results for that cycle
- Continue running (doesn't crash)
- Try again on next scheduled interval

### Running Without Privileges

The sensor can run unprivileged, but some audits may fail:
- **Windows**: Registry and security policy checks require admin
- **Linux**: System file permissions and kernel settings need root

**Recommendation**: Run sensor as:
- Windows: Administrator
- Linux: root or sudo

## Observability

### Structured Logging

All sensor operations emit structured logs:

```
[SENSOR] Initializing scheduler with config: SensorConfig { enabled: true, interval_hours: 24, max_stored_events: 100 }
[SENSOR] Starting autonomous sensing loop (interval: 24h)
[SENSOR] Background thread started
[SENSOR] Starting scheduled audit
[SENSOR] Audit started at 1735564800
[SENSOR] Audit completed: 434 total, 420 passed, 14 failed, 2 drifted
[SENSOR] Snapshot stored with source=agent_sense
[SENSOR] Sleeping for 24 hours
```

### Console Output

User-friendly progress updates:

```
🔍 [SENSOR] Audit started at 1735564800
✅ [SENSOR] Audit completed: 434 policies, 2 drifted
📸 [SENSOR] Snapshot stored
😴 [SENSOR] Next audit in 24 hours
```

## Integration Examples

### Example 1: 24/7 Security Monitoring

```bash
#!/bin/bash
# Run sensor as system service

nogap-cli sense-start --interval 24 --policies /etc/nogap/policies.yaml
```

### Example 2: Development Environment

```bash
# Quick testing with 1-hour interval
nogap-cli sense-start --interval 1
```

### Example 3: CI/CD Pipeline

```yaml
# .github/workflows/security-audit.yml
name: Autonomous Security Sensing

on:
  schedule:
    - cron: '0 */6 * * *'  # Every 6 hours

jobs:
  sense:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run sensor once
        run: |
          # In CI, run single audit instead of loop
          cargo run --bin nogap-cli -- audit --json
```

### Example 4: Programmatic Integration

```rust
use nogap_core::sensor_scheduler::{SensorConfig, SensorScheduler};
use nogap_core::policy_parser;

fn start_background_sensing() -> Result<(), Box<dyn std::error::Error>> {
    // Load policies
    let policies = policy_parser::load_policy("policies.yaml")?;
    
    // Configure sensor
    let config = SensorConfig {
        enabled: true,
        interval_hours: 12,  // Every 12 hours
        max_stored_events: 50,
    };
    
    // Start scheduler
    let scheduler = SensorScheduler::new(config);
    scheduler.start(policies)?;
    
    println!("Sensing started!");
    
    // Keep running (or integrate with your app's lifecycle)
    Ok(())
}
```

## Querying Sense Data

### SQL Queries

Retrieve sense audit history:

```sql
-- Get all sensor audits
SELECT timestamp, description, after_state 
FROM snapshots 
WHERE policy_id = 'agent_sense' 
ORDER BY timestamp DESC;

-- Count sensor audits
SELECT COUNT(*) as total_audits
FROM snapshots
WHERE policy_id = 'agent_sense';

-- Get audits from last 7 days
SELECT timestamp, after_state
FROM snapshots
WHERE policy_id = 'agent_sense'
  AND timestamp > strftime('%s', 'now', '-7 days')
ORDER BY timestamp DESC;
```

### Programmatic Access

```rust
use rusqlite::Connection;
use nogap_core::snapshot::init_db;

fn query_sense_history() -> Result<(), Box<dyn std::error::Error>> {
    let conn = init_db()?;
    
    let mut stmt = conn.prepare(
        "SELECT timestamp, after_state FROM snapshots 
         WHERE policy_id = 'agent_sense' 
         ORDER BY timestamp DESC 
         LIMIT 10"
    )?;
    
    let audits = stmt.query_map([], |row| {
        let ts: u64 = row.get(0)?;
        let state: String = row.get(1)?;
        Ok((ts, state))
    })?;
    
    for audit in audits {
        let (timestamp, state_json) = audit?;
        println!("Audit at {}: {}", timestamp, state_json);
    }
    
    Ok(())
}
```

## Performance Considerations

### Resource Usage
- **CPU**: Minimal (only during audit cycles)
- **Memory**: ~1MB for 100 stored events
- **Disk**: ~10KB per audit snapshot (depends on policy count)
- **Network**: None (local-only operation)

### Scalability
- **Policy Count**: Tested with 434 policies
- **Interval**: Recommended minimum 1 hour
- **Storage**: Snapshots are compressed SQLite (efficient)

### Recommendations
- **Development**: 1-hour intervals for rapid feedback
- **Production**: 12-24 hour intervals for stability monitoring
- **High-Security**: 6-hour intervals for compliance tracking

## Roadmap: Future Agentic Capabilities

This sensor is the foundation for future agentic features:

```
Phase 1: SENSE (Current) ✅
  ↓
Phase 2: PLAN (Future)
  - Analyze drift events
  - Prioritize remediation
  - Generate action plans
  ↓
Phase 3: ACT (Future)
  - Execute remediations
  - Verify outcomes
  - Self-correct
```

The sensor intentionally remains isolated from planning/action to maintain:
- **Observability**: Clear view of system state
- **Safety**: No unintended modifications
- **Composability**: Can add reasoning layers later

## Troubleshooting

### Sensor Won't Start

```
❌ Failed to start sensing loop: Sensor is disabled in configuration
```

**Solution**: Ensure `enabled: true` in config or use CLI (it enables automatically).

### Audits Failing

```
[SENSOR] Audit failed (possibly due to insufficient privileges): Permission denied
```

**Solution**: Run with elevated privileges (admin/root).

### No Drift Detected

Drift only counts when policies change state (pass↔fail). If your system is stable, drift_count will be 0.

### Database Locked

```
❌ Failed to store snapshot: database is locked
```

**Solution**: Another process is using the snapshot DB. Stop other NoGap instances.

## Security Considerations

### Data Privacy
- All data stored locally (no external transmission)
- Snapshots may contain sensitive system state
- Protect snapshot database file permissions

### Privilege Escalation
- Sensor runs with permissions of launching user
- Does not attempt privilege escalation
- Fails gracefully if insufficient privileges

### Audit Integrity
- Sensor does not modify audit logic
- Uses same engine as manual audits
- Results are deterministic and reproducible

## Limitations

### Current Limitations
1. **No persistence between restarts**: Scheduler state is in-memory only
2. **No remote monitoring**: CLI-based status only
3. **Basic drift detection**: Simple state change counting
4. **No alerting**: Logs only, no notifications

### Intentional Constraints
1. **No AI/LLM**: Keeps sensing deterministic
2. **No remediation**: Observation only by design
3. **No prioritization**: Raw data, no interpretation

## API Reference

### SensorConfig

```rust
pub struct SensorConfig {
    pub enabled: bool,
    pub interval_hours: u64,
    pub max_stored_events: usize,
}
```

### SensorScheduler

```rust
pub struct SensorScheduler { /* ... */ }

impl SensorScheduler {
    pub fn new(config: SensorConfig) -> Self;
    pub fn is_running(&self) -> bool;
    pub fn start(&self, policies: Vec<Policy>) -> Result<(), Box<dyn Error>>;
    pub fn stop(&self);
    pub fn get_events(&self) -> Vec<SenseEvent>;
    pub fn get_drift_events(&self) -> Vec<DriftEvent>;
    pub fn clear_events(&self);
}
```

### SenseEvent

```rust
pub struct SenseEvent {
    pub timestamp: u64,
    pub audit_count: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub drift_detected: bool,
    pub drift_count: usize,
}
```

## Contributing

When extending the sensor:
- ❌ Do NOT add decision-making logic
- ❌ Do NOT add remediation capabilities
- ❌ Do NOT add LLM/AI calls
- ✅ DO improve observability
- ✅ DO enhance drift detection accuracy
- ✅ DO optimize performance

## License

Same as NoGap project.

---

**Next Steps**: See [`AGENT_ARCHITECTURE.md`](./AGENT_ARCHITECTURE.md) (future) for the complete Plan-Act cycle design.
