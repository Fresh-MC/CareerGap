#!/usr/bin/env python3
"""
Career Snapshot Module

Implements depth-aware skill classification and profile signal extraction.
This is a read-only data layer - no behavior changes are triggered.

Part 1: Enhanced Career Snapshot (Facts Layer)
- Classifies skills by maturity level (theoretical/applied/production)
- Maps evidence sources for each skill

Part 2: Profile Signal Extraction (Interpreter Layer)
- Derives named career signals from resume patterns
- Assigns confidence scores based on deterministic rules
"""

import json
import sys
from typing import Dict, List, Optional, Any, Set
from dataclasses import dataclass, field, asdict
from enum import Enum


# =============================================================================
# CONSTANTS AND ENUMS
# =============================================================================

class SkillDepth(str, Enum):
    """
    Skill maturity classification.
    
    THEORETICAL: Knowledge from courses/certifications only, no practical application
    APPLIED: Has been used in projects or personal work
    PRODUCTION: Has been used in real-world settings (internships/jobs)
    """
    THEORETICAL = "theoretical"
    APPLIED = "applied"
    PRODUCTION = "production"


class EvidenceSource(str, Enum):
    """
    Where skill evidence was observed.
    Used to determine skill depth.
    """
    COURSE = "course"
    CERTIFICATION = "certification"
    PROJECT = "project"
    INTERNSHIP = "internship"
    JOB = "job"


# Evidence source to depth mapping
# Higher depth takes precedence when multiple sources exist
EVIDENCE_DEPTH_MAP = {
    EvidenceSource.COURSE: SkillDepth.THEORETICAL,
    EvidenceSource.CERTIFICATION: SkillDepth.THEORETICAL,
    EvidenceSource.PROJECT: SkillDepth.APPLIED,
    EvidenceSource.INTERNSHIP: SkillDepth.PRODUCTION,
    EvidenceSource.JOB: SkillDepth.PRODUCTION,
}

# Depth precedence (higher index = higher precedence)
DEPTH_PRECEDENCE = [SkillDepth.THEORETICAL, SkillDepth.APPLIED, SkillDepth.PRODUCTION]


# =============================================================================
# DATA STRUCTURES
# =============================================================================

@dataclass
class SkillProfile:
    """
    Enhanced skill representation with depth and evidence.
    
    Attributes:
        name: Skill name (normalized)
        depth: Maturity level (theoretical/applied/production)
        evidence: List of evidence sources where this skill was observed
        mention_count: How many times this skill appears across resume
    """
    name: str
    depth: str  # SkillDepth value as string for JSON compatibility
    evidence: List[str] = field(default_factory=list)
    mention_count: int = 1
    
    def to_dict(self) -> dict:
        return {
            "depth": self.depth,
            "evidence": self.evidence,
            "mention_count": self.mention_count,
        }


@dataclass
class EnhancedSnapshot:
    """
    Complete career snapshot with depth-aware skill classification.
    
    This is the Facts Layer - what the resume contains, classified by depth.
    """
    # Skill profiles keyed by normalized skill name
    skills: Dict[str, SkillProfile] = field(default_factory=dict)
    
    # Counts for signal extraction
    total_skills: int = 0
    theoretical_count: int = 0
    applied_count: int = 0
    production_count: int = 0
    
    # Experience counts
    project_count: int = 0
    internship_count: int = 0
    job_count: int = 0
    certification_count: int = 0
    course_count: int = 0
    
    # Raw data preserved for reference
    raw_experience: List[dict] = field(default_factory=list)
    raw_education: List[dict] = field(default_factory=list)
    
    def to_dict(self) -> dict:
        """Convert to JSON-compatible dictionary."""
        return {
            "skills": {name: profile.to_dict() for name, profile in self.skills.items()},
            "counts": {
                "total_skills": self.total_skills,
                "theoretical": self.theoretical_count,
                "applied": self.applied_count,
                "production": self.production_count,
                "projects": self.project_count,
                "internships": self.internship_count,
                "jobs": self.job_count,
                "certifications": self.certification_count,
                "courses": self.course_count,
            },
            "raw_experience": self.raw_experience,
            "raw_education": self.raw_education,
        }


@dataclass
class ProfileSignal:
    """
    A derived career signal with confidence and explanation.
    
    Signals are patterns detected in the resume that indicate
    potential career positioning issues or strengths.
    """
    name: str
    confidence: float  # 0.0 to 1.0
    triggered: bool
    derived_from: List[str] = field(default_factory=list)
    explanation: str = ""
    
    def to_dict(self) -> dict:
        return {
            "confidence": round(self.confidence, 2),
            "triggered": self.triggered,
            "derived_from": self.derived_from,
            "explanation": self.explanation,
        }


