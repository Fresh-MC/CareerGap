# Remediation Planner Module

## Overview

The **Planner** module implements the PLAN phase of the agentic security hardening system. It generates remediation plans but **DOES NOT execute any actions**.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    PLANNER MODULE                           │
│                    (Read-Only)                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  INPUTS                         OUTPUTS                     │
│  ───────                        ────────                    │
│  • Latest sensor snapshot   →   • RemediationPlan          │
│  • Previous snapshot           │   - Ordered steps         │
│  • Risk scores                 │   - Deferred policies     │
│  • Policy metadata             │   - Goal alignment        │
│  • Execution history           │   - Human approval = true │
│  • Recommender output          │                           │
│  • Explicit goal               │                           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Architectural Rules (STRICT)

| Rule | Description |
|------|-------------|
| ✅ Read-Only | Planning consumes data but never mutates system state |
| ❌ No Remediation | Planner NEVER executes fixes |
| ❌ No Rescanning | Uses existing snapshots only |
| ❌ No Implicit Goals | Goals must be explicit and stored |
| ✅ Human Approval | `requires_human_approval` is ALWAYS true |
| ✅ Consumes Only | Uses outputs from sensing, scoring, recommender |

## Goal Model

Goals are explicit, stored, and passed into the planner.

### Goal Types

```rust
pub enum GoalType {
    /// Maintain minimum compliance percentage
    ComplianceThreshold,    // e.g., "≥80% compliance on critical"
    /// Focus on specific severity levels
    SeverityFocus,          // e.g., "critical and high only"
    /// Target specific policy categories
    CategoryFocus,          // e.g., "SSH policies only"
    /// Minimize risk score
    RiskMinimization,       // e.g., "reduce risk below 0.5"
    /// Custom goal with flexible parameters
    Custom,
}
```

### Example: Compliance Threshold Goal

```rust
use nogap_core::planner::PlanningGoal;

let goal = PlanningGoal::compliance_threshold(
    0.8,                                    // 80% target
    Some(vec!["critical".to_string()])     // On critical policies
);
// Creates: "Maintain ≥80% compliance on critical policies"
```

## Planning Pipeline

### Step 1: Candidate Generation

The planner calls the existing recommender to get candidate policies:

```rust
// Recommender output is passed as input
let recommendations = RecommendationResult {
    recommendations: vec![...],  // Candidate policies
    invalid_suggestions: vec![], // Rejected by validation
    warnings: vec![],            // Any issues
};
```

**Important**: The planner does NOT reorder candidates at this stage.

### Step 2: Constraint Evaluation (Deterministic)

Each candidate is evaluated against hard and soft constraints:

#### Hard Constraints (Auto-Defer)

| Constraint | Effect |
|------------|--------|
| `platform_mismatch` | Policy platform ≠ current system |
| `policy_not_applicable` | Policy doesn't apply to config |
| `missing_prerequisite` | Required policy not satisfied |
| `explicitly_disabled_by_user` | User blocked this policy |

If **any** hard constraint exists → policy moves to **DEFERRED**.

#### Soft Constraints (Affect Ordering)

| Constraint | Penalty | Description |
|------------|---------|-------------|
| `requires_reboot` | 0.30 | Remediation needs restart |
| `historical_failure` | 0.20-0.60 | Failed before (scales with count) |
| `rollback_unavailable` | 0.40 | Cannot undo remediation |
| `high_blast_radius` | 0.35 | Affects many services |
| `service_disruption_risk` | 0.10-0.50 | Risk of downtime |
| `conflicts_with_other_policy` | 0.45 | Policy conflict |

### Step 3: Plan Synthesis

#### LLM-Assisted (Optional)

When enabled, the planner:
1. Generates a constrained prompt with candidates, goals, risk scores
2. LLM orders steps and provides reasoning/confidence
3. LLM output is **strictly validated**

**LLM MUST NOT:**
- Invent policies
- Change goals
- Suggest execution

If LLM validation fails → automatic fallback to deterministic.

#### Deterministic Fallback

Ordering algorithm:
1. Sort by risk score (descending) - highest risk first
2. Within same risk, sort by penalty (ascending) - prefer fewer constraints
3. Apply goal-specific filtering
4. Cap at `max_steps` configuration

## Output: Canonical Plan Object

