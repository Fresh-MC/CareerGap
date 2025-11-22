use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Represents a single policy's report data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyReport {
    pub policy_id: String,
    pub title: String,
    pub status: String,
}

/// Generate an HTML report from audit results
#[tauri::command]
pub async fn generate_html_report(
    app_handle: AppHandle,
    policies: Vec<PolicyReport>,
    total: usize,
    pass: usize,
    fail: usize,
    windows_score: f32,
    linux_score: f32,
    timestamp: String,
) -> Result<String, String> {
    // Load the template
    let template = load_template(&app_handle).map_err(|e| {
        format!("Failed to load report template: {}", e)
    })?;

    // Generate dynamic content
    let policy_table = render_table(&policies);
    let platform_scores = render_platform_scores(windows_score, linux_score);

    // Create substitution map
    let mut substitutions = HashMap::new();
    substitutions.insert("{{DATE}}".to_string(), timestamp);
    substitutions.insert("{{TOTAL}}".to_string(), total.to_string());
    substitutions.insert("{{PASS}}".to_string(), pass.to_string());
    substitutions.insert("{{FAIL}}".to_string(), fail.to_string());
    substitutions.insert("{{POLICY_TABLE}}".to_string(), policy_table);
    substitutions.insert("{{PLATFORM_SCORES}}".to_string(), platform_scores);

    // Perform substitution
    let rendered_html = substitute(template, substitutions);

    // Write to temp directory
    let temp_dir = std::env::temp_dir();
    let report_path = temp_dir.join("nogap_report.html");
    
    fs::write(&report_path, rendered_html).map_err(|e| {
        format!("Failed to write HTML report: {}", e)
    })?;

    // Return absolute path as string
    report_path
        .to_str()
        .ok_or_else(|| "Failed to convert path to string".to_string())
        .map(|s| s.to_string())
}

/// Export an HTML report to PDF (simplified - frontend handles PDF generation)
/// Verifies HTML exists and returns path for frontend to use browser's print-to-PDF
#[tauri::command]
pub async fn export_pdf(
    _app_handle: AppHandle,
    html_path: String,
) -> Result<String, String> {
    // Verify the HTML file exists
    if !std::path::Path::new(&html_path).exists() {
        return Err(format!("HTML file not found: {}", html_path));
    }

    // Return the HTML path - frontend will handle PDF generation
    // via window.print() or browser's native print-to-PDF functionality
    Ok(html_path)
}

/// Load the HTML report template from the reports directory
fn load_template(app_handle: &AppHandle) -> Result<String, std::io::Error> {
    // Try to load from the app's resource directory
    let resource_path = app_handle
        .path()
        .resource_dir()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotFound, e.to_string()))?
        .join("reports")
        .join("report_template.html");

    // Attempt to read the template file
    fs::read_to_string(&resource_path).or_else(|_| {
        // Fallback: try loading from current working directory during development
        let dev_path = PathBuf::from("reports/report_template.html");
        fs::read_to_string(dev_path)
    })
}

/// Render policy data as an HTML table
fn render_table(policies: &[PolicyReport]) -> String {
    if policies.is_empty() {
        return String::from("<tr><td colspan=\"3\" style=\"text-align: center;\">No policies audited</td></tr>");
    }

    policies
        .iter()
        .map(|policy| {
            let status_badge = match policy.status.to_lowercase().as_str() {
                "pass" | "compliant" => format!(r#"<span class="badge-pass">PASS</span>"#),
                "fail" | "non-compliant" => format!(r#"<span class="badge-fail">FAIL</span>"#),
                _ => format!(r#"<span class="badge-fail">UNKNOWN</span>"#),
            };

            format!(
                r#"<tr>
                    <td>{}</td>
                    <td>{}</td>
                    <td style="text-align: center;">{}</td>
                </tr>"#,
                html_escape(&policy.policy_id),
                html_escape(&policy.title),
                status_badge
            )
        })
        .collect::<Vec<String>>()
        .join("\n")
}

/// Render platform compliance scores as HTML (for new template structure)
fn render_platform_scores(windows_score: f32, linux_score: f32) -> String {
    format!(
        r#"<div class="platform-card">
                <div class="platform-name">🪟 Windows</div>
                <div class="platform-score">{:.1}%</div>
                <div class="platform-label">Compliance Score</div>
            </div>
            <div class="platform-card">
                <div class="platform-name">🐧 Linux</div>
                <div class="platform-score">{:.1}%</div>
                <div class="platform-label">Compliance Score</div>
            </div>"#,
        windows_score, linux_score
    )
}

/// Substitute placeholders in template with actual values
fn substitute(template: String, substitutions: HashMap<String, String>) -> String {
    let mut result = template;
    
    for (placeholder, value) in substitutions {
        result = result.replace(&placeholder, &value);
    }
    
    result
}

/// Escape HTML special characters to prevent XSS
fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_table_empty() {
        let policies = vec![];
        let result = render_table(&policies);
        assert!(result.contains("No policies audited"));
    }

    #[test]
    fn test_render_table_with_policies() {
        let policies = vec![
            PolicyReport {
                policy_id: "WIN-001".to_string(),
                title: "Password Complexity".to_string(),
                status: "Pass".to_string(),
            },
            PolicyReport {
                policy_id: "WIN-002".to_string(),
                title: "Account Lockout".to_string(),
                status: "Fail".to_string(),
            },
        ];
        
        let result = render_table(&policies);
        assert!(result.contains("WIN-001"));
        assert!(result.contains("Password Complexity"));
        assert!(result.contains("badge-pass"));
        assert!(result.contains("WIN-002"));
        assert!(result.contains("Account Lockout"));
        assert!(result.contains("badge-fail"));
    }

    #[test]
    fn test_render_platform_scores() {
        let result = render_platform_scores(85.5, 92.3);
        assert!(result.contains("85.5%"));
        assert!(result.contains("92.3%"));
        assert!(result.contains("Windows"));
        assert!(result.contains("Linux"));
        assert!(result.contains("platform-card"));
    }

    #[test]
    fn test_substitute() {
        let template = "Hello {{NAME}}, you have {{COUNT}} messages.".to_string();
        let mut map = HashMap::new();
        map.insert("{{NAME}}".to_string(), "Alice".to_string());
        map.insert("{{COUNT}}".to_string(), "5".to_string());
        
        let result = substitute(template, map);
        assert_eq!(result, "Hello Alice, you have 5 messages.");
    }

    #[test]
    fn test_html_escape() {
        let input = "<script>alert('XSS')</script>";
        let escaped = html_escape(input);
        assert_eq!(escaped, "&lt;script&gt;alert(&#x27;XSS&#x27;)&lt;/script&gt;");
    }

    #[test]
    fn test_html_escape_ampersand() {
        let input = "A & B";
        let escaped = html_escape(input);
        assert_eq!(escaped, "A &amp; B");
    }
}
