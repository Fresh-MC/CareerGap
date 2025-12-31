#!/usr/bin/env python3
"""
Evidence Mapper - Stage 2: Bottleneck Analysis

Takes Stage 1 output and identifies bottlenecks.
NO recommendations, NO strategies - bottleneck identification only.
"""

import json
import sys

# =============================================================================
# ROLE INFERENCE
# =============================================================================

# Skill -> Role mappings (entry-level roles only)
ROLE_SKILL_MAP = {
    "data_analyst": {
        "primary": ["sql", "excel", "python", "tableau", "power bi", "data analysis"],
        "secondary": ["statistics", "pandas", "numpy", "data visualization"],
    },
    "data_scientist": {
        "primary": ["python", "machine learning", "deep learning", "tensorflow", "pytorch"],
        "secondary": ["sql", "pandas", "numpy", "statistics", "nlp", "data science"],
    },
    "software_engineer": {
        "primary": ["java", "python", "javascript", "c++", "data structures", "algorithms"],
        "secondary": ["git", "sql", "api", "rest", "oop", "dsa"],
    },
    "frontend_developer": {
        "primary": ["javascript", "react", "html", "css", "typescript"],
        "secondary": ["vue.js", "angular", "node.js", "git"],
    },
    "backend_developer": {
        "primary": ["java", "python", "node.js", "sql", "api"],
        "secondary": ["docker", "postgresql", "mongodb", "rest", "spring", "django"],
    },
    "devops_engineer": {
        "primary": ["docker", "kubernetes", "aws", "linux", "ci/cd"],
        "secondary": ["jenkins", "terraform", "ansible", "git", "python", "bash"],
    },
    "ml_engineer": {
        "primary": ["python", "tensorflow", "pytorch", "machine learning", "deep learning"],
        "secondary": ["docker", "aws", "mlops", "pandas", "numpy"],
    },
    "web_developer": {
        "primary": ["html", "css", "javascript", "react"],
        "secondary": ["node.js", "sql", "git", "python"],
    },
}

# Role display names
ROLE_DISPLAY_NAMES = {
    "data_analyst": "Data Analyst",
    "data_scientist": "Data Scientist",
    "software_engineer": "Software Engineer",
    "frontend_developer": "Frontend Developer",
    "backend_developer": "Backend Developer",
    "devops_engineer": "DevOps Engineer",
    "ml_engineer": "ML Engineer",
    "web_developer": "Web Developer",
}


def infer_role(normalized_skills: list) -> str:
    """
    Infer the single most likely entry-level target role.
    Returns exactly one role.
    """
    skills_lower = {s.lower() for s in normalized_skills}
    
    role_scores = {}
    
    for role, skill_sets in ROLE_SKILL_MAP.items():
        primary_matches = sum(1 for s in skill_sets["primary"] if s in skills_lower)
        secondary_matches = sum(1 for s in skill_sets["secondary"] if s in skills_lower)
        
        # Weight primary skills more heavily
        score = (primary_matches * 3) + (secondary_matches * 1)
        role_scores[role] = score
    
    # Get role with highest score
    if not role_scores or max(role_scores.values()) == 0:
        return "Software Engineer"  # Default fallback
    
    best_role = max(role_scores, key=role_scores.get)
    return ROLE_DISPLAY_NAMES.get(best_role, best_role)


# =============================================================================
# BOTTLENECK EVALUATION
# =============================================================================

def evaluate_positioning(skill_evidence_map: dict, section_signals: dict) -> str:
    """
    Evaluate positioning strength.
    Strong: Skills have diverse evidence (project + internship + coursework)
    Weak: Skills mostly listed_only or single source
    Missing: No meaningful evidence mapping
    """
    if not skill_evidence_map:
        return "missing"
    
    # Count skills with multiple evidence sources
    multi_source_count = 0
    single_source_count = 0
    listed_only_count = 0
    
    for skill, evidence in skill_evidence_map.items():
        unique_sources = set(evidence) - {"listed_only"}
        if "listed_only" in evidence and len(evidence) == 1:
            listed_only_count += 1
        elif len(unique_sources) >= 2:
            multi_source_count += 1
        else:
            single_source_count += 1
    
    total = len(skill_evidence_map)
    if total == 0:
        return "missing"
    
    multi_ratio = multi_source_count / total
    listed_ratio = listed_only_count / total
    
    if multi_ratio >= 0.4:
        return "strong"
    elif listed_ratio >= 0.5:
        return "missing"
    else:
        return "weak"


