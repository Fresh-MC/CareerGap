//! Remediation Plan Generator Module
//!
//! This module implements the PLAN phase of the agentic system.
//! It produces remediation plans but DOES NOT execute any actions.
//!
//! ARCHITECTURAL RULES (STRICT):
//! - Planning is read-only
//! - No remediation or system mutation
//! - No rescanning the system
//! - No implicit goals
//! - Human approval is mandatory
//! - Planner consumes existing outputs only
//!
//! PLANNING PIPELINE:
//! 1. Candidate Generation - Call existing recommender
//! 2. Constraint Evaluation - Deterministic filtering
//! 3. Plan Synthesis - Optional LLM-assisted ordering

use crate::ai_recommender::{PolicyRecommendation, RecommendationResult};
use crate::drift_detection::DriftReport;
use crate::engine::AuditResult;
use crate::risk_scoring::PolicyRiskScore;
use crate::types::Policy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// ============================================================
// GOAL MODEL
// ============================================================

/// Explicit goal for the planner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningGoal {
    /// Unique goal identifier
    pub goal_id: String,
    /// Human-readable goal description
    pub description: String,
    /// Goal type for specialized handling
    pub goal_type: GoalType,
    /// Goal-specific parameters
    pub parameters: GoalParameters,
    /// Priority of this goal (higher = more important)
    pub priority: u32,
    /// Whether this goal is currently active
    pub active: bool,
}

/// Types of planning goals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GoalType {
    /// Maintain minimum compliance percentage
    ComplianceThreshold,
    /// Focus on specific severity levels
    SeverityFocus,
    /// Target specific policy categories
    CategoryFocus,
    /// Minimize risk score
    RiskMinimization,
    /// Custom goal with flexible parameters
    Custom,
}

/// Goal-specific parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoalParameters {
    /// Target compliance percentage (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compliance_threshold: Option<f32>,
    /// Target severity levels to focus on
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_severities: Option<Vec<String>>,
    /// Target policy categories/tags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_categories: Option<Vec<String>>,
    /// Maximum risk score to achieve
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_risk_score: Option<f32>,
    /// Custom key-value parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, String>>,
}

impl PlanningGoal {
    /// Create a compliance threshold goal
    /// Example: "Maintain ≥80% compliance on critical policies"
    pub fn compliance_threshold(threshold: f32, severities: Option<Vec<String>>) -> Self {
        Self {
            goal_id: Uuid::new_v4().to_string(),
            description: format!(
                "Maintain ≥{:.0}% compliance{}",
                threshold * 100.0,
                severities
                    .as_ref()
                    .map(|s| format!(" on {} policies", s.join(", ")))
                    .unwrap_or_default()
            ),
            goal_type: GoalType::ComplianceThreshold,
            parameters: GoalParameters {
                compliance_threshold: Some(threshold),
                target_severities: severities,
                ..Default::default()
            },
            priority: 1,
            active: true,
        }
    }

    /// Create a risk minimization goal
    pub fn minimize_risk(max_score: f32) -> Self {
        Self {
            goal_id: Uuid::new_v4().to_string(),
            description: format!("Reduce system risk score below {:.2}", max_score),
            goal_type: GoalType::RiskMinimization,
            parameters: GoalParameters {
                max_risk_score: Some(max_score),
                ..Default::default()
            },
            priority: 1,
            active: true,
        }
    }
}

// ============================================================
// CONSTRAINTS
// ============================================================

/// Hard constraints that auto-defer a policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HardConstraint {
    /// Policy platform doesn't match current system
    PlatformMismatch,
    /// Policy is not applicable to current configuration
    PolicyNotApplicable,
    /// Required prerequisite policy not satisfied
    MissingPrerequisite { prerequisite_id: String },
    /// User explicitly disabled this policy
    ExplicitlyDisabledByUser,
}

impl HardConstraint {
    pub fn as_str(&self) -> &'static str {
        match self {
            HardConstraint::PlatformMismatch => "platform_mismatch",
            HardConstraint::PolicyNotApplicable => "policy_not_applicable",
            HardConstraint::MissingPrerequisite { .. } => "missing_prerequisite",
            HardConstraint::ExplicitlyDisabledByUser => "explicitly_disabled_by_user",
        }
    }

    pub fn description(&self) -> String {
        match self {
            HardConstraint::PlatformMismatch => {
                "Policy platform does not match current system".to_string()
            }
            HardConstraint::PolicyNotApplicable => {
                "Policy is not applicable to current configuration".to_string()
            }
            HardConstraint::MissingPrerequisite { prerequisite_id } => {
                format!("Missing prerequisite policy: {}", prerequisite_id)
            }
            HardConstraint::ExplicitlyDisabledByUser => {
                "Policy explicitly disabled by user".to_string()
            }
        }
    }
}

/// Soft constraints that affect ordering but don't auto-defer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SoftConstraint {
    /// Remediation requires system reboot
    RequiresReboot,
    /// Policy has failed remediation in the past
    HistoricalFailure { failure_count: u32, last_failure: u64 },
    /// Rollback mechanism not available
    RollbackUnavailable,
    /// High blast radius (affects many services/users)
    HighBlastRadius { affected_services: Vec<String> },
    /// Risk of service disruption
    ServiceDisruptionRisk { risk_level: String },
    /// Conflicts with another policy
    ConflictsWithPolicy { conflicting_id: String },
}

impl SoftConstraint {
    pub fn as_str(&self) -> &'static str {
        match self {
            SoftConstraint::RequiresReboot => "requires_reboot",
            SoftConstraint::HistoricalFailure { .. } => "historical_failure",
            SoftConstraint::RollbackUnavailable => "rollback_unavailable",
            SoftConstraint::HighBlastRadius { .. } => "high_blast_radius",
            SoftConstraint::ServiceDisruptionRisk { .. } => "service_disruption_risk",
            SoftConstraint::ConflictsWithPolicy { .. } => "conflicts_with_other_policy",
        }
    }

    /// Returns a penalty score for ordering (higher = lower priority)
    pub fn penalty_score(&self) -> f32 {
        match self {
            SoftConstraint::RequiresReboot => 0.3,
            SoftConstraint::HistoricalFailure { failure_count, .. } => {
                0.2 * (*failure_count as f32).min(3.0)
            }
            SoftConstraint::RollbackUnavailable => 0.4,
            SoftConstraint::HighBlastRadius { .. } => 0.35,
            SoftConstraint::ServiceDisruptionRisk { risk_level } => match risk_level.as_str() {
                "high" => 0.5,
                "medium" => 0.25,
                _ => 0.1,
            },
            SoftConstraint::ConflictsWithPolicy { .. } => 0.45,
        }
    }
}

