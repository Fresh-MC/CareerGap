//! Career Planner Module
//!
//! Implements the PLAN phase of the agentic system.
//! Generates career roadmaps based on goals, constraints, and current skills.
//!
//! ARCHITECTURAL RULES (preserved from original):
//! - Planning is read-only (no side effects)
//! - Human approval is mandatory for execution
//! - Planner consumes existing data only
//! - All decisions are explainable

use super::types::{CareerGoal, CareerRule, ResumeData, SkillAssessment};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

// ============================================================
// PLANNING GOAL (adapted for career context)
// ============================================================

/// Goal type for career planning
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlanningGoalType {
    /// Transition to a new role
    RoleTransition,
    /// Skill acquisition focus
    SkillAcquisition,
    /// Career advancement in current path
    CareerAdvancement,
    /// Industry change
    IndustryChange,
    /// Custom goal
    Custom,
}

/// Parameters for planning goals
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoalParameters {
    /// Target role to achieve
    pub target_role: Option<String>,
    /// Target skills to acquire
    pub target_skills: Option<Vec<String>>,
    /// Target industry
    pub target_industry: Option<String>,
    /// Timeline in months
    pub timeline_months: Option<u32>,
    /// Custom parameters
    pub custom: Option<HashMap<String, String>>,
}

/// A planning goal that drives roadmap generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningGoal {
    pub goal_id: String,
    pub description: String,
    pub goal_type: PlanningGoalType,
    pub parameters: GoalParameters,
    pub priority: u32,
    pub active: bool,
}

impl PlanningGoal {
    pub fn role_transition(target_role: &str, timeline_months: u32) -> Self {
        Self {
            goal_id: Uuid::new_v4().to_string(),
            description: format!("Transition to {} role within {} months", target_role, timeline_months),
            goal_type: PlanningGoalType::RoleTransition,
            parameters: GoalParameters {
                target_role: Some(target_role.to_string()),
                timeline_months: Some(timeline_months),
                ..Default::default()
            },
            priority: 1,
            active: true,
        }
    }

    pub fn skill_acquisition(skills: Vec<String>) -> Self {
        Self {
            goal_id: Uuid::new_v4().to_string(),
            description: format!("Acquire skills: {}", skills.join(", ")),
            goal_type: PlanningGoalType::SkillAcquisition,
            parameters: GoalParameters {
                target_skills: Some(skills),
                ..Default::default()
            },
            priority: 1,
            active: true,
        }
    }
}

// ============================================================
// CONSTRAINTS (adapted for career context)
// ============================================================

/// Hard constraints that block a roadmap step
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum HardConstraint {
    /// Missing prerequisite skill
    MissingPrerequisite { prerequisite: String },
    /// User explicitly deferred this step
    UserDeferred,
    /// Timeline constraint violation
    TimelineExceeded,
}

impl HardConstraint {
    pub fn description(&self) -> String {
        match self {
            HardConstraint::MissingPrerequisite { prerequisite } => {
                format!("Missing prerequisite: {}", prerequisite)
            }
            HardConstraint::UserDeferred => "User deferred this step".to_string(),
            HardConstraint::TimelineExceeded => "Exceeds available timeline".to_string(),
        }
    }
}

/// Soft constraints that affect ordering but don't block
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SoftConstraint {
    /// High time investment required
    HighTimeInvestment { hours: u32 },
    /// Depends on external factors
    ExternalDependency { description: String },
    /// Low confidence in estimate
    LowConfidence,
    /// Conflicts with another step
    ConflictsWithStep { step_id: String },
}

impl SoftConstraint {
    pub fn penalty_score(&self) -> f32 {
        match self {
            SoftConstraint::HighTimeInvestment { hours } => {
                0.1 * (*hours as f32 / 100.0).min(0.5)
            }
            SoftConstraint::ExternalDependency { .. } => 0.3,
            SoftConstraint::LowConfidence => 0.2,
            SoftConstraint::ConflictsWithStep { .. } => 0.4,
        }
    }
}

