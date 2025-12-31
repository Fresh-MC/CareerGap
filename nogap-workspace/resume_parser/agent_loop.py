#!/usr/bin/env python3
"""
Agentic Loop Controller

Handles outcome feedback, strategy re-evaluation, and demo explanations.
Completes the agentic loop for the 3-stage career system.
"""

import json
import sys
from dataclasses import dataclass, field
from typing import Optional, List
from enum import Enum

# Import stage functions
from bottleneck_analyzer import analyze_bottlenecks
from strategy_selector import select_strategy_and_action

# Import career snapshot for profile signal extraction (optional, read-only)
try:
    from career_snapshot import build_enhanced_snapshot, extract_profile_signals
    SNAPSHOT_AVAILABLE = True
except ImportError:
    SNAPSHOT_AVAILABLE = False

# =============================================================================
# TYPES
# =============================================================================

class Outcome(Enum):
    NO_RESPONSE = "no_response"
    REJECTED = "rejected"
    INTERVIEW = "interview"


class StrategyState(str, Enum):
    """
    Strategy Lifecycle States.
    
    State meanings:
    - EXPLORE: Strategy just selected, insufficient evidence to validate
    - VALIDATE: Strategy showing positive signals (received interviews)
    - EXECUTE: Strategy validated and locked (ready for execution/roadmap)
    - RECONSIDER: Strategy invalidated (triggers re-evaluation)
    """
    EXPLORE = "explore"
    VALIDATE = "validate"
    EXECUTE = "execute"
    RECONSIDER = "reconsider"


@dataclass
class StrategyRecord:
    """Record of a strategy attempt with outcomes."""
    strategy: str
    initial_confidence: float
    current_confidence: float
    outcomes: List[str] = field(default_factory=list)
    failed: bool = False
    strategy_state: str = StrategyState.EXPLORE  # State machine field
    
    def to_dict(self) -> dict:
        return {
            "strategy": self.strategy,
            "initial_confidence": self.initial_confidence,
            "current_confidence": self.current_confidence,
            "outcomes": self.outcomes,
            "failed": self.failed,
            "strategy_state": self.strategy_state,
        }


@dataclass
class AgentSession:
    """Complete agent session state."""
    # Stage outputs
    stage1_evidence: dict
    stage2_bottleneck: dict
    stage3_strategy: dict
    
    # Strategy tracking
    current_strategy: Optional[StrategyRecord] = None
    strategy_history: List[StrategyRecord] = field(default_factory=list)
    
    # Loop state
    loop_iteration: int = 0
    explanation_log: List[str] = field(default_factory=list)
    
    def to_dict(self) -> dict:
        return {
            "stage1_evidence": self.stage1_evidence,
            "stage2_bottleneck": self.stage2_bottleneck,
            "stage3_strategy": self.stage3_strategy,
            "current_strategy": self.current_strategy.to_dict() if self.current_strategy else None,
            "strategy_history": [s.to_dict() for s in self.strategy_history],
            "loop_iteration": self.loop_iteration,
            "explanation_log": self.explanation_log,
        }


# =============================================================================
# CONFIDENCE ADJUSTMENT
# =============================================================================

# Confidence adjustments per outcome
CONFIDENCE_DELTA = {
    "interview": 0.15,      # Positive signal
    "rejected": -0.10,      # Negative signal
    "no_response": -0.08,   # Weak negative signal
}

# Threshold for strategy failure
FAILURE_THRESHOLD = 0.30
NEGATIVE_OUTCOME_LIMIT = 3


def update_confidence(current: float, outcome: str) -> float:
    """Update confidence based on outcome. Returns clamped value."""
    delta = CONFIDENCE_DELTA.get(outcome, 0.0)
    new_confidence = current + delta
    return round(max(0.10, min(0.95, new_confidence)), 2)


def should_invalidate(record: StrategyRecord) -> bool:
    """Determine if strategy should be marked as failed."""
    if record.failed:
        return True
    
    # Count negative outcomes
    negative_count = sum(
        1 for o in record.outcomes
        if o in ("rejected", "no_response")
    )
    
    # Fail if: confidence below threshold OR too many negatives
    if record.current_confidence < FAILURE_THRESHOLD:
        return True
    if negative_count >= NEGATIVE_OUTCOME_LIMIT:
        return True
    
    return False


