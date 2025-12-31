//! Web API Module
//!
//! Exposes RESTful endpoints for the Career Assistant frontend.
//! All endpoints return JSON and require no authentication (prototype mode).

use crate::agent::{
    memory::{self, MemoryStore, MemoryEvent, MemoryEventType},
    planner::{self, CareerPlanner, CareerRoadmap, PlannerConfig, PlannerInput, RoadmapEdit},
    reflection::{ReflectionGenerator, ReflectionConfig, ReflectionStore},
    types::{CareerGoal, CareerRule, ResumeData},
};
use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

// ============================================================
// APPLICATION STATE
// ============================================================

/// Shared application state
pub struct AppState {
    pub memory_store: MemoryStore,
    pub reflection_store: ReflectionStore,
    pub roadmaps: Mutex<std::collections::HashMap<String, CareerRoadmap>>,
    pub resumes: Mutex<std::collections::HashMap<String, ResumeData>>,
    pub goals: Mutex<std::collections::HashMap<String, CareerGoal>>,
    /// Default career rules (skills/milestones)
    pub career_rules: Vec<CareerRule>,
}

impl AppState {
    pub fn new() -> Result<Self, rusqlite::Error> {
        Ok(Self {
            memory_store: MemoryStore::in_memory()?,
            reflection_store: ReflectionStore::new(),
            roadmaps: Mutex::new(std::collections::HashMap::new()),
            resumes: Mutex::new(std::collections::HashMap::new()),
            goals: Mutex::new(std::collections::HashMap::new()),
            career_rules: default_career_rules(),
        })
    }
}

/// Default career rules for demo
fn default_career_rules() -> Vec<CareerRule> {
    vec![
        CareerRule {
            id: "programming_fundamentals".to_string(),
            title: "Programming Fundamentals".to_string(),
            description: "Master core programming concepts: variables, loops, functions, data structures".to_string(),
            category: "technical_skill".to_string(),
            priority: "critical".to_string(),
            estimated_weeks: Some(4),
            prerequisites: vec![],
            tags: vec!["programming".to_string(), "foundational".to_string()],
        },
        CareerRule {
            id: "version_control".to_string(),
            title: "Version Control with Git".to_string(),
            description: "Learn Git for code versioning and collaboration".to_string(),
            category: "technical_skill".to_string(),
            priority: "high".to_string(),
            estimated_weeks: Some(2),
            prerequisites: vec!["programming_fundamentals".to_string()],
            tags: vec!["git".to_string(), "collaboration".to_string()],
        },
        CareerRule {
            id: "web_development".to_string(),
            title: "Web Development Basics".to_string(),
            description: "Learn HTML, CSS, and JavaScript fundamentals".to_string(),
            category: "technical_skill".to_string(),
            priority: "high".to_string(),
            estimated_weeks: Some(6),
            prerequisites: vec!["programming_fundamentals".to_string()],
            tags: vec!["web".to_string(), "frontend".to_string()],
        },
        CareerRule {
            id: "api_design".to_string(),
            title: "API Design & REST".to_string(),
            description: "Understand RESTful API design principles".to_string(),
            category: "technical_skill".to_string(),
            priority: "high".to_string(),
            estimated_weeks: Some(3),
            prerequisites: vec!["web_development".to_string()],
            tags: vec!["api".to_string(), "backend".to_string()],
        },
        CareerRule {
            id: "database_fundamentals".to_string(),
            title: "Database Fundamentals".to_string(),
            description: "Learn SQL and database design principles".to_string(),
            category: "technical_skill".to_string(),
            priority: "high".to_string(),
            estimated_weeks: Some(4),
            prerequisites: vec!["programming_fundamentals".to_string()],
            tags: vec!["database".to_string(), "sql".to_string()],
        },
        CareerRule {
            id: "communication_skills".to_string(),
            title: "Technical Communication".to_string(),
            description: "Develop skills in documentation and technical writing".to_string(),
            category: "soft_skill".to_string(),
            priority: "medium".to_string(),
            estimated_weeks: Some(4),
            prerequisites: vec![],
            tags: vec!["communication".to_string(), "documentation".to_string()],
        },
        CareerRule {
            id: "problem_solving".to_string(),
            title: "Problem-Solving & Algorithms".to_string(),
            description: "Practice algorithmic thinking and problem-solving".to_string(),
            category: "technical_skill".to_string(),
            priority: "high".to_string(),
            estimated_weeks: Some(8),
            prerequisites: vec!["programming_fundamentals".to_string()],
            tags: vec!["algorithms".to_string(), "interview".to_string()],
        },
        CareerRule {
            id: "portfolio_project".to_string(),
            title: "Build Portfolio Project".to_string(),
            description: "Create a substantial project to showcase your skills".to_string(),
            category: "experience".to_string(),
            priority: "critical".to_string(),
            estimated_weeks: Some(6),
            prerequisites: vec!["web_development".to_string(), "version_control".to_string()],
            tags: vec!["portfolio".to_string(), "project".to_string()],
        },
        CareerRule {
            id: "networking".to_string(),
            title: "Professional Networking".to_string(),
            description: "Build connections in the tech industry".to_string(),
            category: "soft_skill".to_string(),
            priority: "medium".to_string(),
            estimated_weeks: Some(4),
            prerequisites: vec![],
            tags: vec!["networking".to_string(), "career".to_string()],
        },
        CareerRule {
            id: "interview_prep".to_string(),
            title: "Interview Preparation".to_string(),
            description: "Practice technical and behavioral interviews".to_string(),
            category: "experience".to_string(),
            priority: "critical".to_string(),
            estimated_weeks: Some(4),
            prerequisites: vec!["problem_solving".to_string(), "portfolio_project".to_string()],
            tags: vec!["interview".to_string(), "job_search".to_string()],
        },
    ]
}