```json
{
  "plan_id": "550e8400-e29b-41d4-a716-446655440000",
  "plan_type": "remediation",
  "goal": {
    "goal_id": "...",
    "description": "Maintain ≥80% compliance on critical policies",
    "goal_type": "ComplianceThreshold",
    "parameters": { "compliance_threshold": 0.8 },
    "priority": 1,
    "active": true
  },
  "generated_at": "2025-12-30T10:00:00Z",
  "source_snapshot": {
    "timestamp": 1735556400,
    "source": "agent_sense",
    "policy_count": 50,
    "compliance_rate": 0.65
  },
  "steps": [
    {
      "policy_id": "SEC-CRIT-001",
      "priority": 1,
      "reason": "critical severity policy with risk score 1.00",
      "risk_score": 1.0,
      "confidence": 0.91,
      "constraints_considered": [],
      "expected_impact": "Remediate critical policy",
      "estimated_duration_minutes": 5
    },
    {
      "policy_id": "SEC-HIGH-002",
      "priority": 2,
      "reason": "high severity policy with risk score 0.75",
      "risk_score": 0.75,
      "confidence": 0.85,
      "constraints_considered": ["requires_reboot"],
      "expected_impact": "Remediate high policy",
      "estimated_duration_minutes": 5
    }
  ],
  "deferred": [
    {
      "policy_id": "SEC-LINUX-001",
      "reason": "Policy platform does not match current system",
      "blocking_constraints": ["platform_mismatch"]
    }
  ],
  "requires_human_approval": true,
  "metadata": {
    "llm_assisted": false,
    "llm_model": null,
    "used_deterministic_fallback": true,
    "warnings": [],
    "candidates_considered": 10,
    "planning_duration_ms": 15
  }
}
```

## Validation Rules

The planner validates all generated plans:

| Rule | Description |
|------|-------------|
| No Duplicates | Every policy in steps OR deferred, never both |
| Valid IDs | All policy_ids must exist in candidate input |
| Sequential Priority | Priorities must be 1, 2, 3, ... |
| Confidence Bounds | 0.0 ≤ confidence ≤ 1.0 |
| Human Approval | `requires_human_approval` MUST be true |

## Usage Example

```rust
use nogap_core::planner::{Planner, PlannerConfig, PlannerInput, PlanningGoal};
use nogap_core::ai_recommender::RecommendationResult;
use std::collections::HashSet;

// 1. Create planner
let planner = Planner::with_defaults();

// 2. Define explicit goal
let goal = PlanningGoal::compliance_threshold(0.8, None);

// 3. Gather inputs from existing modules
let input = PlannerInput {
    current_audit: sensor_snapshot.audit_results,
    previous_audit: Some(previous_snapshot.audit_results),
    risk_scores: risk_scoring::calculate_all_risk_scores(&policies, &audit_results),
    policies: policy_parser::load_policies()?,
    execution_history: load_execution_history()?,
    recommendations: ai_recommender::get_recommendations(context, &policies),
    drift_report: Some(drift_report),
    disabled_policies: HashSet::new(),
    current_platform: "windows".to_string(),
};

// 4. Generate plan (read-only operation)
let plan = planner.generate_plan(goal, &input)?;

// 5. Plan requires human approval before any action
assert!(plan.requires_human_approval);

// Present plan to user for review...
```

## Configuration

```rust
pub struct PlannerConfig {
    /// Maximum steps in a single plan (default: 50)
    pub max_steps: usize,
    /// Minimum confidence threshold for LLM suggestions (default: 0.5)
    pub min_confidence: f32,
    /// Whether to use LLM for ordering (default: true)
    pub use_llm_ordering: bool,
    /// Historical failure count to trigger soft constraint (default: 2)
    pub failure_threshold: u32,
}
```

## Tests

The planner module includes comprehensive unit tests:

| Test | Description |
|------|-------------|
| `test_goal_compliance_threshold` | Goal creation with threshold |
| `test_goal_risk_minimization` | Risk-based goal creation |
| `test_hard_constraint_platform_mismatch` | Platform filtering |
| `test_soft_constraint_penalty` | Penalty scoring |
| `test_constraint_evaluation` | Full constraint evaluation |
| `test_planner_generate_plan` | End-to-end plan generation |
| `test_planner_disabled_policy` | Disabled policy deferral |
| `test_planner_historical_failure` | Historical failure handling |
| `test_plan_validation_no_duplicates` | No duplicate policies |
| `test_plan_validation_sequential_priorities` | Priority ordering |
| `test_plan_validation_confidence_bounds` | Confidence validation |
| `test_plan_validation_human_approval_required` | Approval flag |
| `test_platform_matching` | Platform compatibility |
| `test_deterministic_fallback` | Fallback ordering |

Run tests:
```bash
cargo test -p nogap_core planner
```

## Integration Points

| Module | Integration |
|--------|-------------|
| `sensor_scheduler` | Provides latest/previous audit snapshots |
| `risk_scoring` | Provides risk scores for prioritization |
| `ai_recommender` | Provides candidate policy recommendations |
| `drift_detection` | Provides drift context for planning |
| `engine` | AuditResult type for audit data |
| `types` | Policy type for metadata |

## DO NOT DO

| Violation | Why |
|-----------|-----|
| ❌ Auto-remediate | Planner is read-only |
| ❌ Rename to "agent" | Not yet - follows existing naming |
| ❌ Bypass human approval | Safety critical |
| ❌ Loosen validation | Maintains integrity |
| ❌ Implicit goals | Goals must be explicit |
