//! Career Agent Types
//!
//! Core data structures for the career development assistant.
//! These types replace the security-focused Policy/Audit/Remediation
//! with career-focused CareerRule/SkillAssessment/CareerAction.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================
// CAREER RULES (formerly Policy)
// ============================================================

/// A career rule defines a skill, milestone, or requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerRule {
    pub id: String,
    pub title: String,
    pub description: String,
    /// Category: "technical_skill", "soft_skill", "certification", "experience"
    pub category: String,
    /// Priority level: "critical", "high", "medium", "low"
    pub priority: String,
    /// Estimated time to complete (in weeks)
    pub estimated_weeks: Option<u32>,
    /// Prerequisites (other rule IDs)
    pub prerequisites: Vec<String>,
    /// Tags for filtering
    pub tags: Vec<String>,
}

impl CareerRule {
    pub fn new(id: &str, title: &str, category: &str) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            description: String::new(),
            category: category.to_string(),
            priority: "medium".to_string(),
            estimated_weeks: None,
            prerequisites: Vec::new(),
            tags: Vec::new(),
        }
    }
}

// ============================================================
// SKILL ASSESSMENT (formerly Audit)
// ============================================================

/// Result of assessing a skill or career rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAssessment {
    pub rule_id: String,
    /// Has the user met this skill/milestone?
    pub completed: bool,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
    /// Evidence or notes
    pub notes: String,
    /// When this assessment was made
    pub assessed_at: DateTime<Utc>,
}

impl SkillAssessment {
    pub fn new(rule_id: &str, completed: bool) -> Self {
        Self {
            rule_id: rule_id.to_string(),
            completed,
            confidence: 1.0,
            notes: String::new(),
            assessed_at: Utc::now(),
        }
    }
}

// ============================================================
// CAREER ACTION (formerly Remediation)
// ============================================================

/// An action to take for career development
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerAction {
    pub id: String,
    pub rule_id: String,
    pub title: String,
    pub description: String,
    /// Action type: "learn", "practice", "apply", "certify", "network"
    pub action_type: String,
    /// Resources (links, books, courses)
    pub resources: Vec<String>,
    /// Estimated hours to complete
    pub estimated_hours: Option<u32>,
    /// Status: "not_started", "in_progress", "completed", "skipped"
    pub status: String,
}

impl CareerAction {
    pub fn new(rule_id: &str, title: &str, action_type: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            rule_id: rule_id.to_string(),
            title: title.to_string(),
            description: String::new(),
            action_type: action_type.to_string(),
            resources: Vec::new(),
            estimated_hours: None,
            status: "not_started".to_string(),
        }
    }
}

// ============================================================
// CAREER CHECKPOINT (formerly Snapshot)
// ============================================================

/// A checkpoint capturing career state at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerCheckpoint {
    pub id: String,
    pub user_id: String,
    pub timestamp: DateTime<Utc>,
    pub description: String,
    /// All assessments at this checkpoint
    pub assessments: Vec<SkillAssessment>,
    /// The active roadmap at this checkpoint
    pub roadmap_snapshot: Option<String>,
}

impl CareerCheckpoint {
    pub fn new(user_id: &str, description: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            timestamp: Utc::now(),
            description: description.to_string(),
            assessments: Vec::new(),
            roadmap_snapshot: None,
        }
    }
}

// ============================================================
// RESUME DATA
// ============================================================

/// Parsed resume information (simplified)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResumeData {
    pub user_id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub current_role: Option<String>,
    pub years_experience: Option<u32>,
    pub skills: Vec<String>,
    pub education: Vec<EducationEntry>,
    pub experience: Vec<ExperienceEntry>,
    pub raw_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EducationEntry {
    pub institution: String,
    pub degree: String,
    pub field: Option<String>,
    pub year: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceEntry {
    pub company: String,
    pub role: String,
    pub duration: Option<String>,
    pub description: Option<String>,
}

// ============================================================
// CAREER GOAL
// ============================================================

/// User's career goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerGoal {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub description: String,
    /// Target role or position
    pub target_role: Option<String>,
    /// Target industry
    pub target_industry: Option<String>,
    /// Timeline in months
    pub timeline_months: Option<u32>,
    /// Priority: "primary", "secondary"
    pub priority: String,
    pub created_at: DateTime<Utc>,
    pub active: bool,
}

impl CareerGoal {
    pub fn new(user_id: &str, title: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            title: title.to_string(),
            description: String::new(),
            target_role: None,
            target_industry: None,
            timeline_months: None,
            priority: "primary".to_string(),
            created_at: Utc::now(),
            active: true,
        }
    }
}
