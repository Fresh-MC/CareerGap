# Roadmap Generation - Quick Start Guide

## What is Roadmap Generation?

The Roadmap Generation system creates **concrete, time-bound action plans** for validated career strategies. It's strictly gated by the Strategy Lifecycle State Machine to ensure roadmaps are only generated when a strategy has been proven through real-world outcomes (interviews).

**Key Principle**: Roadmaps feel earned, not random. Interviews unlock execution planning naturally.

## When Can You Generate a Roadmap?

**Only when `strategy_state == EXECUTE`**

### State Requirements

| State | Can Generate? | What You Need |
|-------|---------------|---------------|
| **EXPLORE** | ❌ No | At least 1 interview to validate |
| **VALIDATE** | ❌ No | 2+ interviews + confidence ≥ 0.65 |
| **EXECUTE** | ✅ Yes | Strategy validated! Ready for roadmap |
| **RECONSIDER** | ❌ No | Strategy failed, needs re-evaluation |

## How to Generate a Roadmap

### Step 1: Check Your Strategy State

```bash
# Your current session should show strategy_state
{
  "current_strategy": {
    "strategy": "ResumeOptimization",
    "strategy_state": "execute",  # ← Check this!
    "current_confidence": 0.72
  }
}
```

### Step 2: Make API Request

```bash
POST /api/roadmap
{
  "user_id": "your_user_id",
  "session": { ... }  # Include full session with strategy_state
}
```

### Step 3: Handle Response

#### Success (200 OK)
```json
{
  "roadmap": {
    "strategy": "ResumeOptimization",
    "estimated_completion_days": 21,
    "actions": [
      {
        "title": "Rewrite Primary Project Description",
        "deadline_days": 3,
        "priority": "critical"
      },
      // ... more actions
    ]
  },
  "eligible": true
}
```

#### Blocked (403 Forbidden)
```json
{
  "error": "Roadmap generation not allowed",
  "eligible": false,
  "reason": "Strategy is in VALIDATE state. Need 2+ interviews to execute.",
  "current_state": "validate"
}
```

## What You Get: Roadmap Components

### 1. Goals (2-3)
High-level objectives aligned with your strategy
- Example: "Strengthen Resume Evidence"

### 2. Milestones (2-4)
Progress checkpoints with success criteria
- Example: "Resume Draft Complete" (7 days)

### 3. Actions (5-10)
Concrete, time-bound tasks
- Example: "Rewrite Primary Project Description" (deadline: 3 days)

## Strategy-Specific Roadmaps

### ResumeOptimization (21 days)
**7 actions** focusing on transforming resume with concrete outcomes
- Rewrite projects with metrics
- ATS compatibility check
- Get professional review

### SkillGapPatch (42 days)
**8 actions** for acquiring missing skill + building evidence
- Structured learning (14 days)
- Applied project (28 days)
- Public portfolio + applications

### RoleShift (21 days)
**7 actions** for positioning toward adjacent role
- Research target role
- Reframe experience
- Connect with professionals

### HoldPosition (14 days)
**7 actions** for increasing application volume
- Expand target list
- Batch prepare materials
- 30+ targeted applications

## Testing Your Implementation

### Run Tests

```bash
cd resume_parser
python test_roadmap_gating.py
```

### Expected: 7 Tests Pass

```
[PASS] EXPLORE state correctly blocks roadmap generation
[PASS] VALIDATE state correctly blocks roadmap generation
[PASS] EXECUTE state correctly allows roadmap generation
[PASS] RECONSIDER state correctly blocks roadmap generation
[PASS] Full roadmap generated successfully in EXECUTE state
[PASS] Roadmap generation correctly blocked in EXPLORE state
[PASS] All strategy types successfully generated roadmaps

TEST RESULTS: 7 passed, 0 failed
```

## Common Questions

### Q: Why can't I generate a roadmap in VALIDATE state?
**A**: VALIDATE means your strategy shows promise (1 interview) but needs more validation. You need 2+ interviews and confidence ≥ 0.65 to unlock EXECUTE state and roadmap generation.

### Q: What happens if my strategy changes?
**A**: Any existing roadmap will be invalidated. The `invalidated` flag is set and you'll need to generate a new roadmap for the new strategy once it reaches EXECUTE state.

### Q: Can I customize the roadmap?
**A**: Roadmaps are generated from strategy-specific templates to ensure consistency. However, each roadmap is tailored using your session data (bottlenecks, evidence, profile signals).

### Q: How long do roadmaps take?
**A**: Varies by strategy:
- ResumeOptimization: 21 days
- SkillGapPatch: 42 days
- RoleShift: 21 days
- HoldPosition: 14 days

### Q: What if I complete actions faster?
**A**: Great! Deadlines are targets, not hard limits. Complete actions at your own pace. The review period (typically 14 days) is when you should assess overall progress.

## Troubleshooting

### Issue: Getting 403 Forbidden

**Check**:
1. Is `strategy_state` included in session?
2. Is `strategy_state == "execute"`?
3. Did you recently change strategies? (May need to re-validate)

### Issue: Python Script Not Found

**Check**:
```bash
# Verify file exists
ls resume_parser/roadmap_generator.py

# Make sure it's executable
python resume_parser/roadmap_generator.py
```

### Issue: Backend Not Calling Python

**Check backend logs**:
```
✅ Roadmap generated for user X (strategy in EXECUTE state)  # Success
⚠️  Roadmap generation blocked for user X: ...                # Blocked
```

## Integration Steps

1. ✅ Install Python dependencies: `pip install -r requirements.txt`
2. ✅ Run tests: `python test_roadmap_gating.py`
3. ✅ Start backend: `cargo run` (from backend/)
4. ✅ Make API request with session including `strategy_state`
5. ✅ Handle 200 (success) or 403 (blocked) responses in frontend

## Example Flow

```
User Journey:
1. Upload resume → Snapshot created
2. Analyze resume → Strategy selected (EXPLORE state)
3. Apply to jobs → Track outcomes
4. Get 1 interview → Transition to VALIDATE state
5. Get 2nd interview → Transition to EXECUTE state ✅
6. Generate roadmap → Success! 7 concrete actions
7. Follow roadmap → Complete actions over 21 days
8. Review progress → Assess completion and outcomes
```

## Files Reference

- **Main Module**: `resume_parser/roadmap_generator.py`
- **Tests**: `resume_parser/test_roadmap_gating.py`
- **Backend API**: `backend/src/api.rs` (generate_roadmap handler)
- **Full Docs**: `ROADMAP_GENERATION.md`

## Need Help?

1. Check logs: Backend (terminal) and Python (stderr)
2. Verify session structure includes `strategy_state`
3. Run tests to confirm gating logic working
4. Check `ROADMAP_GENERATION.md` for detailed architecture

---

**Remember**: Roadmaps are unlocked by validation through interviews. This ensures you're investing effort in proven strategies, not speculative ones.