def evaluate_evidence_depth(skill_evidence_map: dict, section_signals: dict) -> str:
    """
    Evaluate evidence depth.
    Strong: Has projects AND internship with metrics
    Weak: Has projects OR internship but not both, or no metrics
    Missing: Neither projects nor internship
    """
    has_projects = section_signals.get("has_projects", False)
    has_internship = section_signals.get("has_internship", False)
    has_metrics = section_signals.get("has_metrics", False)
    
    if has_projects and has_internship and has_metrics:
        return "strong"
    elif has_projects and has_internship:
        return "weak"
    elif has_projects or has_internship:
        return "weak"
    else:
        return "missing"


def evaluate_experience_strength(skill_evidence_map: dict, section_signals: dict) -> str:
    """
    Evaluate experience strength.
    Strong: Internship with deployment/production experience
    Weak: Internship exists but no production indicators
    Missing: No internship evidence
    """
    has_internship = section_signals.get("has_internship", False)
    has_deployment = section_signals.get("has_deployment", False)
    
    # Check if any skills have internship evidence
    internship_skills = sum(
        1 for evidence in skill_evidence_map.values()
        if "internship" in evidence
    )
    
    if not has_internship and internship_skills == 0:
        return "missing"
    
    if has_deployment and has_internship:
        return "strong"
    elif has_internship or internship_skills > 0:
        return "weak"
    else:
        return "missing"


def evaluate_skill_alignment(normalized_skills: list, implied_role: str) -> str:
    """
    Evaluate skill alignment with implied role.
    Strong: Has most primary skills for the role
    Weak: Has some primary skills but gaps
    Missing: Minimal overlap with role requirements
    """
    skills_lower = {s.lower() for s in normalized_skills}
    
    # Find the role key
    role_key = None
    for key, display in ROLE_DISPLAY_NAMES.items():
        if display == implied_role:
            role_key = key
            break
    
    if not role_key or role_key not in ROLE_SKILL_MAP:
        return "weak"
    
    primary_skills = set(ROLE_SKILL_MAP[role_key]["primary"])
    primary_matches = len(skills_lower & primary_skills)
    primary_total = len(primary_skills)
    
    if primary_total == 0:
        return "weak"
    
    match_ratio = primary_matches / primary_total
    
    if match_ratio >= 0.6:
        return "strong"
    elif match_ratio >= 0.3:
        return "weak"
    else:
        return "missing"


def evaluate_outcome_visibility(section_signals: dict, skill_evidence_map: dict) -> str:
    """
    Evaluate outcome visibility.
    Strong: Has metrics and project evidence
    Weak: Has projects but no metrics
    Missing: No measurable outcomes
    """
    has_metrics = section_signals.get("has_metrics", False)
    has_projects = section_signals.get("has_projects", False)
    
    # Check if skills appear in projects
    project_skills = sum(
        1 for evidence in skill_evidence_map.values()
        if "project" in evidence
    )
    
    if has_metrics and has_projects and project_skills >= 2:
        return "strong"
    elif has_projects or project_skills > 0:
        return "weak"
    else:
        return "missing"


def evaluate_bottlenecks(
    normalized_skills: list,
    skill_evidence_map: dict,
    section_signals: dict,
    implied_role: str
) -> dict:
    """
    Evaluate all bottleneck categories.
    Returns dict with each category rated as strong/weak/missing.
    """
    return {
        "positioning": evaluate_positioning(skill_evidence_map, section_signals),
        "evidence_depth": evaluate_evidence_depth(skill_evidence_map, section_signals),
        "experience_strength": evaluate_experience_strength(skill_evidence_map, section_signals),
        "skill_alignment": evaluate_skill_alignment(normalized_skills, implied_role),
        "outcome_visibility": evaluate_outcome_visibility(section_signals, skill_evidence_map),
    }


# =============================================================================
# DOMINANT FAILURE SELECTION
# =============================================================================

# Priority order for dominant issue selection (most critical first)
BOTTLENECK_PRIORITY = [
    "experience_strength",
    "evidence_depth",
    "outcome_visibility",
    "positioning",
    "skill_alignment",
]

# Justification templates based on signals
JUSTIFICATION_TEMPLATES = {
    "experience_strength": {
        "missing": "No internship or work experience evidence found in resume signals.",
        "weak": "Internship exists but lacks production/deployment indicators.",
    },
    "evidence_depth": {
        "missing": "Resume lacks both project and internship sections.",
        "weak": "Has {present} but missing {missing}; no quantifiable metrics found.",
    },
    "outcome_visibility": {
        "missing": "No measurable outcomes or project evidence detected.",
        "weak": "Projects present but lack quantifiable metrics or results.",
    },
    "positioning": {
        "missing": "Skills are listed without contextual evidence of application.",
        "weak": "Most skills lack diverse evidence sources (project, internship, coursework).",
    },
    "skill_alignment": {
        "missing": "Minimal overlap between skills and target role requirements.",
        "weak": "Some primary skills for {role} present, but notable gaps remain.",
    },
}


