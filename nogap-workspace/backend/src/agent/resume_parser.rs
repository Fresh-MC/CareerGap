//! Resume Parser Integration
//!
//! Calls the external Python resume parser and deserializes the output.
//! Python is used ONLY for parsing - all intelligence remains in Rust.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use thiserror::Error;

/// Output from the Python resume parser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedResume {
    pub skills: Vec<String>,
    pub experience: Vec<ParsedExperience>,
    pub education: Vec<ParsedEducation>,
    pub total_experience: Option<f64>,
    pub raw_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedExperience {
    pub company: String,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedEducation {
    #[serde(default)]
    pub degree: Option<String>,
    #[serde(default)]
    pub institution: Option<String>,
}

// =============================================================================
// EVIDENCE MAPPER TYPES (Stage 1)
// =============================================================================

/// Output from the Python evidence mapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceMap {
    pub normalized_skills: Vec<String>,
    pub skill_evidence_map: HashMap<String, Vec<String>>,
    pub section_signals: SectionSignals,
}

/// Section presence signals from resume
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionSignals {
    pub has_projects: bool,
    pub has_internship: bool,
    pub has_metrics: bool,
    pub has_deployment: bool,
}

// =============================================================================
// BOTTLENECK ANALYZER TYPES (Stage 2)
// =============================================================================

/// Output from the Python bottleneck analyzer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleneckAnalysis {
    pub implied_role: String,
    pub bottlenecks: Bottlenecks,
    pub dominant_issue: Option<String>,
    pub justification: String,
}

/// Bottleneck evaluation results - each category is "strong", "weak", or "missing"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottlenecks {
    pub positioning: String,
    pub evidence_depth: String,
    pub experience_strength: String,
    pub skill_alignment: String,
    pub outcome_visibility: String,
}

// =============================================================================
// STRATEGY SELECTOR TYPES (Stage 3)
// =============================================================================

/// Output from the Python strategy selector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategySelection {
    pub strategy: Strategy,
    pub action: String,
    pub confidence: f64,
}

/// Available strategies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Strategy {
    ResumeOptimization,
    SkillGapPatch,
    RoleShift,
    HoldPosition,
}

// =============================================================================
// AGENT LOOP TYPES (Feedback & Re-evaluation)
// =============================================================================

/// Outcome of a strategy execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    NoResponse,
    Rejected,
    Interview,
}

/// Record of a strategy attempt with outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRecord {
    pub strategy: String,
    pub initial_confidence: f64,
    pub current_confidence: f64,
    pub outcomes: Vec<String>,
    pub failed: bool,
    #[serde(default = "default_strategy_state")]
    pub strategy_state: String,
}

fn default_strategy_state() -> String {
    "explore".to_string()
}

/// Complete agent session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub stage1_evidence: EvidenceMap,
    pub stage2_bottleneck: BottleneckAnalysis,
    pub stage3_strategy: StrategySelection,
    pub current_strategy: Option<StrategyRecord>,
    pub strategy_history: Vec<StrategyRecord>,
    pub loop_iteration: u32,
    pub explanation_log: Vec<String>,
}

/// Result of processing an outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeResult {
    pub session: AgentSession,
    pub strategy_changed: bool,
    pub current_strategy: StrategySelection,
    pub explanation: String,
}

/// Explanation output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    pub explanation: String,
}

/// Error from Python parser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserError {
    pub error: String,
}

#[derive(Error, Debug)]
pub enum ResumeParseError {
    #[error("Python script not found at {0}")]
    ScriptNotFound(String),

    #[error("Failed to execute Python: {0}")]
    ExecutionFailed(String),

    #[error("Parser error: {0}")]
    ParserError(String),

    #[error("Invalid JSON output: {0}")]
    InvalidJson(String),

    #[error("Resume file not found: {0}")]
    FileNotFound(String),
}

/// Configuration for the resume parser
#[derive(Debug, Clone)]
pub struct ResumeParserConfig {
    /// Path to Python executable (e.g., "python" or full path to venv)
    pub python_path: String,
    /// Path to the parser.py script
    pub script_path: String,
}

impl Default for ResumeParserConfig {
    fn default() -> Self {
        Self {
            python_path: "../resume_parser/venv/Scripts/python.exe".to_string(),
            script_path: "../resume_parser/parser.py".to_string(),
        }
    }
}

impl ResumeParserConfig {
    /// Create config with virtual environment path (Windows)
    pub fn with_venv(venv_path: &str) -> Self {
        let python_path = format!("{}/Scripts/python.exe", venv_path);
        Self {
            python_path,
            script_path: "./resume_parser/parser.py".to_string(),
        }
    }

    /// Resolve a relative path from backend/ to an absolute path
    /// Handles paths like "../resume_parser/parser.py" from backend/ directory
    fn resolve_path(relative_path: &str) -> Result<std::path::PathBuf, ResumeParseError> {
        let current_dir = std::env::current_dir()
            .map_err(|e| ResumeParseError::ExecutionFailed(format!("Cannot get current directory: {}", e)))?;
        
        let full_path = current_dir.join(relative_path);
        
        // Canonicalize to resolve .. and . components
        match full_path.canonicalize() {
            Ok(canonical) => {
                eprintln!("[resolve_path] {} => {}", relative_path, canonical.display());
                Ok(canonical)
            }
            Err(e) => {
                eprintln!("[resolve_path] ERROR: Failed to resolve '{}' from '{}': {}", 
                         relative_path, current_dir.display(), e);
                Err(ResumeParseError::ScriptNotFound(format!(
                    "Cannot resolve path '{}' from '{}': {}", 
                    relative_path, current_dir.display(), e
                )))
            }
        }
    }
}

