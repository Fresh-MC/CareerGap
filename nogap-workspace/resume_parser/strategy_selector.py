#!/usr/bin/env python3
"""
Evidence Mapper - Stage 3: Strategy Selection

Takes Stage 2 output and selects exactly ONE strategy with ONE action.
NO alternatives, NO explanations - single actionable commitment only.
"""

import json
import sys

# =============================================================================
# STRATEGY DEFINITIONS
# =============================================================================

STRATEGIES = {
    "ResumeOptimization",
    "SkillGapPatch",
    "RoleShift",
    "HoldPosition",
}

# =============================================================================
# ACTION TEMPLATES
# =============================================================================

# Actions mapped by (strategy, dominant_issue) for specificity
ACTION_TEMPLATES = {
    # ResumeOptimization actions
    ("ResumeOptimization", "evidence_depth"): {
        "weak": "Rewrite the primary project description to include problem statement, approach, tools used, and quantifiable outcome.",
        "missing": "Add a Projects section with one completed project including problem, solution, and measurable result.",
    },
    ("ResumeOptimization", "positioning"): {
        "weak": "Add context to top 3 listed skills by linking each to a specific project or experience where it was applied.",
        "missing": "Create skill-evidence links by adding brief descriptions of how each skill was used in projects or coursework.",
    },
    ("ResumeOptimization", "outcome_visibility"): {
        "weak": "Add metrics to the primary project or experience entry (e.g., performance improvement %, users served, data processed).",
        "missing": "Quantify one achievement in the experience or project section with a specific number or percentage.",
    },
    # SkillGapPatch actions
    ("SkillGapPatch", "skill_alignment"): {
        "weak": "Identify the top missing primary skill for the target role and add it through a focused micro-project or certification.",
        "missing": "Complete one foundational skill course for the target role and add the credential to the resume.",
    },
    # RoleShift actions
    ("RoleShift", "experience_strength"): {
        "weak": "Reframe existing project work as professional experience by emphasizing deliverables and stakeholder impact.",
        "missing": "Pivot target role to one that values project-based evidence over formal work experience.",
    },
    # HoldPosition actions
    ("HoldPosition", None): {
        "strong": "Maintain current resume positioning and proceed to application phase.",
    },
}

# Fallback actions by strategy (when specific issue not matched)
FALLBACK_ACTIONS = {
    "ResumeOptimization": "Restructure the resume to highlight evidence of applied skills in project and experience sections.",
    "SkillGapPatch": "Add one in-demand skill for the target role through a verifiable project or credential.",
    "RoleShift": "Adjust target role to better align with current evidence profile.",
    "HoldPosition": "Proceed with current resume positioning.",
}

# =============================================================================
# CONFIDENCE ESTIMATION
# =============================================================================

# Base confidence by strategy (conservative)
BASE_CONFIDENCE = {
    "ResumeOptimization": 0.70,
    "SkillGapPatch": 0.55,
    "RoleShift": 0.45,
    "HoldPosition": 0.85,
}

# Confidence adjustments based on bottleneck severity
CONFIDENCE_ADJUSTMENTS = {
    "strong": 0.10,
    "weak": 0.00,
    "missing": -0.10,
}


def calculate_confidence(strategy: str, bottlenecks: dict, dominant_issue: str) -> float:
    """
    Calculate confidence score (0.0-1.0) for the selected strategy.
    Conservative estimation based on bottleneck severity.
    """
    base = BASE_CONFIDENCE.get(strategy, 0.50)
    
    if not dominant_issue:
        # All strong - high confidence in HoldPosition
        return min(0.90, base + 0.05)
    
    # Adjust based on severity of dominant issue
    severity = bottlenecks.get(dominant_issue, "weak")
    adjustment = CONFIDENCE_ADJUSTMENTS.get(severity, 0.0)
    
    # Count other weak/missing bottlenecks (reduces confidence)
    other_issues = sum(
        1 for k, v in bottlenecks.items()
        if k != dominant_issue and v in ("weak", "missing")
    )
    
    # Each additional issue reduces confidence slightly
    other_penalty = other_issues * 0.05
    
    confidence = base + adjustment - other_penalty
    
    # Clamp to valid range
    return round(max(0.20, min(0.95, confidence)), 2)


# =============================================================================
# STRATEGY SELECTION
# =============================================================================

