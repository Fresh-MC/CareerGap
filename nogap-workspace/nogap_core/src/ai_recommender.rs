//! AI-Assisted Policy Recommender Module
//!
//! This module provides OPTIONAL LLM-based policy recommendations.
//! It maps user-provided system descriptions to existing policy IDs.
//!
//! IMPORTANT: This is NON-AGENTIC functionality.
//! - The LLM CANNOT invent new policies
//! - The LLM CANNOT modify existing policies
//! - The LLM CANNOT auto-apply remediations
//! - All enforcement remains rule-based and user-approved

use crate::types::Policy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Metadata extracted from a policy for LLM context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyMetadata {
    pub id: String,
    pub title: String,
    pub description: String,
    pub platform: String,
    pub severity: String,
    pub check_type: String,
}

impl From<&Policy> for PolicyMetadata {
    fn from(policy: &Policy) -> Self {
        Self {
            id: policy.id.clone(),
            title: policy.title.clone().unwrap_or_default(),
            description: policy.description.clone().unwrap_or_default(),
            platform: policy.platform.clone(),
            severity: policy.severity.clone().unwrap_or_else(|| "medium".to_string()),
            check_type: policy.check_type.clone(),
        }
    }
}

/// System context provided by the user for policy recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContext {
    /// Operating system (e.g., "Windows Server 2022", "Ubuntu 22.04")
    pub os: String,
    /// System role (e.g., "web server", "database server", "workstation")
    pub role: String,
    /// Environment (e.g., "production", "development", "air-gapped")
    pub environment: String,
    /// Additional context or requirements
    #[serde(default)]
    pub additional_context: Option<String>,
}

/// A policy recommendation with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRecommendation {
    pub policy_id: String,
    pub relevance_score: f32,
    pub reason: String,
}

/// Result of the recommendation process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationResult {
    /// Successfully recommended policy IDs (validated against known policies)
    pub recommendations: Vec<PolicyRecommendation>,
    /// Policy IDs suggested by LLM but not found in the policy library
    pub invalid_suggestions: Vec<String>,
    /// Warning messages (e.g., if LLM tried to invent policies)
    pub warnings: Vec<String>,
}

/// Extracts metadata from all policies for LLM context
pub fn extract_policy_metadata(policies: &[Policy]) -> Vec<PolicyMetadata> {
    policies.iter().map(PolicyMetadata::from).collect()
}

/// Builds a policy ID lookup set for validation
pub fn build_policy_id_set(policies: &[Policy]) -> HashSet<String> {
    policies.iter().map(|p| p.id.clone()).collect()
}

