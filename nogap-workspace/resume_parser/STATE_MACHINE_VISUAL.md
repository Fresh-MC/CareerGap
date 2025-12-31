# Strategy Lifecycle State Machine - Visual Guide

## State Transition Diagram

```
                    ┌─────────────────────────────────────────┐
                    │                                         │
                    │         INITIALIZATION                  │
                    │                                         │
                    └──────────────────┬──────────────────────┘
                                       │
                                       │ All strategies
                                       │ start here
                                       ▼
                    ┌─────────────────────────────────────────┐
                    │                                         │
                    │           ① EXPLORE                     │
                    │                                         │
                    │  • Just selected                        │
                    │  • Insufficient evidence                │
                    │  • Building initial confidence          │
                    │                                         │
                    └──────────┬──────────────┬───────────────┘
                               │              │
                  ≥1 interview │              │ 3+ negatives OR
                  confidence   │              │ confidence < 0.30
                  ≥ 0.55       │              │
                               │              │
                               ▼              ▼
            ┌─────────────────────────┐   ┌──────────────────────┐
            │                         │   │                      │
            │    ② VALIDATE           │   │   ④ RECONSIDER       │
            │                         │   │                      │
            │  • Positive signals     │   │  • Strategy failed   │
            │  • 1+ interviews        │   │  • Will re-evaluate  │
            │  • Gaining confidence   │   │  • Marked as failed  │
            │                         │   │                      │
            └─────┬──────────┬────────┘   └──────────┬───────────┘
                  │          │                       │
     ≥2 interviews│          │ positioning           │ New strategy
     confidence   │          │ issue detected        │ selected
     ≥ 0.65       │          │                       │
     no issues    │          ▼                       │
                  │   ┌──────────────┐               │
                  │   │   EXPLORE    │◄──────────────┘
                  │   │  (return)    │
                  │   └──────────────┘
                  │
                  ▼
    ┌─────────────────────────────────────┐
    │                                     │
    │        ③ EXECUTE                    │
    │                                     │
    │  • Strategy validated & locked      │
    │  • 2+ interviews received           │
    │  • High confidence (≥0.65)          │
    │  • Ready for roadmap generation     │
    │                                     │
    └─────────────────────────────────────┘
```

## State Details

### ① EXPLORE (Initial State)
**Meaning**: Strategy just selected, gathering evidence

**Entry Conditions**:
- New strategy initialized
- Returning from VALIDATE due to positioning issues
- New strategy after RECONSIDER re-evaluation

**Exit Conditions**:
- → VALIDATE: ≥1 interview AND confidence ≥ 0.55
- → RECONSIDER: Failure conditions met

**Typical Outcomes in This State**:
- Rejections (expected, building experience)
- No responses (normal early on)
- First interview (progress!)

---

### ② VALIDATE (Showing Promise)
**Meaning**: Strategy receiving positive signals, needs more evidence

**Entry Conditions**:
- From EXPLORE: ≥1 interview AND confidence ≥ 0.55

**Exit Conditions**:
- → EXECUTE: ≥2 interviews AND confidence ≥ 0.65 AND no positioning issues
- → EXPLORE: Positioning issue detected
- → RECONSIDER: Failure conditions met

**Typical Outcomes in This State**:
- Additional interviews (building validation)
- Some rejections (testing continues)

---

### ③ EXECUTE (Locked & Ready)
**Meaning**: Strategy validated, ready for execution/roadmap

**Entry Conditions**:
- From VALIDATE: ≥2 interviews AND confidence ≥ 0.65 AND no positioning issues

**Exit Conditions**:
- → RECONSIDER: Failure conditions met (rare at this point)

**What Happens Here**:
- Strategy is LOCKED
- Roadmap can be generated (Step 3)
- Continue with current approach
- Very unlikely to fail from here

---

### ④ RECONSIDER (Strategy Failed)
**Meaning**: Strategy invalidated, will trigger re-evaluation

**Entry Conditions** (from ANY state):
- Confidence drops below 0.30
- ≥3 negative outcomes (rejected/no_response)

**Exit Conditions**:
- → EXPLORE: Automatic re-evaluation creates new strategy

**What Happens Here**:
- Strategy marked as failed
- Moved to history
- New strategy automatically selected
- Returns to EXPLORE with fresh start

---

## Transition Triggers (Deterministic Rules)

### Forward Transitions (Success Path)

```
EXPLORE → VALIDATE
━━━━━━━━━━━━━━━━━━
Conditions:
  ✓ interview_count >= 1
  ✓ confidence >= 0.55

Logic:
  if interview_count >= 1 and confidence >= 0.55:
      transition_to(VALIDATE)
```

```
VALIDATE → EXECUTE
━━━━━━━━━━━━━━━━━━
Conditions:
  ✓ interview_count >= 2
  ✓ confidence >= 0.65
  ✓ no resume_positioning_issue

Logic:
  if (interview_count >= 2 and 
      confidence >= 0.65 and 
      not has_positioning_issue):
      transition_to(EXECUTE)
```

