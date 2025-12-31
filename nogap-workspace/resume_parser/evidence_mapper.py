#!/usr/bin/env python3
"""
Evidence Mapper - Stage 1: Signal Extraction

Takes parsed resume JSON and extracts normalized, reliable signals.
NO scoring, NO recommendations, NO strategy - signal extraction only.
"""

import re
import json
import sys

# =============================================================================
# SKILL WHITELIST - Technical skills only
# =============================================================================

TECHNICAL_SKILLS_WHITELIST = {
    # Programming Languages
    "python", "java", "javascript", "typescript", "c++", "c#", "c", "ruby",
    "go", "golang", "rust", "swift", "kotlin", "php", "scala", "r", "matlab",
    "perl", "lua", "haskell", "elixir", "clojure", "dart", "objective-c",
    
    # Web Frameworks & Libraries
    "react", "angular", "vue", "vue.js", "node.js", "nodejs", "express",
    "django", "flask", "fastapi", "spring", "spring boot", "rails", "laravel",
    "next.js", "nextjs", "nuxt", "svelte", "jquery", "bootstrap", "tailwind",
    
    # Data & ML
    "machine learning", "deep learning", "nlp", "natural language processing",
    "computer vision", "tensorflow", "pytorch", "keras", "scikit-learn",
    "pandas", "numpy", "scipy", "matplotlib", "seaborn", "opencv",
    "data science", "data analysis", "data engineering", "data visualization",
    "statistics", "big data", "spark", "hadoop", "hive", "kafka",
    
    # Databases
    "sql", "mysql", "postgresql", "postgres", "mongodb", "redis", "sqlite",
    "oracle", "cassandra", "dynamodb", "elasticsearch", "neo4j", "graphql",
    "nosql", "firebase", "supabase",
    
    # Cloud & DevOps
    "aws", "azure", "gcp", "google cloud", "docker", "kubernetes", "k8s",
    "jenkins", "ci/cd", "terraform", "ansible", "linux", "unix", "bash",
    "git", "github", "gitlab", "bitbucket", "devops", "nginx", "apache",
    
    # Tools & Platforms
    "excel", "power bi", "tableau", "jupyter", "anaconda", "vscode",
    "jira", "confluence", "slack", "figma", "postman", "swagger",
    
    # Core CS Concepts
    "data structures", "algorithms", "dsa", "oop", "object oriented programming",
    "api", "rest", "restful", "microservices", "system design", "dbms",
    "operating systems", "networking", "cybersecurity", "agile", "scrum",
    
    # Mobile
    "android", "ios", "react native", "flutter", "xamarin",
    
    # Other Technical
    "html", "css", "sass", "less", "webpack", "npm", "yarn",
    "blockchain", "web3", "solidity", "unity", "unreal engine",
}

# Words to exclude (locations, verbs, generic terms)
EXCLUSION_PATTERNS = [
    r"^(created|worked|developed|built|designed|managed|led|used|using)$",
    r"^(coimbatore|chennai|bangalore|mumbai|delhi|hyderabad|pune|kolkata)$",
    r"^(india|usa|uk|canada|australia|germany|singapore)$",
    r"^(university|college|school|institute|academy|campus)$",
    r"^(club|team|group|department|organization)$",
    r"^(technical tools|tools|skills|technologies)$",
    r"^(artificial intelligence and|science & technology)$",
    r"^\d+$",  # Pure numbers
    r"^.{1,2}$",  # Too short
]


def normalize_skill(skill: str) -> str:
    """Normalize skill name: lowercase, trim, standardize spacing."""
    skill = skill.lower().strip()
    skill = re.sub(r'\s+', ' ', skill)  # Normalize whitespace
    
    # Common normalizations
    normalizations = {
        "nodejs": "node.js",
        "vuejs": "vue.js",
        "nextjs": "next.js",
        "golang": "go",
        "postgres": "postgresql",
        "k8s": "kubernetes",
    }
    return normalizations.get(skill, skill)


def is_valid_skill(skill: str) -> bool:
    """Check if skill passes whitelist and exclusion filters."""
    normalized = normalize_skill(skill)
    
    # Check exclusion patterns
    for pattern in EXCLUSION_PATTERNS:
        if re.match(pattern, normalized, re.IGNORECASE):
            return False
    
    # Check whitelist
    return normalized in TECHNICAL_SKILLS_WHITELIST