@dataclass
class ProfileSignals:
    """
    Collection of all extracted profile signals.
    
    This is the Interpreter Layer - what the resume implies.
    """
    signals: Dict[str, ProfileSignal] = field(default_factory=dict)
    extraction_notes: List[str] = field(default_factory=list)
    
    def to_dict(self) -> dict:
        return {
            "signals": {name: signal.to_dict() for name, signal in self.signals.items()},
            "extraction_notes": self.extraction_notes,
        }


# =============================================================================
# PART 1: ENHANCED CAREER SNAPSHOT
# =============================================================================

def normalize_skill_name(skill: str) -> str:
    """
    Normalize skill name for consistent matching.
    
    Rules:
    - Strip whitespace
    - Convert to title case
    - Handle common variations
    """
    if not skill:
        return ""
    
    normalized = skill.strip()
    
    # Common normalizations
    normalizations = {
        "javascript": "JavaScript",
        "typescript": "TypeScript",
        "python": "Python",
        "java": "Java",
        "c++": "C++",
        "c#": "C#",
        "node.js": "Node.js",
        "nodejs": "Node.js",
        "react.js": "React",
        "reactjs": "React",
        "vue.js": "Vue",
        "vuejs": "Vue",
        "angular.js": "Angular",
        "angularjs": "Angular",
        "postgresql": "PostgreSQL",
        "postgres": "PostgreSQL",
        "mysql": "MySQL",
        "mongodb": "MongoDB",
        "aws": "AWS",
        "gcp": "GCP",
        "azure": "Azure",
        "docker": "Docker",
        "kubernetes": "Kubernetes",
        "k8s": "Kubernetes",
        "git": "Git",
        "github": "GitHub",
        "gitlab": "GitLab",
        "ci/cd": "CI/CD",
        "cicd": "CI/CD",
        "html": "HTML",
        "css": "CSS",
        "sql": "SQL",
        "nosql": "NoSQL",
        "rest": "REST",
        "restful": "REST",
        "graphql": "GraphQL",
        "api": "API",
        "ml": "Machine Learning",
        "machine learning": "Machine Learning",
        "ai": "AI",
        "artificial intelligence": "AI",
        "nlp": "NLP",
        "natural language processing": "NLP",
    }
    
    lower = normalized.lower()
    if lower in normalizations:
        return normalizations[lower]
    
    # Default: title case
    return normalized.title()


def classify_experience_type(experience: dict) -> EvidenceSource:
    """
    Classify an experience entry as job, internship, or project.
    
    Rules:
    - Contains "intern" in role/company → INTERNSHIP
    - Has company and role, not intern → JOB
    - Otherwise → PROJECT
    """
    role = (experience.get("role") or experience.get("title") or "").lower()
    company = (experience.get("company") or experience.get("organization") or "").lower()
    exp_type = (experience.get("type") or "").lower()
    
    # Explicit type field
    if exp_type:
        if "intern" in exp_type:
            return EvidenceSource.INTERNSHIP
        if "project" in exp_type:
            return EvidenceSource.PROJECT
        if "job" in exp_type or "work" in exp_type or "employ" in exp_type:
            return EvidenceSource.JOB
    
    # Check for internship indicators
    if "intern" in role or "intern" in company:
        return EvidenceSource.INTERNSHIP
    
    # Has company = likely job
    if company and role:
        return EvidenceSource.JOB
    
    # Default to project
    return EvidenceSource.PROJECT


def extract_skills_from_experience(experience: dict) -> List[str]:
    """
    Extract skills mentioned in an experience entry.
    
    Looks in: skills, technologies, tools, description fields.
    """
    skills = []
    
    # Direct skills field
    if "skills" in experience:
        if isinstance(experience["skills"], list):
            skills.extend(experience["skills"])
        elif isinstance(experience["skills"], str):
            skills.extend([s.strip() for s in experience["skills"].split(",")])
    
    # Technologies field
    if "technologies" in experience:
        if isinstance(experience["technologies"], list):
            skills.extend(experience["technologies"])
        elif isinstance(experience["technologies"], str):
            skills.extend([s.strip() for s in experience["technologies"].split(",")])
    
    # Tools field
    if "tools" in experience:
        if isinstance(experience["tools"], list):
            skills.extend(experience["tools"])
    
    return [s for s in skills if s]


