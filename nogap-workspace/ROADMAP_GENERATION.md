# Roadmap Generation System Documentation

## Overview

The **Roadmap Generation System** is strictly gated by the **Strategy Lifecycle State Machine**. This ensures that roadmaps are only generated when a strategy has been thoroughly validated through real-world outcomes (interviews).

**Core Principle**: Roadmaps feel earned, not random. Interviews unlock execution planning naturally.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                  ROADMAP GENERATION FLOW                     │
└─────────────────────────────────────────────────────────────┘

1. Client Request
   POST /api/roadmap
   {
     "user_id": "...",
     "session": { ... }  // Must include strategy_state
   }
   │
   ├─> Backend API (api.rs)
   │   generate_roadmap()
   │   │
   │   ├─> Check if session provided
   │   │   │
   │   │   YES → Call Python roadmap_generator.py via subprocess
   │   │   │
   │   │   └─> roadmap_generator.py (stdin/stdout)
   │   │       │
   │   │       ├─> check_roadmap_eligibility(session)
   │   │       │   │
   │   │       │   ├─> strategy_state == EXECUTE?
   │   │       │   │   YES → {"eligible": True, "reason": "..."}
   │   │       │   │   NO  → {"eligible": False, "reason": "...", "current_state": "..."}
   │   │       │   │
   │   │       ├─> IF eligible:
   │   │       │   generate_roadmap_for_<strategy>()
   │   │       │   └─> Return {"roadmap": {...}, "eligible": True}
   │   │       │
   │   │       └─> IF NOT eligible:
   │   │           └─> Return {"error": "...", "eligible": False, "reason": "..."}
   │   │
   │   └─> Backend Response
   │       │
   │       ├─> eligible=True  → HTTP 200 + roadmap JSON
   │       └─> eligible=False → HTTP 403 + error message
   │
   └─> Client receives response
       │
       ├─> 200 OK: Display roadmap
       └─> 403 Forbidden: Show helpful message (e.g., "Need 2+ interviews to unlock")
```

## State-Gating Rules

### THE GATE

**A roadmap may ONLY be generated when `strategy_state == EXECUTE`**

### Blocked States

| State | Blocked Reason | User Message |
|-------|----------------|--------------|
| **EXPLORE** | Strategy just selected, needs validation | "Need at least 1 interview to validate before generating roadmap" |
| **VALIDATE** | Strategy showing promise but needs more evidence | "Need 2+ interviews and confidence ≥ 0.65 to execute" |
| **RECONSIDER** | Strategy failed and under re-evaluation | "Cannot generate roadmap for failed strategy" |

### Eligibility Check

```python
def check_roadmap_eligibility(session: dict) -> dict:
    current_strategy = session.get("current_strategy")
    strategy_state = current_strategy.get("strategy_state")
    
    if strategy_state == StrategyState.EXECUTE:
        return {"eligible": True, "reason": "Strategy in EXECUTE state"}
    
    # Blocked - return helpful message
    return {
        "eligible": False,
        "reason": "<state-specific message>",
        "current_state": strategy_state,
        "recommendation": "<actionable advice>"
    }
```

## Roadmap Structure

### Components

A complete roadmap includes:

```python
@dataclass
class Roadmap:
    roadmap_id: str                    # UUID
    strategy: str                      # "ResumeOptimization", "SkillGapPatch", etc.
    phase: str                         # Always "execute" for valid roadmaps
    created_at: str                    # ISO timestamp
    strategy_confidence: float         # Current strategy confidence
    
    goals: List[RoadmapGoal]          # High-level goals (2-3)
    milestones: List[RoadmapMilestone] # Progress markers (2-4)
    actions: List[RoadmapAction]       # Concrete tasks (5-10)
    
    review_after_days: int             # When to review progress
    estimated_completion_days: int     # Expected duration
    version: int                       # Roadmap version
    strategy_version: str              # Hash of strategy when created
    invalidated: bool                  # Invalidation flag
    invalidation_reason: Optional[str] # Why invalidated
```

### Action Structure

```python
@dataclass
class RoadmapAction:
    action_id: str
    title: str                  # Concrete action title
    description: str            # Detailed instructions
    deadline_days: int          # Days from roadmap creation
    priority: str               # "critical", "high", "medium"
    category: str               # "resume", "skill", "application", etc.
    completed: bool = False