// ============================================================
// API REQUEST/RESPONSE TYPES
// ============================================================

#[derive(Deserialize)]
pub struct UploadResumeRequest {
    pub user_id: String,
    pub name: Option<String>,
    pub current_role: Option<String>,
    pub skills: Vec<String>,
    pub years_experience: Option<u32>,
    pub raw_text: Option<String>,
}

#[derive(Deserialize)]
pub struct SetGoalRequest {
    pub user_id: String,
    pub title: String,
    pub target_role: Option<String>,
    pub timeline_months: Option<u32>,
}

#[derive(Deserialize)]
pub struct GenerateRoadmapRequest {
    pub user_id: String,
    pub session: Option<serde_json::Value>,  // Optional agent session for state-gating
}

#[derive(Deserialize)]
pub struct EditRoadmapRequest {
    pub user_id: String,
    pub edit: RoadmapEdit,
}

#[derive(Deserialize)]
pub struct OutcomeRequest {
    pub session: serde_json::Value,
    pub outcome: String,
}

#[derive(Serialize)]
pub struct OutcomeResponse {
    pub strategy: String,
    pub action: String,
    pub confidence: f64,
    pub strategy_changed: bool,
    pub session: serde_json::Value,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: &str) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message.to_string()),
        }
    }
}

// ============================================================
// API HANDLERS
// ============================================================

/// Health check endpoint
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "Career Agent API",
        "version": "0.1.0"
    }))
}

/// Upload/update resume (SENSE phase)
async fn upload_resume(
    data: web::Data<Arc<AppState>>,
    req: web::Json<UploadResumeRequest>,
) -> impl Responder {
    let resume = ResumeData {
        user_id: req.user_id.clone(),
        name: req.name.clone(),
        email: None,
        current_role: req.current_role.clone(),
        years_experience: req.years_experience,
        skills: req.skills.clone(),
        education: vec![],
        experience: vec![],
        raw_text: req.raw_text.clone().unwrap_or_default(),
    };

    // Store resume
    {
        let mut resumes = data.resumes.lock().unwrap();
        resumes.insert(req.user_id.clone(), resume.clone());
    }

    // Record in memory
    let _ = memory::record_resume_upload(&data.memory_store, &req.user_id, "resume_data");

    HttpResponse::Ok().json(ApiResponse::success(resume))
}

/// Set career goal
async fn set_goal(
    data: web::Data<Arc<AppState>>,
    req: web::Json<SetGoalRequest>,
) -> impl Responder {
    let mut goal = CareerGoal::new(&req.user_id, &req.title);
    goal.target_role = req.target_role.clone();
    goal.timeline_months = req.timeline_months;

    // Store goal
    {
        let mut goals = data.goals.lock().unwrap();
        goals.insert(req.user_id.clone(), goal.clone());
    }

    // Record in memory
    let event = MemoryEvent::new(
        &req.user_id,
        MemoryEventType::GoalSet,
        &format!("Set career goal: {}", req.title),
    );
    let _ = data.memory_store.record_event(&event);

    HttpResponse::Ok().json(ApiResponse::success(goal))
}