/// Combined constraint evaluation result for a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintEvaluation {
    pub policy_id: String,
    pub hard_constraints: Vec<HardConstraint>,
    pub soft_constraints: Vec<SoftConstraint>,
    pub is_deferred: bool,
    pub total_penalty: f32,
}

impl ConstraintEvaluation {
    pub fn new(policy_id: &str) -> Self {
        Self {
            policy_id: policy_id.to_string(),
            hard_constraints: Vec::new(),
            soft_constraints: Vec::new(),
            is_deferred: false,
            total_penalty: 0.0,
        }
    }

    pub fn add_hard_constraint(&mut self, constraint: HardConstraint) {
        self.hard_constraints.push(constraint);
        self.is_deferred = true;
    }

    pub fn add_soft_constraint(&mut self, constraint: SoftConstraint) {
        self.total_penalty += constraint.penalty_score();
        self.soft_constraints.push(constraint);
    }
}

// ============================================================
// PLAN OUTPUT STRUCTURES
// ============================================================

/// A single step in the remediation plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Policy ID to remediate
    pub policy_id: String,
    /// Priority in execution order (1 = highest)
    pub priority: u32,
    /// Human-readable reason for inclusion
    pub reason: String,
    /// Risk score from risk_scoring module
    pub risk_score: f32,
    /// Confidence in this step (0.0 to 1.0)
    pub confidence: f32,
    /// Constraints considered during planning
    pub constraints_considered: Vec<String>,
    /// Expected impact description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_impact: Option<String>,
    /// Estimated time to execute (minutes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_duration_minutes: Option<u32>,
}

/// A deferred policy (excluded from the plan)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredPolicy {
    /// Policy ID that was deferred
    pub policy_id: String,
    /// Human-readable reason for deferral
    pub reason: String,
    /// Blocking hard constraints
    pub blocking_constraints: Vec<String>,
}

/// Snapshot reference for plan provenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotReference {
    /// Snapshot timestamp
    pub timestamp: u64,
    /// Snapshot source (e.g., "agent_sense", "manual")
    pub source: String,
    /// Number of policies in snapshot
    pub policy_count: usize,
    /// Compliance rate at snapshot time
    pub compliance_rate: f32,
}

/// The canonical plan object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationPlan {
    /// Unique plan identifier
    pub plan_id: String,
    /// Plan type (always "remediation" for now)
    pub plan_type: String,
    /// Goal that drove this plan
    pub goal: PlanningGoal,
    /// When the plan was generated
    pub generated_at: String,
    /// Source snapshot information
    pub source_snapshot: SnapshotReference,
    /// Previous snapshot (for drift context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_snapshot: Option<SnapshotReference>,
    /// Ordered steps to execute
    pub steps: Vec<PlanStep>,
    /// Policies deferred from this plan
    pub deferred: Vec<DeferredPolicy>,
    /// Human approval is always required
    pub requires_human_approval: bool,
    /// Plan generation metadata
    pub metadata: PlanMetadata,
}

/// Metadata about plan generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanMetadata {
    /// Whether LLM was used for ordering
    pub llm_assisted: bool,
    /// LLM model used (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
    /// Whether fallback to deterministic ordering was used
    pub used_deterministic_fallback: bool,
    /// Warnings generated during planning
    pub warnings: Vec<String>,
    /// Total candidates considered
    pub candidates_considered: usize,
    /// Planning duration in milliseconds
    pub planning_duration_ms: u64,
}

// ============================================================
// PLANNER INPUTS
// ============================================================

/// All inputs required by the planner
#[derive(Debug, Clone)]
pub struct PlannerInput {
    /// Latest sensor snapshot audit results
    pub current_audit: Vec<AuditResult>,
    /// Previous snapshot audit results (for drift)
    pub previous_audit: Option<Vec<AuditResult>>,
    /// Risk scores from risk_scoring module
    pub risk_scores: Vec<PolicyRiskScore>,
    /// Policy metadata
    pub policies: Vec<Policy>,
    /// Historical execution outcomes
    pub execution_history: Vec<ExecutionOutcome>,
    /// Output from the existing recommender
    pub recommendations: RecommendationResult,
    /// Drift report (if available)
    pub drift_report: Option<DriftReport>,
    /// User-disabled policy IDs
    pub disabled_policies: HashSet<String>,
    /// Current system platform
    pub current_platform: String,
}

/// Historical execution outcome for a policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutcome {
    pub policy_id: String,
    pub timestamp: u64,
    pub success: bool,
    pub error_message: Option<String>,
    pub rollback_available: bool,
    pub duration_ms: u64,
}

// ============================================================
// PLANNER IMPLEMENTATION
// ============================================================

/// The main planner struct
pub struct Planner {
    /// Configuration for the planner
    config: PlannerConfig,
}

/// Planner configuration
#[derive(Debug, Clone)]
pub struct PlannerConfig {
    /// Maximum steps in a single plan
    pub max_steps: usize,
    /// Minimum confidence threshold for LLM suggestions
    pub min_confidence: f32,
    /// Whether to use LLM for ordering (if available)
    pub use_llm_ordering: bool,
    /// Historical failure threshold to trigger soft constraint
    pub failure_threshold: u32,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            max_steps: 50,
            min_confidence: 0.5,
            use_llm_ordering: true,
            failure_threshold: 2,
        }
    }
}

impl Planner {
    /// Create a new planner with given configuration
    pub fn new(config: PlannerConfig) -> Self {
        log::info!("[PLANNER] Initializing with config: {:?}", config);
        Self { config }
    }