def format_skill_display(skill: str) -> str:
    """Format skill for display with proper casing."""
    normalized = normalize_skill(skill)
    
    # Special casing rules
    special_cases = {
        "python": "Python",
        "java": "Java",
        "javascript": "JavaScript",
        "typescript": "TypeScript",
        "c++": "C++",
        "c#": "C#",
        "sql": "SQL",
        "mysql": "MySQL",
        "postgresql": "PostgreSQL",
        "mongodb": "MongoDB",
        "node.js": "Node.js",
        "react": "React",
        "angular": "Angular",
        "vue.js": "Vue.js",
        "django": "Django",
        "flask": "Flask",
        "aws": "AWS",
        "azure": "Azure",
        "gcp": "GCP",
        "docker": "Docker",
        "kubernetes": "Kubernetes",
        "git": "Git",
        "linux": "Linux",
        "excel": "Excel",
        "html": "HTML",
        "css": "CSS",
        "api": "API",
        "rest": "REST",
        "graphql": "GraphQL",
        "nosql": "NoSQL",
        "machine learning": "Machine Learning",
        "deep learning": "Deep Learning",
        "data science": "Data Science",
        "data structures": "Data Structures",
        "dbms": "DBMS",
        "dsa": "DSA",
        "nlp": "NLP",
        "tensorflow": "TensorFlow",
        "pytorch": "PyTorch",
        "pandas": "Pandas",
        "numpy": "NumPy",
        "jupyter": "Jupyter",
        "power bi": "Power BI",
        "tableau": "Tableau",
        "agile": "Agile",
        "scrum": "Scrum",
        "ci/cd": "CI/CD",
        "devops": "DevOps",
    }
    
    return special_cases.get(normalized, skill.title())


# =============================================================================
# EVIDENCE LOCATION MAPPING
# =============================================================================

SECTION_PATTERNS = {
    "internship": [
        r"internship\s*experience",
        r"work\s*experience",
        r"professional\s*experience",
        r"internship",
        r"intern\s+at",
        r"worked\s+(?:at|with|on)",
    ],
    "project": [
        r"projects?\s*(?:section)?",
        r"academic\s*projects?",
        r"personal\s*projects?",
        r"worked\s+on\s+.*project",
        r"built\s+(?:a|an|the)",
        r"developed\s+(?:a|an|the)",
    ],
    "coursework": [
        r"relevant\s*coursework",
        r"coursework",
        r"courses?\s*(?:taken|completed)",
        r"academic\s*courses?",
    ],
    "tools": [
        r"technical\s*(?:tools|skills)",
        r"tools?\s*(?:and\s*)?(?:technologies|skills)",
        r"skills?\s*(?:section)?",
        r"technologies?\s*used",
    ],
}


def find_skill_evidence(skill: str, raw_text: str) -> list:
    """
    Determine where a skill appears in the resume.
    Returns list of evidence locations.
    """
    evidence = []
    skill_lower = skill.lower()
    text_lower = raw_text.lower()
    
    # Find all occurrences of the skill
    skill_pattern = re.compile(r'\b' + re.escape(skill_lower) + r'\b', re.IGNORECASE)
    matches = list(skill_pattern.finditer(text_lower))
    
    if not matches:
        return ["listed_only"]
    
    # For each match, determine the context
    for match in matches:
        start = max(0, match.start() - 500)
        end = min(len(text_lower), match.end() + 200)
        context = text_lower[start:end]
        
        # Check which section this appears in
        found_section = False
        
        for section, patterns in SECTION_PATTERNS.items():
            for pattern in patterns:
                if re.search(pattern, context, re.IGNORECASE):
                    if section not in evidence:
                        evidence.append(section)
                    found_section = True
                    break
    
    # If no specific section found, mark as listed_only
    if not evidence:
        evidence.append("listed_only")
    
    return evidence


# =============================================================================
# SECTION SIGNAL EXTRACTION
# =============================================================================

def extract_section_signals(raw_text: str) -> dict:
    """
    Detect presence of key resume sections and qualities.
    Returns boolean signals only.
    """
    text_lower = raw_text.lower()
    
    # Check for projects section
    has_projects = bool(re.search(
        r'projects?\s*(?:section|:|$)|'
        r'academic\s*projects?|'
        r'personal\s*projects?|'
        r'(?:built|developed|created)\s+(?:a|an|the)\s+\w+',
        text_lower
    ))
    
    # Check for internship/experience section
    has_internship = bool(re.search(
        r'internship|'
        r'work\s*experience|'
        r'professional\s*experience|'
        r'intern\s+at|'
        r'worked\s+(?:at|with)\s+\w+',
        text_lower
    ))
    
    # Check for measurable outcomes (numbers, percentages, metrics)
    has_metrics = bool(re.search(
        r'\d+\s*%|'
        r'increased\s+.*\d+|'
        r'reduced\s+.*\d+|'
        r'improved\s+.*\d+|'
        r'\d+\s*(?:users?|customers?|clients?)|'
        r'\$\s*\d+|'
        r'\d+x\s+(?:faster|better|improvement)',
        text_lower
    ))
    
    # Check for deployment/production keywords
    has_deployment = bool(re.search(
        r'deploy(?:ed|ment|ing)?|'
        r'production|'
        r'live\s+(?:server|environment|system)|'
        r'launched|'
        r'shipped|'
        r'released\s+to|'
        r'hosted\s+on',
        text_lower
    ))
    
    return {
        "has_projects": has_projects,
        "has_internship": has_internship,
        "has_metrics": has_metrics,
        "has_deployment": has_deployment,
    }