/// Evaluation of constraints for a roadmap step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintEvaluation {
    pub step_id: String,
    pub hard_constraints: Vec<HardConstraint>,
    pub soft_constraints: Vec<SoftConstraint>,
    pub is_blocked: bool,
    pub total_penalty: f32,
}

impl ConstraintEvaluation {
    pub fn new(step_id: &str) -> Self {
        Self {
            step_id: step_id.to_string(),
            hard_constraints: Vec::new(),
            soft_constraints: Vec::new(),
            is_blocked: false,
            total_penalty: 0.0,
        }
    }

    pub fn add_hard_constraint(&mut self, constraint: HardConstraint) {
        self.hard_constraints.push(constraint);
        self.is_blocked = true;
    }

    pub fn add_soft_constraint(&mut self, constraint: SoftConstraint) {
        self.total_penalty += constraint.penalty_score();
        self.soft_constraints.push(constraint);
    }
}

// ============================================================
// ROADMAP STRUCTURES
// ============================================================

/// A single step in the career roadmap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadmapStep {
    pub id: String,
    /// Step order (1 = first)
    pub order: u32,
    pub title: String,
    pub description: String,
    /// Type: "learn", "practice", "apply", "certify", "network"
    pub step_type: String,
    /// Associated skill or rule
    pub skill_id: Option<String>,
    /// Estimated weeks to complete
    pub estimated_weeks: u32,
    /// Human-readable reason for inclusion
    pub reason: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Resources and links
    pub resources: Vec<String>,
    /// Status: "not_started", "in_progress", "completed", "skipped"
    pub status: String,
    /// Constraints considered
    pub constraints_considered: Vec<String>,
    /// User can edit this step
    pub editable: bool,
}

impl RoadmapStep {
    pub fn new(order: u32, title: &str, step_type: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            order,
            title: title.to_string(),
            description: String::new(),
            step_type: step_type.to_string(),
            skill_id: None,
            estimated_weeks: 2,
            reason: String::new(),
            confidence: 0.8,
            resources: Vec::new(),
            status: "not_started".to_string(),
            constraints_considered: Vec::new(),
            editable: true,
        }
    }

    pub fn with_reason(mut self, reason: &str) -> Self {
        self.reason = reason.to_string();
        self
    }

    pub fn with_weeks(mut self, weeks: u32) -> Self {
        self.estimated_weeks = weeks;
        self
    }

    pub fn with_resources(mut self, resources: Vec<String>) -> Self {
        self.resources = resources;
        self
    }
}

/// A deferred step (blocked from current roadmap)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeferredStep {
    pub step_id: String,
    pub title: String,
    pub reason: String,
    pub blocking_constraints: Vec<String>,
}

/// The complete career roadmap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerRoadmap {
    pub id: String,
    pub user_id: String,
    pub goal: PlanningGoal,
    pub generated_at: DateTime<Utc>,
    /// Ordered steps to follow
    pub steps: Vec<RoadmapStep>,
    /// Steps deferred for later
    pub deferred: Vec<DeferredStep>,
    /// Human approval required before "executing" (marking complete)
    pub requires_human_approval: bool,
    /// Metadata about generation
    pub metadata: RoadmapMetadata,
}

/// Metadata about roadmap generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoadmapMetadata {
    /// Whether LLM was used for generation
    pub llm_assisted: bool,
    /// LLM model if used
    pub llm_model: Option<String>,
    /// Warnings during generation
    pub warnings: Vec<String>,
    /// Total steps considered
    pub steps_considered: usize,
    /// Generation time in ms
    pub generation_duration_ms: u64,
}

// ============================================================
// PLANNER INPUT
// ============================================================