def evaluate_strategy_state(
    session: AgentSession,
    profile_signals: dict = None
) -> AgentSession:
    """
    Evaluate and update strategy lifecycle state based on deterministic rules.
    
    This is the core state machine that governs strategy transitions.
    State changes are based on:
    - Number of interviews received
    - Current confidence level
    - Profile signals (e.g., resume_positioning_issue)
    - Failure conditions
    
    State Transition Rules:
    1. explore → validate:
       - Trigger: ≥1 interview AND confidence ≥ 0.55
       
    2. validate → execute:
       - Trigger: ≥2 interviews AND no resume_positioning_issue AND confidence ≥ 0.65
       
    3. any → reconsider:
       - Trigger: confidence < FAILURE_THRESHOLD OR ≥3 negative outcomes
       
    4. reconsider → explore:
       - Trigger: new strategy selected (handled in re_evaluate_strategy)
    
    IMPORTANT: Interviews NEVER directly trigger strategy switching.
    Strategy switching ONLY occurs when entering 'reconsider' state.
    
    Args:
        session: Current agent session
        profile_signals: Optional profile signals from career_snapshot
        
    Returns:
        Updated session with new strategy_state
    """
    if session.current_strategy is None:
        return session
    
    record = session.current_strategy
    current_state = record.strategy_state
    
    # Count interview outcomes (positive signals)
    interview_count = sum(1 for o in record.outcomes if o == "interview")
    
    # Count negative outcomes
    negative_count = sum(
        1 for o in record.outcomes if o in ("rejected", "no_response")
    )
    
    # Check for resume_positioning_issue signal
    has_positioning_issue = False
    if profile_signals:
        signals = profile_signals.get("signals", {})
        positioning_signal = signals.get("resume_positioning_issue", {})
        has_positioning_issue = positioning_signal.get("triggered", False)
    
    # Determine new state based on current state and conditions
    new_state = current_state
    transition_reason = None
    
    # Rule: any → reconsider (failure conditions)
    # This takes precedence over all other transitions
    if record.current_confidence < FAILURE_THRESHOLD:
        new_state = StrategyState.RECONSIDER
        transition_reason = f"confidence dropped to {record.current_confidence} (below threshold {FAILURE_THRESHOLD})"
    elif negative_count >= NEGATIVE_OUTCOME_LIMIT:
        new_state = StrategyState.RECONSIDER
        transition_reason = f"{negative_count} negative outcomes (limit: {NEGATIVE_OUTCOME_LIMIT})"
    
    # If not failing, evaluate forward transitions based on current state
    elif current_state == StrategyState.EXPLORE:
        # Rule: explore → validate
        # Trigger: ≥1 interview AND confidence ≥ 0.55
        if interview_count >= 1 and record.current_confidence >= 0.55:
            new_state = StrategyState.VALIDATE
            transition_reason = f"{interview_count} interview(s) received, confidence {record.current_confidence} ≥ 0.55"
    
    elif current_state == StrategyState.VALIDATE:
        # Rule: validate → execute
        # Trigger: ≥2 interviews AND no resume_positioning_issue AND confidence ≥ 0.65
        if interview_count >= 2 and record.current_confidence >= 0.65 and not has_positioning_issue:
            new_state = StrategyState.EXECUTE
            transition_reason = f"{interview_count} interviews, confidence {record.current_confidence} ≥ 0.65, no positioning issues"
        # If positioning issue emerged, return to explore
        elif has_positioning_issue and interview_count < 2:
            new_state = StrategyState.EXPLORE
            transition_reason = "resume positioning issue detected"
    
    # Note: EXECUTE and RECONSIDER states don't have forward transitions
    # - EXECUTE means strategy is locked (ready for roadmap)
    # - RECONSIDER will trigger re-evaluation which creates a new strategy in EXPLORE
    
    # Update state if changed
    if new_state != current_state:
        record.strategy_state = new_state
        
        # Log the state transition
        log_message = (
            f"State transition: {current_state} → {new_state}. "
            f"Reason: {transition_reason}"
        )
        session.explanation_log.append(log_message)
        
        # Mark as failed if entering RECONSIDER state
        if new_state == StrategyState.RECONSIDER:
            record.failed = True
    
    return session