```

## Strategy-Specific Roadmaps

### 1. ResumeOptimization

**Timeline**: 2-4 weeks (21 days)

**Focus**: Transform resume to showcase applied skills with concrete outcomes

**Key Actions** (7 total):
1. Rewrite primary project description (3 days)
2. Add metrics to experience entries (5 days)
3. Create skill-evidence matrix (4 days)
4. Run ATS compatibility check (7 days)
5. Get resume reviewed (10 days)
6. Create tailored versions (12 days)
7. Begin targeted applications (14 days)

### 2. SkillGapPatch

**Timeline**: 3-6 weeks (42 days)

**Focus**: Acquire missing critical skill + create applied evidence

**Key Actions** (8 total):
1. Identify skill gap and learning resources (2 days)
2. Complete structured learning (14 days)
3. Design applied project (16 days)
4. Build core functionality (21 days)
5. Deploy project (28 days)
6. Document project (30 days)
7. Add to resume (35 days)
8. Share publicly and apply (42 days)

### 3. RoleShift

**Timeline**: 2-3 weeks (21 days)

**Focus**: Position for adjacent role by reframing experience

**Key Actions** (7 total):
1. Research target role requirements (3 days)
2. Map transferable skills (5 days)
3. Rewrite resume for target role (7 days)
4. Create role-specific portfolio (10 days)
5. Prepare positioning statement (12 days)
6. Connect with target role professionals (14 days)
7. Apply to target roles (21 days)

### 4. HoldPosition

**Timeline**: 2 weeks (14 days)

**Focus**: Increase application volume with optimized targeting

**Key Actions** (7 total):
1. Expand target company list (1 day)
2. Set up application tracking (2 days)
3. Prepare batch of tailored materials (4 days)
4. Week 1: Submit 15 applications (7 days)
5. Refine approach based on responses (9 days)
6. Week 2: Submit 15 additional applications (14 days)
7. Schedule progress review (14 days)

## API Integration

### Request Format

```json
POST /api/roadmap
{
  "user_id": "user123",
  "session": {
    "current_strategy": {
      "strategy": "ResumeOptimization",
      "strategy_state": "execute",
      "current_confidence": 0.72,
      "outcomes": ["interview", "interview"]
    },
    "stage1_evidence": { ... },
    "stage2_bottleneck": { ... },
    "stage3_strategy": { ... }
  }
}
```

### Success Response (HTTP 200)

```json
{
  "success": true,
  "data": {
    "roadmap": {
      "roadmap_id": "550e8400-e29b-41d4-a716-446655440000",
      "strategy": "ResumeOptimization",
      "phase": "execute",
      "created_at": "2024-01-15T10:30:00Z",
      "strategy_confidence": 0.72,
      "goals": [...],
      "milestones": [...],
      "actions": [...],
      "estimated_completion_days": 21,
      "review_after_days": 14
    },
    "eligible": true,
    "strategy": "ResumeOptimization"
  }
}
```

### Blocked Response (HTTP 403)

```json
{
  "success": false,
  "error": "Roadmap generation not allowed",
  "data": null,
  "reason": "Strategy 'ResumeOptimization' is in VALIDATE state. Need 2+ interviews and confidence ≥ 0.65 to execute.",
  "current_state": "validate",
  "recommendation": "Continue applying and tracking outcomes. Roadmap will unlock after strategy validation."
}
```

## Roadmap Invalidation

A roadmap MUST be invalidated if:

1. **Strategy Changed**: Current strategy name differs from roadmap's strategy
2. **State Regression**: Strategy dropped from EXECUTE to EXPLORE/VALIDATE/RECONSIDER
3. **Strategy Failed**: Strategy entered RECONSIDER state

```python
def invalidate_roadmap_if_strategy_changed(roadmap: dict, current_session: dict) -> dict:
    current_strategy = current_session.get("current_strategy", {})
    current_strategy_name = current_strategy.get("strategy")
    current_state = current_strategy.get("strategy_state")
    
    # Check if strategy changed
    if roadmap["strategy"] != current_strategy_name:
        roadmap["invalidated"] = True
        roadmap["invalidation_reason"] = f"Strategy changed from {roadmap['strategy']} to {current_strategy_name}"
    
    # Check if strategy entered RECONSIDER
    if current_state == StrategyState.RECONSIDER:
        roadmap["invalidated"] = True
        roadmap["invalidation_reason"] = "Strategy entered RECONSIDER state (failed)"
    
    # Check if strategy left EXECUTE state
    if current_state != StrategyState.EXECUTE:
        roadmap["invalidated"] = True
        roadmap["invalidation_reason"] = f"Strategy no longer in EXECUTE state (current: {current_state})"
    
    return roadmap
```

## Logging

### Backend Logging (Rust)

```rust
// Successful generation
println!("✅ Roadmap generated for user {} (strategy in EXECUTE state)", user_id);

// Blocked attempt
println!("⚠️  Roadmap generation blocked for user {}: {}", user_id, error_msg);
```

### Python Logging

```python
# Eligibility check
print(f"[roadmap_generator.py] Roadmap generation ALLOWED - state is EXECUTE", file=sys.stderr)
print(f"[roadmap_generator.py] Roadmap generation BLOCKED - {reason}", file=sys.stderr)
```

## Testing

### Test Coverage

**7 Tests** covering all gating scenarios:

1. ✅ EXPLORE state blocks roadmap
2. ✅ VALIDATE state blocks roadmap
3. ✅ EXECUTE state allows roadmap
4. ✅ RECONSIDER state blocks roadmap
5. ✅ Full roadmap generation works in EXECUTE
6. ✅ Full roadmap generation blocked in EXPLORE
7. ✅ All 4 strategy types generate successfully in EXECUTE

### Running Tests

```bash
cd resume_parser
python test_roadmap_gating.py
```

### Expected Output

```
============================================================
ROADMAP GATING TEST SUITE
============================================================
[TEST 1] Roadmap blocked in EXPLORE state
[PASS] EXPLORE state correctly blocks roadmap generation