/// Generate career roadmap (PLAN phase) - STATE-GATED
async fn generate_roadmap(
    data: web::Data<Arc<AppState>>,
    req: web::Json<GenerateRoadmapRequest>,
) -> impl Responder {
    use crate::agent::resume_parser::ResumeParserConfig;
    use std::process::{Command, Stdio};
    use std::io::Write;

    // Check if session is provided for state-gating
    if let Some(session_json) = &req.session {
        // Call Python roadmap_generator.py with session
        let config = ResumeParserConfig::default();
        let script_path = std::path::Path::new("resume_parser/roadmap_generator.py");
        
        if !script_path.exists() {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Roadmap generator script not found"));
        }

        // Prepare input JSON
        let input = json!({
            "session": session_json,
            "enhanced_snapshot": {},  // TODO: Get from session if available
            "profile_signals": {}     // TODO: Get from session if available
        });

        // Execute Python script
        let mut child = match Command::new(&config.python_path)
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error(&format!("Failed to execute Python: {}", e)));
            }
        };

        // Write input to stdin
        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(input.to_string().as_bytes()) {
                return HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error(&format!("Failed to write to Python stdin: {}", e)));
            }
        }

        // Wait for output
        let output = match child.wait_with_output() {
            Ok(o) => o,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error(&format!("Failed to get Python output: {}", e)));
            }
        };

        // Check for errors
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            // Check for eligibility errors (BLOCKED state)
            if stderr.contains("not eligible") || stderr.contains("BLOCKED") {
                // Parse error JSON if available
                let error_msg = if let Ok(error_json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                    if let Some(reason) = error_json.get("reason") {
                        reason.as_str().unwrap_or("Strategy not in EXECUTE state").to_string()
                    } else {
                        stderr.to_string()
                    }
                } else {
                    stderr.to_string()
                };
                
                // Log blocked attempt
                println!("⚠️  Roadmap generation blocked for user {}: {}", req.user_id, error_msg);
                
                return HttpResponse::Forbidden()
                    .json(ApiResponse::<()>::error(&error_msg));
            }
            
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("Roadmap generation failed: {}", stderr)));
        }

        // Parse roadmap from stdout
        let roadmap_json: serde_json::Value = match serde_json::from_slice(&output.stdout) {
            Ok(j) => j,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::error(&format!("Failed to parse roadmap JSON: {}", e)));
            }
        };

        // Log successful generation
        println!("✅ Roadmap generated for user {} (strategy in EXECUTE state)", req.user_id);

        return HttpResponse::Ok().json(ApiResponse::success(roadmap_json));
    }

    // Fallback: Use old Rust planner if no session provided
    // Get resume
    let resume = {
        let resumes = data.resumes.lock().unwrap();
        match resumes.get(&req.user_id) {
            Some(r) => r.clone(),
            None => {
                return HttpResponse::BadRequest()
                    .json(ApiResponse::<()>::error("Resume not found. Please upload resume first."));
            }
        }
    };

    // Get goal
    let goal = {
        let goals = data.goals.lock().unwrap();
        match goals.get(&req.user_id) {
            Some(g) => g.clone(),
            None => CareerGoal::new(&req.user_id, "General Career Development"),
        }
    };

    // Build planner input
    let input = PlannerInput {
        resume,
        assessments: vec![],
        goal,
        available_rules: data.career_rules.clone(),
        deferred_steps: HashSet::new(),
    };

    // Generate roadmap (old way - not state-gated)
    let planner = CareerPlanner::new(PlannerConfig::default());
    let roadmap = planner.generate_roadmap(&input);

    // Store roadmap
    {
        let mut roadmaps = data.roadmaps.lock().unwrap();
        roadmaps.insert(req.user_id.clone(), roadmap.clone());
    }

    // Record in memory
    let _ = memory::record_plan_generated(&data.memory_store, &req.user_id, roadmap.steps.len());

    HttpResponse::Ok().json(ApiResponse::success(roadmap))
}

/// Get current roadmap
async fn get_roadmap(
    data: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> impl Responder {
    let user_id = path.into_inner();
    
    let roadmaps = data.roadmaps.lock().unwrap();
    match roadmaps.get(&user_id) {
        Some(roadmap) => HttpResponse::Ok().json(ApiResponse::success(roadmap.clone())),
        None => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("Roadmap not found. Generate one first.")),
    }
}

/// Edit roadmap (HUMAN-IN-THE-LOOP)
async fn edit_roadmap(
    data: web::Data<Arc<AppState>>,
    req: web::Json<EditRoadmapRequest>,
) -> impl Responder {
    let mut roadmaps = data.roadmaps.lock().unwrap();
    
    match roadmaps.get_mut(&req.user_id) {
        Some(roadmap) => {
            match planner::apply_edit(roadmap, req.edit.clone()) {
                Ok(description) => {
                    // Record in memory
                    let _ = memory::record_plan_modified(
                        &data.memory_store,
                        &req.user_id,
                        &description,
                    );
                    HttpResponse::Ok().json(ApiResponse::success(roadmap.clone()))
                }
                Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
            }
        }
        None => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("Roadmap not found")),
    }
}

