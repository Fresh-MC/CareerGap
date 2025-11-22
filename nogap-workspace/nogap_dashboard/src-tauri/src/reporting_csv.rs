use crate::reporting::PolicyReport;
use std::fs::File;
use std::io::Write;
use tauri::AppHandle;

/// Escapes CSV field values to handle commas, quotes, and newlines
fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        // Escape double quotes by doubling them, then wrap in quotes
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// Generates a CSV report from policy audit results
#[tauri::command]
pub async fn generate_csv_report(
    _app_handle: AppHandle,
    policies: Vec<PolicyReport>,
    total: usize,
    pass: usize,
    fail: usize,
    timestamp: String,
) -> Result<String, String> {
    // Get system temp directory
    let temp_dir = std::env::temp_dir();
    
    // Generate unique filename with timestamp
    let filename = format!("nogap_report_{}.csv", timestamp.replace([':', '-', '.'], "_"));
    let file_path = temp_dir.join(&filename);
    
    // Create CSV content
    let mut csv_content = String::new();
    
    // Add metadata header comments (optional, but useful)
    csv_content.push_str(&format!("# NoGap Compliance Report\n"));
    csv_content.push_str(&format!("# Generated: {}\n", timestamp));
    csv_content.push_str(&format!("# Total Policies: {}\n", total));
    csv_content.push_str(&format!("# Passed: {}\n", pass));
    csv_content.push_str(&format!("# Failed: {}\n", fail));
    csv_content.push_str(&format!("# Compliance Rate: {:.1}%\n", if total > 0 { (pass as f64 / total as f64) * 100.0 } else { 0.0 }));
    csv_content.push_str("\n");
    
    // Add CSV header row
    csv_content.push_str("policy_id,title,status,platform,severity,compliant\n");
    
    // Add policy rows
    for policy in policies {
        let policy_id = escape_csv_field(&policy.policy_id);
        let title = escape_csv_field(&policy.title);
        let status = escape_csv_field(&policy.status);
        
        // Extract platform and severity from title or use defaults
        // In a real implementation, you'd have these as separate fields in PolicyReport
        // For now, we'll derive them intelligently
        let platform = if policy.policy_id.contains("WIN") || policy.title.to_lowercase().contains("windows") {
            "Windows"
        } else if policy.policy_id.contains("LIN") || policy.title.to_lowercase().contains("linux") {
            "Linux"
        } else {
            "Cross-Platform"
        };
        
        let severity = if policy.policy_id.contains("CRIT") || policy.title.to_lowercase().contains("critical") {
            "Critical"
        } else if policy.policy_id.contains("HIGH") || policy.title.to_lowercase().contains("high") {
            "High"
        } else if policy.policy_id.contains("MED") || policy.title.to_lowercase().contains("medium") {
            "Medium"
        } else {
            "Low"
        };
        
        // Determine compliant boolean from status
        let compliant = match status.to_lowercase().as_str() {
            "pass" | "compliant" => "true",
            "fail" | "non-compliant" => "false",
            _ => "unknown",
        };
        
        // Build CSV row
        csv_content.push_str(&format!(
            "{},{},{},{},{},{}\n",
            policy_id,
            title,
            status,
            escape_csv_field(platform),
            escape_csv_field(severity),
            compliant
        ));
    }
    
    // Write to file
    let mut file = File::create(&file_path)
        .map_err(|e| format!("Failed to create CSV file: {}", e))?;
    
    file.write_all(csv_content.as_bytes())
        .map_err(|e| format!("Failed to write CSV content: {}", e))?;
    
    // Return absolute path as string
    file_path
        .to_str()
        .ok_or_else(|| "Failed to convert path to string".to_string())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_csv_field_simple() {
        assert_eq!(escape_csv_field("simple text"), "simple text");
    }

    #[test]
    fn test_escape_csv_field_with_comma() {
        assert_eq!(escape_csv_field("text, with comma"), "\"text, with comma\"");
    }

    #[test]
    fn test_escape_csv_field_with_quotes() {
        assert_eq!(escape_csv_field("text \"with\" quotes"), "\"text \"\"with\"\" quotes\"");
    }

    #[test]
    fn test_escape_csv_field_with_newline() {
        assert_eq!(escape_csv_field("text\nwith newline"), "\"text\nwith newline\"");
    }

    #[test]
    fn test_escape_csv_field_complex() {
        assert_eq!(
            escape_csv_field("Policy: \"Test, Value\"\nDescription"),
            "\"Policy: \"\"Test, Value\"\"\nDescription\""
        );
    }
}