/// Parse a resume file using the external Python parser
///
/// # Arguments
/// * `resume_path` - Path to the PDF or DOCX resume file
/// * `config` - Parser configuration (Python path, script path)
///
/// # Returns
/// * `Ok(ParsedResume)` - Parsed resume data
/// * `Err(ResumeParseError)` - If parsing fails
///
/// # Example
/// ```ignore
/// let config = ResumeParserConfig::default();
/// let result = parse_resume("uploads/resume.pdf", &config)?;
/// println!("Skills: {:?}", result.skills);
/// ```
pub fn parse_resume(
    resume_path: &str,
    config: &ResumeParserConfig,
) -> Result<ParsedResume, ResumeParseError> {
    // Validate resume file exists
    if !Path::new(resume_path).exists() {
        eprintln!("[parse_resume] ERROR: Resume file not found: {}", resume_path);
        return Err(ResumeParseError::FileNotFound(resume_path.to_string()));
    }

    // Resolve and validate Python executable path
    let python_exe = ResumeParserConfig::resolve_path(&config.python_path)?;
    if !python_exe.exists() {
        eprintln!("[parse_resume] ERROR: Python executable not found: {}", python_exe.display());
        return Err(ResumeParseError::ScriptNotFound(format!("Python not found: {}", python_exe.display())));
    }

    // Resolve and validate script path
    let script_path = ResumeParserConfig::resolve_path(&config.script_path)?;
    if !script_path.exists() {
        eprintln!("[parse_resume] ERROR: Script not found: {}", script_path.display());
        return Err(ResumeParseError::ScriptNotFound(format!("Script not found: {}", script_path.display())));
    }

    // ===== PIPELINE STAGE: parse_resume =====
    eprintln!("[pipeline:parse_resume] ========== STAGE START ==========");
    eprintln!("[pipeline:parse_resume] Resolved paths:");
    eprintln!("[pipeline:parse_resume]   Python: {}", python_exe.display());
    eprintln!("[pipeline:parse_resume]   Script: {}", script_path.display());
    eprintln!("[pipeline:parse_resume]   Resume: {}", resume_path);
    eprintln!("[pipeline:parse_resume] Command: {:?} {:?} {:?}", python_exe, script_path, resume_path);

    // Execute Python script with resolved paths
    let output = Command::new(&python_exe)
        .arg(&script_path)
        .arg(resume_path)
        .output()
        .map_err(|e| {
            eprintln!("[pipeline:parse_resume] ❌ FAILED to spawn process: {}", e);
            ResumeParseError::ExecutionFailed(format!("Stage: parse_resume | Error: {}", e))
        })?;

    // Get stdout and stderr
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code();
    
    eprintln!("[pipeline:parse_resume] Exit code: {:?}", exit_code);
    eprintln!("[pipeline:parse_resume] stdout length: {} bytes", stdout.len());
    eprintln!("[pipeline:parse_resume] stderr length: {} bytes", stderr.len());
    
    if !stderr.is_empty() {
        eprintln!("[pipeline:parse_resume] stderr output:");
        eprintln!("--- stderr start ---");
        eprintln!("{}", stderr);
        eprintln!("--- stderr end ---");
    }
    
    if stdout.is_empty() {
        eprintln!("[pipeline:parse_resume] ❌ ERROR: stdout is empty");
        eprintln!("[pipeline:parse_resume] This indicates Python script did not produce output");
    }
    
    if exit_code != Some(0) {
        eprintln!("[pipeline:parse_resume] ❌ Non-zero exit code: {:?}", exit_code);
        return Err(ResumeParseError::ExecutionFailed(format!(
            "Stage: parse_resume | Exit code: {:?} | stderr: {}",
            exit_code, stderr
        )));
    }

    // Check if it's an error response
    if let Ok(error) = serde_json::from_str::<ParserError>(&stdout) {
        eprintln!("[pipeline:parse_resume] ❌ Parser returned error: {}", error.error);
        return Err(ResumeParseError::ParserError(format!("Stage: parse_resume | {}", error.error)));
    }

    // Parse successful response
    let parsed: ParsedResume = serde_json::from_str(&stdout)
        .map_err(|e| {
            let preview = &stdout[..stdout.len().min(500)];
            eprintln!("[pipeline:parse_resume] ❌ JSON parse failed: {}", e);
            eprintln!("[pipeline:parse_resume] stdout preview:");
            eprintln!("--- stdout preview (first 500 chars) ---");
            eprintln!("{}", preview);
            eprintln!("--- end preview ---");
            ResumeParseError::InvalidJson(format!(
                "Stage: parse_resume | JSON error: {} | stdout length: {} bytes | preview: {}",
                e, stdout.len(), preview
            ))
        })?;

    eprintln!("[pipeline:parse_resume] ✅ SUCCESS");
    eprintln!("[pipeline:parse_resume] ========== STAGE END ==========");
    Ok(parsed)
}

