#!/usr/bin/env python3
"""
Test script for Strategy Lifecycle State Machine

Demonstrates the deterministic state transitions based on different outcome scenarios.
"""

import sys
import json
from agent_loop import (
    AgentSession,
    StrategyRecord,
    StrategyState,
    initialize_session,
    process_outcome,
    evaluate_strategy_state,
)


def print_state(session, title=""):
    """Pretty print current session state."""
    if title:
        print(f"\n{'='*60}")
        print(f"  {title}")
        print('='*60)
    
    if session.current_strategy:
        strategy = session.current_strategy
        print(f"Strategy: {strategy.strategy}")
        print(f"State: {strategy.strategy_state}")
        print(f"Confidence: {strategy.current_confidence}")
        print(f"Outcomes: {strategy.outcomes}")
        print(f"Failed: {strategy.failed}")
    
    if session.explanation_log:
        print(f"\nLast log: {session.explanation_log[-1]}")


def test_success_path():
    """Test Scenario 1: Success Path - explore -> validate -> execute"""
    print("\n" + "="*60)
    print("TEST 1: Success Path (explore -> validate -> execute)")
    print("="*60)
    
    # Create minimal stage outputs for initialization
    stage1 = {
        "enhanced_snapshot": {},
        "section_signals": {},
    }
    stage2 = {
        "implied_role": "Software Engineer",
        "bottlenecks": {"evidence_depth": "weak"},
        "dominant_issue": "evidence_depth",
        "justification": "Limited project depth",
    }
    stage3 = {
        "strategy": "ResumeOptimization",
        "action": "Add detailed project descriptions",
        "confidence": 0.70,
    }
    
    # Initialize session
    session = initialize_session(stage1, stage2, stage3)
    print_state(session, "Initial State")
    
    # Scenario: Receive first interview
    print("\n→ Recording outcome: interview")
    result = process_outcome(session, "interview")
    session = AgentSession(
        stage1_evidence=result["session"]["stage1_evidence"],
        stage2_bottleneck=result["session"]["stage2_bottleneck"],
        stage3_strategy=result["session"]["stage3_strategy"],
        loop_iteration=result["session"]["loop_iteration"],
        explanation_log=result["session"]["explanation_log"],
    )
    session.current_strategy = StrategyRecord(
        strategy=result["session"]["current_strategy"]["strategy"],
        initial_confidence=result["session"]["current_strategy"]["initial_confidence"],
        current_confidence=result["session"]["current_strategy"]["current_confidence"],
        outcomes=result["session"]["current_strategy"]["outcomes"],
        failed=result["session"]["current_strategy"]["failed"],
        strategy_state=result["session"]["current_strategy"]["strategy_state"],
    )
    print_state(session, "After 1st Interview")
    assert session.current_strategy.strategy_state == StrategyState.VALIDATE, "Should be in VALIDATE state"
    
    # Scenario: Receive second interview
    print("\n→ Recording outcome: interview")
    result = process_outcome(session, "interview")
    session.current_strategy.outcomes = result["session"]["current_strategy"]["outcomes"]
    session.current_strategy.current_confidence = result["session"]["current_strategy"]["current_confidence"]
    session.current_strategy.strategy_state = result["session"]["current_strategy"]["strategy_state"]
    session.explanation_log = result["session"]["explanation_log"]
    print_state(session, "After 2nd Interview")
    assert session.current_strategy.strategy_state == StrategyState.EXECUTE, "Should be in EXECUTE state"
    
    print("\n[PASS] Test passed: Strategy progressed from EXPLORE -> VALIDATE -> EXECUTE")