# =============================================================================
# OUTCOME HANDLING
# =============================================================================

def record_outcome(session: AgentSession, outcome: str) -> AgentSession:
    """
    Record an outcome for the current strategy.
    Updates confidence and checks for strategy failure.
    
    Returns updated session.
    """
    if outcome not in ("no_response", "rejected", "interview"):
        raise ValueError(f"Invalid outcome: {outcome}. Must be one of: no_response, rejected, interview")
    
    if session.current_strategy is None:
        raise ValueError("No active strategy to record outcome for")
    
    record = session.current_strategy
    
    # Record the outcome
    record.outcomes.append(outcome)
    
    # Update confidence
    record.current_confidence = update_confidence(record.current_confidence, outcome)
    
    # Check for failure (legacy - now handled by state machine)
    if should_invalidate(record):
        record.failed = True
        
        # Log the failure
        session.explanation_log.append(
            f"Strategy '{record.strategy}' marked as failed after {len(record.outcomes)} outcome(s). "
            f"Confidence dropped to {record.current_confidence}."
        )
    else:
        # Log the update
        session.explanation_log.append(
            f"Recorded '{outcome}' for strategy '{record.strategy}'. "
            f"Confidence: {record.current_confidence}. State: {record.strategy_state}."
        )
    
    return session


# =============================================================================
# STRATEGY RE-EVALUATION
# =============================================================================

def re_evaluate_strategy(session: AgentSession) -> AgentSession:
    """
    Re-run Stage 2 and Stage 3 when current strategy has failed.
    
    The new strategy is allowed to override the previous one.
    This is the learning loop.
    """
    if session.current_strategy is None or not session.current_strategy.failed:
        # No re-evaluation needed
        return session
    
    # Archive the failed strategy
    session.strategy_history.append(session.current_strategy)
    failed_strategy = session.current_strategy.strategy
    
    # Increment loop iteration
    session.loop_iteration += 1
    
    # Get list of failed strategies to potentially exclude
    failed_strategies = {s.strategy for s in session.strategy_history if s.failed}
    
    # Re-run Stage 2: Bottleneck Analysis
    # (The bottleneck analysis is re-run on the same evidence, but the agent
    # can now factor in that certain strategies have failed)
    new_stage2 = analyze_bottlenecks(session.stage1_evidence)
    session.stage2_bottleneck = new_stage2
    
    # Re-run Stage 3: Strategy Selection
    section_signals = session.stage1_evidence.get("section_signals")
    new_stage3 = select_strategy_and_action(
        new_stage2, 
        section_signals
    )
    
    # Check if we got the same strategy that already failed
    if new_stage3["strategy"] in failed_strategies:
        # Force a different strategy by adjusting
        new_stage3 = _select_fallback_strategy(
            new_stage2, 
            failed_strategies,
            section_signals
        )
    
    session.stage3_strategy = new_stage3
    
    # Create new strategy record in EXPLORE state
    # Rule: reconsider → explore (new strategy selected)
    session.current_strategy = StrategyRecord(
        strategy=new_stage3["strategy"],
        initial_confidence=new_stage3["confidence"],
        current_confidence=new_stage3["confidence"],
        strategy_state=StrategyState.EXPLORE,  # New strategy always starts in EXPLORE
    )
    
    # Log the change with state information
    session.explanation_log.append(
        f"Re-evaluated strategy. Changed from '{failed_strategy}' to '{new_stage3['strategy']}'. "
        f"New confidence: {new_stage3['confidence']}. State: {StrategyState.EXPLORE}."
    )
    
    return session


