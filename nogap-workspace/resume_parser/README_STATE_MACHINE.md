# Strategy Lifecycle State Machine - Quick Start

## Overview

This implementation adds a **deterministic Strategy Lifecycle State Machine** to the Agentic AI Career Development system. The state machine governs when a strategy is explored, validated, locked, or re-evaluated.

## Quick Start

### Run Tests
```bash
cd resume_parser
python test_state_machine.py
```

**Expected Output**: `[SUCCESS] ALL TESTS PASSED`

### Import and Use
```python
from agent_loop import (
    StrategyState,
    evaluate_strategy_state,
    process_outcome,
)

# Process an outcome (automatically evaluates state)
result = process_outcome(session, "interview")

# Check current state
current_state = result["strategy_state"]  # "explore", "validate", "execute", or "reconsider"

# In Step 3: Check before generating roadmap
if current_state == "execute":
    generate_roadmap(session)
else:
    return {"error": f"Strategy not validated yet. Current state: {current_state}"}
```

## State Machine Flow

```
EXPLORE (initial) 
    → VALIDATE (1+ interview, confidence >= 0.55)
        → EXECUTE (2+ interviews, confidence >= 0.65, no issues)
            [Ready for roadmap!]
    
Any state → RECONSIDER (confidence < 0.30 OR 3+ negatives)
    → EXPLORE (new strategy selected)
```

## Key Features

✅ **Deterministic**: Clear rules, no guesswork
✅ **Interview-Driven**: Interviews build certainty, don't trigger switching
✅ **Phase-Based**: Strategies progress through defined lifecycle
✅ **Failure Recovery**: Automatic re-evaluation when entering RECONSIDER
✅ **Read-Only**: No side effects yet (ready for Step 3)

## Files

### Implementation
- `agent_loop.py` - Core state machine logic

### Documentation
- `STRATEGY_STATE_MACHINE.md` - Technical details
- `STATE_MACHINE_VISUAL.md` - Visual guide with diagrams
- `IMPLEMENTATION_COMPLETE.md` - Implementation summary
- `DELIVERY_CHECKLIST.md` - Verification checklist
- `README_STATE_MACHINE.md` - This file

### Testing
- `test_state_machine.py` - Test suite (4 scenarios)

## API Changes

The `/api/outcome` endpoint now returns `strategy_state`:

```json
{
  "strategy_state": "validate",
  "strategy_changed": false,
  "explanation": "Confidence: 0.83. State: validate."
}
```

## State Meanings

| State | Meaning | Next Step |
|-------|---------|-----------|
| **explore** | Just selected, gathering evidence | Get 1 interview to validate |
| **validate** | Showing promise, needs more proof | Get 2+ interviews to execute |
| **execute** | Validated & locked | Generate roadmap (Step 3) |
| **reconsider** | Failed, will re-evaluate | Automatic → new strategy in explore |

## Transition Rules

### Forward Progress
- **explore → validate**: ≥1 interview AND confidence ≥ 0.55
- **validate → execute**: ≥2 interviews AND no positioning issues AND confidence ≥ 0.65

### Failure Handling
- **any → reconsider**: confidence < 0.30 OR ≥3 negative outcomes
- **reconsider → explore**: New strategy selected (automatic)

### Special Cases
- **validate → explore**: Positioning issue detected (needs fixing)

## Examples

### Success Path
```
1. Initialize → explore (conf: 0.70)
2. Interview → validate (conf: 0.85)
3. Interview → execute (conf: 0.95)
4. Ready for roadmap!
```

### Failure Recovery
```
1. Initialize → explore (conf: 0.55)
2. Rejected → explore (conf: 0.45)
3. Rejected → explore (conf: 0.35)
4. Rejected → reconsider (conf: 0.25)
5. Re-evaluate → explore (new strategy, conf: 0.50)
```

## Integration with Step 3

When implementing roadmap generation:

```python
def generate_roadmap_handler(session):
    # Check state before generating
    if session.current_strategy.strategy_state != StrategyState.EXECUTE:
        return {
            "error": "Strategy not validated yet",
            "current_state": session.current_strategy.strategy_state,
            "message": "Please validate strategy with more interviews"
        }
    
    # Generate roadmap only for EXECUTE state
    roadmap = create_roadmap(session.stage3_strategy)
    return {"roadmap": roadmap}
```

## Logging

All state transitions are logged:

```
State transition: explore → validate. Reason: 1 interview(s) received, confidence 0.85 ≥ 0.55
State transition: validate → execute. Reason: 2 interviews, confidence 0.95 ≥ 0.65, no positioning issues
State transition: any → reconsider. Reason: confidence dropped to 0.29 (below threshold 0.30)
```

## Constraints Honored

❌ NO JWT or authentication
❌ NO UI modifications
❌ NO roadmap generation (yet)
❌ NO changes to strategy selection
❌ NO new signals
✅ State + transition logic ONLY

## Benefits

1. **Stop Exploring at Right Time** - EXECUTE state signals readiness
2. **Interviews Without Chaos** - Build confidence through states
3. **Phase-Based Switching** - Only switch in RECONSIDER
4. **Clean Roadmap Unlock** - Check for EXECUTE before generating

## Next Steps (Step 3)

1. Implement roadmap generation
2. Check for `strategy_state == "execute"` before generating
3. Lock roadmap during execution
4. Add roadmap progress tracking

## Support

For questions or issues:
1. Check `STRATEGY_STATE_MACHINE.md` for technical details
2. Check `STATE_MACHINE_VISUAL.md` for diagrams
3. Run `test_state_machine.py` to verify implementation
4. Check `DELIVERY_CHECKLIST.md` for verification

---

**Status**: ✅ **Implementation Complete**

**Tests**: ✅ **4/4 Passing**

**Ready For**: ✅ **Step 3 (Roadmap Generation)**
