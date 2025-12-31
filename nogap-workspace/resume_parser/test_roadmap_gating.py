#!/usr/bin/env python3
"""
Test Roadmap Gating

Validates that roadmap generation is strictly gated by strategy_state.
Only EXECUTE state should allow roadmap generation.
"""

import json
import sys
from agent_loop import AgentSession, StrategyRecord, StrategyState
from roadmap_generator import check_roadmap_eligibility, generate_roadmap


def create_test_session(strategy_state: str, strategy_name: str = "ResumeOptimization") -> dict:
    """Create a test session with the specified strategy state."""
    return {
        "stage1_evidence": {"years_of_experience": 5, "skill_count": 10},
        "stage2_bottleneck": {"primary_issue": "weak_skills"},
        "stage3_strategy": {
            "strategy": strategy_name,
            "action": "Test action",
            "confidence": 0.7
        },
        "current_strategy": {
            "strategy": strategy_name,
            "initial_confidence": 0.6,
            "current_confidence": 0.7,
            "outcomes": ["interview"],
            "failed": False,
            "strategy_state": strategy_state
        },
        "strategy_history": [],
        "loop_iteration": 1,
        "explanation_log": []
    }


def test_eligibility_explore_state():
    """Test that EXPLORE state blocks roadmap generation."""
    print("\n[TEST 1] Roadmap blocked in EXPLORE state")
    print("=" * 60)
    
    session = create_test_session(StrategyState.EXPLORE)
    result = check_roadmap_eligibility(session)
    
    print(f"Strategy State: {result['current_state']}")
    print(f"Eligible: {result['eligible']}")
    print(f"Reason: {result['reason']}")
    
    assert not result["eligible"], "EXPLORE state should block roadmap"
    assert "EXPLORE" in result["reason"] or "not eligible" in result["reason"]
    print("[PASS] EXPLORE state correctly blocks roadmap generation")


def test_eligibility_validate_state():
    """Test that VALIDATE state blocks roadmap generation."""
    print("\n[TEST 2] Roadmap blocked in VALIDATE state")
    print("=" * 60)
    
    session = create_test_session(StrategyState.VALIDATE)
    result = check_roadmap_eligibility(session)
    
    print(f"Strategy State: {result['current_state']}")
    print(f"Eligible: {result['eligible']}")
    print(f"Reason: {result['reason']}")
    
    assert not result["eligible"], "VALIDATE state should block roadmap"
    assert "VALIDATE" in result["reason"] or "not eligible" in result["reason"]
    print("[PASS] VALIDATE state correctly blocks roadmap generation")


def test_eligibility_execute_state():
    """Test that EXECUTE state allows roadmap generation."""
    print("\n[TEST 3] Roadmap allowed in EXECUTE state")
    print("=" * 60)
    
    session = create_test_session(StrategyState.EXECUTE)
    result = check_roadmap_eligibility(session)
    
    print(f"Strategy State: {result['current_state']}")
    print(f"Eligible: {result['eligible']}")
    print(f"Reason: {result['reason']}")
    
    assert result["eligible"], "EXECUTE state should allow roadmap"
    assert "EXECUTE" in result["reason"]
    print("[PASS] EXECUTE state correctly allows roadmap generation")


def test_eligibility_reconsider_state():
    """Test that RECONSIDER state blocks roadmap generation."""
    print("\n[TEST 4] Roadmap blocked in RECONSIDER state")
    print("=" * 60)
    
    session = create_test_session(StrategyState.RECONSIDER)
    result = check_roadmap_eligibility(session)
    
    print(f"Strategy State: {result['current_state']}")
    print(f"Eligible: {result['eligible']}")
    print(f"Reason: {result['reason']}")
    
    assert not result["eligible"], "RECONSIDER state should block roadmap"
    assert "RECONSIDER" in result["reason"] or "not eligible" in result["reason"]
    print("[PASS] RECONSIDER state correctly blocks roadmap generation")