def determine_skill_depth(evidence_sources: List[str]) -> str:
    """
    Determine the highest skill depth from evidence sources.
    
    Rule: Highest precedence wins.
    - production > applied > theoretical
    
    Evidence mapping:
    - internship/job → production
    - project → applied
    - course/certification → theoretical
    """
    if not evidence_sources:
        return SkillDepth.THEORETICAL.value
    
    max_depth = SkillDepth.THEORETICAL
    
    for source_str in evidence_sources:
        try:
            source = EvidenceSource(source_str)
            depth = EVIDENCE_DEPTH_MAP.get(source, SkillDepth.THEORETICAL)
            
            # Check if this depth is higher precedence
            if DEPTH_PRECEDENCE.index(depth) > DEPTH_PRECEDENCE.index(max_depth):
                max_depth = depth
        except ValueError:
            # Unknown source, skip
            continue
    
    return max_depth.value


def build_enhanced_snapshot(parsed_resume: dict) -> EnhancedSnapshot:
    """
    Build an enhanced career snapshot from parsed resume data.
    
    This function creates a depth-aware skill classification where each skill
    is tagged with its maturity level based on evidence sources.
    
    Args:
        parsed_resume: Output from parser.py containing:
            - skills: List of skill strings
            - experience: List of experience dicts
            - education: List of education dicts
            - certifications: List of certification dicts (if available)
    
    Returns:
        EnhancedSnapshot with classified skills and counts
    
    Classification Rules:
        1. Skills from internships/jobs → production depth
        2. Skills from projects → applied depth
        3. Skills from courses/certifications only → theoretical depth
        4. If skill appears in multiple contexts, highest depth wins
    """
    snapshot = EnhancedSnapshot()
    skill_evidence_map: Dict[str, Set[str]] = {}  # skill_name -> set of evidence sources
    
    # Preserve raw data
    snapshot.raw_experience = parsed_resume.get("experience", [])
    snapshot.raw_education = parsed_resume.get("education", [])
    
    # --- Process Experience Entries ---
    for exp in parsed_resume.get("experience", []):
        exp_type = classify_experience_type(exp)
        
        # Count experience types
        if exp_type == EvidenceSource.INTERNSHIP:
            snapshot.internship_count += 1
        elif exp_type == EvidenceSource.JOB:
            snapshot.job_count += 1
        elif exp_type == EvidenceSource.PROJECT:
            snapshot.project_count += 1
        
        # Extract and map skills from this experience
        exp_skills = extract_skills_from_experience(exp)
        for skill in exp_skills:
            norm_skill = normalize_skill_name(skill)
            if not norm_skill:
                continue
            
            if norm_skill not in skill_evidence_map:
                skill_evidence_map[norm_skill] = set()
            skill_evidence_map[norm_skill].add(exp_type.value)
    
    # --- Process Top-Level Skills (assume theoretical if no other evidence) ---
    for skill in parsed_resume.get("skills", []):
        norm_skill = normalize_skill_name(skill)
        if not norm_skill:
            continue
        
        if norm_skill not in skill_evidence_map:
            # No evidence from experience - mark as course-level (theoretical)
            skill_evidence_map[norm_skill] = {EvidenceSource.COURSE.value}
    
    # --- Process Certifications ---
    certifications = parsed_resume.get("certifications", [])
    if certifications:
        snapshot.certification_count = len(certifications)
        
        for cert in certifications:
            # Extract skills from certification if available
            cert_skills = []
            if isinstance(cert, dict):
                cert_skills = cert.get("skills", [])
                if isinstance(cert_skills, str):
                    cert_skills = [s.strip() for s in cert_skills.split(",")]
            
            for skill in cert_skills:
                norm_skill = normalize_skill_name(skill)
                if not norm_skill:
                    continue
                
                if norm_skill not in skill_evidence_map:
                    skill_evidence_map[norm_skill] = set()
                skill_evidence_map[norm_skill].add(EvidenceSource.CERTIFICATION.value)
    
    # --- Process Education (may have courses) ---
    for edu in parsed_resume.get("education", []):
        courses = edu.get("courses", []) or edu.get("relevant_courses", [])
        if courses:
            snapshot.course_count += len(courses)
        
        # Extract skills from coursework
        for course in courses:
            if isinstance(course, str):
                # Course name might be a skill
                norm_skill = normalize_skill_name(course)
                if norm_skill and len(norm_skill) < 30:  # Reasonable skill name length
                    if norm_skill not in skill_evidence_map:
                        skill_evidence_map[norm_skill] = set()
                    skill_evidence_map[norm_skill].add(EvidenceSource.COURSE.value)
    
    # --- Build Final Skill Profiles ---
    for skill_name, evidence_set in skill_evidence_map.items():
        evidence_list = sorted(list(evidence_set))
        depth = determine_skill_depth(evidence_list)
        
        profile = SkillProfile(
            name=skill_name,
            depth=depth,
            evidence=evidence_list,
            mention_count=1,  # Could be enhanced to count actual mentions
        )
        
        snapshot.skills[skill_name] = profile
        
        # Update depth counts
        if depth == SkillDepth.THEORETICAL.value:
            snapshot.theoretical_count += 1
        elif depth == SkillDepth.APPLIED.value:
            snapshot.applied_count += 1
        elif depth == SkillDepth.PRODUCTION.value:
            snapshot.production_count += 1
    
    snapshot.total_skills = len(snapshot.skills)
    
    return snapshot