### Backward Transitions (Issues Detected)

```
VALIDATE → EXPLORE
━━━━━━━━━━━━━━━━━━
Conditions:
  ✓ resume_positioning_issue detected
  ✓ interview_count < 2

Logic:
  if has_positioning_issue and interview_count < 2:
      transition_to(EXPLORE)
```

### Failure Transitions (Any State)

```
ANY → RECONSIDER
━━━━━━━━━━━━━━━━
Conditions (either):
  ✓ confidence < 0.30
  ✓ negative_outcomes >= 3

Logic:
  if confidence < FAILURE_THRESHOLD:
      transition_to(RECONSIDER)
  elif negative_count >= 3:
      transition_to(RECONSIDER)
```

### Recovery Transition

```
RECONSIDER → EXPLORE
━━━━━━━━━━━━━━━━━━━━
Conditions:
  ✓ New strategy selected (automatic)

Logic:
  # Handled in re_evaluate_strategy()
  new_strategy = select_new_strategy()
  new_strategy.state = EXPLORE
```

---

## Example Scenarios

### Scenario 1: Happy Path 🎉
```
Session Start
    ↓
EXPLORE (conf: 0.70)
    ↓ [+interview]
VALIDATE (conf: 0.85)
    ↓ [+interview]
EXECUTE (conf: 0.95)
    ↓
Ready for roadmap!
```

### Scenario 2: Early Failure 😞
```
Session Start
    ↓
EXPLORE (conf: 0.55)
    ↓ [+rejected]
EXPLORE (conf: 0.45)
    ↓ [+rejected]
EXPLORE (conf: 0.35)
    ↓ [+rejected]
RECONSIDER (conf: 0.25)
    ↓ [re-evaluation]
EXPLORE (new strategy, conf: 0.50)
```

### Scenario 3: Validation then Failure 😐
```
Session Start
    ↓
EXPLORE (conf: 0.68)
    ↓ [+interview]
VALIDATE (conf: 0.83)
    ↓ [+rejected]
VALIDATE (conf: 0.73)
    ↓ [+rejected]
VALIDATE (conf: 0.63)
    ↓ [+no_response]
VALIDATE (conf: 0.55)
    ↓ [+rejected]
RECONSIDER (conf: 0.45, 3 negatives)
    ↓ [re-evaluation]
EXPLORE (new strategy)
```

### Scenario 4: Positioning Issue Detected 🔄
```
Session Start
    ↓
EXPLORE (conf: 0.68)
    ↓ [+interview]
VALIDATE (conf: 0.83)
    ↓ [positioning issue detected]
EXPLORE (conf: 0.83)
    ↓ [fix positioning, +interview]
VALIDATE (conf: 0.98)
    ↓ [+interview, no issues]
EXECUTE (conf: 1.0)
```

---

## Key Properties

### 1. Deterministic
Every transition has clear, objective triggers
- No guesswork
- No heuristics
- Repeatable behavior

### 2. Interview-Driven Confidence
Interviews advance states, not switch strategies
- EXPLORE → VALIDATE → EXECUTE
- Builds confidence systematically
- No reactive switching

### 3. Phase-Based Management
Clear lifecycle phases
- Each phase has purpose
- Specific entry/exit criteria
- Progressive validation

### 4. Failure Handling
Automatic recovery mechanism
- RECONSIDER triggers re-evaluation
- New strategy in EXPLORE
- Learning from failure

### 5. Separation of Concerns
State machine isolated
- No side effects yet
- Ready for Step 3 integration
- Clean architecture

---

## Integration Points

### Current (Step 2)
```python
# In process_outcome()
session = record_outcome(session, outcome)  # Update confidence
session = evaluate_strategy_state(session)  # STATE MACHINE
if session.current_strategy.strategy_state == RECONSIDER:
    session = re_evaluate_strategy(session)
```

### Future (Step 3)
```python
# In generate_roadmap()
if session.current_strategy.strategy_state != EXECUTE:
    return error("Strategy not validated yet")

roadmap = create_roadmap(session)  # Generate only for EXECUTE
```

---

## Monitoring State Transitions

### Logs Show:
```
State transition: explore → validate. Reason: 1 interview(s) received, confidence 0.85 ≥ 0.55

State transition: validate → execute. Reason: 2 interviews, confidence 0.95 ≥ 0.65, no positioning issues

State transition: explore → reconsider. Reason: 3 negative outcomes (limit: 3)
```

### API Response Includes:
```json
{
  "strategy_state": "validate",
  "strategy_changed": false,
  "explanation": "Selected ResumeOptimization due to evidence_depth. Confidence: 0.83. State: validate."
}
```

---

**This state machine ensures:**
- ✅ Strategies progress through validation before execution
- ✅ Interviews build confidence without causing chaos
- ✅ Failures trigger automatic re-evaluation
- ✅ Clean separation between exploration and execution
- ✅ Ready for roadmap generation in Step 3