/// Map evidence from a parsed resume using the Python evidence mapper
///
/// # Arguments
/// * `parsed_resume` - Previously parsed resume data
/// * `config` - Parser configuration (Python path, script path)
///
/// # Returns
/// * `Ok(EvidenceMap)` - Evidence mapping with normalized skills and signals
/// * `Err(ResumeParseError)` - If mapping fails
pub fn map_evidence(
    parsed_resume: &ParsedResume,
    config: &ResumeParserConfig,
) -> Result<EvidenceMap, ResumeParseError> {
    let evidence_script_rel = config.script_path.replace("parser.py", "evidence_mapper.py");
    
    // Resolve paths
    let python_exe = ResumeParserConfig::resolve_path(&config.python_path)?;
    let evidence_script = ResumeParserConfig::resolve_path(&evidence_script_rel)?;
    
    if !evidence_script.exists() {
        return Err(ResumeParseError::ScriptNotFound(evidence_script.display().to_string()));
    }

    // Serialize parsed resume to JSON
    let input_json = serde_json::to_string(parsed_resume)
        .map_err(|e| ResumeParseError::InvalidJson(e.to_string()))?;

    // ===== PIPELINE STAGE: map_evidence =====
    eprintln!("[pipeline:map_evidence] ========== STAGE START ==========");
    eprintln!("[pipeline:map_evidence] Python: {}", python_exe.display());
    eprintln!("[pipeline:map_evidence] Script: {}", evidence_script.display());
    eprintln!("[pipeline:map_evidence] Input JSON length: {} bytes (via stdin)", input_json.len());

    // Execute Python evidence mapper with JSON via stdin
    use std::process::Stdio;
    use std::io::Write;
    
    let mut child = Command::new(&python_exe)
        .arg(&evidence_script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            eprintln!("[pipeline:map_evidence] ❌ FAILED to spawn process: {}", e);
            ResumeParseError::ExecutionFailed(format!("Stage: map_evidence | Error: {}", e))
        })?;
    
    // Write JSON to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input_json.as_bytes())
            .map_err(|e| {
                eprintln!("[pipeline:map_evidence] ❌ FAILED to write to stdin: {}", e);
                ResumeParseError::ExecutionFailed(format!("Stage: map_evidence | stdin write error: {}", e))
            })?;
        drop(stdin); // Close stdin to signal EOF
    } else {
        eprintln!("[pipeline:map_evidence] ❌ FAILED to get stdin handle");
        return Err(ResumeParseError::ExecutionFailed("Stage: map_evidence | No stdin handle".to_string()));
    }
    
    // Wait for process and capture output
    let output = child.wait_with_output()
        .map_err(|e| {
            eprintln!("[pipeline:map_evidence] ❌ FAILED to wait for process: {}", e);
            ResumeParseError::ExecutionFailed(format!("Stage: map_evidence | Wait error: {}", e))
        })?;

    // Get stdout and stderr
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code();
    
    eprintln!("[pipeline:map_evidence] Exit code: {:?}", exit_code);
    eprintln!("[pipeline:map_evidence] stdout length: {} bytes", stdout.len());
    if !stderr.is_empty() {
        eprintln!("[pipeline:map_evidence] stderr: {}", stderr);
    }
    
    if exit_code != Some(0) {
        eprintln!("[pipeline:map_evidence] ❌ Non-zero exit code");
        return Err(ResumeParseError::ExecutionFailed(format!(
            "Stage: map_evidence | Exit code: {:?} | stderr: {}",
            exit_code, stderr
        )));
    }

    // Check if it's an error response
    if let Ok(error) = serde_json::from_str::<ParserError>(&stdout) {
        eprintln!("[pipeline:map_evidence] ❌ Parser error: {}", error.error);
        return Err(ResumeParseError::ParserError(format!("Stage: map_evidence | {}", error.error)));
    }

    // Parse successful response
    let evidence: EvidenceMap = serde_json::from_str(&stdout)
        .map_err(|e| {
            let preview = &stdout[..stdout.len().min(500)];
            eprintln!("[pipeline:map_evidence] ❌ JSON parse failed: {}", e);
            eprintln!("[pipeline:map_evidence] stdout preview: {}", preview);
            ResumeParseError::InvalidJson(format!(
                "Stage: map_evidence | JSON error: {} | stdout length: {} bytes",
                e, stdout.len()
            ))
        })?;

    eprintln!("[pipeline:map_evidence] ✅ SUCCESS");
    eprintln!("[pipeline:map_evidence] ========== STAGE END ==========");
    Ok(evidence)
}

/// Parse resume and map evidence in one call (full pipeline)
///
/// # Arguments
/// * `resume_path` - Path to the PDF or DOCX resume file
/// * `config` - Parser configuration
///
/// # Returns
/// * `Ok((ParsedResume, EvidenceMap))` - Both parsed data and evidence
pub fn parse_and_map(
    resume_path: &str,
    config: &ResumeParserConfig,
) -> Result<(ParsedResume, EvidenceMap), ResumeParseError> {
    let parsed = parse_resume(resume_path, config)?;
    let evidence = map_evidence(&parsed, config)?;
    Ok((parsed, evidence))
}