def _select_fallback_strategy(
    stage2_output: dict,
    failed_strategies: set,
    section_signals: dict = None
) -> dict:
    """
    Select an alternative strategy when the preferred one has already failed.
    Follows priority order: ResumeOptimization > SkillGapPatch > RoleShift > HoldPosition
    """
    STRATEGY_PRIORITY = [
        "ResumeOptimization",
        "SkillGapPatch", 
        "RoleShift",
        "HoldPosition",
    ]
    
    # Fallback actions
    FALLBACK_ACTIONS = {
        "ResumeOptimization": "Restructure the resume to highlight evidence of applied skills.",
        "SkillGapPatch": "Add one in-demand skill through a verifiable project or credential.",
        "RoleShift": "Adjust target role to better align with current evidence profile.",
        "HoldPosition": "Maintain current positioning and continue application process.",
    }
    
    # Base confidence (reduced for fallback)
    FALLBACK_CONFIDENCE = {
        "ResumeOptimization": 0.55,
        "SkillGapPatch": 0.45,
        "RoleShift": 0.35,
        "HoldPosition": 0.70,
    }
    
    # Find first non-failed strategy
    for strategy in STRATEGY_PRIORITY:
        if strategy not in failed_strategies:
            return {
                "strategy": strategy,
                "action": FALLBACK_ACTIONS[strategy],
                "confidence": FALLBACK_CONFIDENCE[strategy],
            }
    
    # All strategies failed - return HoldPosition with low confidence
    return {
        "strategy": "HoldPosition",
        "action": "All strategies exhausted. Recommend manual review of career positioning.",
        "confidence": 0.25,
    }


# =============================================================================
# INITIALIZATION
# =============================================================================

def initialize_session(
    stage1_evidence: dict,
    stage2_bottleneck: dict,
    stage3_strategy: dict
) -> AgentSession:
    """
    Initialize a new agent session with the outputs from all 3 stages.
    """
    session = AgentSession(
        stage1_evidence=stage1_evidence,
        stage2_bottleneck=stage2_bottleneck,
        stage3_strategy=stage3_strategy,
    )
    
    # Create initial strategy record in EXPLORE state
    session.current_strategy = StrategyRecord(
        strategy=stage3_strategy["strategy"],
        initial_confidence=stage3_strategy["confidence"],
        current_confidence=stage3_strategy["confidence"],
        strategy_state=StrategyState.EXPLORE,  # All strategies start in EXPLORE
    )
    
    # Initial explanation with state
    dominant_issue = stage2_bottleneck.get("dominant_issue", "none identified")
    session.explanation_log.append(
        f"Agent initialized. Selected '{stage3_strategy['strategy']}' strategy "
        f"due to dominant issue: {dominant_issue}. "
        f"Initial confidence: {stage3_strategy['confidence']}. State: {StrategyState.EXPLORE}."
    )
    
    return session


# =============================================================================
# DEMO-SAFE EXPLANATION
# =============================================================================

def generate_explanation(session: AgentSession) -> str:
    """
    Generate a plain-English explanation of the agent's decision process.
    
    Covers:
    - Why the original strategy was chosen
    - What outcomes occurred
    - Why the agent changed (or did not change) strategy
    
    Suitable for demo narration.
    """
    lines = []
    
    # Get initial context
    implied_role = session.stage2_bottleneck.get("implied_role", "unknown role")
    dominant_issue = session.stage2_bottleneck.get("dominant_issue")
    justification = session.stage2_bottleneck.get("justification", "")
    
    # Part 1: Original strategy selection
    if session.strategy_history:
        original = session.strategy_history[0]
        lines.append(
            f"The agent initially selected '{original.strategy}' strategy "
            f"for the implied role of {implied_role}."
        )
        if dominant_issue:
            lines.append(
                f"This was based on the dominant issue: {dominant_issue}. "
                f"{justification}"
            )
    else:
        current = session.current_strategy
        if current:
            lines.append(
                f"The agent selected '{current.strategy}' strategy "
                f"for the implied role of {implied_role}."
            )
            if dominant_issue:
                lines.append(f"Dominant issue identified: {dominant_issue}.")
    
    # Part 2: Outcome history
    all_strategies = session.strategy_history + (
        [session.current_strategy] if session.current_strategy else []
    )
    
    for i, record in enumerate(all_strategies):
        if record.outcomes:
            outcome_summary = ", ".join(record.outcomes)
            lines.append(
                f"Strategy '{record.strategy}' received outcomes: [{outcome_summary}]. "
                f"Confidence adjusted from {record.initial_confidence} to {record.current_confidence}."
            )
            if record.failed:
                lines.append(f"Strategy was marked as failed.")
    
    # Part 3: Strategy changes
    if len(session.strategy_history) > 0:
        lines.append(
            f"The agent performed {session.loop_iteration} strategy re-evaluation(s)."
        )
        
        changes = []
        for i, record in enumerate(session.strategy_history):
            if i == 0:
                continue
            prev = session.strategy_history[i-1]
            changes.append(f"'{prev.strategy}' → '{record.strategy}'")
        
        if session.current_strategy and session.strategy_history:
            prev = session.strategy_history[-1]
            changes.append(f"'{prev.strategy}' → '{session.current_strategy.strategy}'")
        
        if changes:
            lines.append(f"Strategy transitions: {', '.join(changes)}.")
    elif session.current_strategy and not session.current_strategy.failed:
        lines.append("No strategy changes were required.")
    
    # Part 4: Current state
    if session.current_strategy:
        current = session.current_strategy
        if current.failed or current.strategy_state == StrategyState.RECONSIDER:
            lines.append(
                f"Current strategy '{current.strategy}' has failed (state: {current.strategy_state}). "
                "Re-evaluation recommended."
            )
        else:
            lines.append(
                f"Current strategy: '{current.strategy}' "
                f"with confidence {current.current_confidence}. "
                f"State: {current.strategy_state}."
            )
            lines.append(f"Action: {session.stage3_strategy.get('action', 'N/A')}")
    
    return " ".join(lines)