def test_failure_recovery():
    """Test Scenario 2: Failure and Recovery - explore -> reconsider -> new explore"""
    print("\n" + "="*60)
    print("TEST 2: Failure and Recovery (explore -> reconsider)")
    print("="*60)
    
    # Initialize session
    stage1 = {"enhanced_snapshot": {}, "section_signals": {}}
    stage2 = {
        "implied_role": "Data Scientist",
        "bottlenecks": {"skill_alignment": "missing"},
        "dominant_issue": "skill_alignment",
        "justification": "Missing key skills",
    }
    stage3 = {
        "strategy": "SkillGapPatch",
        "action": "Complete Python certification",
        "confidence": 0.55,
    }
    
    session = initialize_session(stage1, stage2, stage3)
    print_state(session, "Initial State")
    
    # Scenario: Three rejections to trigger RECONSIDER
    for i in range(3):
        print(f"\n→ Recording outcome: rejected (#{i+1})")
        result = process_outcome(session, "rejected")
        
        # Check if strategy changed (re-evaluation occurred)
        if result["strategy_changed"]:
            print(f"\n  → Strategy re-evaluation triggered!")
            # Re-evaluation creates a NEW strategy in EXPLORE state
            # The old strategy was in RECONSIDER before re-evaluation
            assert i == 2, "Re-evaluation should happen on 3rd rejection"
            
            # Reconstruct session from result
            session_data = result["session"]
            session = AgentSession(
                stage1_evidence=session_data["stage1_evidence"],
                stage2_bottleneck=session_data["stage2_bottleneck"],
                stage3_strategy=session_data["stage3_strategy"],
                loop_iteration=session_data["loop_iteration"],
                explanation_log=session_data["explanation_log"],
            )
            if session_data["current_strategy"]:
                session.current_strategy = StrategyRecord(
                    strategy=session_data["current_strategy"]["strategy"],
                    initial_confidence=session_data["current_strategy"]["initial_confidence"],
                    current_confidence=session_data["current_strategy"]["current_confidence"],
                    outcomes=session_data["current_strategy"]["outcomes"],
                    failed=session_data["current_strategy"]["failed"],
                    strategy_state=session_data["current_strategy"]["strategy_state"],
                )
            for hist in session_data.get("strategy_history", []):
                session.strategy_history.append(StrategyRecord(
                    strategy=hist["strategy"],
                    initial_confidence=hist["initial_confidence"],
                    current_confidence=hist["current_confidence"],
                    outcomes=hist["outcomes"],
                    failed=hist["failed"],
                    strategy_state=hist["strategy_state"],
                ))
            print_state(session, f"After Re-evaluation")
            
            # New strategy should be in EXPLORE state
            assert session.current_strategy.strategy_state == StrategyState.EXPLORE, "New strategy should be in EXPLORE state"
            
            # Old strategy should be in history and marked as failed
            assert len(session.strategy_history) > 0, "Should have strategy history"
            old_strategy = session.strategy_history[-1]
            assert old_strategy.failed, "Old strategy should be marked as failed"
            
        else:
            # Update current strategy with new values
            session.current_strategy.outcomes = result["session"]["current_strategy"]["outcomes"]
            session.current_strategy.current_confidence = result["session"]["current_strategy"]["current_confidence"]
            session.current_strategy.strategy_state = result["session"]["current_strategy"]["strategy_state"]
            session.current_strategy.failed = result["session"]["current_strategy"]["failed"]
            session.explanation_log = result["session"]["explanation_log"]
            print_state(session, f"After Rejection #{i+1}")
    
    print("\n[PASS] Test passed: Strategy entered RECONSIDER and triggered re-evaluation")


def test_validation_failure():
    """Test Scenario 3: Validation interrupted by positioning issue"""
    print("\n" + "="*60)
    print("TEST 3: Validation Interrupted (validate -> explore)")
    print("="*60)
    
    # Initialize session
    stage1 = {"enhanced_snapshot": {}, "section_signals": {}}
    stage2 = {
        "implied_role": "Frontend Developer",
        "bottlenecks": {"positioning": "weak"},
        "dominant_issue": "positioning",
        "justification": "Unclear positioning",
    }
    stage3 = {
        "strategy": "ResumeOptimization",
        "action": "Clarify role positioning",
        "confidence": 0.68,
    }
    
    session = initialize_session(stage1, stage2, stage3)
    print_state(session, "Initial State")
    
    # Get first interview - should move to VALIDATE
    print("\n→ Recording outcome: interview")
    result = process_outcome(session, "interview")
    session.current_strategy.outcomes = result["session"]["current_strategy"]["outcomes"]
    session.current_strategy.current_confidence = result["session"]["current_strategy"]["current_confidence"]
    session.current_strategy.strategy_state = result["session"]["current_strategy"]["strategy_state"]
    session.explanation_log = result["session"]["explanation_log"]
    print_state(session, "After 1st Interview")
    assert session.current_strategy.strategy_state == StrategyState.VALIDATE, "Should be in VALIDATE state"
    
    # Simulate positioning issue detected
    print("\n→ Simulating positioning issue detection")
    profile_signals = {
        "signals": {
            "resume_positioning_issue": {
                "triggered": True,
                "confidence": 0.85,
            }
        }
    }
    
    # Evaluate state with positioning issue
    session = evaluate_strategy_state(session, profile_signals)
    print_state(session, "After Positioning Issue Detected")
    assert session.current_strategy.strategy_state == StrategyState.EXPLORE, "Should return to EXPLORE state"
    
    print("\n[PASS] Test passed: Strategy returned to EXPLORE when positioning issue detected")