/// Analyze bottlenecks from evidence map (Stage 2)
///
/// # Arguments
/// * `evidence` - Evidence map from Stage 1
/// * `config` - Parser configuration
///
/// # Returns
/// * `Ok(BottleneckAnalysis)` - Bottleneck analysis with implied role and dominant issue
pub fn analyze_bottlenecks(
    evidence: &EvidenceMap,
    config: &ResumeParserConfig,
) -> Result<BottleneckAnalysis, ResumeParseError> {
    let analyzer_script_rel = config.script_path.replace("parser.py", "bottleneck_analyzer.py");
    
    // Resolve paths
    let python_exe = ResumeParserConfig::resolve_path(&config.python_path)?;
    let analyzer_script = ResumeParserConfig::resolve_path(&analyzer_script_rel)?;
    
    if !analyzer_script.exists() {
        return Err(ResumeParseError::ScriptNotFound(analyzer_script.display().to_string()));
    }

    // Serialize evidence to JSON
    let input_json = serde_json::to_string(evidence)
        .map_err(|e| ResumeParseError::InvalidJson(e.to_string()))?;

    // ===== PIPELINE STAGE: analyze_bottlenecks =====
    eprintln!("[pipeline:analyze_bottlenecks] ========== STAGE START ==========");
    eprintln!("[pipeline:analyze_bottlenecks] Python: {}", python_exe.display());
    eprintln!("[pipeline:analyze_bottlenecks] Script: {}", analyzer_script.display());
    eprintln!("[pipeline:analyze_bottlenecks] Input JSON length: {} bytes (via stdin)", input_json.len());

    // Execute Python bottleneck analyzer with JSON via stdin
    use std::process::Stdio;
    use std::io::Write;
    
    let mut child = Command::new(&python_exe)
        .arg(&analyzer_script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            eprintln!("[pipeline:analyze_bottlenecks] ❌ FAILED to spawn process: {}", e);
            ResumeParseError::ExecutionFailed(format!("Stage: analyze_bottlenecks | Error: {}", e))
        })?;
    
    // Write JSON to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input_json.as_bytes())
            .map_err(|e| {
                eprintln!("[pipeline:analyze_bottlenecks] ❌ FAILED to write to stdin: {}", e);
                ResumeParseError::ExecutionFailed(format!("Stage: analyze_bottlenecks | stdin write error: {}", e))
            })?;
        drop(stdin); // Close stdin to signal EOF
    } else {
        eprintln!("[pipeline:analyze_bottlenecks] ❌ FAILED to get stdin handle");
        return Err(ResumeParseError::ExecutionFailed("Stage: analyze_bottlenecks | No stdin handle".to_string()));
    }
    
    // Wait for process and capture output
    let output = child.wait_with_output()
        .map_err(|e| {
            eprintln!("[pipeline:analyze_bottlenecks] ❌ FAILED to wait for process: {}", e);
            ResumeParseError::ExecutionFailed(format!("Stage: analyze_bottlenecks | Wait error: {}", e))
        })?;

    // Get stdout and stderr
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code();
    
    eprintln!("[pipeline:analyze_bottlenecks] Exit code: {:?}", exit_code);
    eprintln!("[pipeline:analyze_bottlenecks] stdout length: {} bytes", stdout.len());
    if !stderr.is_empty() {
        eprintln!("[pipeline:analyze_bottlenecks] stderr: {}", stderr);
    }
    
    if exit_code != Some(0) {
        eprintln!("[pipeline:analyze_bottlenecks] ❌ Non-zero exit code");
        return Err(ResumeParseError::ExecutionFailed(format!(
            "Stage: analyze_bottlenecks | Exit code: {:?} | stderr: {}",
            exit_code, stderr
        )));
    }

    // Check if it's an error response
    if let Ok(error) = serde_json::from_str::<ParserError>(&stdout) {
        eprintln!("[pipeline:analyze_bottlenecks] ❌ Parser error: {}", error.error);
        return Err(ResumeParseError::ParserError(format!("Stage: analyze_bottlenecks | {}", error.error)));
    }

    // Parse successful response
    let analysis: BottleneckAnalysis = serde_json::from_str(&stdout)
        .map_err(|e| {
            let preview = &stdout[..stdout.len().min(500)];
            eprintln!("[pipeline:analyze_bottlenecks] ❌ JSON parse failed: {}", e);
            eprintln!("[pipeline:analyze_bottlenecks] stdout preview: {}", preview);
            ResumeParseError::InvalidJson(format!(
                "Stage: analyze_bottlenecks | JSON error: {} | stdout length: {} bytes",
                e, stdout.len()
            ))
        })?;

    eprintln!("[pipeline:analyze_bottlenecks] ✅ SUCCESS");
    eprintln!("[pipeline:analyze_bottlenecks] ========== STAGE END ==========");
    Ok(analysis)
}