def select_strategy(dominant_issue: str, bottlenecks: dict, section_signals: dict = None) -> str:
    """
    Select exactly one strategy based on dominant issue.
    Follows strict mapping rules from requirements.
    """
    if dominant_issue is None:
        return "HoldPosition"
    
    # Rule: evidence_depth or positioning → ResumeOptimization
    if dominant_issue in ("evidence_depth", "positioning"):
        return "ResumeOptimization"
    
    # Rule: outcome_visibility → ResumeOptimization (make outcomes visible)
    if dominant_issue == "outcome_visibility":
        return "ResumeOptimization"
    
    # Rule: skill_alignment → SkillGapPatch
    if dominant_issue == "skill_alignment":
        return "SkillGapPatch"
    
    # Rule: experience_strength with no internship/projects → RoleShift
    if dominant_issue == "experience_strength":
        # Check if we have any experience signals
        if section_signals:
            has_internship = section_signals.get("has_internship", False)
            has_projects = section_signals.get("has_projects", False)
            if not has_internship and not has_projects:
                return "RoleShift"
        # If weak but has some evidence, try optimization first
        severity = bottlenecks.get("experience_strength", "missing")
        if severity == "missing":
            return "RoleShift"
        else:
            return "ResumeOptimization"
    
    # Fallback
    return "ResumeOptimization"


def get_action(strategy: str, dominant_issue: str, bottlenecks: dict) -> str:
    """
    Get the single concrete action for the selected strategy.
    Action must be immediately executable.
    """
    if dominant_issue is None:
        # HoldPosition case
        return ACTION_TEMPLATES.get(("HoldPosition", None), {}).get("strong", FALLBACK_ACTIONS["HoldPosition"])
    
    # Get severity of dominant issue
    severity = bottlenecks.get(dominant_issue, "weak")
    
    # Try to get specific action for (strategy, issue) combination
    action_map = ACTION_TEMPLATES.get((strategy, dominant_issue), {})
    action = action_map.get(severity)
    
    if action:
        return action
    
    # Try weak as fallback severity
    action = action_map.get("weak")
    if action:
        return action
    
    # Use strategy fallback
    return FALLBACK_ACTIONS.get(strategy, "Review and optimize resume content.")


# =============================================================================
# MAIN STAGE 3 SELECTOR
# =============================================================================

def select_strategy_and_action(stage2_output: dict, section_signals: dict = None) -> dict:
    """
    Stage 3: Select strategy, action, and estimate confidence.
    
    Input: Stage 2 output with implied_role, bottlenecks, dominant_issue, justification
    Output: {
        "strategy": "...",
        "action": "...",
        "confidence": 0.XX
    }
    """
    bottlenecks = stage2_output.get("bottlenecks", {})
    dominant_issue = stage2_output.get("dominant_issue")
    
    # 1. Select strategy
    strategy = select_strategy(dominant_issue, bottlenecks, section_signals)
    
    # 2. Get concrete action
    action = get_action(strategy, dominant_issue, bottlenecks)
    
    # 3. Estimate confidence
    confidence = calculate_confidence(strategy, bottlenecks, dominant_issue)
    
    return {
        "strategy": strategy,
        "action": action,
        "confidence": confidence,
    }


def main():
    """CLI interface for Stage 3 selector."""
    import traceback
    
    try:
        print(f"[strategy_selector.py] Python: {sys.version}", file=sys.stderr)
        print(f"[strategy_selector.py] Executable: {sys.executable}", file=sys.stderr)
        print(f"[strategy_selector.py] Reading input from stdin", file=sys.stderr)
        
        # Read combined JSON from stdin
        stdin_data = sys.stdin.read()
        print(f"[strategy_selector.py] Stdin length: {len(stdin_data)} chars", file=sys.stderr)
        
        if not stdin_data:
            print(json.dumps({"error": "No input received on stdin"}))
            sys.exit(1)
        
        combined_input = json.loads(stdin_data)
        print(f"[strategy_selector.py] Parsed combined JSON from stdin successfully", file=sys.stderr)
        
        # Extract stage2 and stage1 from combined input
        stage2_output = combined_input.get("stage2")
        stage1_output = combined_input.get("stage1")
        
        if not stage2_output:
            print(json.dumps({"error": "Missing 'stage2' in input"}))
            sys.exit(1)
        
        print(f"[strategy_selector.py] Extracted stage2 and stage1 data", file=sys.stderr)
        
        # Extract section_signals from stage1 if available
        section_signals = None
        if stage1_output:
            section_signals = stage1_output.get("section_signals")
            print(f"[strategy_selector.py] Section signals available: {section_signals is not None}", file=sys.stderr)
        
        result = select_strategy_and_action(stage2_output, section_signals)
        print(f"[strategy_selector.py] Strategy selection completed", file=sys.stderr)
        print(f"[strategy_selector.py] Selected strategy: {result.get('strategy')}", file=sys.stderr)
        print(json.dumps(result))
        
    except json.JSONDecodeError as e:
        print(f"[strategy_selector.py] JSON decode error: {e}", file=sys.stderr)
        print(traceback.format_exc(), file=sys.stderr)
        print(json.dumps({"error": f"Invalid JSON: {str(e)}"}))
        sys.exit(1)
    except Exception as e:
        print(f"[strategy_selector.py] EXCEPTION: {str(e)}", file=sys.stderr)
        print(traceback.format_exc(), file=sys.stderr)
        print(json.dumps({"error": f"Error: {str(e)}"}))
        sys.exit(1)


if __name__ == "__main__":
    main()
