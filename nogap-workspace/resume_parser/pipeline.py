#!/usr/bin/env python3
"""
Full Resume Pipeline: Parse + Evidence Map + Bottleneck Analysis + Strategy Selection

Usage: python pipeline.py <resume_file_path>
Output: Combined JSON with parsed data, evidence signals, bottleneck analysis, and strategy
"""

import sys
import json
import os

# Add current directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from parser import parse_resume
from evidence_mapper import map_evidence
from bottleneck_analyzer import analyze_bottlenecks
from strategy_selector import select_strategy_and_action


def run_pipeline(file_path: str, include_stage2: bool = True, include_stage3: bool = True) -> dict:
    """Run full pipeline: parse resume, map evidence, analyze bottlenecks, select strategy."""
    
    # Stage 0: Parse resume
    parsed = parse_resume(file_path)
    
    if parsed is None:
        return {"error": "Failed to parse resume"}
    
    # Stage 1: Map evidence
    evidence = map_evidence(parsed)
    
    # Stage 2: Bottleneck analysis (optional)
    bottleneck_analysis = None
    if include_stage2:
        bottleneck_analysis = analyze_bottlenecks(evidence)
    
    # Stage 3: Strategy selection (requires Stage 2)
    strategy_selection = None
    if include_stage3 and bottleneck_analysis:
        section_signals = evidence.get("section_signals")
        strategy_selection = select_strategy_and_action(bottleneck_analysis, section_signals)
    
    # Combine outputs
    result = {
        "parsed": {
            "skills_raw": parsed.get("skills", []),
            "experience_raw": parsed.get("experience", []),
            "education": parsed.get("education", []),
            "total_experience": parsed.get("total_experience"),
        },
        "evidence": evidence,
    }
    
    if bottleneck_analysis:
        result["bottleneck_analysis"] = bottleneck_analysis
    
    if strategy_selection:
        result["strategy_selection"] = strategy_selection
    
    return result


def main():
    if len(sys.argv) < 2:
        print(json.dumps({"error": "Usage: python pipeline.py <resume_file_path>"}))
        sys.exit(1)
    
    file_path = sys.argv[1]
    
    if not os.path.isfile(file_path):
        print(json.dumps({"error": f"File not found: {file_path}"}))
        sys.exit(1)
    
    result = run_pipeline(file_path)
    print(json.dumps(result))


if __name__ == "__main__":
    main()