/// Generates a prompt for the LLM to recommend policies
///
/// This prompt is designed to:
/// 1. Constrain the LLM to ONLY recommend existing policy IDs
/// 2. Prevent the LLM from inventing new policies
/// 3. Request structured output for easy parsing
pub fn generate_recommendation_prompt(
    context: &SystemContext,
    metadata: &[PolicyMetadata],
) -> String {
    let policy_list: String = metadata
        .iter()
        .map(|m| {
            format!(
                "- ID: {}, Platform: {}, Severity: {}, Title: {}, Description: {}",
                m.id, m.platform, m.severity, m.title, m.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"You are a security policy recommendation assistant. Your task is to recommend relevant security policies from the EXISTING policy library below.

CRITICAL CONSTRAINTS:
1. You MUST ONLY recommend policy IDs that exist in the list below
2. You MUST NOT invent or create new policies
3. You MUST NOT suggest modifications to existing policies
4. You MUST NOT recommend auto-applying any remediations

SYSTEM CONTEXT:
- Operating System: {}
- System Role: {}
- Environment: {}
- Additional Context: {}

AVAILABLE POLICIES:
{}

INSTRUCTIONS:
Based on the system context above, recommend the most relevant policies from the AVAILABLE POLICIES list.
For each recommendation, provide:
1. The exact policy ID (must match exactly from the list)
2. A relevance score from 0.0 to 1.0
3. A brief reason why this policy is relevant

Format your response as JSON:
{{
  "recommendations": [
    {{"policy_id": "exact_id_from_list", "relevance_score": 0.95, "reason": "Brief explanation"}}
  ]
}}

Only recommend policies that are:
1. Compatible with the specified operating system ({})
2. Relevant to the system role ({})
3. Appropriate for the environment ({})

Prioritize high-severity policies for production environments."#,
        context.os,
        context.role,
        context.environment,
        context.additional_context.clone().unwrap_or_else(|| "None".to_string()),
        policy_list,
        context.os,
        context.role,
        context.environment
    )
}

/// Validates LLM recommendations against known policy IDs
///
/// This function implements FAIL-CLOSED behavior:
/// - Only recommendations matching known policy IDs are returned
/// - Invalid suggestions are tracked but rejected
/// - Warnings are generated for any suspicious behavior
pub fn validate_recommendations(
    llm_response: &str,
    valid_policy_ids: &HashSet<String>,
) -> RecommendationResult {
    let mut result = RecommendationResult {
        recommendations: Vec::new(),
        invalid_suggestions: Vec::new(),
        warnings: Vec::new(),
    };

    // Parse LLM response as JSON
    let parsed: Result<LlmRecommendationResponse, _> = serde_json::from_str(llm_response);

    match parsed {
        Ok(response) => {
            for rec in response.recommendations {
                if valid_policy_ids.contains(&rec.policy_id) {
                    // Valid recommendation - include it
                    result.recommendations.push(PolicyRecommendation {
                        policy_id: rec.policy_id,
                        relevance_score: rec.relevance_score.clamp(0.0, 1.0),
                        reason: rec.reason,
                    });
                } else {
                    // Invalid policy ID - reject and track
                    result.invalid_suggestions.push(rec.policy_id.clone());
                    result.warnings.push(format!(
                        "LLM suggested non-existent policy ID '{}' - rejected",
                        rec.policy_id
                    ));
                }
            }
        }
        Err(e) => {
            result.warnings.push(format!(
                "Failed to parse LLM response: {}. Returning empty recommendations.",
                e
            ));
        }
    }

    // Sort by relevance score (descending)
    result
        .recommendations
        .sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());

    result
}

/// Internal structure for parsing LLM JSON response
#[derive(Debug, Deserialize)]
struct LlmRecommendationResponse {
    recommendations: Vec<LlmRecommendation>,
}

#[derive(Debug, Deserialize)]
struct LlmRecommendation {
    policy_id: String,
    relevance_score: f32,
    reason: String,
}

/// Filters policies by platform for pre-filtering before LLM
pub fn filter_policies_by_platform<'a>(policies: &'a [Policy], platform: &str) -> Vec<&'a Policy> {
    let platform_lower = platform.to_lowercase();
    policies
        .iter()
        .filter(|p| {
            let policy_platform = p.platform.to_lowercase();
            if platform_lower.contains("windows") {
                policy_platform == "windows"
            } else if platform_lower.contains("linux")
                || platform_lower.contains("ubuntu")
                || platform_lower.contains("debian")
                || platform_lower.contains("rhel")
                || platform_lower.contains("centos")
            {
                policy_platform == "linux"
            } else {
                true // Include all if platform unclear
            }
        })
        .collect()
}