/// All inputs for the planner
#[derive(Debug, Clone)]
pub struct PlannerInput {
    /// User's resume data
    pub resume: ResumeData,
    /// Current skill assessments
    pub assessments: Vec<SkillAssessment>,
    /// User's career goal
    pub goal: CareerGoal,
    /// Available career rules/skills to consider
    pub available_rules: Vec<CareerRule>,
    /// User-deferred step IDs
    pub deferred_steps: HashSet<String>,
}

// ============================================================
// PLANNER IMPLEMENTATION
// ============================================================

/// Configuration for the planner
#[derive(Debug, Clone)]
pub struct PlannerConfig {
    pub max_steps: usize,
    pub min_confidence: f32,
    pub enable_llm: bool,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            max_steps: 10,
            min_confidence: 0.5,
            enable_llm: false,
        }
    }
}

/// The career planner
pub struct CareerPlanner {
    config: PlannerConfig,
}

impl CareerPlanner {
    pub fn new(config: PlannerConfig) -> Self {
        Self { config }
    }

    /// Generate a career roadmap from input
    pub fn generate_roadmap(&self, input: &PlannerInput) -> CareerRoadmap {
        let start_time = std::time::Instant::now();
        let mut steps = Vec::new();
        let mut deferred = Vec::new();
        let mut warnings = Vec::new();

        // Step 1: Identify skill gaps
        let current_skills: HashSet<String> = input.resume.skills.iter().cloned().collect();
        let completed_rules: HashSet<String> = input.assessments
            .iter()
            .filter(|a| a.completed)
            .map(|a| a.rule_id.clone())
            .collect();

        // Step 2: Filter and rank rules based on goal
        let mut candidate_rules: Vec<&CareerRule> = input.available_rules
            .iter()
            .filter(|r| !completed_rules.contains(&r.id))
            .filter(|r| !input.deferred_steps.contains(&r.id))
            .collect();

        // Sort by priority
        candidate_rules.sort_by(|a, b| {
            let priority_a = match a.priority.as_str() {
                "critical" => 0,
                "high" => 1,
                "medium" => 2,
                _ => 3,
            };
            let priority_b = match b.priority.as_str() {
                "critical" => 0,
                "high" => 1,
                "medium" => 2,
                _ => 3,
            };
            priority_a.cmp(&priority_b)
        });

        // Step 3: Evaluate constraints and build steps
        let mut order = 1u32;
        for rule in candidate_rules.iter().take(self.config.max_steps) {
            let mut eval = ConstraintEvaluation::new(&rule.id);

            // Check prerequisites
            for prereq in &rule.prerequisites {
                if !current_skills.contains(prereq) && !completed_rules.contains(prereq) {
                    eval.add_hard_constraint(HardConstraint::MissingPrerequisite {
                        prerequisite: prereq.clone(),
                    });
                }
            }

            // Check if high time investment
            if let Some(weeks) = rule.estimated_weeks {
                if weeks > 8 {
                    eval.add_soft_constraint(SoftConstraint::HighTimeInvestment {
                        hours: weeks * 10, // Rough estimate
                    });
                }
            }

            if eval.is_blocked {
                deferred.push(DeferredStep {
                    step_id: rule.id.clone(),
                    title: rule.title.clone(),
                    reason: "Blocked by constraints".to_string(),
                    blocking_constraints: eval.hard_constraints
                        .iter()
                        .map(|c| c.description())
                        .collect(),
                });
            } else {
                let step = RoadmapStep {
                    id: Uuid::new_v4().to_string(),
                    order,
                    title: rule.title.clone(),
                    description: rule.description.clone(),
                    step_type: self.infer_step_type(&rule.category),
                    skill_id: Some(rule.id.clone()),
                    estimated_weeks: rule.estimated_weeks.unwrap_or(2),
                    reason: self.generate_reason(rule, &input.goal),
                    confidence: 1.0 - eval.total_penalty,
                    resources: Vec::new(),
                    status: "not_started".to_string(),
                    constraints_considered: eval.soft_constraints
                        .iter()
                        .map(|c| format!("{:?}", c))
                        .collect(),
                    editable: true,
                };
                steps.push(step);
                order += 1;
            }
        }

        // Add warning if no steps
        if steps.is_empty() {
            warnings.push("No actionable steps found. Consider adjusting your goal or completing prerequisites.".to_string());
        }

        let goal = PlanningGoal {
            goal_id: input.goal.id.clone(),
            description: input.goal.title.clone(),
            goal_type: PlanningGoalType::RoleTransition,
            parameters: GoalParameters {
                target_role: input.goal.target_role.clone(),
                timeline_months: input.goal.timeline_months,
                ..Default::default()
            },
            priority: 1,
            active: true,
        };

        CareerRoadmap {
            id: Uuid::new_v4().to_string(),
            user_id: input.resume.user_id.clone(),
            goal,
            generated_at: Utc::now(),
            steps,
            deferred,
            requires_human_approval: true,
            metadata: RoadmapMetadata {
                llm_assisted: self.config.enable_llm,
                llm_model: None,
                warnings,
                steps_considered: candidate_rules.len(),
                generation_duration_ms: start_time.elapsed().as_millis() as u64,
            },
        }
    }