def test_full_roadmap_generation_execute():
    """Test that full roadmap generation works in EXECUTE state."""
    print("\n[TEST 5] Full roadmap generation in EXECUTE state")
    print("=" * 60)
    
    session = create_test_session(StrategyState.EXECUTE, "ResumeOptimization")
    enhanced_snapshot = {
        "years_of_experience": 5,
        "skill_count": 10,
        "education_level": "Bachelor"
    }
    profile_signals = {
        "application_volume": "medium",
        "response_quality": "high"
    }
    
    result = generate_roadmap(session, enhanced_snapshot, profile_signals)
    
    if not result.get("eligible"):
        print(f"[FAIL] Expected eligible=True but got: {result}")
        raise AssertionError("Roadmap should be eligible in EXECUTE state")
    
    if "error" in result:
        print(f"[FAIL] Got error: {result['error']}")
        raise AssertionError(f"Roadmap generation failed: {result['error']}")
    
    roadmap = result["roadmap"]
    print(f"Strategy: {roadmap['strategy']}")
    print(f"Estimated Duration: {roadmap['estimated_completion_days']} days")
    print(f"Number of Actions: {len(roadmap['actions'])}")
    print(f"\nActions:")
    for i, action in enumerate(roadmap['actions'][:3], 1):  # Show first 3
        print(f"  {i}. {action['title']} (deadline: {action['deadline_days']} days, {action['priority']})")
    
    assert roadmap['strategy'] == "ResumeOptimization"
    assert len(roadmap['actions']) > 0
    assert roadmap['estimated_completion_days'] > 0
    print("\n[PASS] Full roadmap generated successfully in EXECUTE state")


def test_full_roadmap_generation_explore_blocked():
    """Test that full roadmap generation is blocked in EXPLORE state."""
    print("\n[TEST 6] Full roadmap generation blocked in EXPLORE state")
    print("=" * 60)
    
    session = create_test_session(StrategyState.EXPLORE, "ResumeOptimization")
    enhanced_snapshot = {
        "years_of_experience": 5,
        "skill_count": 10
    }
    profile_signals = {
        "application_volume": "medium"
    }
    
    result = generate_roadmap(session, enhanced_snapshot, profile_signals)
    
    if result.get("eligible"):
        print(f"[FAIL] Expected blocked but got eligible roadmap: {result}")
        raise AssertionError("Should have blocked EXPLORE state")
    
    if "error" not in result:
        print(f"[FAIL] Expected error field but got: {result}")
        raise AssertionError("Should have error message for EXPLORE state")
    
    error_msg = result["error"]
    reason = result.get("reason", "")
    print(f"Error (expected): {error_msg}")
    print(f"Reason: {reason}")
    
    assert not result["eligible"], "Should not be eligible in EXPLORE state"
    assert "EXPLORE" in reason or "not eligible" in reason
    print("[PASS] Roadmap generation correctly blocked in EXPLORE state")


def test_all_strategy_types_in_execute():
    """Test that all strategy types can generate roadmaps in EXECUTE state."""
    print("\n[TEST 7] All strategy types in EXECUTE state")
    print("=" * 60)
    
    strategies = ["ResumeOptimization", "SkillGapPatch", "RoleShift", "HoldPosition"]
    
    for strategy in strategies:
        print(f"\n  Testing: {strategy}")
        session = create_test_session(StrategyState.EXECUTE, strategy)
        enhanced_snapshot = {"years_of_experience": 5}
        profile_signals = {"application_volume": "medium"}
        
        result = generate_roadmap(session, enhanced_snapshot, profile_signals)
        
        if not result.get("eligible"):
            print(f"    -> [FAIL] {strategy} not eligible: {result}")
            raise AssertionError(f"{strategy} should be eligible in EXECUTE state")
        
        if "error" in result:
            print(f"    -> [FAIL] {strategy} error: {result['error']}")
            raise AssertionError(f"{strategy} generation failed")
        
        roadmap = result["roadmap"]
        print(f"    -> Generated {len(roadmap['actions'])} actions, {roadmap['estimated_completion_days']} days")
        assert roadmap['strategy'] == strategy
    
    print("\n[PASS] All strategy types successfully generated roadmaps")


def run_all_tests():
    """Run all roadmap gating tests."""
    print("=" * 60)
    print("ROADMAP GATING TEST SUITE")
    print("=" * 60)
    print("Testing that roadmap generation is strictly gated by strategy_state")
    print("Only EXECUTE state should allow roadmap generation")
    
    tests = [
        test_eligibility_explore_state,
        test_eligibility_validate_state,
        test_eligibility_execute_state,
        test_eligibility_reconsider_state,
        test_full_roadmap_generation_execute,
        test_full_roadmap_generation_explore_blocked,
        test_all_strategy_types_in_execute,
    ]
    
    passed = 0
    failed = 0
    
    for test in tests:
        try:
            test()
            passed += 1
        except AssertionError as e:
            print(f"\n[FAIL] {test.__name__}: {e}")
            failed += 1
        except Exception as e:
            print(f"\n[ERROR] {test.__name__}: {e}")
            failed += 1
    
    print("\n" + "=" * 60)
    print(f"TEST RESULTS: {passed} passed, {failed} failed")
    print("=" * 60)
    
    return failed == 0


if __name__ == "__main__":
    success = run_all_tests()
    sys.exit(0 if success else 1)