    /// Create a planner with default configuration
    pub fn with_defaults() -> Self {
        Self::new(PlannerConfig::default())
    }

    /// Generate a remediation plan
    ///
    /// This is the main entry point for plan generation.
    /// The planner DOES NOT execute any actions.
    ///
    /// # Arguments
    /// * `goal` - The explicit planning goal
    /// * `input` - All required inputs (snapshots, scores, recommendations, etc.)
    ///
    /// # Returns
    /// A RemediationPlan ready for human approval
    pub fn generate_plan(
        &self,
        goal: PlanningGoal,
        input: &PlannerInput,
    ) -> Result<RemediationPlan, PlannerError> {
        let start_time = std::time::Instant::now();
        log::info!("[PLANNER] Starting plan generation for goal: {}", goal.description);

        // STEP 1: Candidate Generation
        // Use the existing recommender output (already provided)
        let candidates = self.extract_candidates(&input.recommendations, input);
        log::info!("[PLANNER] Step 1: {} candidates from recommender", candidates.len());

        // STEP 2: Constraint Evaluation (Deterministic)
        let (actionable, deferred) = self.evaluate_constraints(&candidates, input);
        log::info!(
            "[PLANNER] Step 2: {} actionable, {} deferred",
            actionable.len(),
            deferred.len()
        );

        // STEP 3: Plan Synthesis
        // Try LLM-assisted ordering if enabled, fallback to deterministic
        let (steps, metadata) = if self.config.use_llm_ordering {
            match self.llm_assisted_ordering(&actionable, &goal, input) {
                Ok((steps, meta)) => {
                    log::info!("[PLANNER] Step 3: LLM-assisted ordering successful");
                    (steps, meta)
                }
                Err(e) => {
                    log::warn!("[PLANNER] LLM ordering failed: {}, using deterministic fallback", e);
                    self.deterministic_ordering(&actionable, &goal, input)
                }
            }
        } else {
            log::info!("[PLANNER] Step 3: Using deterministic ordering (LLM disabled)");
            self.deterministic_ordering(&actionable, &goal, input)
        };

        // Build plan object
        let plan = self.build_plan(goal, steps, deferred, input, metadata, start_time.elapsed());

        // Validate plan integrity
        self.validate_plan(&plan, &candidates)?;

        log::info!(
            "[PLANNER] Plan generated: {} steps, {} deferred, {} ms",
            plan.steps.len(),
            plan.deferred.len(),
            plan.metadata.planning_duration_ms
        );

        Ok(plan)
    }

    /// Extract candidates from recommender output
    fn extract_candidates(
        &self,
        recommendations: &RecommendationResult,
        input: &PlannerInput,
    ) -> Vec<CandidatePolicy> {
        let policy_map: HashMap<&str, &Policy> =
            input.policies.iter().map(|p| (p.id.as_str(), p)).collect();

        let risk_map: HashMap<&str, &PolicyRiskScore> = input
            .risk_scores
            .iter()
            .map(|r| (r.policy_id.as_str(), r))
            .collect();

        recommendations
            .recommendations
            .iter()
            .filter_map(|rec| {
                let policy = policy_map.get(rec.policy_id.as_str())?;
                let risk = risk_map.get(rec.policy_id.as_str());

                Some(CandidatePolicy {
                    policy_id: rec.policy_id.clone(),
                    policy: (*policy).clone(),
                    recommendation: rec.clone(),
                    risk_score: risk.map(|r| r.risk_score).unwrap_or(0.0),
                    is_compliant: risk.map(|r| r.is_compliant).unwrap_or(true),
                })
            })
            .filter(|c| !c.is_compliant) // Only include non-compliant policies
            .collect()
    }

    /// Evaluate constraints for all candidates
    fn evaluate_constraints(
        &self,
        candidates: &[CandidatePolicy],
        input: &PlannerInput,
    ) -> (Vec<EvaluatedCandidate>, Vec<DeferredPolicy>) {
        let mut actionable = Vec::new();
        let mut deferred = Vec::new();

        // Build execution history lookup
        let history_map: HashMap<&str, Vec<&ExecutionOutcome>> = {
            let mut map = HashMap::new();
            for outcome in &input.execution_history {
                map.entry(outcome.policy_id.as_str())
                    .or_insert_with(Vec::new)
                    .push(outcome);
            }
            map
        };

        for candidate in candidates {
            let mut eval = ConstraintEvaluation::new(&candidate.policy_id);

            // Check hard constraints
            self.check_hard_constraints(&candidate.policy, input, &mut eval);

            if eval.is_deferred {
                // Policy has hard constraints - defer it
                deferred.push(DeferredPolicy {
                    policy_id: candidate.policy_id.clone(),
                    reason: eval
                        .hard_constraints
                        .first()
                        .map(|c| c.description())
                        .unwrap_or_else(|| "Unknown constraint".to_string()),
                    blocking_constraints: eval
                        .hard_constraints
                        .iter()
                        .map(|c| c.as_str().to_string())
                        .collect(),
                });
            } else {
                // Check soft constraints
                self.check_soft_constraints(
                    &candidate.policy,
                    history_map.get(candidate.policy_id.as_str()),
                    &mut eval,
                );

                actionable.push(EvaluatedCandidate {
                    candidate: candidate.clone(),
                    evaluation: eval,
                });
            }
        }

        (actionable, deferred)
    }

    /// Check hard constraints for a policy
    fn check_hard_constraints(
        &self,
        policy: &Policy,
        input: &PlannerInput,
        eval: &mut ConstraintEvaluation,
    ) {
        // Platform mismatch
        if !self.platform_matches(&policy.platform, &input.current_platform) {
            eval.add_hard_constraint(HardConstraint::PlatformMismatch);
        }

        // Explicitly disabled by user
        if input.disabled_policies.contains(&policy.id) {
            eval.add_hard_constraint(HardConstraint::ExplicitlyDisabledByUser);
        }

        // TODO: Add prerequisite checking when dependency graph is available
        // For now, we don't have prerequisite metadata in Policy
    }