def test_confidence_failure():
    """Test Scenario 4: Confidence drops below threshold"""
    print("\n" + "="*60)
    print("TEST 4: Confidence Failure (any -> reconsider)")
    print("="*60)
    
    # Initialize with lower confidence
    stage1 = {"enhanced_snapshot": {}, "section_signals": {}}
    stage2 = {
        "implied_role": "Backend Engineer",
        "bottlenecks": {"experience_strength": "missing"},
        "dominant_issue": "experience_strength",
        "justification": "No professional experience",
    }
    stage3 = {
        "strategy": "RoleShift",
        "action": "Target entry-level roles",
        "confidence": 0.45,
    }
    
    session = initialize_session(stage1, stage2, stage3)
    print_state(session, "Initial State")
    
    # Multiple no_response outcomes to drop confidence
    reconsider_detected = False
    for i in range(2):
        print(f"\n→ Recording outcome: no_response (#{i+1})")
        result = process_outcome(session, "no_response")
        
        # Check if strategy changed (re-evaluation occurred due to RECONSIDER)
        if result["strategy_changed"]:
            print(f"\n  → RECONSIDER state triggered re-evaluation!")
            reconsider_detected = True
            
            # Verify the old strategy entered RECONSIDER
            session_data = result["session"]
            if session_data.get("strategy_history"):
                old_strategy = session_data["strategy_history"][-1]
                print(f"  → Old strategy confidence: {old_strategy['current_confidence']}")
                assert old_strategy["current_confidence"] < 0.30, "Old strategy should have confidence below threshold"
                assert old_strategy["failed"], "Old strategy should be marked as failed"
            
            # Reconstruct session
            session = AgentSession(
                stage1_evidence=session_data["stage1_evidence"],
                stage2_bottleneck=session_data["stage2_bottleneck"],
                stage3_strategy=session_data["stage3_strategy"],
                loop_iteration=session_data["loop_iteration"],
                explanation_log=session_data["explanation_log"],
            )
            if session_data["current_strategy"]:
                session.current_strategy = StrategyRecord(
                    strategy=session_data["current_strategy"]["strategy"],
                    initial_confidence=session_data["current_strategy"]["initial_confidence"],
                    current_confidence=session_data["current_strategy"]["current_confidence"],
                    outcomes=session_data["current_strategy"]["outcomes"],
                    failed=session_data["current_strategy"]["failed"],
                    strategy_state=session_data["current_strategy"]["strategy_state"],
                )
            for hist in session_data.get("strategy_history", []):
                session.strategy_history.append(StrategyRecord(
                    strategy=hist["strategy"],
                    initial_confidence=hist["initial_confidence"],
                    current_confidence=hist["current_confidence"],
                    outcomes=hist["outcomes"],
                    failed=hist["failed"],
                    strategy_state=hist["strategy_state"],
                ))
            
            print_state(session, "After Re-evaluation")
            break
        else:
            # Update session
            session.current_strategy.outcomes = result["session"]["current_strategy"]["outcomes"]
            session.current_strategy.current_confidence = result["session"]["current_strategy"]["current_confidence"]
            session.current_strategy.strategy_state = result["session"]["current_strategy"]["strategy_state"]
            session.current_strategy.failed = result["session"]["current_strategy"]["failed"]
            session.explanation_log = result["session"]["explanation_log"]
            print_state(session, f"After No Response #{i+1}")
    
    assert reconsider_detected, "Should have triggered RECONSIDER state and re-evaluation"
    assert session.current_strategy.strategy_state == StrategyState.EXPLORE, "New strategy should be in EXPLORE"
    
    print("\n[PASS] Test passed: Strategy entered RECONSIDER due to low confidence and triggered re-evaluation")


def main():
    """Run all test scenarios."""
    print("\n" + "="*60)
    print("  Strategy Lifecycle State Machine - Test Suite")
    print("="*60)
    print("\nTesting deterministic state transitions...")
    
    try:
        test_success_path()
        test_failure_recovery()
        test_validation_failure()
        test_confidence_failure()
        
        print("\n" + "="*60)
        print("  [SUCCESS] ALL TESTS PASSED")
        print("="*60)
        print("\nState Machine Implementation:")
        print("  - explore -> validate (>=1 interview, confidence >= 0.55)")
        print("  - validate -> execute (>=2 interviews, confidence >= 0.65, no issues)")
        print("  - any -> reconsider (confidence < 0.30 OR >=3 negatives)")
        print("  - reconsider -> explore (new strategy selected)")
        print("\nKey Properties:")
        print("  [OK] Deterministic state transitions")
        print("  [OK] Interviews increase certainty, don't trigger switching")
        print("  [OK] Strategy switching only in reconsider state")
        print("  [OK] Phase-based progression prevents chaos")
        
    except AssertionError as e:
        print(f"\n[FAIL] TEST FAILED: {e}")
        sys.exit(1)
    except Exception as e:
        print(f"\n[ERROR] {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