# =============================================================================
# PART 2: PROFILE SIGNAL EXTRACTION
# =============================================================================

# Signal thresholds (easily adjustable)
THRESHOLDS = {
    # Overlearning trap: many skills, few projects
    "overlearning_skill_min": 10,
    "overlearning_project_max": 2,
    "overlearning_theoretical_ratio": 0.6,  # >60% theoretical skills
    
    # Market exposure gap: no internships, many certifications
    "exposure_internship_max": 0,
    "exposure_certification_min": 2,
    
    # Resume positioning: interviews but rejections
    "positioning_interview_min": 1,
    "positioning_rejection_ratio": 0.8,  # >80% rejection rate
    
    # Execution gap: high interview rate but repeated rejections
    "execution_interview_min": 3,
    "execution_rejection_consecutive": 2,
}


def extract_overlearning_trap(snapshot: EnhancedSnapshot) -> ProfileSignal:
    """
    Detect overlearning trap signal.
    
    Trigger: Many skills listed + few/no projects
    Interpretation: Candidate may be collecting credentials without applying them
    
    Confidence calculation:
    - Base: 0.5 if triggered
    - +0.2 if theoretical ratio > 60%
    - +0.2 if project count == 0
    - +0.1 if skill count > 20
    """
    derived_from = []
    confidence = 0.0
    triggered = False
    
    skill_count = snapshot.total_skills
    project_count = snapshot.project_count
    theoretical_ratio = (
        snapshot.theoretical_count / snapshot.total_skills 
        if snapshot.total_skills > 0 else 0
    )
    
    # Check trigger conditions
    if skill_count >= THRESHOLDS["overlearning_skill_min"]:
        derived_from.append(f"skill_count={skill_count}")
        
        if project_count <= THRESHOLDS["overlearning_project_max"]:
            derived_from.append(f"project_count={project_count}")
            triggered = True
            confidence = 0.5
            
            # Boost confidence based on severity
            if theoretical_ratio > THRESHOLDS["overlearning_theoretical_ratio"]:
                confidence += 0.2
                derived_from.append(f"theoretical_ratio={theoretical_ratio:.1%}")
            
            if project_count == 0:
                confidence += 0.2
                derived_from.append("no_projects")
            
            if skill_count > 20:
                confidence += 0.1
                derived_from.append("excessive_skills")
    
    return ProfileSignal(
        name="overlearning_trap",
        confidence=min(confidence, 1.0),
        triggered=triggered,
        derived_from=derived_from,
        explanation=(
            "Candidate lists many skills but has few practical applications. "
            "May indicate credential collection without real-world validation."
            if triggered else "Not detected"
        ),
    )


def extract_market_exposure_gap(snapshot: EnhancedSnapshot) -> ProfileSignal:
    """
    Detect market exposure gap signal.
    
    Trigger: No internships + many certifications
    Interpretation: Candidate lacks real market exposure despite training
    
    Confidence calculation:
    - Base: 0.6 if triggered
    - +0.2 if certification count > 4
    - +0.2 if no jobs either
    """
    derived_from = []
    confidence = 0.0
    triggered = False
    
    internship_count = snapshot.internship_count
    job_count = snapshot.job_count
    certification_count = snapshot.certification_count
    
    # Check trigger conditions
    if internship_count <= THRESHOLDS["exposure_internship_max"]:
        derived_from.append("no_internships")
        
        if certification_count >= THRESHOLDS["exposure_certification_min"]:
            derived_from.append(f"certifications={certification_count}")
            triggered = True
            confidence = 0.6
            
            # Boost confidence based on severity
            if certification_count > 4:
                confidence += 0.2
                derived_from.append("many_certifications")
            
            if job_count == 0:
                confidence += 0.2
                derived_from.append("no_jobs")
    
    return ProfileSignal(
        name="market_exposure_gap",
        confidence=min(confidence, 1.0),
        triggered=triggered,
        derived_from=derived_from,
        explanation=(
            "Candidate has training credentials but lacks market exposure "
            "through internships or employment."
            if triggered else "Not detected"
        ),
    )