    /// Check soft constraints for a policy
    fn check_soft_constraints(
        &self,
        policy: &Policy,
        history: Option<&Vec<&ExecutionOutcome>>,
        eval: &mut ConstraintEvaluation,
    ) {
        // Requires reboot
        if policy.post_reboot_required.unwrap_or(false) {
            eval.add_soft_constraint(SoftConstraint::RequiresReboot);
        }

        // Rollback unavailable
        if !policy.reversible.unwrap_or(true) {
            eval.add_soft_constraint(SoftConstraint::RollbackUnavailable);
        }

        // Historical failures
        if let Some(outcomes) = history {
            let failure_count = outcomes.iter().filter(|o| !o.success).count() as u32;
            if failure_count >= self.config.failure_threshold {
                let last_failure = outcomes
                    .iter()
                    .filter(|o| !o.success)
                    .map(|o| o.timestamp)
                    .max()
                    .unwrap_or(0);

                eval.add_soft_constraint(SoftConstraint::HistoricalFailure {
                    failure_count,
                    last_failure,
                });
            }
        }
    }

    /// Check if policy platform matches current system
    fn platform_matches(&self, policy_platform: &str, current_platform: &str) -> bool {
        let policy_lower = policy_platform.to_lowercase();
        let current_lower = current_platform.to_lowercase();

        if policy_lower == current_lower {
            return true;
        }

        // Handle Windows variants
        if policy_lower == "windows"
            && (current_lower.contains("windows")
                || current_lower.contains("win32")
                || current_lower.contains("win64"))
        {
            return true;
        }

        // Handle Linux variants
        if policy_lower == "linux"
            && (current_lower.contains("linux")
                || current_lower.contains("ubuntu")
                || current_lower.contains("debian")
                || current_lower.contains("rhel")
                || current_lower.contains("centos"))
        {
            return true;
        }

        false
    }

    /// LLM-assisted ordering (returns error if LLM unavailable)
    fn llm_assisted_ordering(
        &self,
        candidates: &[EvaluatedCandidate],
        goal: &PlanningGoal,
        _input: &PlannerInput,
    ) -> Result<(Vec<PlanStep>, PlanMetadata), PlannerError> {
        // Generate prompt for LLM
        let _prompt = self.generate_ordering_prompt(candidates, goal);

        // For now, we don't have an actual LLM integration
        // This would call an LLM API and validate the response
        // Return error to trigger deterministic fallback
        Err(PlannerError::LlmUnavailable(
            "LLM integration not configured".to_string(),
        ))
    }