[TEST 2] Roadmap blocked in VALIDATE state
[PASS] VALIDATE state correctly blocks roadmap generation

[TEST 3] Roadmap allowed in EXECUTE state
[PASS] EXECUTE state correctly allows roadmap generation

[TEST 4] Roadmap blocked in RECONSIDER state
[PASS] RECONSIDER state correctly blocks roadmap generation

[TEST 5] Full roadmap generation in EXECUTE state
Strategy: ResumeOptimization
Estimated Duration: 21 days
Number of Actions: 7
[PASS] Full roadmap generated successfully in EXECUTE state

[TEST 6] Full roadmap generation blocked in EXPLORE state
[PASS] Roadmap generation correctly blocked in EXPLORE state

[TEST 7] All strategy types in EXECUTE state
  Testing: ResumeOptimization
    -> Generated 7 actions, 21 days
  Testing: SkillGapPatch
    -> Generated 8 actions, 42 days
  Testing: RoleShift
    -> Generated 7 actions, 21 days
  Testing: HoldPosition
    -> Generated 7 actions, 14 days
[PASS] All strategy types successfully generated roadmaps

============================================================
TEST RESULTS: 7 passed, 0 failed
============================================================
```

## Safety and Consistency

### Version Tracking

Each roadmap includes:
- `strategy_version`: Hash/timestamp of strategy when roadmap created
- `version`: Roadmap version number (for updates)

### Idempotency

Roadmap generation is idempotent:
- Same session state → same roadmap structure
- Strategy-specific templates ensure consistency

### No State Mutation

Roadmap generation is **read-only**:
- Does NOT mutate session state
- Does NOT change strategy_state
- Does NOT affect confidence scores

## Files

| File | Purpose |
|------|---------|
| `resume_parser/roadmap_generator.py` | Python roadmap generation module (1034 lines) |
| `resume_parser/test_roadmap_gating.py` | Test suite for gating logic (210 lines) |
| `backend/src/api.rs` | Rust API endpoint with state-gating integration |
| `backend/src/agent/resume_parser.rs` | StrategyRecord with strategy_state field |

## Integration Checklist

- [x] `roadmap_generator.py` module created
- [x] `check_roadmap_eligibility()` function enforces EXECUTE-only gate
- [x] 4 strategy-specific roadmap templates implemented
- [x] Backend API calls Python module via subprocess
- [x] Eligibility errors return HTTP 403 with helpful messages
- [x] Logging for blocked attempts
- [x] 7 tests passing (100% coverage of gating logic)
- [x] Documentation complete

## Usage Example

### Scenario: User with Strategy in EXECUTE State

```python
# Session after 2 interviews → EXECUTE state
session = {
    "current_strategy": {
        "strategy": "ResumeOptimization",
        "strategy_state": "execute",  # ✅ UNLOCKED
        "current_confidence": 0.72,
        "outcomes": ["interview", "interview"]
    },
    # ... other fields
}

# Generate roadmap
result = generate_roadmap(session)

# Result
{
    "roadmap": {
        "strategy": "ResumeOptimization",
        "estimated_completion_days": 21,
        "actions": [
            {"title": "Rewrite Primary Project Description", "deadline_days": 3, ...},
            # ... 6 more actions
        ],
        ...
    },
    "eligible": True
}
```

### Scenario: User with Strategy in VALIDATE State

```python
# Session after 1 interview → VALIDATE state
session = {
    "current_strategy": {
        "strategy": "ResumeOptimization",
        "strategy_state": "validate",  # ❌ BLOCKED
        "current_confidence": 0.58,
        "outcomes": ["interview"]
    },
    # ... other fields
}

# Attempt to generate roadmap
result = generate_roadmap(session)

# Result
{
    "error": "Roadmap generation not allowed",
    "eligible": False,
    "reason": "Strategy 'ResumeOptimization' is in VALIDATE state. Need 2+ interviews and confidence ≥ 0.65 to execute.",
    "current_state": "validate",
    "recommendation": "Continue applying and tracking outcomes. Roadmap will unlock after strategy validation."
}
```

## Next Steps

### Frontend Integration

1. Check `eligible` field in response
2. If `eligible=false`:
   - Show blocked state message
   - Display `reason` and `recommendation`
   - Highlight progress toward unlock (e.g., "1/2 interviews completed")
3. If `eligible=true`:
   - Display roadmap with actions
   - Allow action completion tracking
   - Show milestones and progress

### Future Enhancements

1. **Roadmap Updates**: Allow editing completed actions
2. **Progress Tracking**: Track action completion over time
3. **Milestone Notifications**: Alert when milestones reached
4. **Roadmap Versioning**: Support multiple roadmap versions
5. **Roadmap Analytics**: Track completion rates and time-to-completion

---

**Last Updated**: January 2024  
**Version**: 1.0