/// Get agent memory timeline
async fn get_memory(
    data: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> impl Responder {
    let user_id = path.into_inner();
    
    match data.memory_store.get_user_memory(&user_id) {
        Ok(memory) => HttpResponse::Ok().json(ApiResponse::success(memory)),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Database error: {}", e))),
    }
}

/// Get latest weekly reflection
async fn get_latest_reflection(
    data: web::Data<Arc<AppState>>,
    path: web::Path<String>,
) -> impl Responder {
    let user_id = path.into_inner();
    
    // Get memory
    let memory = match data.memory_store.get_user_memory(&user_id) {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("Database error: {}", e)));
        }
    };

    // Get roadmap
    let roadmap = {
        let roadmaps = data.roadmaps.lock().unwrap();
        match roadmaps.get(&user_id) {
            Some(r) => r.clone(),
            None => {
                return HttpResponse::NotFound()
                    .json(ApiResponse::<()>::error("Roadmap not found. Generate one first."));
            }
        }
    };

    // Generate reflection
    let generator = ReflectionGenerator::new(ReflectionConfig::default());
    let reflection = generator.generate_weekly_reflection(&memory, &roadmap);

    // Store and record
    data.reflection_store.save_reflection(reflection.clone());
    let _ = memory::record_reflection(&data.memory_store, &user_id, &reflection.summary);

    HttpResponse::Ok().json(ApiResponse::success(reflection))
}

/// Get all available career rules
async fn get_career_rules(data: web::Data<Arc<AppState>>) -> impl Responder {
    HttpResponse::Ok().json(ApiResponse::success(data.career_rules.clone()))
}

/// Mark a step as completed
async fn complete_step(
    data: web::Data<Arc<AppState>>,
    path: web::Path<(String, String)>,
) -> impl Responder {
    let (user_id, step_id) = path.into_inner();
    
    let mut roadmaps = data.roadmaps.lock().unwrap();
    match roadmaps.get_mut(&user_id) {
        Some(roadmap) => {
            let edit = RoadmapEdit::CompleteStep { step_id };
            match planner::apply_edit(roadmap, edit) {
                Ok(description) => {
                    let _ = memory::record_step_completed(&data.memory_store, &user_id, &description);
                    HttpResponse::Ok().json(ApiResponse::success(roadmap.clone()))
                }
                Err(e) => HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e)),
            }
        }
        None => HttpResponse::NotFound().json(ApiResponse::<()>::error("Roadmap not found")),
    }
}

/// Analyze resume and create agent session (full pipeline)
async fn analyze_resume(
    req: web::Json<UploadResumeRequest>,
) -> impl Responder {
    use crate::agent::resume_parser::{
        ResumeParserConfig, full_pipeline, initialize_session,
    };
    use std::io::Write;

    println!("[analyze_resume] Starting analysis for user: {}", req.user_id);

    // Get raw text from request
    let raw_text = req.raw_text.clone().unwrap_or_default();
    
    if raw_text.is_empty() {
        println!("[analyze_resume] ERROR: No resume text provided");
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("No resume text provided"));
    }

    println!("[analyze_resume] Received {} bytes of text", raw_text.len());

    // Write to temp file for Python parser
    // Use tempfile to generate short, safe filenames (avoids Windows MAX_PATH issues)
    let mut temp_file = match tempfile::Builder::new()
        .prefix("resume_")
        .suffix(".txt")
        .tempfile()
    {
        Ok(f) => f,
        Err(e) => {
            println!("[analyze_resume] ERROR: Failed to create temp file: {}", e);
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("Failed to create temp file: {}", e)));
        }
    };
    
    let temp_path = temp_file.path().to_path_buf();
    println!("[analyze_resume] Writing to temp file: {:?}", temp_path);
    
    if let Err(e) = temp_file.write_all(raw_text.as_bytes()) {
        println!("[analyze_resume] ERROR: Failed to write temp file: {}", e);
        return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed to write temp file: {}", e)));
    }

    // Configure parser with correct paths
    // The server runs from backend/, scripts are in ../resume_parser/
    // Use relative paths that will be resolved by ResumeParserConfig::resolve_path()
    let config = ResumeParserConfig {
        python_path: "../resume_parser/venv/Scripts/python.exe".to_string(),
        script_path: "../resume_parser/parser.py".to_string(),
    };
    
    eprintln!("[analyze_resume] Config paths (relative):");
    eprintln!("  Python: {}", config.python_path);
    eprintln!("  Script: {}", config.script_path);
    eprintln!("  Current dir: {:?}", std::env::current_dir());

    // Run full pipeline
    // Note: temp_file is kept alive until pipeline completes, then auto-deleted on drop
    println!("[analyze_resume] Running full pipeline...");
    match full_pipeline(temp_path.to_str().unwrap_or(""), &config) {
        Ok((parsed, evidence, bottleneck, strategy)) => {
            println!("[analyze_resume] Pipeline succeeded!");
            println!("[analyze_resume] Strategy: {:?}", strategy.strategy);
            
            // temp_file auto-deletes on drop

            // Initialize agent session
            let session = initialize_session(evidence, bottleneck, strategy);
            println!("[analyze_resume] Session initialized");

            // Return session
            HttpResponse::Ok().json(ApiResponse::success(serde_json::json!({
                "session": session,
                "parsed": parsed,
            })))
        }
        Err(e) => {
            println!("[analyze_resume] ERROR: Pipeline failed: {:?}", e);
            
            // temp_file auto-deletes on drop
            
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("Pipeline error: {}", e)))
        }
    }
}