    /// Generate prompt for LLM ordering
    fn generate_ordering_prompt(
        &self,
        candidates: &[EvaluatedCandidate],
        goal: &PlanningGoal,
    ) -> String {
        let candidate_list: String = candidates
            .iter()
            .map(|c| {
                format!(
                    "- ID: {}, Risk: {:.2}, Penalty: {:.2}, Severity: {}",
                    c.candidate.policy_id,
                    c.candidate.risk_score,
                    c.evaluation.total_penalty,
                    c.candidate
                        .policy
                        .severity
                        .clone()
                        .unwrap_or_else(|| "medium".to_string())
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"You are a security remediation planner. Order the following policies for remediation.

GOAL: {}

CANDIDATES:
{}

RULES:
1. You MUST NOT invent new policies
2. You MUST NOT change the goal
3. You MUST NOT suggest execution
4. Order by: risk score (descending), then penalty (ascending)
5. Provide confidence (0.0-1.0) for each ordering decision

Output JSON array of policy_ids in execution order with confidence and reason."#,
            goal.description, candidate_list
        )
    }

    /// Deterministic ordering (fallback)
    fn deterministic_ordering(
        &self,
        candidates: &[EvaluatedCandidate],
        goal: &PlanningGoal,
        _input: &PlannerInput,
    ) -> (Vec<PlanStep>, PlanMetadata) {
        // Sort candidates by: risk score (desc), then penalty (asc)
        let mut sorted: Vec<_> = candidates.iter().collect();
        sorted.sort_by(|a, b| {
            // Primary: risk score descending
            let risk_cmp = b
                .candidate
                .risk_score
                .partial_cmp(&a.candidate.risk_score)
                .unwrap_or(std::cmp::Ordering::Equal);

            if risk_cmp != std::cmp::Ordering::Equal {
                return risk_cmp;
            }

            // Secondary: penalty ascending (prefer lower penalty)
            a.evaluation
                .total_penalty
                .partial_cmp(&b.evaluation.total_penalty)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply goal-specific filtering
        let filtered = self.apply_goal_filter(&sorted, goal);

        // Build plan steps
        let steps: Vec<PlanStep> = filtered
            .iter()
            .enumerate()
            .take(self.config.max_steps)
            .map(|(i, candidate)| {
                let constraints_considered: Vec<String> = candidate
                    .evaluation
                    .soft_constraints
                    .iter()
                    .map(|c| c.as_str().to_string())
                    .collect();

                PlanStep {
                    policy_id: candidate.candidate.policy_id.clone(),
                    priority: (i + 1) as u32,
                    reason: self.generate_step_reason(candidate, goal),
                    risk_score: candidate.candidate.risk_score,
                    confidence: self.calculate_confidence(candidate),
                    constraints_considered,
                    expected_impact: Some(format!(
                        "Remediate {} policy",
                        candidate
                            .candidate
                            .policy
                            .severity
                            .clone()
                            .unwrap_or_else(|| "medium".to_string())
                    )),
                    estimated_duration_minutes: Some(5), // Default estimate
                }
            })
            .collect();

        let metadata = PlanMetadata {
            llm_assisted: false,
            llm_model: None,
            used_deterministic_fallback: true,
            warnings: Vec::new(),
            candidates_considered: candidates.len(),
            planning_duration_ms: 0, // Will be updated later
        };

        (steps, metadata)
    }

    /// Apply goal-specific filtering to sorted candidates
    fn apply_goal_filter<'a>(
        &self,
        candidates: &[&'a EvaluatedCandidate],
        goal: &PlanningGoal,
    ) -> Vec<&'a EvaluatedCandidate> {
        match goal.goal_type {
            GoalType::SeverityFocus => {
                if let Some(target_severities) = &goal.parameters.target_severities {
                    candidates
                        .iter()
                        .filter(|c| {
                            let severity = c
                                .candidate
                                .policy
                                .severity
                                .clone()
                                .unwrap_or_else(|| "medium".to_string())
                                .to_lowercase();
                            target_severities
                                .iter()
                                .any(|s| s.to_lowercase() == severity)
                        })
                        .cloned()
                        .collect()
                } else {
                    candidates.to_vec()
                }
            }
            _ => candidates.to_vec(),
        }
    }

    /// Generate human-readable reason for a step
    fn generate_step_reason(&self, candidate: &EvaluatedCandidate, _goal: &PlanningGoal) -> String {
        let severity = candidate
            .candidate
            .policy
            .severity
            .clone()
            .unwrap_or_else(|| "medium".to_string());

        let mut reasons = vec![format!(
            "{} severity policy with risk score {:.2}",
            severity, candidate.candidate.risk_score
        )];

        if candidate.candidate.recommendation.relevance_score > 0.8 {
            reasons.push("highly relevant to system context".to_string());
        }

        if !candidate.evaluation.soft_constraints.is_empty() {
            reasons.push(format!(
                "{} soft constraints considered",
                candidate.evaluation.soft_constraints.len()
            ));
        }

        reasons.join("; ")
    }

    /// Calculate confidence for a step
    fn calculate_confidence(&self, candidate: &EvaluatedCandidate) -> f32 {
        let base_confidence = candidate.candidate.recommendation.relevance_score;
        let penalty = candidate.evaluation.total_penalty;

        // Reduce confidence based on soft constraint penalties
        (base_confidence * (1.0 - penalty * 0.5)).clamp(0.1, 1.0)
    }

    /// Build the final plan object
    fn build_plan(
        &self,
        goal: PlanningGoal,
        steps: Vec<PlanStep>,
        deferred: Vec<DeferredPolicy>,
        input: &PlannerInput,
        mut metadata: PlanMetadata,
        elapsed: std::time::Duration,
    ) -> RemediationPlan {
        metadata.planning_duration_ms = elapsed.as_millis() as u64;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Calculate compliance rate
        let compliant_count = input.risk_scores.iter().filter(|r| r.is_compliant).count();
        let compliance_rate = if input.risk_scores.is_empty() {
            1.0
        } else {
            compliant_count as f32 / input.risk_scores.len() as f32
        };

        RemediationPlan {
            plan_id: Uuid::new_v4().to_string(),
            plan_type: "remediation".to_string(),
            goal,
            generated_at: chrono::Utc::now().to_rfc3339(),
            source_snapshot: SnapshotReference {
                timestamp: now,
                source: "agent_sense".to_string(),
                policy_count: input.current_audit.len(),
                compliance_rate,
            },
            previous_snapshot: input.previous_audit.as_ref().map(|prev| {
                let prev_compliant = prev.iter().filter(|r| r.passed).count();
                SnapshotReference {
                    timestamp: now.saturating_sub(3600), // Approximate
                    source: "agent_sense".to_string(),
                    policy_count: prev.len(),
                    compliance_rate: if prev.is_empty() {
                        1.0
                    } else {
                        prev_compliant as f32 / prev.len() as f32
                    },
                }
            }),
            steps,
            deferred,
            requires_human_approval: true, // ALWAYS true
            metadata,
        }
    }

    /// Validate plan integrity
    fn validate_plan(
        &self,
        plan: &RemediationPlan,
        candidates: &[CandidatePolicy],
    ) -> Result<(), PlannerError> {
        let candidate_ids: HashSet<_> = candidates.iter().map(|c| c.policy_id.as_str()).collect();

        // Every policy appears either in steps OR deferred, never both
        let step_ids: HashSet<_> = plan.steps.iter().map(|s| s.policy_id.as_str()).collect();
        let deferred_ids: HashSet<_> = plan.deferred.iter().map(|d| d.policy_id.as_str()).collect();

        let overlap: Vec<_> = step_ids.intersection(&deferred_ids).collect();
        if !overlap.is_empty() {
            return Err(PlannerError::ValidationFailed(format!(
                "Policies appear in both steps and deferred: {:?}",
                overlap
            )));
        }

        // All policy_ids must exist in candidate input
        for step in &plan.steps {
            if !candidate_ids.contains(step.policy_id.as_str()) {
                return Err(PlannerError::ValidationFailed(format!(
                    "Step policy '{}' not in candidate input",
                    step.policy_id
                )));
            }
        }

        for deferred in &plan.deferred {
            if !candidate_ids.contains(deferred.policy_id.as_str()) {
                return Err(PlannerError::ValidationFailed(format!(
                    "Deferred policy '{}' not in candidate input",
                    deferred.policy_id
                )));
            }
        }

        // Priorities must be sequential
        let priorities: Vec<u32> = plan.steps.iter().map(|s| s.priority).collect();
        for (i, p) in priorities.iter().enumerate() {
            if *p != (i + 1) as u32 {
                return Err(PlannerError::ValidationFailed(format!(
                    "Non-sequential priority: expected {}, got {}",
                    i + 1,
                    p
                )));
            }
        }

        // Confidence must be between 0 and 1
        for step in &plan.steps {
            if step.confidence < 0.0 || step.confidence > 1.0 {
                return Err(PlannerError::ValidationFailed(format!(
                    "Invalid confidence {} for policy {}",
                    step.confidence, step.policy_id
                )));
            }
        }

        // requires_human_approval must always be true
        if !plan.requires_human_approval {
            return Err(PlannerError::ValidationFailed(
                "requires_human_approval must always be true".to_string(),
            ));
        }

        Ok(())
    }
}

// ============================================================
// INTERNAL STRUCTURES
// ============================================================

/// Candidate policy with associated data
#[derive(Debug, Clone)]
struct CandidatePolicy {
    policy_id: String,
    policy: Policy,
    recommendation: PolicyRecommendation,
    risk_score: f32,
    is_compliant: bool,
}

/// Candidate with constraint evaluation
#[derive(Debug, Clone)]
struct EvaluatedCandidate {
    candidate: CandidatePolicy,
    evaluation: ConstraintEvaluation,
}

// ============================================================
// ERRORS
// ============================================================

/// Planner errors
#[derive(Debug, Clone)]
pub enum PlannerError {
    /// LLM is not available or failed
    LlmUnavailable(String),
    /// LLM response validation failed
    LlmValidationFailed(String),
    /// Plan validation failed
    ValidationFailed(String),
    /// Missing required input
    MissingInput(String),
    /// Internal error
    Internal(String),
}

impl std::fmt::Display for PlannerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlannerError::LlmUnavailable(msg) => write!(f, "LLM unavailable: {}", msg),
            PlannerError::LlmValidationFailed(msg) => write!(f, "LLM validation failed: {}", msg),
            PlannerError::ValidationFailed(msg) => write!(f, "Plan validation failed: {}", msg),
            PlannerError::MissingInput(msg) => write!(f, "Missing input: {}", msg),
            PlannerError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for PlannerError {}

// ============================================================
// TESTS
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_policy(id: &str, platform: &str, severity: &str) -> Policy {
        Policy {
            id: id.to_string(),
            title: Some(format!("Test Policy {}", id)),
            description: Some(format!("Description for {}", id)),
            platform: platform.to_string(),
            severity: Some(severity.to_string()),
            check_type: "test".to_string(),
            reversible: Some(true),
            post_reboot_required: Some(false),
            ..Default::default()
        }
    }

    fn create_test_audit_result(policy_id: &str, passed: bool) -> AuditResult {
        AuditResult {
            policy_id: policy_id.to_string(),
            passed,
            message: if passed {
                "Compliant".to_string()
            } else {
                "Non-compliant".to_string()
            },
        }
    }

    fn create_test_recommendation(policy_id: &str, score: f32) -> PolicyRecommendation {
        PolicyRecommendation {
            policy_id: policy_id.to_string(),
            relevance_score: score,
            reason: "Test recommendation".to_string(),
        }
    }

    fn create_test_risk_score(policy_id: &str, risk: f32, compliant: bool) -> PolicyRiskScore {
        PolicyRiskScore {
            policy_id: policy_id.to_string(),
            policy_title: policy_id.to_string(),
            severity: "high".to_string(),
            severity_weight: 0.75,
            compliance_state: if compliant { 1.0 } else { 0.0 },
            risk_score: risk,
            is_compliant: compliant,
        }
    }

    #[test]
    fn test_goal_compliance_threshold() {
        let goal = PlanningGoal::compliance_threshold(0.8, Some(vec!["critical".to_string()]));
        assert_eq!(goal.goal_type, GoalType::ComplianceThreshold);
        assert!(goal.description.contains("80%"));
        assert!(goal.description.contains("critical"));
    }

    #[test]
    fn test_goal_risk_minimization() {
        let goal = PlanningGoal::minimize_risk(0.5);
        assert_eq!(goal.goal_type, GoalType::RiskMinimization);
        assert!(goal.parameters.max_risk_score.is_some());
    }

    #[test]
    fn test_hard_constraint_platform_mismatch() {
        let constraint = HardConstraint::PlatformMismatch;
        assert_eq!(constraint.as_str(), "platform_mismatch");
        assert!(constraint.description().contains("platform"));
    }

    #[test]
    fn test_soft_constraint_penalty() {
        let reboot = SoftConstraint::RequiresReboot;
        assert!(reboot.penalty_score() > 0.0);

        let failure = SoftConstraint::HistoricalFailure {
            failure_count: 3,
            last_failure: 0,
        };
        assert!(failure.penalty_score() > reboot.penalty_score());
    }

    #[test]
    fn test_constraint_evaluation() {
        let mut eval = ConstraintEvaluation::new("test-policy");
        assert!(!eval.is_deferred);
        assert_eq!(eval.total_penalty, 0.0);

        eval.add_soft_constraint(SoftConstraint::RequiresReboot);
        assert!(!eval.is_deferred);
        assert!(eval.total_penalty > 0.0);

        eval.add_hard_constraint(HardConstraint::PlatformMismatch);
        assert!(eval.is_deferred);
    }

    #[test]
    fn test_planner_generate_plan() {
        let planner = Planner::with_defaults();
        let goal = PlanningGoal::compliance_threshold(0.8, None);

        let policies = vec![
            create_test_policy("POL-001", "windows", "high"),
            create_test_policy("POL-002", "windows", "critical"),
            create_test_policy("POL-003", "linux", "medium"), // Will be deferred
        ];

        let current_audit = vec![
            create_test_audit_result("POL-001", false),
            create_test_audit_result("POL-002", false),
            create_test_audit_result("POL-003", false),
        ];

        let risk_scores = vec![
            create_test_risk_score("POL-001", 0.75, false),
            create_test_risk_score("POL-002", 1.0, false),
            create_test_risk_score("POL-003", 0.5, false),
        ];

        let recommendations = RecommendationResult {
            recommendations: vec![
                create_test_recommendation("POL-001", 0.9),
                create_test_recommendation("POL-002", 0.95),
                create_test_recommendation("POL-003", 0.7),
            ],
            invalid_suggestions: vec![],
            warnings: vec![],
        };

        let input = PlannerInput {
            current_audit,
            previous_audit: None,
            risk_scores,
            policies,
            execution_history: vec![],
            recommendations,
            drift_report: None,
            disabled_policies: HashSet::new(),
            current_platform: "windows".to_string(),
        };

        let plan = planner.generate_plan(goal, &input).unwrap();

        // Validate plan structure
        assert!(plan.requires_human_approval);
        assert_eq!(plan.plan_type, "remediation");
        assert!(!plan.plan_id.is_empty());

        // POL-003 should be deferred (linux vs windows)
        assert!(plan.deferred.iter().any(|d| d.policy_id == "POL-003"));

        // POL-001 and POL-002 should be in steps
        assert!(plan.steps.iter().any(|s| s.policy_id == "POL-001"));
        assert!(plan.steps.iter().any(|s| s.policy_id == "POL-002"));

        // POL-002 (critical, higher risk) should be first
        assert_eq!(plan.steps[0].policy_id, "POL-002");
        assert_eq!(plan.steps[0].priority, 1);

        // Priorities should be sequential
        for (i, step) in plan.steps.iter().enumerate() {
            assert_eq!(step.priority, (i + 1) as u32);
        }

        // Confidence should be valid
        for step in &plan.steps {
            assert!(step.confidence >= 0.0 && step.confidence <= 1.0);
        }
    }

    #[test]
    fn test_planner_disabled_policy() {
        let planner = Planner::with_defaults();
        let goal = PlanningGoal::compliance_threshold(0.8, None);

        let policies = vec![create_test_policy("POL-001", "windows", "high")];

        let current_audit = vec![create_test_audit_result("POL-001", false)];

        let risk_scores = vec![create_test_risk_score("POL-001", 0.75, false)];

        let recommendations = RecommendationResult {
            recommendations: vec![create_test_recommendation("POL-001", 0.9)],
            invalid_suggestions: vec![],
            warnings: vec![],
        };

        let mut disabled_policies = HashSet::new();
        disabled_policies.insert("POL-001".to_string());

        let input = PlannerInput {
            current_audit,
            previous_audit: None,
            risk_scores,
            policies,
            execution_history: vec![],
            recommendations,
            drift_report: None,
            disabled_policies,
            current_platform: "windows".to_string(),
        };

        let plan = planner.generate_plan(goal, &input).unwrap();

        // POL-001 should be deferred due to being disabled
        assert!(plan.deferred.iter().any(|d| d.policy_id == "POL-001"));
        assert!(plan.deferred[0]
            .blocking_constraints
            .contains(&"explicitly_disabled_by_user".to_string()));
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn test_planner_historical_failure() {
        let planner = Planner::with_defaults();
        let goal = PlanningGoal::compliance_threshold(0.8, None);

        let policies = vec![
            create_test_policy("POL-001", "windows", "high"),
            create_test_policy("POL-002", "windows", "high"),
        ];

        let current_audit = vec![
            create_test_audit_result("POL-001", false),
            create_test_audit_result("POL-002", false),
        ];

        let risk_scores = vec![
            create_test_risk_score("POL-001", 0.75, false),
            create_test_risk_score("POL-002", 0.75, false),
        ];

        let recommendations = RecommendationResult {
            recommendations: vec![
                create_test_recommendation("POL-001", 0.9),
                create_test_recommendation("POL-002", 0.9),
            ],
            invalid_suggestions: vec![],
            warnings: vec![],
        };

        // POL-001 has historical failures
        let execution_history = vec![
            ExecutionOutcome {
                policy_id: "POL-001".to_string(),
                timestamp: 1000,
                success: false,
                error_message: Some("Failed".to_string()),
                rollback_available: true,
                duration_ms: 100,
            },
            ExecutionOutcome {
                policy_id: "POL-001".to_string(),
                timestamp: 2000,
                success: false,
                error_message: Some("Failed again".to_string()),
                rollback_available: true,
                duration_ms: 100,
            },
        ];

        let input = PlannerInput {
            current_audit,
            previous_audit: None,
            risk_scores,
            policies,
            execution_history,
            recommendations,
            drift_report: None,
            disabled_policies: HashSet::new(),
            current_platform: "windows".to_string(),
        };

        let plan = planner.generate_plan(goal, &input).unwrap();

        // Both should be in steps (historical failure is soft constraint)
        assert_eq!(plan.steps.len(), 2);

        // POL-002 should be first (no historical failures = lower penalty)
        assert_eq!(plan.steps[0].policy_id, "POL-002");

        // POL-001 should have constraint noted
        let pol1_step = plan.steps.iter().find(|s| s.policy_id == "POL-001").unwrap();
        assert!(pol1_step
            .constraints_considered
            .contains(&"historical_failure".to_string()));
    }

    #[test]
    fn test_plan_validation_no_duplicates() {
        let planner = Planner::with_defaults();

        // Create a plan manually with duplicates (should fail validation)
        let plan = RemediationPlan {
            plan_id: "test".to_string(),
            plan_type: "remediation".to_string(),
            goal: PlanningGoal::compliance_threshold(0.8, None),
            generated_at: "2025-01-01T00:00:00Z".to_string(),
            source_snapshot: SnapshotReference {
                timestamp: 0,
                source: "test".to_string(),
                policy_count: 1,
                compliance_rate: 0.5,
            },
            previous_snapshot: None,
            steps: vec![PlanStep {
                policy_id: "POL-001".to_string(),
                priority: 1,
                reason: "Test".to_string(),
                risk_score: 0.5,
                confidence: 0.8,
                constraints_considered: vec![],
                expected_impact: None,
                estimated_duration_minutes: None,
            }],
            deferred: vec![DeferredPolicy {
                policy_id: "POL-001".to_string(), // Duplicate!
                reason: "Test".to_string(),
                blocking_constraints: vec![],
            }],
            requires_human_approval: true,
            metadata: PlanMetadata {
                llm_assisted: false,
                llm_model: None,
                used_deterministic_fallback: true,
                warnings: vec![],
                candidates_considered: 1,
                planning_duration_ms: 0,
            },
        };

        let candidates = vec![CandidatePolicy {
            policy_id: "POL-001".to_string(),
            policy: create_test_policy("POL-001", "windows", "high"),
            recommendation: create_test_recommendation("POL-001", 0.9),
            risk_score: 0.5,
            is_compliant: false,
        }];

        let result = planner.validate_plan(&plan, &candidates);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("both steps and deferred"));
    }

    #[test]
    fn test_plan_validation_sequential_priorities() {
        let planner = Planner::with_defaults();

        let plan = RemediationPlan {
            plan_id: "test".to_string(),
            plan_type: "remediation".to_string(),
            goal: PlanningGoal::compliance_threshold(0.8, None),
            generated_at: "2025-01-01T00:00:00Z".to_string(),
            source_snapshot: SnapshotReference {
                timestamp: 0,
                source: "test".to_string(),
                policy_count: 2,
                compliance_rate: 0.0,
            },
            previous_snapshot: None,
            steps: vec![
                PlanStep {
                    policy_id: "POL-001".to_string(),
                    priority: 1,
                    reason: "Test".to_string(),
                    risk_score: 0.5,
                    confidence: 0.8,
                    constraints_considered: vec![],
                    expected_impact: None,
                    estimated_duration_minutes: None,
                },
                PlanStep {
                    policy_id: "POL-002".to_string(),
                    priority: 5, // Non-sequential!
                    reason: "Test".to_string(),
                    risk_score: 0.3,
                    confidence: 0.8,
                    constraints_considered: vec![],
                    expected_impact: None,
                    estimated_duration_minutes: None,
                },
            ],
            deferred: vec![],
            requires_human_approval: true,
            metadata: PlanMetadata {
                llm_assisted: false,
                llm_model: None,
                used_deterministic_fallback: true,
                warnings: vec![],
                candidates_considered: 2,
                planning_duration_ms: 0,
            },
        };

        let candidates = vec![
            CandidatePolicy {
                policy_id: "POL-001".to_string(),
                policy: create_test_policy("POL-001", "windows", "high"),
                recommendation: create_test_recommendation("POL-001", 0.9),
                risk_score: 0.5,
                is_compliant: false,
            },
            CandidatePolicy {
                policy_id: "POL-002".to_string(),
                policy: create_test_policy("POL-002", "windows", "medium"),
                recommendation: create_test_recommendation("POL-002", 0.8),
                risk_score: 0.3,
                is_compliant: false,
            },
        ];

        let result = planner.validate_plan(&plan, &candidates);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Non-sequential priority"));
    }

    #[test]
    fn test_plan_validation_confidence_bounds() {
        let planner = Planner::with_defaults();

        let plan = RemediationPlan {
            plan_id: "test".to_string(),
            plan_type: "remediation".to_string(),
            goal: PlanningGoal::compliance_threshold(0.8, None),
            generated_at: "2025-01-01T00:00:00Z".to_string(),
            source_snapshot: SnapshotReference {
                timestamp: 0,
                source: "test".to_string(),
                policy_count: 1,
                compliance_rate: 0.0,
            },
            previous_snapshot: None,
            steps: vec![PlanStep {
                policy_id: "POL-001".to_string(),
                priority: 1,
                reason: "Test".to_string(),
                risk_score: 0.5,
                confidence: 1.5, // Invalid!
                constraints_considered: vec![],
                expected_impact: None,
                estimated_duration_minutes: None,
            }],
            deferred: vec![],
            requires_human_approval: true,
            metadata: PlanMetadata {
                llm_assisted: false,
                llm_model: None,
                used_deterministic_fallback: true,
                warnings: vec![],
                candidates_considered: 1,
                planning_duration_ms: 0,
            },
        };

        let candidates = vec![CandidatePolicy {
            policy_id: "POL-001".to_string(),
            policy: create_test_policy("POL-001", "windows", "high"),
            recommendation: create_test_recommendation("POL-001", 0.9),
            risk_score: 0.5,
            is_compliant: false,
        }];

        let result = planner.validate_plan(&plan, &candidates);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid confidence"));
    }

    #[test]
    fn test_plan_validation_human_approval_required() {
        let planner = Planner::with_defaults();

        let plan = RemediationPlan {
            plan_id: "test".to_string(),
            plan_type: "remediation".to_string(),
            goal: PlanningGoal::compliance_threshold(0.8, None),
            generated_at: "2025-01-01T00:00:00Z".to_string(),
            source_snapshot: SnapshotReference {
                timestamp: 0,
                source: "test".to_string(),
                policy_count: 0,
                compliance_rate: 1.0,
            },
            previous_snapshot: None,
            steps: vec![],
            deferred: vec![],
            requires_human_approval: false, // Invalid!
            metadata: PlanMetadata {
                llm_assisted: false,
                llm_model: None,
                used_deterministic_fallback: true,
                warnings: vec![],
                candidates_considered: 0,
                planning_duration_ms: 0,
            },
        };

        let candidates = vec![];

        let result = planner.validate_plan(&plan, &candidates);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("requires_human_approval"));
    }

    #[test]
    fn test_platform_matching() {
        let planner = Planner::with_defaults();

        // Exact match
        assert!(planner.platform_matches("windows", "windows"));
        assert!(planner.platform_matches("linux", "linux"));

        // Windows variants
        assert!(planner.platform_matches("windows", "Windows 10"));
        assert!(planner.platform_matches("windows", "Windows Server 2022"));

        // Linux variants
        assert!(planner.platform_matches("linux", "Ubuntu 22.04"));
        assert!(planner.platform_matches("linux", "RHEL 8"));
        assert!(planner.platform_matches("linux", "Debian 12"));

        // Mismatches
        assert!(!planner.platform_matches("linux", "windows"));
        assert!(!planner.platform_matches("windows", "linux"));
    }

    #[test]
    fn test_deterministic_fallback() {
        let planner = Planner::new(PlannerConfig {
            use_llm_ordering: false,
            ..Default::default()
        });

        let goal = PlanningGoal::compliance_threshold(0.8, None);

        let policies = vec![
            create_test_policy("POL-001", "windows", "medium"),
            create_test_policy("POL-002", "windows", "critical"),
        ];

        let current_audit = vec![
            create_test_audit_result("POL-001", false),
            create_test_audit_result("POL-002", false),
        ];

        let risk_scores = vec![
            create_test_risk_score("POL-001", 0.5, false),
            create_test_risk_score("POL-002", 1.0, false),
        ];

        let recommendations = RecommendationResult {
            recommendations: vec![
                create_test_recommendation("POL-001", 0.8),
                create_test_recommendation("POL-002", 0.9),
            ],
            invalid_suggestions: vec![],
            warnings: vec![],
        };

        let input = PlannerInput {
            current_audit,
            previous_audit: None,
            risk_scores,
            policies,
            execution_history: vec![],
            recommendations,
            drift_report: None,
            disabled_policies: HashSet::new(),
            current_platform: "windows".to_string(),
        };

        let plan = planner.generate_plan(goal, &input).unwrap();

        // Should use deterministic fallback
        assert!(plan.metadata.used_deterministic_fallback);
        assert!(!plan.metadata.llm_assisted);

        // Higher risk (POL-002) should be first
        assert_eq!(plan.steps[0].policy_id, "POL-002");
    }
}