def extract_resume_positioning_issue(
    snapshot: EnhancedSnapshot, 
    outcomes: List[str]
) -> ProfileSignal:
    """
    Detect resume positioning issue signal.
    
    Trigger: Has interviews but mostly rejected
    Interpretation: Resume gets attention but fails to convert
    
    Confidence calculation:
    - Base: 0.5 if triggered
    - Scales with rejection ratio (0.5 + rejection_ratio * 0.4)
    
    Note: Requires outcome history to calculate.
    """
    derived_from = []
    confidence = 0.0
    triggered = False
    
    if not outcomes:
        return ProfileSignal(
            name="resume_positioning_issue",
            confidence=0.0,
            triggered=False,
            derived_from=["no_outcome_data"],
            explanation="Cannot evaluate without outcome history",
        )
    
    interview_count = outcomes.count("interview")
    rejection_count = outcomes.count("rejected")
    total_outcomes = len(outcomes)
    
    if interview_count >= THRESHOLDS["positioning_interview_min"]:
        derived_from.append(f"interviews={interview_count}")
        
        # Calculate rejection ratio (excluding no_response)
        meaningful_outcomes = interview_count + rejection_count
        if meaningful_outcomes > 0:
            rejection_ratio = rejection_count / meaningful_outcomes
            
            if rejection_ratio >= THRESHOLDS["positioning_rejection_ratio"]:
                derived_from.append(f"rejection_ratio={rejection_ratio:.1%}")
                triggered = True
                confidence = 0.5 + (rejection_ratio * 0.4)
    
    return ProfileSignal(
        name="resume_positioning_issue",
        confidence=min(confidence, 1.0),
        triggered=triggered,
        derived_from=derived_from,
        explanation=(
            "Resume attracts interviews but fails to convert. "
            "May indicate mismatch between resume positioning and actual capabilities."
            if triggered else "Not detected"
        ),
    )


def extract_execution_gap(
    snapshot: EnhancedSnapshot, 
    outcomes: List[str]
) -> ProfileSignal:
    """
    Detect execution gap signal.
    
    Trigger: High interview rate + repeated consecutive rejections
    Interpretation: Candidate performs well initially but fails execution
    
    Confidence calculation:
    - Base: 0.6 if triggered
    - +0.2 for each additional consecutive rejection (max 0.3)
    
    Note: Requires outcome history to calculate.
    """
    derived_from = []
    confidence = 0.0
    triggered = False
    
    if not outcomes:
        return ProfileSignal(
            name="execution_gap",
            confidence=0.0,
            triggered=False,
            derived_from=["no_outcome_data"],
            explanation="Cannot evaluate without outcome history",
        )
    
    interview_count = outcomes.count("interview")
    
    # Count consecutive rejections after interviews
    max_consecutive_rejections = 0
    current_streak = 0
    had_interview = False
    
    for outcome in outcomes:
        if outcome == "interview":
            had_interview = True
            current_streak = 0  # Reset streak on interview
        elif outcome == "rejected" and had_interview:
            current_streak += 1
            max_consecutive_rejections = max(max_consecutive_rejections, current_streak)
    
    if interview_count >= THRESHOLDS["execution_interview_min"]:
        derived_from.append(f"interviews={interview_count}")
        
        if max_consecutive_rejections >= THRESHOLDS["execution_rejection_consecutive"]:
            derived_from.append(f"consecutive_rejections={max_consecutive_rejections}")
            triggered = True
            confidence = 0.6
            
            # Boost for additional consecutive rejections
            extra_rejections = max_consecutive_rejections - THRESHOLDS["execution_rejection_consecutive"]
            confidence += min(extra_rejections * 0.1, 0.3)
    
    return ProfileSignal(
        name="execution_gap",
        confidence=min(confidence, 1.0),
        triggered=triggered,
        derived_from=derived_from,
        explanation=(
            "Candidate gets interviews but shows pattern of repeated rejections. "
            "May indicate interview performance issues or role-fit problems."
            if triggered else "Not detected"
        ),
    )