# =============================================================================
# MAIN EVIDENCE MAPPER
# =============================================================================

# Import career snapshot module for enhanced analysis
try:
    from career_snapshot import build_enhanced_snapshot, extract_profile_signals
    SNAPSHOT_AVAILABLE = True
except ImportError:
    SNAPSHOT_AVAILABLE = False


def map_evidence(parsed_resume: dict, outcomes: list = None) -> dict:
    """
    Stage 1 Evidence Mapper: Extract normalized signals from parsed resume.
    
    Input: Parsed resume JSON with skills, experience, education, raw_text
    Output: {
        "normalized_skills": [...],
        "skill_evidence_map": {...},
        "section_signals": {...},
        "enhanced_snapshot": {...},  # NEW: depth-aware skill classification
        "profile_signals": {...}     # NEW: derived career signals
    }
    
    The enhanced_snapshot and profile_signals are read-only data layers
    that provide deeper insight without changing any behavior.
    """
    raw_text = parsed_resume.get("raw_text", "")
    input_skills = parsed_resume.get("skills", [])
    
    # 1. Normalize and filter skills
    normalized_skills = []
    seen = set()
    
    for skill in input_skills:
        if is_valid_skill(skill):
            formatted = format_skill_display(skill)
            normalized = normalize_skill(skill)
            if normalized not in seen:
                seen.add(normalized)
                normalized_skills.append(formatted)
    
    # 2. Map evidence locations for each skill
    skill_evidence_map = {}
    for skill in normalized_skills:
        evidence = find_skill_evidence(skill, raw_text)
        skill_evidence_map[skill] = evidence
    
    # 3. Extract section signals
    section_signals = extract_section_signals(raw_text)
    
    # 4. Build enhanced career snapshot (NEW - read-only data layer)
    enhanced_snapshot = None
    profile_signals = None
    
    if SNAPSHOT_AVAILABLE:
        try:
            snapshot = build_enhanced_snapshot(parsed_resume)
            enhanced_snapshot = snapshot.to_dict()
            
            # Extract profile signals (outcomes optional for now)
            signals = extract_profile_signals(snapshot, outcomes or [])
            profile_signals = signals.to_dict()
        except Exception as e:
            # Log error but don't fail - snapshot is optional
            print(f"[evidence_mapper.py] Snapshot extraction warning: {e}", file=sys.stderr)
    
    result = {
        "normalized_skills": normalized_skills,
        "skill_evidence_map": skill_evidence_map,
        "section_signals": section_signals,
    }
    
    # Include enhanced data if available (read-only, no behavior change)
    if enhanced_snapshot:
        result["enhanced_snapshot"] = enhanced_snapshot
    if profile_signals:
        result["profile_signals"] = profile_signals
    
    return result


def main():
    """CLI interface for evidence mapper."""
    import traceback
    
    try:
        print(f"[evidence_mapper.py] Python: {sys.version}", file=sys.stderr)
        print(f"[evidence_mapper.py] Executable: {sys.executable}", file=sys.stderr)
        print(f"[evidence_mapper.py] Reading input from stdin", file=sys.stderr)
        
        # Read JSON from stdin
        stdin_data = sys.stdin.read()
        print(f"[evidence_mapper.py] Stdin length: {len(stdin_data)} chars", file=sys.stderr)
        
        if not stdin_data:
            print(json.dumps({"error": "No input received on stdin"}))
            sys.exit(1)
        
        parsed_resume = json.loads(stdin_data)
        print(f"[evidence_mapper.py] Parsed JSON from stdin successfully", file=sys.stderr)
        
        result = map_evidence(parsed_resume)
        print(f"[evidence_mapper.py] Evidence mapping completed", file=sys.stderr)
        print(f"[evidence_mapper.py] Normalized skills: {len(result['normalized_skills'])}", file=sys.stderr)
        print(json.dumps(result))
        
    except json.JSONDecodeError as e:
        print(f"[evidence_mapper.py] JSON decode error: {e}", file=sys.stderr)
        print(traceback.format_exc(), file=sys.stderr)
        print(json.dumps({"error": f"Invalid JSON: {str(e)}"}))
        sys.exit(1)
    except Exception as e:
        print(f"[evidence_mapper.py] EXCEPTION: {str(e)}", file=sys.stderr)
        print(traceback.format_exc(), file=sys.stderr)
        print(json.dumps({"error": f"Error: {str(e)}"}))
        sys.exit(1)


if __name__ == "__main__":
    main()