    fn infer_step_type(&self, category: &str) -> String {
        match category {
            "technical_skill" => "learn".to_string(),
            "soft_skill" => "practice".to_string(),
            "certification" => "certify".to_string(),
            "experience" => "apply".to_string(),
            _ => "learn".to_string(),
        }
    }

    fn generate_reason(&self, rule: &CareerRule, goal: &CareerGoal) -> String {
        format!(
            "This step helps you {} by developing '{}' skills, which is {} priority for your goal.",
            goal.target_role.as_deref().unwrap_or("advance your career"),
            rule.title,
            rule.priority
        )
    }
}

// ============================================================
// HUMAN-IN-THE-LOOP EDITING
// ============================================================

/// Edit operation for a roadmap step
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RoadmapEdit {
    /// Reorder a step
    Reorder { step_id: String, new_order: u32 },
    /// Update step details
    UpdateStep { step_id: String, title: Option<String>, description: Option<String>, estimated_weeks: Option<u32> },
    /// Mark step as completed
    CompleteStep { step_id: String },
    /// Skip a step
    SkipStep { step_id: String, reason: String },
    /// Add a custom step
    AddStep { title: String, description: String, step_type: String, after_step_id: Option<String> },
    /// Remove a step
    RemoveStep { step_id: String },
}