/// Select strategy from bottleneck analysis (Stage 3)
///
/// # Arguments
/// * `analysis` - Bottleneck analysis from Stage 2
/// * `evidence` - Evidence map from Stage 1 (for section_signals)
/// * `config` - Parser configuration
///
/// # Returns
/// * `Ok(StrategySelection)` - Selected strategy with action and confidence
pub fn select_strategy(
    analysis: &BottleneckAnalysis,
    evidence: &EvidenceMap,
    config: &ResumeParserConfig,
) -> Result<StrategySelection, ResumeParseError> {
    let selector_script_rel = config.script_path.replace("parser.py", "strategy_selector.py");
    
    // Resolve paths
    let python_exe = ResumeParserConfig::resolve_path(&config.python_path)?;
    let selector_script = ResumeParserConfig::resolve_path(&selector_script_rel)?;
    
    if !selector_script.exists() {
        return Err(ResumeParseError::ScriptNotFound(selector_script.display().to_string()));
    }

    // Serialize inputs to JSON - combine both stages into one object for stdin
    let combined_input = serde_json::json!({
        "stage2": analysis,
        "stage1": evidence
    });
    let input_json = serde_json::to_string(&combined_input)
        .map_err(|e| ResumeParseError::InvalidJson(e.to_string()))?;

    // ===== PIPELINE STAGE: select_strategy =====
    eprintln!("[pipeline:select_strategy] ========== STAGE START ==========");
    eprintln!("[pipeline:select_strategy] Python: {}", python_exe.display());
    eprintln!("[pipeline:select_strategy] Script: {}", selector_script.display());
    eprintln!("[pipeline:select_strategy] Combined JSON length: {} bytes (via stdin)", input_json.len());

    // Execute Python strategy selector with JSON via stdin
    use std::process::Stdio;
    use std::io::Write;
    
    let mut child = Command::new(&python_exe)
        .arg(&selector_script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            eprintln!("[pipeline:select_strategy] ❌ FAILED to spawn process: {}", e);
            ResumeParseError::ExecutionFailed(format!("Stage: select_strategy | Error: {}", e))
        })?;
    
    // Write JSON to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input_json.as_bytes())
            .map_err(|e| {
                eprintln!("[pipeline:select_strategy] ❌ FAILED to write to stdin: {}", e);
                ResumeParseError::ExecutionFailed(format!("Stage: select_strategy | stdin write error: {}", e))
            })?;
        drop(stdin); // Close stdin to signal EOF
    } else {
        eprintln!("[pipeline:select_strategy] ❌ FAILED to get stdin handle");
        return Err(ResumeParseError::ExecutionFailed("Stage: select_strategy | No stdin handle".to_string()));
    }
    
    // Wait for process and capture output
    let output = child.wait_with_output()
        .map_err(|e| {
            eprintln!("[pipeline:select_strategy] ❌ FAILED to wait for process: {}", e);
            ResumeParseError::ExecutionFailed(format!("Stage: select_strategy | Wait error: {}", e))
        })?;

    // Get stdout and stderr
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code();
    
    eprintln!("[pipeline:select_strategy] Exit code: {:?}", exit_code);
    eprintln!("[pipeline:select_strategy] stdout length: {} bytes", stdout.len());
    if !stderr.is_empty() {
        eprintln!("[pipeline:select_strategy] stderr: {}", stderr);
    }
    
    if exit_code != Some(0) {
        eprintln!("[pipeline:select_strategy] ❌ Non-zero exit code");
        return Err(ResumeParseError::ExecutionFailed(format!(
            "Stage: select_strategy | Exit code: {:?} | stderr: {}",
            exit_code, stderr
        )));
    }

    // Check if it's an error response
    if let Ok(error) = serde_json::from_str::<ParserError>(&stdout) {
        eprintln!("[pipeline:select_strategy] ❌ Parser error: {}", error.error);
        return Err(ResumeParseError::ParserError(format!("Stage: select_strategy | {}", error.error)));
    }

    // Parse successful response
    let selection: StrategySelection = serde_json::from_str(&stdout)
        .map_err(|e| {
            let preview = &stdout[..stdout.len().min(500)];
            eprintln!("[pipeline:select_strategy] ❌ JSON parse failed: {}", e);
            eprintln!("[pipeline:select_strategy] stdout preview: {}", preview);
            ResumeParseError::InvalidJson(format!(
                "Stage: select_strategy | JSON error: {} | stdout length: {} bytes",
                e, stdout.len()
            ))
        })?;

    eprintln!("[pipeline:select_strategy] ✅ SUCCESS");
    eprintln!("[pipeline:select_strategy] ========== STAGE END ==========");
    Ok(selection)
}

/// Full pipeline: parse, map evidence, analyze bottlenecks, select strategy
///
/// # Arguments
/// * `resume_path` - Path to the PDF or DOCX resume file
/// * `config` - Parser configuration
///
/// # Returns
/// * `Ok((ParsedResume, EvidenceMap, BottleneckAnalysis, StrategySelection))` - All stages
pub fn full_pipeline(
    resume_path: &str,
    config: &ResumeParserConfig,
) -> Result<(ParsedResume, EvidenceMap, BottleneckAnalysis, StrategySelection), ResumeParseError> {
    let parsed = parse_resume(resume_path, config)?;
    let evidence = map_evidence(&parsed, config)?;
    let analysis = analyze_bottlenecks(&evidence, config)?;
    let strategy = select_strategy(&analysis, &evidence, config)?;
    Ok((parsed, evidence, analysis, strategy))
}

