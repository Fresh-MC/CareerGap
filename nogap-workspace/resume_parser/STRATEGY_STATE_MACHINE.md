# Strategy Lifecycle State Machine - Implementation Summary

## Overview
This document explains the Strategy Lifecycle State Machine implementation added to the Agentic AI Career Development system.

## Architecture

### State Definitions
The system now tracks strategy lifecycle through four deterministic states:

- **EXPLORE**: Strategy just selected, insufficient evidence to validate
- **VALIDATE**: Strategy showing positive signals (received interviews)
- **EXECUTE**: Strategy validated and locked (ready for execution/roadmap)
- **RECONSIDER**: Strategy invalidated (triggers re-evaluation)

### State Transition Rules (Deterministic)

#### 1. explore → validate
**Trigger**: ≥1 interview AND confidence ≥ 0.55
- Indicates the strategy is showing promise
- Positive market response received

#### 2. validate → execute
**Trigger**: ≥2 interviews AND no resume_positioning_issue AND confidence ≥ 0.65
- Strong validation signals
- No structural issues detected
- Strategy is locked and ready for roadmap generation (Step 3)

#### 3. any → reconsider
**Trigger**: confidence < FAILURE_THRESHOLD (0.30) OR ≥3 negative outcomes
- Strategy failing based on objective metrics
- Will trigger automatic re-evaluation

#### 4. reconsider → explore
**Trigger**: New strategy selected (automatic in re_evaluate_strategy)
- Fresh start with new strategy
- Learning from previous failure

## Key Design Decisions

### 1. Interviews Never Directly Trigger Strategy Switching
- Interviews increase certainty (state transitions)
- Strategy switching ONLY occurs when entering reconsider state
- This prevents reactive, chaotic behavior

### 2. Phase-Based Strategy Management
- Strategies progress through clear phases
- Each phase has specific entry/exit criteria
- No ambiguous "should we switch?" decisions

### 3. State Machine as Gatekeeper
- Roadmaps are only generated in EXECUTE state
- Prevents premature roadmap generation
- Ensures strategy is validated before commitment

## Implementation Details

### Modified Files
- `agent_loop.py`: Core state machine logic

### New Components

#### 1. StrategyState Enum
```python
class StrategyState(str, Enum):
    EXPLORE = "explore"
    VALIDATE = "validate"
    EXECUTE = "execute"
    RECONSIDER = "reconsider"
```

#### 2. evaluate_strategy_state() Function
- Core state transition logic
- Takes session and optional profile_signals
- Returns updated session with new state
- Logs all state transitions

#### 3. Updated StrategyRecord
- Added `strategy_state` field
- Serialized in to_dict()
- Preserved in session history

### Integration Points

#### 1. Initialization
- All new strategies start in EXPLORE state
- Documented in initialize_session()

#### 2. Outcome Processing
- State evaluation happens after confidence update
- Before re-evaluation check
- Exposes state in API response

#### 3. Re-evaluation
- New strategies return to EXPLORE
- Clean state reset
- Implements reconsider → explore transition

## Data Flow

```
User submits outcome
    ↓
record_outcome() - Update confidence, outcomes
    ↓
evaluate_strategy_state() - STATE MACHINE
    ↓
Check if state == RECONSIDER
    ↓
re_evaluate_strategy() if needed
    ↓
Return session with strategy_state
```

## API Response Changes

The `/api/outcome` endpoint now returns:
```json
{
  "session": {...},
  "strategy_changed": bool,
  "current_strategy": {...},
  "strategy_state": "explore|validate|execute|reconsider",
  "explanation": "...",
  "profile_signals": {...}
}
```

## Testing Scenarios

### Scenario 1: Success Path
1. Initialize → EXPLORE (confidence 0.70)
2. Interview → VALIDATE (confidence 0.85)
3. Interview → EXECUTE (confidence 1.00)
4. Ready for roadmap

### Scenario 2: Failure and Recovery
1. Initialize → EXPLORE (confidence 0.70)
2. Rejected → EXPLORE (confidence 0.60)
3. Rejected → EXPLORE (confidence 0.50)
4. Rejected → RECONSIDER (confidence 0.40)
5. Re-evaluate → New strategy in EXPLORE

### Scenario 3: Validation Failure
1. Initialize → EXPLORE
2. Interview → VALIDATE
3. Rejected (positioning issue detected) → EXPLORE
4. Continue exploring

## Constraints Honored

✅ No JWT or authentication added
✅ No UI modifications
✅ No roadmap generation triggered
✅ No changes to strategy selection heuristics
✅ No new signals added
✅ State + transition logic only

## Read-Only Effects

State changes currently:
- Update session data (strategy_state field)
- Are returned in /api/outcome response
- Are logged to explanation_log
- Do NOT trigger side effects (roadmaps, etc.)

This prepares Step 3 (Roadmap Generation) without implementing it yet.

## Future Integration (Step 3)

When implementing roadmap generation:
- Check `strategy_state == StrategyState.EXECUTE`
- Only generate roadmaps for EXECUTE state
- Lock roadmap to prevent switching
- This ensures validated strategies only

## Benefits

1. **Deterministic Behavior**: Clear rules, no guesswork
2. **Phase-Based Progression**: Strategies mature through stages
3. **Interview Certainty**: Interviews validate without causing chaos
4. **Clean Separation**: State machine logic isolated from outcome processing
5. **Preparation for Step 3**: Ready to unlock roadmaps cleanly

## Logging

All state transitions are logged with:
- Previous state → New state
- Reason for transition
- Relevant metrics (interviews, confidence, signals)

Example:
```
State transition: explore → validate. Reason: 1 interview(s) received, confidence 0.75 ≥ 0.55
```

## Backward Compatibility

The implementation handles legacy sessions without strategy_state:
- Defaults to EXPLORE if field missing
- _reconstruct_session() handles missing field gracefully
- No breaking changes to existing API contracts
