//! Risk Scoring Module
//!
//! Provides DETERMINISTIC risk scoring for security policies.
//! No AI/ML is used - scores are calculated from existing metadata.
//!
//! Formula: risk_score = severity_weight × (1 - compliance_state)
//! Where:
//! - severity_weight: Derived from policy severity (critical=1.0, high=0.75, medium=0.5, low=0.25)
//! - compliance_state: 1.0 if compliant, 0.0 if non-compliant

use crate::engine::AuditResult;
use crate::types::Policy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Severity weight mappings (deterministic)
const SEVERITY_CRITICAL: f32 = 1.0;
const SEVERITY_HIGH: f32 = 0.75;
const SEVERITY_MEDIUM: f32 = 0.5;
const SEVERITY_LOW: f32 = 0.25;
const SEVERITY_DEFAULT: f32 = 0.5;

/// Risk score for a single policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRiskScore {
    pub policy_id: String,
    pub policy_title: String,
    pub severity: String,
    pub severity_weight: f32,
    pub compliance_state: f32,
    pub risk_score: f32,
    pub is_compliant: bool,
}

/// Aggregate risk metrics for the entire system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemRiskSummary {
    /// Total number of policies evaluated
    pub total_policies: usize,
    /// Number of compliant policies
    pub compliant_count: usize,
    /// Number of non-compliant policies
    pub non_compliant_count: usize,
    /// Aggregate risk score (sum of all risk scores)
    pub total_risk_score: f32,
    /// Normalized risk score (0.0 = fully compliant, 1.0 = maximum risk)
    pub normalized_risk_score: f32,
    /// Maximum possible risk score (if all policies were non-compliant)
    pub max_possible_risk: f32,
    /// Risk breakdown by severity
    pub risk_by_severity: HashMap<String, SeverityRiskBreakdown>,
}

/// Risk breakdown for a specific severity level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityRiskBreakdown {
    pub total_policies: usize,
    pub non_compliant_count: usize,
    pub total_risk: f32,
}

/// Converts severity string to numeric weight
pub fn severity_to_weight(severity: &str) -> f32 {
    match severity.to_lowercase().as_str() {
        "critical" => SEVERITY_CRITICAL,
        "high" => SEVERITY_HIGH,
        "medium" => SEVERITY_MEDIUM,
        "low" => SEVERITY_LOW,
        _ => SEVERITY_DEFAULT,
    }
}

/// Calculates risk score for a single policy
///
/// Formula: risk_score = severity_weight × (1 - compliance_state)
///
/// # Arguments
/// * `policy` - The policy definition
/// * `audit_result` - The audit result for this policy
///
/// # Returns
/// PolicyRiskScore with calculated risk
pub fn calculate_policy_risk(policy: &Policy, audit_result: &AuditResult) -> PolicyRiskScore {
    let severity = policy
        .severity
        .clone()
        .unwrap_or_else(|| "medium".to_string());
    let severity_weight = severity_to_weight(&severity);
    let compliance_state = if audit_result.passed { 1.0 } else { 0.0 };
    let risk_score = severity_weight * (1.0 - compliance_state);

    PolicyRiskScore {
        policy_id: policy.id.clone(),
        policy_title: policy.title.clone().unwrap_or_else(|| policy.id.clone()),
        severity: severity.clone(),
        severity_weight,
        compliance_state,
        risk_score,
        is_compliant: audit_result.passed,
    }
}

/// Calculates risk scores for all policies
pub fn calculate_all_risk_scores(
    policies: &[Policy],
    audit_results: &[AuditResult],
) -> Vec<PolicyRiskScore> {
    // Build a map of audit results by policy ID
    let result_map: HashMap<&str, &AuditResult> = audit_results
        .iter()
        .map(|r| (r.policy_id.as_str(), r))
        .collect();

    policies
        .iter()
        .filter_map(|policy| {
            result_map
                .get(policy.id.as_str())
                .map(|result| calculate_policy_risk(policy, result))
        })
        .collect()
}