/// Process outcome through agent loop
async fn process_outcome_handler(
    req: web::Json<OutcomeRequest>,
) -> impl Responder {
    use crate::agent::resume_parser::{AgentSession, ResumeParserConfig, process_outcome};

    // Validate outcome
    let outcome = req.outcome.as_str();
    if !matches!(outcome, "no_response" | "rejected" | "interview") {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("Invalid outcome. Must be: no_response, rejected, or interview"));
    }

    // Deserialize session from JSON
    let session: AgentSession = match serde_json::from_value(req.session.clone()) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error(&format!("Invalid session: {}", e)));
        }
    };

    // Configure parser (use venv if available)
    let config = ResumeParserConfig::default();

    // Process outcome through agent loop
    match process_outcome(&session, outcome, &config) {
        Ok(result) => {
            // Convert Strategy enum to string for JSON response
            let strategy_str = format!("{:?}", result.current_strategy.strategy);
            let response = OutcomeResponse {
                strategy: strategy_str,
                action: result.current_strategy.action.clone(),
                confidence: result.current_strategy.confidence,
                strategy_changed: result.strategy_changed,
                session: serde_json::to_value(&result.session).unwrap_or_default(),
            };
            HttpResponse::Ok().json(ApiResponse::success(response))
        }
        Err(e) => {
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("Agent loop error: {}", e)))
        }
    }
}

// ============================================================
// SERVER CONFIGURATION
// ============================================================

/// Configure and run the API server
pub async fn run_server(host: &str, port: u16) -> std::io::Result<()> {
    let state = Arc::new(AppState::new().expect("Failed to initialize app state"));

    println!("🚀 Career Agent API starting at http://{}:{}", host, port);
    println!("📚 API Endpoints:");
    println!("   POST /api/resume          - Upload resume");
    println!("   POST /api/analyze         - Analyze resume (full pipeline)");
    println!("   POST /api/goal            - Set career goal");
    println!("   POST /api/roadmap         - Generate roadmap");
    println!("   GET  /api/roadmap/:id     - Get roadmap");
    println!("   POST /api/roadmap/edit    - Edit roadmap");
    println!("   POST /api/outcome         - Process outcome");
    println!("   GET  /api/memory/:id      - Get memory timeline");
    println!("   GET  /api/reflection/:id  - Get weekly reflection");
    println!("   GET  /api/rules           - Get career rules");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(state.clone()))
            .route("/health", web::get().to(health_check))
            .route("/api/resume", web::post().to(upload_resume))
            .route("/api/analyze", web::post().to(analyze_resume))
            .route("/api/goal", web::post().to(set_goal))
            .route("/api/roadmap", web::post().to(generate_roadmap))
            .route("/api/roadmap/{user_id}", web::get().to(get_roadmap))
            .route("/api/roadmap/edit", web::post().to(edit_roadmap))
            .route("/api/roadmap/{user_id}/step/{step_id}/complete", web::post().to(complete_step))
            .route("/api/outcome", web::post().to(process_outcome_handler))
            .route("/api/memory/{user_id}", web::get().to(get_memory))
            .route("/api/reflection/{user_id}", web::get().to(get_latest_reflection))
            .route("/api/rules", web::get().to(get_career_rules))
    })
    .bind((host, port))?
    .run()
    .await
}