/// Apply an edit to a roadmap
pub fn apply_edit(roadmap: &mut CareerRoadmap, edit: RoadmapEdit) -> Result<String, String> {
    match edit {
        RoadmapEdit::Reorder { step_id, new_order } => {
            let pos = roadmap.steps.iter().position(|s| s.id == step_id)
                .ok_or("Step not found")?;
            let step = roadmap.steps.remove(pos);
            let insert_pos = (new_order as usize).saturating_sub(1).min(roadmap.steps.len());
            roadmap.steps.insert(insert_pos, step);
            // Renumber
            for (i, s) in roadmap.steps.iter_mut().enumerate() {
                s.order = (i + 1) as u32;
            }
            Ok(format!("Reordered step to position {}", new_order))
        }
        RoadmapEdit::UpdateStep { step_id, title, description, estimated_weeks } => {
            let step = roadmap.steps.iter_mut().find(|s| s.id == step_id)
                .ok_or("Step not found")?;
            if let Some(t) = title { step.title = t; }
            if let Some(d) = description { step.description = d; }
            if let Some(w) = estimated_weeks { step.estimated_weeks = w; }
            Ok("Step updated".to_string())
        }
        RoadmapEdit::CompleteStep { step_id } => {
            let step = roadmap.steps.iter_mut().find(|s| s.id == step_id)
                .ok_or("Step not found")?;
            step.status = "completed".to_string();
            Ok(format!("Completed: {}", step.title))
        }
        RoadmapEdit::SkipStep { step_id, reason } => {
            let step = roadmap.steps.iter_mut().find(|s| s.id == step_id)
                .ok_or("Step not found")?;
            step.status = "skipped".to_string();
            roadmap.deferred.push(DeferredStep {
                step_id: step.id.clone(),
                title: step.title.clone(),
                reason,
                blocking_constraints: vec!["User skipped".to_string()],
            });
            Ok("Step skipped".to_string())
        }
        RoadmapEdit::AddStep { title, description, step_type, after_step_id } => {
            let new_step = RoadmapStep {
                id: Uuid::new_v4().to_string(),
                order: 0, // Will be set below
                title: title.clone(),
                description,
                step_type,
                skill_id: None,
                estimated_weeks: 2,
                reason: "Added by user".to_string(),
                confidence: 1.0,
                resources: Vec::new(),
                status: "not_started".to_string(),
                constraints_considered: Vec::new(),
                editable: true,
            };
            
            let insert_pos = if let Some(after_id) = after_step_id {
                roadmap.steps.iter().position(|s| s.id == after_id)
                    .map(|p| p + 1)
                    .unwrap_or(roadmap.steps.len())
            } else {
                roadmap.steps.len()
            };
            
            roadmap.steps.insert(insert_pos, new_step);
            
            // Renumber
            for (i, s) in roadmap.steps.iter_mut().enumerate() {
                s.order = (i + 1) as u32;
            }
            
            Ok(format!("Added step: {}", title))
        }
        RoadmapEdit::RemoveStep { step_id } => {
            let pos = roadmap.steps.iter().position(|s| s.id == step_id)
                .ok_or("Step not found")?;
            let removed = roadmap.steps.remove(pos);
            // Renumber
            for (i, s) in roadmap.steps.iter_mut().enumerate() {
                s.order = (i + 1) as u32;
            }
            Ok(format!("Removed step: {}", removed.title))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planner_generates_roadmap() {
        let planner = CareerPlanner::new(PlannerConfig::default());
        
        let resume = ResumeData {
            user_id: "test_user".to_string(),
            skills: vec!["Python".to_string(), "SQL".to_string()],
            ..Default::default()
        };

        let goal = CareerGoal::new("test_user", "Become a Data Scientist");
        
        let rules = vec![
            CareerRule::new("ml_basics", "Machine Learning Basics", "technical_skill"),
            CareerRule::new("stats", "Statistics", "technical_skill"),
        ];

        let input = PlannerInput {
            resume,
            assessments: Vec::new(),
            goal,
            available_rules: rules,
            deferred_steps: HashSet::new(),
        };

        let roadmap = planner.generate_roadmap(&input);
        assert_eq!(roadmap.steps.len(), 2);
        assert!(roadmap.requires_human_approval);
    }

    #[test]
    fn test_roadmap_editing() {
        let planner = CareerPlanner::new(PlannerConfig::default());
        
        let resume = ResumeData {
            user_id: "test_user".to_string(),
            ..Default::default()
        };

        let goal = CareerGoal::new("test_user", "Test Goal");
        
        let rules = vec![
            CareerRule::new("step1", "First Step", "technical_skill"),
            CareerRule::new("step2", "Second Step", "technical_skill"),
        ];

        let input = PlannerInput {
            resume,
            assessments: Vec::new(),
            goal,
            available_rules: rules,
            deferred_steps: HashSet::new(),
        };

        let mut roadmap = planner.generate_roadmap(&input);
        let step_id = roadmap.steps[0].id.clone();

        // Test completion
        let result = apply_edit(&mut roadmap, RoadmapEdit::CompleteStep { step_id });
        assert!(result.is_ok());
        assert_eq!(roadmap.steps[0].status, "completed");
    }
}