// =============================================================================
// AGENT LOOP FUNCTIONS
// =============================================================================

/// Initialize an agent session from stage outputs
///
/// Creates a new session ready to receive outcome feedback.
pub fn initialize_session(
    evidence: EvidenceMap,
    bottleneck: BottleneckAnalysis,
    strategy: StrategySelection,
) -> AgentSession {
    let strategy_record = StrategyRecord {
        strategy: format!("{:?}", strategy.strategy),
        initial_confidence: strategy.confidence,
        current_confidence: strategy.confidence,
        outcomes: vec![],
        failed: false,
        strategy_state: "explore".to_string(),  // Initial state is always EXPLORE
    };
    
    let explanation = format!(
        "Agent initialized. Selected '{:?}' strategy due to dominant issue: {}. Initial confidence: {}.",
        strategy.strategy,
        bottleneck.dominant_issue.as_deref().unwrap_or("none identified"),
        strategy.confidence
    );
    
    AgentSession {
        stage1_evidence: evidence,
        stage2_bottleneck: bottleneck,
        stage3_strategy: strategy,
        current_strategy: Some(strategy_record),
        strategy_history: vec![],
        loop_iteration: 0,
        explanation_log: vec![explanation],
    }
}

/// Process an outcome through the agent loop (calls Python)
///
/// # Arguments
/// * `session` - Current agent session (serialized)
/// * `outcome` - One of: "no_response", "rejected", "interview"
/// * `config` - Parser configuration
///
/// # Returns
/// Updated session with potential strategy change
pub fn process_outcome(
    session: &AgentSession,
    outcome: &str,
    config: &ResumeParserConfig,
) -> Result<OutcomeResult, ResumeParseError> {
    // Explicitly use correct relative path from backend/ directory
    let loop_script_rel = "../resume_parser/agent_loop.py";
    
    // Resolve paths
    let python_exe = ResumeParserConfig::resolve_path(&config.python_path)?;
    let loop_script = ResumeParserConfig::resolve_path(loop_script_rel)?;
    
    eprintln!("[pipeline:process_outcome] Resolved paths:");
    eprintln!("[pipeline:process_outcome]   Python: {}", python_exe.display());
    eprintln!("[pipeline:process_outcome]   Script: {}", loop_script.display());
    
    if !loop_script.exists() {
        return Err(ResumeParseError::ScriptNotFound(loop_script.display().to_string()));
    }

    // Write session to temp file (PowerShell JSON escaping is problematic)
    let session_json = serde_json::to_string(session)
        .map_err(|e| ResumeParseError::InvalidJson(e.to_string()))?;
    
    let temp_path = std::env::temp_dir().join("agent_session.json");
    std::fs::write(&temp_path, &session_json)
        .map_err(|e| ResumeParseError::ExecutionFailed(e.to_string()))?;

    // Execute Python agent loop
    let output = Command::new(&python_exe)
        .arg(&loop_script)
        .arg("outcome")
        .arg(&temp_path)
        .arg(outcome)
        .output()
        .map_err(|e| ResumeParseError::ExecutionFailed(e.to_string()))?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    // Get stdout
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    // Check if it's an error response
    if let Ok(error) = serde_json::from_str::<ParserError>(&stdout) {
        return Err(ResumeParseError::ParserError(error.error));
    }

    // Parse successful response
    let result: OutcomeResult = serde_json::from_str(&stdout)
        .map_err(|e| {
            let preview = &stdout[..stdout.len().min(500)];
            eprintln!("[process_outcome] JSON parse error: {} | stdout length: {} bytes", e, stdout.len());
            ResumeParseError::InvalidJson(format!("JSON parse failed: {} | stdout length: {} bytes | preview: {}", e, stdout.len(), preview))
        })?;

    Ok(result)
}

/// Generate a full explanation for demo purposes
pub fn generate_explanation(
    session: &AgentSession,
    config: &ResumeParserConfig,
) -> Result<String, ResumeParseError> {
    // Explicitly use correct relative path from backend/ directory
    let loop_script_rel = "../resume_parser/agent_loop.py";
    
    // Resolve paths
    let python_exe = ResumeParserConfig::resolve_path(&config.python_path)?;
    let loop_script = ResumeParserConfig::resolve_path(loop_script_rel)?;
    
    eprintln!("[pipeline:generate_explanation] Resolved paths:");
    eprintln!("[pipeline:generate_explanation]   Python: {}", python_exe.display());
    eprintln!("[pipeline:generate_explanation]   Script: {}", loop_script.display());
    
    if !loop_script.exists() {
        return Err(ResumeParseError::ScriptNotFound(loop_script.display().to_string()));
    }

    // Write session to temp file
    let session_json = serde_json::to_string(session)
        .map_err(|e| ResumeParseError::InvalidJson(e.to_string()))?;
    
    let temp_path = std::env::temp_dir().join("agent_session_explain.json");
    std::fs::write(&temp_path, &session_json)
        .map_err(|e| ResumeParseError::ExecutionFailed(e.to_string()))?;

    // Execute Python agent loop
    let output = Command::new(&python_exe)
        .arg(&loop_script)
        .arg("explain")
        .arg(&temp_path)
        .output()
        .map_err(|e| ResumeParseError::ExecutionFailed(e.to_string()))?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    // Get stdout
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    // Check if it's an error response
    if let Ok(error) = serde_json::from_str::<ParserError>(&stdout) {
        return Err(ResumeParseError::ParserError(error.error));
    }

    // Parse successful response
    let explanation: Explanation = serde_json::from_str(&stdout)
        .map_err(|e| {
            let preview = &stdout[..stdout.len().min(500)];
            eprintln!("[explain_session] JSON parse error: {} | stdout length: {} bytes", e, stdout.len());
            ResumeParseError::InvalidJson(format!("JSON parse failed: {} | stdout length: {} bytes | preview: {}", e, stdout.len(), preview))
        })?;

    Ok(explanation.explanation)
}

