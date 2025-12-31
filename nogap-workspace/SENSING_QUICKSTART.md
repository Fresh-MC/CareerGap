# Autonomous Sensing - Quick Start Guide

## What is Autonomous Sensing?

Autonomous Sensing is the **"eyes"** of NoGap's agentic system. It continuously monitors your security compliance state **without taking any action**. Think of it as a security camera—it watches and records, but doesn't interfere.

## Key Features

✅ **Automatic Audits**: Runs security checks on a schedule (default: 24 hours)  
✅ **Drift Detection**: Notices when compliance state changes  
✅ **No Manual Work**: Set it and forget it  
✅ **Safe**: Only observes, never modifies your system  
✅ **Lightweight**: Minimal resource usage  

## Quick Start

### 1. Start the Sensor (Default 24-hour interval)
```bash
nogap-cli sense-start
```

### 2. Start with Custom Interval (e.g., 12 hours)
```bash
nogap-cli sense-start --interval 12
```

### 3. Check Status
```bash
nogap-cli sense-status
```

## What Happens When Running?

```
🔍 [SENSOR] Audit started at 1735564800
✅ [SENSOR] Audit completed: 434 policies, 2 drifted
📸 [SENSOR] Snapshot stored
😴 [SENSOR] Next audit in 24 hours
```

The sensor will:
1. Run a full audit of all policies
2. Compare results with previous audit
3. Count how many policies changed state (drift)
4. Store results in database
5. Sleep until next interval
6. Repeat forever (until you stop it)

## When to Use Autonomous Sensing

### ✅ Good Use Cases
- **Continuous Compliance Monitoring**: Track if your system stays compliant over time
- **Drift Detection**: Know immediately when something changes
- **Audit History**: Build a timeline of security state
- **Hands-Off Monitoring**: Set up once, runs forever
- **Server Environments**: Perfect for headless systems

### ❌ Not Suitable For
- **One-Time Audits**: Use `nogap-cli audit` instead
- **Immediate Fixes**: Sensing doesn't remediate, only observes
- **Interactive Work**: Use the TUI (`nogap-cli tui`) for hands-on work

## Understanding Drift

**Drift** means a policy's compliance state changed between audits:

| Before | After | Drift? | Meaning |
|--------|-------|--------|---------|
| ✅ Pass | ❌ Fail | Yes | Regression (something broke) |
| ❌ Fail | ✅ Pass | Yes | Improvement (something fixed) |
| ✅ Pass | ✅ Pass | No | Stable (still good) |
| ❌ Fail | ❌ Fail | No | Stable (still broken) |

**Example Output**:
```
[SENSOR] Drift detected: A.1.a.i changed from PASS to FAIL
[SENSOR] Drift detected: B.2.c.iv changed from FAIL to PASS
```

## Common Commands

```bash
# Start with 1-hour interval (development/testing)
nogap-cli sense-start --interval 1

# Start with 6-hour interval (high-security environments)
nogap-cli sense-start --interval 6

# Start with 24-hour interval (default, production)
nogap-cli sense-start

# Start with custom policy file
nogap-cli sense-start --policies /path/to/policies.yaml

# Check if sensor is running
nogap-cli sense-status

# Check status with JSON output
nogap-cli sense-status --json
```

## Stopping the Sensor

Press `Ctrl+C` to stop. The sensor will:
- Finish current audit if running
- Save all data
- Shut down gracefully

## Where is Data Stored?

All audit results are saved in:
- **Windows**: `%LOCALAPPDATA%\nogap\snapshots.db`
- **Linux/Mac**: `~/.local/share/nogap/snapshots.db`

You can query this database to see history:
```sql
SELECT timestamp, after_state 
FROM snapshots 
WHERE policy_id = 'agent_sense' 
ORDER BY timestamp DESC;
```

## Troubleshooting

### "Sensor is disabled in configuration"
This shouldn't happen with the CLI (it enables automatically). If you see this programmatically, set `enabled: true` in config.

### "Permission denied" errors
Some audits require admin/root privileges. Run as:
- **Windows**: Administrator
- **Linux**: root or with sudo

### "Database is locked"
Another NoGap process is using the database. Stop other instances first.

### No drift detected
This is normal! If your system is stable, drift will be 0. Drift only counts when policies change state.

## Example: 24/7 Security Monitoring

```bash
#!/bin/bash
# Start sensor and log to file

nogap-cli sense-start --interval 24 2>&1 | tee sensor.log
```

Add this to system startup (systemd, cron @reboot, etc.) for continuous monitoring.

## What's Next?

This is **Phase 1: Sense** of the agentic architecture:

```
Phase 1: SENSE (Current) ✅
  ↓
Phase 2: PLAN (Future)
  - Analyze drift patterns
  - Prioritize issues
  - Generate action plans
  ↓
Phase 3: ACT (Future)
  - Execute fixes
  - Verify results
  - Self-correct
```

The sensor provides the foundation—pure observation data—that future planning and action layers will build upon.

## Need More Details?

- **Full documentation**: See [`AUTONOMOUS_SENSING.md`](AUTONOMOUS_SENSING.md)
- **Implementation details**: See [`SENSING_IMPLEMENTATION.md`](SENSING_IMPLEMENTATION.md)
- **Code example**: Run `cargo run --example autonomous_sensor`

---

**Quick Summary**: Run `nogap-cli sense-start`, let it observe your security posture continuously, and check `nogap-cli sense-status` anytime to see what's happening. It's that simple! 🔍