def extract_profile_signals(
    snapshot: EnhancedSnapshot, 
    outcomes: Optional[List[str]] = None
) -> ProfileSignals:
    """
    Extract all profile signals from an enhanced snapshot.
    
    This function interprets the resume patterns and converts them into
    named career signals with confidence scores.
    
    Args:
        snapshot: Enhanced career snapshot from build_enhanced_snapshot()
        outcomes: Optional list of outcome strings ("interview", "rejected", "no_response")
    
    Returns:
        ProfileSignals containing all extracted signals
    
    Signal Types:
        1. overlearning_trap: Many skills, few projects
        2. market_exposure_gap: No internships, many certifications
        3. resume_positioning_issue: Interviews but rejections (requires outcomes)
        4. execution_gap: High interviews, consecutive rejections (requires outcomes)
    """
    outcomes = outcomes or []
    signals = ProfileSignals()
    
    # Extract each signal
    signals.signals["overlearning_trap"] = extract_overlearning_trap(snapshot)
    signals.signals["market_exposure_gap"] = extract_market_exposure_gap(snapshot)
    signals.signals["resume_positioning_issue"] = extract_resume_positioning_issue(
        snapshot, outcomes
    )
    signals.signals["execution_gap"] = extract_execution_gap(snapshot, outcomes)
    
    # Add extraction notes
    triggered_count = sum(1 for s in signals.signals.values() if s.triggered)
    signals.extraction_notes.append(f"Extracted {len(signals.signals)} signals")
    signals.extraction_notes.append(f"Triggered signals: {triggered_count}")
    
    if outcomes:
        signals.extraction_notes.append(f"Outcome history: {len(outcomes)} entries")
    else:
        signals.extraction_notes.append("No outcome history provided")
    
    return signals


# =============================================================================
# COMBINED OUTPUT
# =============================================================================

def create_career_analysis(
    parsed_resume: dict, 
    outcomes: Optional[List[str]] = None
) -> dict:
    """
    Create complete career analysis combining snapshot and signals.
    
    This is the main entry point for the career snapshot module.
    Returns a JSON-compatible dictionary that can be stored in session data.
    
    Args:
        parsed_resume: Output from parser.py
        outcomes: Optional outcome history
    
    Returns:
        {
            "enhanced_snapshot": {...},
            "profile_signals": {...},
            "metadata": {...}
        }
    """
    # Build enhanced snapshot
    snapshot = build_enhanced_snapshot(parsed_resume)
    
    # Extract profile signals
    signals = extract_profile_signals(snapshot, outcomes)
    
    return {
        "enhanced_snapshot": snapshot.to_dict(),
        "profile_signals": signals.to_dict(),
        "metadata": {
            "version": "1.0.0",
            "outcome_count": len(outcomes) if outcomes else 0,
            "skill_count": snapshot.total_skills,
            "signals_triggered": sum(1 for s in signals.signals.values() if s.triggered),
        }
    }


# =============================================================================
# CLI INTERFACE
# =============================================================================

def main():
    """CLI interface for testing career snapshot module."""
    if len(sys.argv) < 2:
        print(json.dumps({
            "error": "Usage: python career_snapshot.py <parsed_resume.json> [outcomes.json]",
            "description": "Build enhanced career snapshot and extract profile signals",
        }))
        sys.exit(1)
    
    try:
        # Load parsed resume
        with open(sys.argv[1], 'r', encoding='utf-8') as f:
            parsed_resume = json.load(f)
        
        # Load outcomes if provided
        outcomes = None
        if len(sys.argv) > 2:
            with open(sys.argv[2], 'r', encoding='utf-8') as f:
                outcomes = json.load(f)
        
        # Create analysis
        result = create_career_analysis(parsed_resume, outcomes)
        
        print(json.dumps(result, indent=2))
        
    except json.JSONDecodeError as e:
        print(json.dumps({"error": f"Invalid JSON: {str(e)}"}))
        sys.exit(1)
    except FileNotFoundError as e:
        print(json.dumps({"error": f"File not found: {str(e)}"}))
        sys.exit(1)
    except Exception as e:
        print(json.dumps({"error": f"Error: {str(e)}"}))
        sys.exit(1)


if __name__ == "__main__":
    main()