def generate_short_explanation(session: AgentSession) -> str:
    """
    Generate a brief one-liner explanation suitable for UI display.
    """
    current = session.current_strategy
    if not current:
        return "No active strategy."
    
    dominant_issue = session.stage2_bottleneck.get("dominant_issue", "identified issues")
    
    if session.loop_iteration == 0:
        return (
            f"Selected {current.strategy} due to {dominant_issue}. "
            f"Confidence: {current.current_confidence}. State: {current.strategy_state}."
        )
    else:
        prev_strategy = session.strategy_history[-1].strategy if session.strategy_history else "previous"
        return (
            f"Shifted from {prev_strategy} to {current.strategy} "
            f"after {session.loop_iteration} re-evaluation(s). "
            f"Confidence: {current.current_confidence}. State: {current.strategy_state}."
        )


# =============================================================================
# FULL LOOP STEP
# =============================================================================

def process_outcome(session: AgentSession, outcome: str) -> dict:
    """
    Process a single outcome through the agentic loop.
    
    1. Record the outcome
    2. Update confidence
    3. Evaluate strategy state transitions (STATE MACHINE)
    4. Re-evaluate if strategy failed (entered RECONSIDER state)
    5. Return updated state with explanation
    6. (Optional) Update profile signals if snapshot available
    
    This is the main entry point for the feedback loop.
    """
    # Step 1: Record outcome
    session = record_outcome(session, outcome)
    
    # Step 2: Extract profile signals for state evaluation (read-only data layer)
    profile_signals = None
    if SNAPSHOT_AVAILABLE:
        try:
            # Get existing enhanced snapshot from stage1 evidence
            enhanced_snapshot = session.stage1_evidence.get("enhanced_snapshot")
            if enhanced_snapshot:
                # Collect all outcomes from current strategy
                all_outcomes = []
                if session.current_strategy:
                    all_outcomes = session.current_strategy.outcomes
                
                # Re-extract signals with updated outcomes
                profile_signals = extract_profile_signals(enhanced_snapshot, all_outcomes)
        except Exception as e:
            # Graceful degradation - signals are optional
            sys.stderr.write(f"[agent_loop] Profile signal extraction skipped: {e}\n")
    
    # Step 3: Evaluate strategy state transitions (CORE STATE MACHINE)
    session = evaluate_strategy_state(session, profile_signals)
    
    # Step 4: Check for re-evaluation (if state is RECONSIDER)
    strategy_changed = False
    if session.current_strategy and session.current_strategy.strategy_state == StrategyState.RECONSIDER:
        old_strategy = session.current_strategy.strategy
        session = re_evaluate_strategy(session)
        strategy_changed = session.current_strategy.strategy != old_strategy
    
    # Step 5: Generate explanation
    explanation = generate_short_explanation(session)
    
    # Step 6: Build result with state information
    result = {
        "session": session.to_dict(),
        "strategy_changed": strategy_changed,
        "current_strategy": session.stage3_strategy,
        "explanation": explanation,
        "strategy_state": session.current_strategy.strategy_state if session.current_strategy else None,  # Expose state
    }
    
    # Step 7: Include profile signals if available
    if profile_signals:
        result["profile_signals"] = profile_signals
    
    return result