/// Keyword-based fallback recommender (no LLM required)
///
/// This provides basic recommendations when LLM is unavailable
/// by matching keywords in system context against policy metadata.
pub fn keyword_based_recommendations(
    context: &SystemContext,
    policies: &[Policy],
    max_results: usize,
) -> Vec<PolicyRecommendation> {
    let mut scored_policies: Vec<(String, f32, String)> = Vec::new();
    
    let context_keywords: Vec<&str> = vec![
        &context.os,
        &context.role,
        &context.environment,
    ]
    .into_iter()
    .flat_map(|s| s.to_lowercase().split_whitespace().map(str::to_string).collect::<Vec<_>>())
    .map(|s| Box::leak(s.into_boxed_str()) as &str)
    .collect();

    for policy in policies {
        let mut score = 0.0f32;
        let mut reasons = Vec::new();

        // Platform matching (highest weight)
        let platform_match = if context.os.to_lowercase().contains("windows") {
            policy.platform == "windows"
        } else {
            policy.platform == "linux"
        };
        
        if platform_match {
            score += 0.4;
            reasons.push("Platform match");
        } else {
            continue; // Skip non-matching platforms
        }

        // Severity bonus for production
        if context.environment.to_lowercase().contains("production") {
            if let Some(ref sev) = policy.severity {
                match sev.to_lowercase().as_str() {
                    "critical" => { score += 0.3; reasons.push("Critical severity for production"); }
                    "high" => { score += 0.2; reasons.push("High severity for production"); }
                    _ => {}
                }
            }
        }

        // Keyword matching in title/description
        let title = policy.title.clone().unwrap_or_default().to_lowercase();
        let desc = policy.description.clone().unwrap_or_default().to_lowercase();
        
        for keyword in &context_keywords {
            if title.contains(keyword) || desc.contains(keyword) {
                score += 0.1;
                reasons.push("Keyword match");
                break;
            }
        }

        // Role-based matching
        let role_lower = context.role.to_lowercase();
        if role_lower.contains("server") {
            if title.contains("server") || desc.contains("server") || title.contains("service") {
                score += 0.15;
                reasons.push("Server-related policy");
            }
        }
        if role_lower.contains("database") || role_lower.contains("db") {
            if title.contains("database") || title.contains("sql") || desc.contains("database") {
                score += 0.15;
                reasons.push("Database-related policy");
            }
        }
        if role_lower.contains("web") {
            if title.contains("web") || title.contains("http") || desc.contains("web") {
                score += 0.15;
                reasons.push("Web-related policy");
            }
        }

        if score > 0.4 {
            scored_policies.push((
                policy.id.clone(),
                score.min(1.0),
                reasons.join(", "),
            ));
        }
    }

    // Sort by score descending
    scored_policies.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Take top N
    scored_policies
        .into_iter()
        .take(max_results)
        .map(|(id, score, reason)| PolicyRecommendation {
            policy_id: id,
            relevance_score: score,
            reason,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_policies() -> Vec<Policy> {
        vec![
            Policy {
                id: "A.1.a.i".to_string(),
                title: Some("Ensure Windows Firewall is enabled".to_string()),
                description: Some("Enable Windows Defender Firewall for all profiles".to_string()),
                platform: "windows".to_string(),
                severity: Some("critical".to_string()),
                check_type: "registry_key".to_string(),
                ..Default::default()
            },
            Policy {
                id: "B.1.a.i".to_string(),
                title: Some("Ensure SSH root login is disabled".to_string()),
                description: Some("Prevent direct root SSH access".to_string()),
                platform: "linux".to_string(),
                severity: Some("high".to_string()),
                check_type: "file_content".to_string(),
                ..Default::default()
            },
        ]
    }

    #[test]
    fn test_extract_policy_metadata() {
        let policies = create_test_policies();
        let metadata = extract_policy_metadata(&policies);
        
        assert_eq!(metadata.len(), 2);
        assert_eq!(metadata[0].id, "A.1.a.i");
        assert_eq!(metadata[0].platform, "windows");
    }

    #[test]
    fn test_build_policy_id_set() {
        let policies = create_test_policies();
        let id_set = build_policy_id_set(&policies);
        
        assert!(id_set.contains("A.1.a.i"));
        assert!(id_set.contains("B.1.a.i"));
        assert!(!id_set.contains("X.0.0.0"));
    }

    #[test]
    fn test_validate_recommendations_valid() {
        let policies = create_test_policies();
        let id_set = build_policy_id_set(&policies);
        
        let llm_response = r#"{"recommendations": [
            {"policy_id": "A.1.a.i", "relevance_score": 0.95, "reason": "Critical firewall policy"}
        ]}"#;
        
        let result = validate_recommendations(llm_response, &id_set);
        
        assert_eq!(result.recommendations.len(), 1);
        assert_eq!(result.recommendations[0].policy_id, "A.1.a.i");
        assert!(result.invalid_suggestions.is_empty());
    }

    #[test]
    fn test_validate_recommendations_invalid_rejected() {
        let policies = create_test_policies();
        let id_set = build_policy_id_set(&policies);
        
        let llm_response = r#"{"recommendations": [
            {"policy_id": "FAKE.POLICY.ID", "relevance_score": 0.9, "reason": "Invented policy"}
        ]}"#;
        
        let result = validate_recommendations(llm_response, &id_set);
        
        assert!(result.recommendations.is_empty());
        assert_eq!(result.invalid_suggestions.len(), 1);
        assert!(result.warnings.len() > 0);
    }

    #[test]
    fn test_keyword_recommendations() {
        let policies = create_test_policies();
        let context = SystemContext {
            os: "Windows Server 2022".to_string(),
            role: "Web Server".to_string(),
            environment: "Production".to_string(),
            additional_context: None,
        };
        
        let recs = keyword_based_recommendations(&context, &policies, 10);
        
        // Should only include Windows policies
        assert!(recs.iter().all(|r| r.policy_id.starts_with("A.")));
    }

    #[test]
    fn test_filter_policies_by_platform() {
        let policies = create_test_policies();
        
        let windows_policies = filter_policies_by_platform(&policies, "Windows Server");
        assert_eq!(windows_policies.len(), 1);
        assert_eq!(windows_policies[0].platform, "windows");
        
        let linux_policies = filter_policies_by_platform(&policies, "Ubuntu 22.04");
        assert_eq!(linux_policies.len(), 1);
        assert_eq!(linux_policies[0].platform, "linux");
    }
}