/// Convert ParsedResume to the internal ResumeData type
impl ParsedResume {
    pub fn to_resume_data(&self, user_id: &str) -> super::types::ResumeData {
        super::types::ResumeData {
            user_id: user_id.to_string(),
            name: None, // Not extracted by pyresparser
            email: None,
            current_role: self
                .experience
                .first()
                .and_then(|e| e.role.clone()),
            years_experience: self.total_experience.map(|y| y as u32),
            skills: self.skills.clone(),
            education: self
                .education
                .iter()
                .filter_map(|e| {
                    Some(super::types::EducationEntry {
                        institution: e.institution.clone().unwrap_or_default(),
                        degree: e.degree.clone().unwrap_or_default(),
                        field: None,
                        year: None,
                    })
                })
                .collect(),
            experience: self
                .experience
                .iter()
                .map(|e| super::types::ExperienceEntry {
                    company: e.company.clone(),
                    role: e.role.clone().unwrap_or_default(),
                    duration: None,
                    description: None,
                })
                .collect(),
            raw_text: self.raw_text.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_output() {
        let json = r#"{
            "skills": ["Python", "Rust", "JavaScript"],
            "experience": [{"company": "Tech Corp", "role": "Engineer"}],
            "education": [{"degree": "BS", "institution": "MIT"}],
            "total_experience": 5.0,
            "raw_text": "John Doe Resume..."
        }"#;

        let parsed: ParsedResume = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.skills.len(), 3);
        assert_eq!(parsed.experience[0].company, "Tech Corp");
        assert_eq!(parsed.total_experience, Some(5.0));
    }

    #[test]
    fn test_error_json_output() {
        let json = r#"{"error": "File not found"}"#;
        let error: ParserError = serde_json::from_str(json).unwrap();
        assert_eq!(error.error, "File not found");
    }

    #[test]
    fn test_to_resume_data() {
        let parsed = ParsedResume {
            skills: vec!["Python".to_string()],
            experience: vec![ParsedExperience {
                company: "Acme".to_string(),
                role: Some("Dev".to_string()),
            }],
            education: vec![ParsedEducation {
                degree: Some("MS".to_string()),
                institution: Some("Stanford".to_string()),
            }],
            total_experience: Some(3.5),
            raw_text: "test".to_string(),
        };

        let data = parsed.to_resume_data("user123");
        assert_eq!(data.user_id, "user123");
        assert_eq!(data.skills, vec!["Python"]);
        assert_eq!(data.years_experience, Some(3));
        assert_eq!(data.current_role, Some("Dev".to_string()));
    }

    #[test]
    fn test_evidence_map_json() {
        let json = r#"{
            "normalized_skills": ["Python", "SQL", "Java"],
            "skill_evidence_map": {
                "Python": ["internship", "project"],
                "SQL": ["coursework"],
                "Java": ["listed_only"]
            },
            "section_signals": {
                "has_projects": true,
                "has_internship": true,
                "has_metrics": false,
                "has_deployment": false
            }
        }"#;