# =============================================================================
# CLI INTERFACE
# =============================================================================

def main():
    """CLI interface for agent loop operations."""
    if len(sys.argv) < 3:
        print(json.dumps({
            "error": "Usage: python agent_loop.py <command> <args...>",
            "commands": {
                "init": "agent_loop.py init <stage1.json> <stage2.json> <stage3.json>",
                "outcome": "agent_loop.py outcome <session.json> <outcome>",
                "explain": "agent_loop.py explain <session.json>",
            }
        }))
        sys.exit(1)
    
    command = sys.argv[1]
    
    try:
        if command == "init":
            # Initialize session from stage outputs
            if len(sys.argv) < 5:
                print(json.dumps({"error": "init requires stage1, stage2, stage3 JSON files"}))
                sys.exit(1)
            
            with open(sys.argv[2], 'r') as f:
                stage1 = json.load(f)
            with open(sys.argv[3], 'r') as f:
                stage2 = json.load(f)
            with open(sys.argv[4], 'r') as f:
                stage3 = json.load(f)
            
            session = initialize_session(stage1, stage2, stage3)
            print(json.dumps({
                "session": session.to_dict(),
                "explanation": generate_short_explanation(session),
            }, indent=2))
        
        elif command == "outcome":
            # Process an outcome
            if len(sys.argv) < 4:
                print(json.dumps({"error": "outcome requires session.json and outcome value"}))
                sys.exit(1)
            
            with open(sys.argv[2], 'r') as f:
                session_data = json.load(f)
            outcome = sys.argv[3]
            
            # Reconstruct session
            session = _reconstruct_session(session_data)
            
            result = process_outcome(session, outcome)
            print(json.dumps(result))
        
        elif command == "explain":
            # Generate full explanation
            if len(sys.argv) < 3:
                print(json.dumps({"error": "explain requires session.json"}))
                sys.exit(1)
            
            with open(sys.argv[2], 'r') as f:
                session_data = json.load(f)
            
            session = _reconstruct_session(session_data)
            explanation = generate_explanation(session)
            print(json.dumps({"explanation": explanation}))
        
        else:
            print(json.dumps({"error": f"Unknown command: {command}"}))
            sys.exit(1)
    
    except json.JSONDecodeError as e:
        print(json.dumps({"error": f"Invalid JSON: {str(e)}"}))
        sys.exit(1)
    except FileNotFoundError as e:
        print(json.dumps({"error": f"File not found: {str(e)}"}))
        sys.exit(1)
    except Exception as e:
        print(json.dumps({"error": f"Error: {str(e)}"}))
        sys.exit(1)


def _reconstruct_session(data: dict) -> AgentSession:
    """Reconstruct AgentSession from serialized dict."""
    session = AgentSession(
        stage1_evidence=data.get("stage1_evidence", {}),
        stage2_bottleneck=data.get("stage2_bottleneck", {}),
        stage3_strategy=data.get("stage3_strategy", {}),
        loop_iteration=data.get("loop_iteration", 0),
        explanation_log=data.get("explanation_log", []),
    )
    
    # Reconstruct current strategy
    current = data.get("current_strategy")
    if current:
        session.current_strategy = StrategyRecord(
            strategy=current["strategy"],
            initial_confidence=current["initial_confidence"],
            current_confidence=current["current_confidence"],
            outcomes=current.get("outcomes", []),
            failed=current.get("failed", False),
            strategy_state=current.get("strategy_state", StrategyState.EXPLORE),  # Handle legacy sessions
        )
    
    # Reconstruct history
    for hist in data.get("strategy_history", []):
        session.strategy_history.append(StrategyRecord(
            strategy=hist["strategy"],
            initial_confidence=hist["initial_confidence"],
            current_confidence=hist["current_confidence"],
            outcomes=hist.get("outcomes", []),
            failed=hist.get("failed", False),
            strategy_state=hist.get("strategy_state", StrategyState.EXPLORE),  # Handle legacy sessions
        ))
    
    return session


if __name__ == "__main__":
    main()