/// Returns top N non-compliant policies by risk score
pub fn get_top_risks(risk_scores: &[PolicyRiskScore], n: usize) -> Vec<PolicyRiskScore> {
    let mut non_compliant: Vec<_> = risk_scores
        .iter()
        .filter(|s| !s.is_compliant)
        .cloned()
        .collect();

    // Sort by risk score descending
    non_compliant.sort_by(|a, b| {
        b.risk_score
            .partial_cmp(&a.risk_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    non_compliant.into_iter().take(n).collect()
}

/// Calculates aggregate system risk summary
pub fn calculate_system_risk(risk_scores: &[PolicyRiskScore]) -> SystemRiskSummary {
    let total_policies = risk_scores.len();
    let compliant_count = risk_scores.iter().filter(|s| s.is_compliant).count();
    let non_compliant_count = total_policies - compliant_count;

    let total_risk_score: f32 = risk_scores.iter().map(|s| s.risk_score).sum();
    let max_possible_risk: f32 = risk_scores.iter().map(|s| s.severity_weight).sum();

    let normalized_risk_score = if max_possible_risk > 0.0 {
        total_risk_score / max_possible_risk
    } else {
        0.0
    };

    // Calculate breakdown by severity
    let mut risk_by_severity: HashMap<String, SeverityRiskBreakdown> = HashMap::new();

    for score in risk_scores {
        let entry = risk_by_severity
            .entry(score.severity.to_lowercase())
            .or_insert(SeverityRiskBreakdown {
                total_policies: 0,
                non_compliant_count: 0,
                total_risk: 0.0,
            });

        entry.total_policies += 1;
        if !score.is_compliant {
            entry.non_compliant_count += 1;
        }
        entry.total_risk += score.risk_score;
    }

    SystemRiskSummary {
        total_policies,
        compliant_count,
        non_compliant_count,
        total_risk_score,
        normalized_risk_score,
        max_possible_risk,
        risk_by_severity,
    }
}

/// Risk level classification based on normalized score
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskLevel {
    Critical, // > 0.75
    High,     // 0.5 - 0.75
    Medium,   // 0.25 - 0.5
    Low,      // < 0.25
}

impl RiskLevel {
    pub fn from_normalized_score(score: f32) -> Self {
        if score > 0.75 {
            RiskLevel::Critical
        } else if score > 0.5 {
            RiskLevel::High
        } else if score > 0.25 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Critical => "CRITICAL",
            RiskLevel::High => "HIGH",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::Low => "LOW",
        }
    }

    pub fn color_code(&self) -> &'static str {
        match self {
            RiskLevel::Critical => "🔴",
            RiskLevel::High => "🟠",
            RiskLevel::Medium => "🟡",
            RiskLevel::Low => "🟢",
        }
    }
}

/// Formats risk summary for display
pub fn format_risk_summary(summary: &SystemRiskSummary) -> String {
    let risk_level = RiskLevel::from_normalized_score(summary.normalized_risk_score);
    let compliance_pct = if summary.total_policies > 0 {
        (summary.compliant_count as f32 / summary.total_policies as f32) * 100.0
    } else {
        100.0
    };

    format!(
        r#"
╔══════════════════════════════════════════════════════════════╗
║                    SYSTEM RISK SUMMARY                       ║
╠══════════════════════════════════════════════════════════════╣
║  {} Risk Level: {:10}                                   ║
║  Compliance: {:.1}% ({}/{} policies)                     
║  Total Risk Score: {:.2} / {:.2} max                        
║  Normalized Risk: {:.1}%                                     
╠══════════════════════════════════════════════════════════════╣
║  BREAKDOWN BY SEVERITY:                                      ║
║  🔴 Critical: {} non-compliant / {} total                   
║  🟠 High:     {} non-compliant / {} total                   
║  🟡 Medium:   {} non-compliant / {} total                   
║  🟢 Low:      {} non-compliant / {} total                   
╚══════════════════════════════════════════════════════════════╝
"#,
        risk_level.color_code(),
        risk_level.as_str(),
        compliance_pct,
        summary.compliant_count,
        summary.total_policies,
        summary.total_risk_score,
        summary.max_possible_risk,
        summary.normalized_risk_score * 100.0,
        summary.risk_by_severity.get("critical").map(|b| b.non_compliant_count).unwrap_or(0),
        summary.risk_by_severity.get("critical").map(|b| b.total_policies).unwrap_or(0),
        summary.risk_by_severity.get("high").map(|b| b.non_compliant_count).unwrap_or(0),
        summary.risk_by_severity.get("high").map(|b| b.total_policies).unwrap_or(0),
        summary.risk_by_severity.get("medium").map(|b| b.non_compliant_count).unwrap_or(0),
        summary.risk_by_severity.get("medium").map(|b| b.total_policies).unwrap_or(0),
        summary.risk_by_severity.get("low").map(|b| b.non_compliant_count).unwrap_or(0),
        summary.risk_by_severity.get("low").map(|b| b.total_policies).unwrap_or(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_policy(id: &str, severity: &str) -> Policy {
        Policy {
            id: id.to_string(),
            title: Some(format!("Test Policy {}", id)),
            severity: Some(severity.to_string()),
            platform: "windows".to_string(),
            check_type: "registry_key".to_string(),
            ..Default::default()
        }
    }

    fn create_audit_result(policy_id: &str, passed: bool) -> AuditResult {
        AuditResult {
            policy_id: policy_id.to_string(),
            passed,
            message: if passed { "Compliant" } else { "Non-compliant" }.to_string(),
        }
    }

    #[test]
    fn test_severity_to_weight() {
        assert_eq!(severity_to_weight("critical"), 1.0);
        assert_eq!(severity_to_weight("HIGH"), 0.75);
        assert_eq!(severity_to_weight("Medium"), 0.5);
        assert_eq!(severity_to_weight("low"), 0.25);
        assert_eq!(severity_to_weight("unknown"), 0.5);
    }

    #[test]
    fn test_calculate_policy_risk_compliant() {
        let policy = create_test_policy("A.1", "critical");
        let result = create_audit_result("A.1", true);
        
        let risk = calculate_policy_risk(&policy, &result);
        
        assert_eq!(risk.policy_id, "A.1");
        assert_eq!(risk.severity_weight, 1.0);
        assert_eq!(risk.compliance_state, 1.0);
        assert_eq!(risk.risk_score, 0.0); // Compliant = no risk
        assert!(risk.is_compliant);
    }

    #[test]
    fn test_calculate_policy_risk_non_compliant() {
        let policy = create_test_policy("A.1", "critical");
        let result = create_audit_result("A.1", false);
        
        let risk = calculate_policy_risk(&policy, &result);
        
        assert_eq!(risk.risk_score, 1.0); // Critical + non-compliant = max risk
        assert!(!risk.is_compliant);
    }

    #[test]
    fn test_get_top_risks() {
        let policies = vec![
            create_test_policy("A.1", "critical"),
            create_test_policy("A.2", "high"),
            create_test_policy("A.3", "low"),
        ];
        let results = vec![
            create_audit_result("A.1", false),
            create_audit_result("A.2", false),
            create_audit_result("A.3", false),
        ];
        
        let risk_scores = calculate_all_risk_scores(&policies, &results);
        let top = get_top_risks(&risk_scores, 2);
        
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].policy_id, "A.1"); // Critical first
        assert_eq!(top[1].policy_id, "A.2"); // High second
    }

    #[test]
    fn test_system_risk_summary() {
        let policies = vec![
            create_test_policy("A.1", "critical"),
            create_test_policy("A.2", "high"),
            create_test_policy("A.3", "medium"),
            create_test_policy("A.4", "low"),
        ];
        let results = vec![
            create_audit_result("A.1", false), // Non-compliant critical
            create_audit_result("A.2", true),  // Compliant high
            create_audit_result("A.3", false), // Non-compliant medium
            create_audit_result("A.4", true),  // Compliant low
        ];
        
        let risk_scores = calculate_all_risk_scores(&policies, &results);
        let summary = calculate_system_risk(&risk_scores);
        
        assert_eq!(summary.total_policies, 4);
        assert_eq!(summary.compliant_count, 2);
        assert_eq!(summary.non_compliant_count, 2);
        // Risk = 1.0 (critical) + 0.5 (medium) = 1.5
        assert!((summary.total_risk_score - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_risk_level_classification() {
        assert_eq!(RiskLevel::from_normalized_score(0.9), RiskLevel::Critical);
        assert_eq!(RiskLevel::from_normalized_score(0.6), RiskLevel::High);
        assert_eq!(RiskLevel::from_normalized_score(0.3), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_normalized_score(0.1), RiskLevel::Low);
    }
}