        let evidence: EvidenceMap = serde_json::from_str(json).unwrap();
        assert_eq!(evidence.normalized_skills.len(), 3);
        assert_eq!(evidence.skill_evidence_map.get("Python").unwrap(), &vec!["internship", "project"]);
        assert!(evidence.section_signals.has_projects);
        assert!(!evidence.section_signals.has_deployment);
    }

    #[test]
    fn test_bottleneck_analysis_json() {
        let json = r#"{
            "implied_role": "Data Analyst",
            "bottlenecks": {
                "positioning": "weak",
                "evidence_depth": "strong",
                "experience_strength": "missing",
                "skill_alignment": "strong",
                "outcome_visibility": "weak"
            },
            "dominant_issue": "experience_strength",
            "justification": "No internship or work experience evidence found in resume signals."
        }"#;

        let analysis: BottleneckAnalysis = serde_json::from_str(json).unwrap();
        assert_eq!(analysis.implied_role, "Data Analyst");
        assert_eq!(analysis.bottlenecks.positioning, "weak");
        assert_eq!(analysis.bottlenecks.experience_strength, "missing");
        assert_eq!(analysis.dominant_issue, Some("experience_strength".to_string()));
        assert!(analysis.justification.contains("internship"));
    }

    #[test]
    fn test_bottleneck_all_strong() {
        let json = r#"{
            "implied_role": "Software Engineer",
            "bottlenecks": {
                "positioning": "strong",
                "evidence_depth": "strong",
                "experience_strength": "strong",
                "skill_alignment": "strong",
                "outcome_visibility": "strong"
            },
            "dominant_issue": null,
            "justification": "All bottleneck categories are strong."
        }"#;

        let analysis: BottleneckAnalysis = serde_json::from_str(json).unwrap();
        assert_eq!(analysis.implied_role, "Software Engineer");
        assert_eq!(analysis.dominant_issue, None);
    }

    #[test]
    fn test_strategy_selection_json() {
        let json = r#"{
            "strategy": "ResumeOptimization",
            "action": "Rewrite the primary project description to include problem statement, approach, tools used, and quantifiable outcome.",
            "confidence": 0.70
        }"#;

        let selection: StrategySelection = serde_json::from_str(json).unwrap();
        assert_eq!(selection.strategy, Strategy::ResumeOptimization);
        assert!(selection.action.contains("project"));
        assert!((selection.confidence - 0.70).abs() < 0.01);
    }

    #[test]
    fn test_strategy_skill_gap_patch() {
        let json = r#"{
            "strategy": "SkillGapPatch",
            "action": "Identify the top missing primary skill for the target role and add it through a focused micro-project or certification.",
            "confidence": 0.55
        }"#;

        let selection: StrategySelection = serde_json::from_str(json).unwrap();
        assert_eq!(selection.strategy, Strategy::SkillGapPatch);
        assert!(selection.confidence < 0.60);
    }

    #[test]
    fn test_strategy_hold_position() {
        let json = r#"{
            "strategy": "HoldPosition",
            "action": "Maintain current resume positioning and proceed to application phase.",
            "confidence": 0.85
        }"#;

        let selection: StrategySelection = serde_json::from_str(json).unwrap();
        assert_eq!(selection.strategy, Strategy::HoldPosition);
        assert!(selection.confidence > 0.80);
    }

    #[test]
    fn test_strategy_role_shift() {
        let json = r#"{
            "strategy": "RoleShift",
            "action": "Pivot target role to one that values project-based evidence over formal work experience.",
            "confidence": 0.45
        }"#;

        let selection: StrategySelection = serde_json::from_str(json).unwrap();
        assert_eq!(selection.strategy, Strategy::RoleShift);
        assert!(selection.confidence < 0.50);
    }

    #[test]
    fn test_strategy_record_json() {
        let json = r#"{
            "strategy": "ResumeOptimization",
            "initial_confidence": 0.70,
            "current_confidence": 0.52,
            "outcomes": ["no_response", "rejected"],
            "failed": false
        }"#;

        let record: StrategyRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.strategy, "ResumeOptimization");
        assert_eq!(record.outcomes.len(), 2);
        assert!((record.current_confidence - 0.52).abs() < 0.01);
        assert!(!record.failed);
    }

    #[test]
    fn test_strategy_record_failed() {
        let json = r#"{
            "strategy": "SkillGapPatch",
            "initial_confidence": 0.55,
            "current_confidence": 0.27,
            "outcomes": ["rejected", "rejected", "no_response"],
            "failed": true
        }"#;

        let record: StrategyRecord = serde_json::from_str(json).unwrap();
        assert!(record.failed);
        assert!(record.current_confidence < 0.30);
    }

    #[test]
    fn test_outcome_result_json() {
        let json = r#"{
            "session": {
                "stage1_evidence": {
                    "normalized_skills": ["Python"],
                    "skill_evidence_map": {},
                    "section_signals": {
                        "has_projects": true,
                        "has_internship": false,
                        "has_metrics": false,
                        "has_deployment": false
                    }
                },
                "stage2_bottleneck": {
                    "implied_role": "Data Analyst",
                    "bottlenecks": {
                        "positioning": "weak",
                        "evidence_depth": "weak",
                        "experience_strength": "missing",
                        "skill_alignment": "strong",
                        "outcome_visibility": "weak"
                    },
                    "dominant_issue": "experience_strength",
                    "justification": "No internship found."
                },
                "stage3_strategy": {
                    "strategy": "RoleShift",
                    "action": "Adjust target role.",
                    "confidence": 0.35
                },
                "current_strategy": {
                    "strategy": "RoleShift",
                    "initial_confidence": 0.45,
                    "current_confidence": 0.35,
                    "outcomes": ["rejected"],
                    "failed": false
                },
                "strategy_history": [],
                "loop_iteration": 0,
                "explanation_log": ["Agent initialized."]
            },
            "strategy_changed": false,
            "current_strategy": {
                "strategy": "RoleShift",
                "action": "Adjust target role.",
                "confidence": 0.35
            },
            "explanation": "Selected RoleShift due to experience_strength."
        }"#;

        let result: OutcomeResult = serde_json::from_str(json).unwrap();
        assert!(!result.strategy_changed);
        assert_eq!(result.session.loop_iteration, 0);
        assert!(result.explanation.contains("RoleShift"));
    }

    #[test]
    fn test_explanation_json() {
        let json = r#"{
            "explanation": "The agent initially selected ResumeOptimization. After two rejections, confidence dropped below threshold."
        }"#;

        let exp: Explanation = serde_json::from_str(json).unwrap();
        assert!(exp.explanation.contains("ResumeOptimization"));
        assert!(exp.explanation.contains("rejections"));
    }
}