def select_dominant_issue(bottlenecks: dict, section_signals: dict, implied_role: str) -> tuple:
    """
    Select exactly one dominant bottleneck and provide justification.
    Returns (dominant_issue, justification).
    """
    # Find missing bottlenecks first, then weak ones
    missing = [b for b in BOTTLENECK_PRIORITY if bottlenecks.get(b) == "missing"]
    weak = [b for b in BOTTLENECK_PRIORITY if bottlenecks.get(b) == "weak"]
    
    if missing:
        dominant = missing[0]
        template = JUSTIFICATION_TEMPLATES.get(dominant, {}).get("missing", "")
    elif weak:
        dominant = weak[0]
        template = JUSTIFICATION_TEMPLATES.get(dominant, {}).get("weak", "")
    else:
        # All strong - no dominant issue
        return None, "All bottleneck categories are strong."
    
    # Fill in template placeholders
    justification = template
    
    if dominant == "evidence_depth":
        has_projects = section_signals.get("has_projects", False)
        has_internship = section_signals.get("has_internship", False)
        present = []
        missing_items = []
        if has_projects:
            present.append("projects")
        else:
            missing_items.append("projects")
        if has_internship:
            present.append("internship")
        else:
            missing_items.append("internship")
        justification = template.format(
            present=", ".join(present) if present else "neither",
            missing=", ".join(missing_items) if missing_items else "none"
        )
    elif dominant == "skill_alignment":
        justification = template.format(role=implied_role)
    
    return dominant, justification


# =============================================================================
# MAIN STAGE 2 ANALYZER
# =============================================================================

def analyze_bottlenecks(stage1_output: dict) -> dict:
    """
    Stage 2 Analysis: Identify bottlenecks from Stage 1 signals.
    
    Input: Stage 1 output with normalized_skills, skill_evidence_map, section_signals
    Output: {
        "implied_role": "...",
        "bottlenecks": {...},
        "dominant_issue": "...",
        "justification": "..."
    }
    """
    normalized_skills = stage1_output.get("normalized_skills", [])
    skill_evidence_map = stage1_output.get("skill_evidence_map", {})
    section_signals = stage1_output.get("section_signals", {})
    
    # 1. Infer implied role
    implied_role = infer_role(normalized_skills)
    
    # 2. Evaluate all bottlenecks
    bottlenecks = evaluate_bottlenecks(
        normalized_skills,
        skill_evidence_map,
        section_signals,
        implied_role
    )
    
    # 3. Select dominant issue
    dominant_issue, justification = select_dominant_issue(
        bottlenecks,
        section_signals,
        implied_role
    )
    
    return {
        "implied_role": implied_role,
        "bottlenecks": bottlenecks,
        "dominant_issue": dominant_issue,
        "justification": justification,
    }


def main():
    """CLI interface for Stage 2 analyzer."""
    import traceback
    
    try:
        print(f"[bottleneck_analyzer.py] Python: {sys.version}", file=sys.stderr)
        print(f"[bottleneck_analyzer.py] Executable: {sys.executable}", file=sys.stderr)
        print(f"[bottleneck_analyzer.py] Reading input from stdin", file=sys.stderr)
        
        # Read JSON from stdin
        stdin_data = sys.stdin.read()
        print(f"[bottleneck_analyzer.py] Stdin length: {len(stdin_data)} chars", file=sys.stderr)
        
        if not stdin_data:
            print(json.dumps({"error": "No input received on stdin"}))
            sys.exit(1)
        
        stage1_output = json.loads(stdin_data)
        print(f"[bottleneck_analyzer.py] Parsed JSON from stdin successfully", file=sys.stderr)
        
        result = analyze_bottlenecks(stage1_output)
        print(f"[bottleneck_analyzer.py] Analysis completed", file=sys.stderr)
        print(f"[bottleneck_analyzer.py] Implied role: {result.get('implied_role')}", file=sys.stderr)
        print(json.dumps(result))
        
    except json.JSONDecodeError as e:
        print(f"[bottleneck_analyzer.py] JSON decode error: {e}", file=sys.stderr)
        print(traceback.format_exc(), file=sys.stderr)
        print(json.dumps({"error": f"Invalid JSON: {str(e)}"}))
        sys.exit(1)
    except Exception as e:
        print(f"[bottleneck_analyzer.py] EXCEPTION: {str(e)}", file=sys.stderr)
        print(traceback.format_exc(), file=sys.stderr)
        print(json.dumps({"error": f"Error: {str(e)}"}))
        sys.exit(1)


if __name__ == "__main__":
    main()
