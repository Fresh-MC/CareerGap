#!/usr/bin/env python3
"""
Roadmap Generator Module

Generates execution roadmaps strictly gated by strategy lifecycle state.
A roadmap may ONLY be generated when strategy_state == EXECUTE.

This module enforces the principle that roadmaps are earned, not automatic.
Interviews unlock execution planning naturally by validating the strategy first.
"""

import json
import sys
from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional
from datetime import datetime, timedelta
from enum import Enum

# Import agent loop to access StrategyState
try:
    from agent_loop import StrategyState, AgentSession
    AGENT_LOOP_AVAILABLE = True
except ImportError:
    AGENT_LOOP_AVAILABLE = False
    # Define fallback if agent_loop not available
    class StrategyState(str, Enum):
        EXPLORE = "explore"
        VALIDATE = "validate"
        EXECUTE = "execute"
        RECONSIDER = "reconsider"


# =============================================================================
# ROADMAP STRUCTURES
# =============================================================================

@dataclass
class RoadmapAction:
    """
    A single concrete action in the roadmap.
    
    Actions must be:
    - Concrete (specific and actionable)
    - Time-bound (with deadline)
    - Strategy-specific (aligned with current strategy)
    """
    action_id: str
    title: str
    description: str
    deadline_days: int  # Days from roadmap creation
    priority: str  # "critical", "high", "medium"
    category: str  # "resume", "skill", "application", "networking", "preparation"
    completed: bool = False
    
    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class RoadmapMilestone:
    """
    A milestone that marks progress in the roadmap.
    """
    milestone_id: str
    title: str
    description: str
    target_days: int  # Days from roadmap creation
    success_criteria: List[str]
    
    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class RoadmapGoal:
    """
    High-level goal for the roadmap.
    """
    goal_id: str
    title: str
    description: str
    measurable: str  # How to measure success
    
    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class Roadmap:
    """
    Complete career roadmap for a validated strategy.
    
    A roadmap is ONLY generated when strategy_state == EXECUTE.
    This ensures the strategy has been validated with interviews before
    committing to an execution plan.
    """
    roadmap_id: str
    strategy: str
    phase: str  # Always "execute" for valid roadmaps
    created_at: str
    strategy_confidence: float
    
    # Core components
    goals: List[RoadmapGoal]
    milestones: List[RoadmapMilestone]
    actions: List[RoadmapAction]
    
    # Metadata
    review_after_days: int
    estimated_completion_days: int
    version: int = 1
    
    # Validation tracking
    strategy_version: str = ""  # Hash or timestamp of strategy when roadmap created
    invalidated: bool = False
    invalidation_reason: Optional[str] = None
    
    def to_dict(self) -> dict:
        return {
            "roadmap_id": self.roadmap_id,
            "strategy": self.strategy,
            "phase": self.phase,
            "created_at": self.created_at,
            "strategy_confidence": self.strategy_confidence,
            "goals": [g.to_dict() for g in self.goals],
            "milestones": [m.to_dict() for m in self.milestones],
            "actions": [a.to_dict() for a in self.actions],
            "review_after_days": self.review_after_days,
            "estimated_completion_days": self.estimated_completion_days,
            "version": self.version,
            "strategy_version": self.strategy_version,
            "invalidated": self.invalidated,
            "invalidation_reason": self.invalidation_reason,
        }


# =============================================================================
# ELIGIBILITY GATING
# =============================================================================

class RoadmapEligibilityError(Exception):
    """Raised when roadmap generation is attempted but state requirements not met."""
    pass


def check_roadmap_eligibility(session: dict) -> dict:
    """
    Check if roadmap generation is allowed based on strategy lifecycle state.
    
    ELIGIBILITY RULE:
    A roadmap may ONLY be generated when strategy_state == EXECUTE
    
    Blocked states:
    - EXPLORE: Strategy just selected, needs validation
    - VALIDATE: Strategy showing promise, needs more evidence
    - RECONSIDER: Strategy failed, under re-evaluation
    
    Args:
        session: AgentSession dict with current_strategy containing strategy_state
        
    Returns:
        dict with 'eligible' (bool) and 'reason' (str)
        
    Raises:
        ValueError if session structure invalid
    """
    if not session:
        return {
            "eligible": False,
            "reason": "No session provided",
            "current_state": None,
        }
    
    current_strategy = session.get("current_strategy")
    if not current_strategy:
        return {
            "eligible": False,
            "reason": "No active strategy found",
            "current_state": None,
        }
    
    strategy_state = current_strategy.get("strategy_state", "explore")
    strategy_name = current_strategy.get("strategy", "Unknown")
    confidence = current_strategy.get("current_confidence", 0.0)
    
    # THE GATE: Only EXECUTE state allows roadmap generation
    if strategy_state == StrategyState.EXECUTE or strategy_state == "execute":
        return {
            "eligible": True,
            "reason": f"Strategy '{strategy_name}' in EXECUTE state (confidence: {confidence})",
            "current_state": strategy_state,
        }
    
    # Build helpful error messages for each blocked state
    if strategy_state == StrategyState.EXPLORE or strategy_state == "explore":
        return {
            "eligible": False,
            "reason": f"Strategy '{strategy_name}' is in EXPLORE state. Need at least 1 interview to validate before generating roadmap.",
            "current_state": strategy_state,
            "recommendation": "Continue applying to roles and track outcomes. Roadmap will unlock after validation.",
        }
    
    elif strategy_state == StrategyState.VALIDATE or strategy_state == "validate":
        return {
            "eligible": False,
            "reason": f"Strategy '{strategy_name}' is in VALIDATE state. Need 2+ interviews and confidence ≥ 0.65 to execute.",
            "current_state": strategy_state,
            "recommendation": "Strategy is showing promise! Get more interviews to lock it in for execution.",
        }
    
    elif strategy_state == StrategyState.RECONSIDER or strategy_state == "reconsider":
        return {
            "eligible": False,
            "reason": f"Strategy '{strategy_name}' is in RECONSIDER state (failed). Cannot generate roadmap for failed strategy.",
            "current_state": strategy_state,
            "recommendation": "System is re-evaluating strategy. Wait for new strategy selection.",
        }
    
    else:
        return {
            "eligible": False,
            "reason": f"Unknown strategy state: {strategy_state}",
            "current_state": strategy_state,
        }


# =============================================================================
# STRATEGY-SPECIFIC ROADMAP TEMPLATES
# =============================================================================

def generate_roadmap_for_resume_optimization(
    session: dict,
    enhanced_snapshot: dict,
    profile_signals: dict
) -> Roadmap:
    """
    Generate roadmap for ResumeOptimization strategy.
    
    Focus: Improve resume structure, evidence depth, and positioning.
    Timeline: 2-4 weeks
    """
    import uuid
    
    strategy = session["current_strategy"]
    roadmap_id = str(uuid.uuid4())
    created_at = datetime.utcnow().isoformat()
    
    # Extract bottleneck information
    bottlenecks = session.get("stage2_bottleneck", {}).get("bottlenecks", {})
    dominant_issue = session.get("stage2_bottleneck", {}).get("dominant_issue")
    
    # Goals
    goals = [
        RoadmapGoal(
            goal_id=f"{roadmap_id}-goal-1",
            title="Strengthen Resume Evidence",
            description="Transform resume to showcase applied skills with concrete outcomes",
            measurable="All projects include problem, solution, and quantifiable results",
        ),
        RoadmapGoal(
            goal_id=f"{roadmap_id}-goal-2",
            title="Optimize for Target Role",
            description="Position resume clearly for intended role",
            measurable="Resume passes ATS screening for target roles with 80%+ match",
        ),
    ]
    
    # Milestones
    milestones = [
        RoadmapMilestone(
            milestone_id=f"{roadmap_id}-milestone-1",
            title="Resume Draft Complete",
            description="First revision with improved evidence and structure",
            target_days=7,
            success_criteria=[
                "All experience entries have quantifiable outcomes",
                "Projects section shows problem-solution-impact",
                "Skills linked to specific evidence",
            ],
        ),
        RoadmapMilestone(
            milestone_id=f"{roadmap_id}-milestone-2",
            title="Resume Validated",
            description="Resume reviewed and optimized for target roles",
            target_days=14,
            success_criteria=[
                "ATS compatibility verified",
                "Peer or professional review completed",
                "Tailored versions for top 3 target companies",
            ],
        ),
    ]
    
    # Actions (concrete and time-bound)
    actions = [
        RoadmapAction(
            action_id=f"{roadmap_id}-action-1",
            title="Rewrite Primary Project Description",
            description="Add problem statement, approach, tools used, and quantifiable outcome. Example: 'Reduced load time by 40% using Redis caching for 10K daily users'",
            deadline_days=3,
            priority="critical",
            category="resume",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-2",
            title="Add Metrics to Experience Entries",
            description="Quantify at least 2 achievements per role with specific numbers (%, users, time saved, revenue impact)",
            deadline_days=5,
            priority="critical",
            category="resume",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-3",
            title="Create Skill-Evidence Matrix",
            description="Link each listed skill to specific project or experience where applied. Remove skills without evidence.",
            deadline_days=4,
            priority="high",
            category="resume",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-4",
            title="Run ATS Compatibility Check",
            description="Use Jobscan or similar tool to verify resume passes ATS for target job descriptions (aim for 75%+ match)",
            deadline_days=7,
            priority="high",
            category="preparation",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-5",
            title="Get Resume Reviewed",
            description="Submit to resume review service or experienced peer in target industry. Incorporate feedback.",
            deadline_days=10,
            priority="medium",
            category="preparation",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-6",
            title="Create Tailored Versions",
            description="Develop 3 targeted resume versions for top priority companies/roles",
            deadline_days=12,
            priority="high",
            category="application",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-7",
            title="Begin Targeted Applications",
            description="Apply to 10 well-matched roles with optimized resume",
            deadline_days=14,
            priority="critical",
            category="application",
        ),
    ]
    
    return Roadmap(
        roadmap_id=roadmap_id,
        strategy="ResumeOptimization",
        phase="execute",
        created_at=created_at,
        strategy_confidence=strategy["current_confidence"],
        goals=goals,
        milestones=milestones,
        actions=actions,
        review_after_days=14,
        estimated_completion_days=21,
        strategy_version=f"{strategy['strategy']}-{strategy['initial_confidence']}-{created_at}",
    )


def generate_roadmap_for_skill_gap_patch(
    session: dict,
    enhanced_snapshot: dict,
    profile_signals: dict
) -> Roadmap:
    """
    Generate roadmap for SkillGapPatch strategy.
    
    Focus: Acquire missing critical skill + create applied evidence.
    Timeline: 3-6 weeks
    """
    import uuid
    
    strategy = session["current_strategy"]
    roadmap_id = str(uuid.uuid4())
    created_at = datetime.utcnow().isoformat()
    
    goals = [
        RoadmapGoal(
            goal_id=f"{roadmap_id}-goal-1",
            title="Acquire Target Skill",
            description="Master one critical skill identified as gap for target role",
            measurable="Complete structured learning + build applied project demonstrating skill",
        ),
        RoadmapGoal(
            goal_id=f"{roadmap_id}-goal-2",
            title="Create Verifiable Evidence",
            description="Build portfolio project that proves skill application",
            measurable="Project deployed and documented on GitHub with README",
        ),
    ]
    
    milestones = [
        RoadmapMilestone(
            milestone_id=f"{roadmap_id}-milestone-1",
            title="Skill Foundation Complete",
            description="Completed structured learning for target skill",
            target_days=14,
            success_criteria=[
                "Course or certification completed",
                "Core concepts documented in notes",
                "Practice exercises completed",
            ],
        ),
        RoadmapMilestone(
            milestone_id=f"{roadmap_id}-milestone-2",
            title="Applied Project Deployed",
            description="Project demonstrating skill in real-world context",
            target_days=28,
            success_criteria=[
                "Project solves real problem",
                "Code on GitHub with documentation",
                "Project deployed and accessible",
            ],
        ),
    ]
    
    actions = [
        RoadmapAction(
            action_id=f"{roadmap_id}-action-1",
            title="Identify Top Missing Skill",
            description="Based on target role analysis, confirm the #1 skill to acquire (e.g., React, AWS, Python)",
            deadline_days=2,
            priority="critical",
            category="skill",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-2",
            title="Enroll in Focused Course",
            description="Start structured learning (Udemy, Coursera, freeCodeCamp) for identified skill. Complete 50%.",
            deadline_days=7,
            priority="critical",
            category="skill",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-3",
            title="Complete Course/Certification",
            description="Finish structured learning and earn certificate if available. Add to resume.",
            deadline_days=14,
            priority="critical",
            category="skill",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-4",
            title="Design Applied Project",
            description="Plan a project that uses new skill to solve real problem. Define scope, tech stack, and outcome.",
            deadline_days=16,
            priority="high",
            category="skill",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-5",
            title="Build Project (Phase 1)",
            description="Implement core functionality. Aim for MVP (minimum viable product).",
            deadline_days=24,
            priority="critical",
            category="skill",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-6",
            title="Deploy and Document Project",
            description="Deploy project (GitHub Pages, Vercel, AWS). Write comprehensive README with screenshots, tech stack, and learnings.",
            deadline_days=28,
            priority="critical",
            category="skill",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-7",
            title="Update Resume with New Skill",
            description="Add skill to skills section. Add project to projects section with impact statement.",
            deadline_days=30,
            priority="high",
            category="resume",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-8",
            title="Apply to Roles Requiring Skill",
            description="Target 15 applications for roles where this skill is listed as required/preferred",
            deadline_days=35,
            priority="critical",
            category="application",
        ),
    ]
    
    return Roadmap(
        roadmap_id=roadmap_id,
        strategy="SkillGapPatch",
        phase="execute",
        created_at=created_at,
        strategy_confidence=strategy["current_confidence"],
        goals=goals,
        milestones=milestones,
        actions=actions,
        review_after_days=21,
        estimated_completion_days=42,
        strategy_version=f"{strategy['strategy']}-{strategy['initial_confidence']}-{created_at}",
    )


def generate_roadmap_for_role_shift(
    session: dict,
    enhanced_snapshot: dict,
    profile_signals: dict
) -> Roadmap:
    """
    Generate roadmap for RoleShift strategy.
    
    Focus: Reposition for different role type that matches current evidence.
    Timeline: 2-3 weeks
    """
    import uuid
    
    strategy = session["current_strategy"]
    roadmap_id = str(uuid.uuid4())
    created_at = datetime.utcnow().isoformat()
    
    goals = [
        RoadmapGoal(
            goal_id=f"{roadmap_id}-goal-1",
            title="Reframe Career Narrative",
            description="Position existing experience for new target role type",
            measurable="Resume and profiles consistently communicate new target role",
        ),
        RoadmapGoal(
            goal_id=f"{roadmap_id}-goal-2",
            title="Target Role-Aligned Opportunities",
            description="Apply to roles that value project experience over formal employment",
            measurable="50% of applications to entry-level or project-focused roles",
        ),
    ]
    
    milestones = [
        RoadmapMilestone(
            milestone_id=f"{roadmap_id}-milestone-1",
            title="Narrative Repositioned",
            description="All materials reflect new target role positioning",
            target_days=7,
            success_criteria=[
                "Resume headline updated to target role",
                "LinkedIn summary rewritten",
                "Project descriptions emphasize relevant skills",
            ],
        ),
        RoadmapMilestone(
            milestone_id=f"{roadmap_id}-milestone-2",
            title="Application Campaign Launched",
            description="Actively applying to role-aligned positions",
            target_days=14,
            success_criteria=[
                "20+ applications submitted",
                "All to entry-level or project-friendly roles",
                "Cover letters customized per role",
            ],
        ),
    ]
    
    actions = [
        RoadmapAction(
            action_id=f"{roadmap_id}-action-1",
            title="Define New Target Role",
            description="Identify specific role titles that better match current experience profile (e.g., Junior Developer, Associate Engineer, Technical Support Engineer)",
            deadline_days=2,
            priority="critical",
            category="preparation",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-2",
            title="Rewrite Resume Headline",
            description="Change resume headline/summary to reflect new target role. Emphasize transferable skills.",
            deadline_days=4,
            priority="critical",
            category="resume",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-3",
            title="Reframe Project Experience",
            description="Rewrite project descriptions to emphasize deliverables, stakeholder impact, and professional-level outcomes",
            deadline_days=5,
            priority="high",
            category="resume",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-4",
            title="Update LinkedIn Profile",
            description="Rewrite LinkedIn headline and summary to match new positioning. Update experience descriptions.",
            deadline_days=6,
            priority="high",
            category="preparation",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-5",
            title="Research Target Companies",
            description="Identify 30 companies known for hiring project-based candidates (startups, tech companies with apprenticeships)",
            deadline_days=7,
            priority="high",
            category="application",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-6",
            title="Customize Cover Letter Template",
            description="Create cover letter addressing career transition narrative. Emphasize project work as professional experience.",
            deadline_days=8,
            priority="medium",
            category="application",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-7",
            title="Launch Application Campaign",
            description="Apply to 20 entry-level roles aligned with repositioned profile. Track responses.",
            deadline_days=14,
            priority="critical",
            category="application",
        ),
    ]
    
    return Roadmap(
        roadmap_id=roadmap_id,
        strategy="RoleShift",
        phase="execute",
        created_at=created_at,
        strategy_confidence=strategy["current_confidence"],
        goals=goals,
        milestones=milestones,
        actions=actions,
        review_after_days=14,
        estimated_completion_days=21,
        strategy_version=f"{strategy['strategy']}-{strategy['initial_confidence']}-{created_at}",
    )


def generate_roadmap_for_hold_position(
    session: dict,
    enhanced_snapshot: dict,
    profile_signals: dict
) -> Roadmap:
    """
    Generate roadmap for HoldPosition strategy.
    
    Focus: Maintain current approach, optimize application volume and targeting.
    Timeline: 2 weeks
    """
    import uuid
    
    strategy = session["current_strategy"]
    roadmap_id = str(uuid.uuid4())
    created_at = datetime.utcnow().isoformat()
    
    goals = [
        RoadmapGoal(
            goal_id=f"{roadmap_id}-goal-1",
            title="Maximize Application Quality",
            description="Maintain strong resume positioning while increasing application volume",
            measurable="30+ targeted applications in 2 weeks",
        ),
        RoadmapGoal(
            goal_id=f"{roadmap_id}-goal-2",
            title="Optimize Interview Preparation",
            description="Prepare for technical and behavioral interviews",
            measurable="Practice sessions completed for both interview types",
        ),
    ]
    
    milestones = [
        RoadmapMilestone(
            milestone_id=f"{roadmap_id}-milestone-1",
            title="Application Pipeline Established",
            description="Consistent application cadence with quality targeting",
            target_days=7,
            success_criteria=[
                "15+ applications submitted",
                "Job tracker established",
                "Follow-up system in place",
            ],
        ),
        RoadmapMilestone(
            milestone_id=f"{roadmap_id}-milestone-2",
            title="Interview-Ready",
            description="Prepared for both technical and behavioral interviews",
            target_days=14,
            success_criteria=[
                "Technical practice completed",
                "Behavioral stories prepared",
                "Mock interview conducted",
            ],
        ),
    ]
    
    actions = [
        RoadmapAction(
            action_id=f"{roadmap_id}-action-1",
            title="Set Up Job Tracker",
            description="Create spreadsheet to track applications, responses, and follow-ups. Include company, role, date applied, and status.",
            deadline_days=1,
            priority="high",
            category="preparation",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-2",
            title="Apply to 15 Target Roles",
            description="Week 1: Submit 15 applications to well-matched positions. Customize each application.",
            deadline_days=7,
            priority="critical",
            category="application",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-3",
            title="Prepare Technical Interview",
            description="Practice coding problems on LeetCode/HackerRank. Focus on easy/medium problems. Complete 10 problems.",
            deadline_days=10,
            priority="high",
            category="preparation",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-4",
            title="Prepare Behavioral Stories",
            description="Document 5 STAR-format stories covering: leadership, conflict, failure, success, teamwork.",
            deadline_days=8,
            priority="high",
            category="preparation",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-5",
            title="Apply to 15 More Roles",
            description="Week 2: Submit 15 additional applications. Continue customization and tracking.",
            deadline_days=14,
            priority="critical",
            category="application",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-6",
            title="Conduct Mock Interview",
            description="Schedule mock interview with peer or use Pramp/Interviewing.io. Practice both technical and behavioral.",
            deadline_days=12,
            priority="medium",
            category="preparation",
        ),
        RoadmapAction(
            action_id=f"{roadmap_id}-action-7",
            title="Optimize LinkedIn Activity",
            description="Engage with target companies: comment on posts, connect with employees, join relevant groups.",
            deadline_days=14,
            priority="medium",
            category="networking",
        ),
    ]
    
    return Roadmap(
        roadmap_id=roadmap_id,
        strategy="HoldPosition",
        phase="execute",
        created_at=created_at,
        strategy_confidence=strategy["current_confidence"],
        goals=goals,
        milestones=milestones,
        actions=actions,
        review_after_days=14,
        estimated_completion_days=14,
        strategy_version=f"{strategy['strategy']}-{strategy['initial_confidence']}-{created_at}",
    )


# =============================================================================
# MAIN ROADMAP GENERATOR
# =============================================================================

def generate_roadmap(
    session: dict,
    enhanced_snapshot: dict = None,
    profile_signals: dict = None,
) -> dict:
    """
    Generate a roadmap for the current strategy.
    
    ELIGIBILITY GATE: A roadmap may ONLY be generated when strategy_state == EXECUTE
    
    This is the main entry point for roadmap generation. It:
    1. Checks eligibility (EXECUTE state required)
    2. Routes to strategy-specific roadmap generator
    3. Returns roadmap or error with clear reason
    
    Args:
        session: AgentSession dict with current_strategy
        enhanced_snapshot: Optional enhanced career snapshot
        profile_signals: Optional profile signals
        
    Returns:
        dict with either:
        - {"roadmap": Roadmap dict, "eligible": True}
        - {"error": str, "eligible": False, "reason": str, "current_state": str}
    """
    # Step 1: Check eligibility (THE GATE)
    eligibility = check_roadmap_eligibility(session)
    
    if not eligibility["eligible"]:
        # Roadmap generation blocked - return clear error
        return {
            "error": "Roadmap generation not allowed",
            "eligible": False,
            "reason": eligibility["reason"],
            "current_state": eligibility["current_state"],
            "recommendation": eligibility.get("recommendation", ""),
        }
    
    # Step 2: Eligibility passed - generate roadmap
    current_strategy = session["current_strategy"]
    strategy_name = current_strategy["strategy"]
    
    # Default snapshot and signals if not provided
    if enhanced_snapshot is None:
        enhanced_snapshot = session.get("stage1_evidence", {}).get("enhanced_snapshot", {})
    if profile_signals is None:
        profile_signals = session.get("stage1_evidence", {}).get("profile_signals", {})
    
    # Step 3: Route to strategy-specific generator
    try:
        if strategy_name == "ResumeOptimization":
            roadmap = generate_roadmap_for_resume_optimization(
                session, enhanced_snapshot, profile_signals
            )
        elif strategy_name == "SkillGapPatch":
            roadmap = generate_roadmap_for_skill_gap_patch(
                session, enhanced_snapshot, profile_signals
            )
        elif strategy_name == "RoleShift":
            roadmap = generate_roadmap_for_role_shift(
                session, enhanced_snapshot, profile_signals
            )
        elif strategy_name == "HoldPosition":
            roadmap = generate_roadmap_for_hold_position(
                session, enhanced_snapshot, profile_signals
            )
        else:
            return {
                "error": f"No roadmap template for strategy: {strategy_name}",
                "eligible": True,  # Eligible but no template
                "reason": f"Strategy '{strategy_name}' is in EXECUTE state but no roadmap generator exists",
            }
        
        # Step 4: Return successful roadmap
        return {
            "roadmap": roadmap.to_dict(),
            "eligible": True,
            "strategy": strategy_name,
            "generated_at": roadmap.created_at,
        }
        
    except Exception as e:
        # Generation error (should be rare)
        import traceback
        return {
            "error": f"Roadmap generation failed: {str(e)}",
            "eligible": True,
            "reason": "Eligible but generation encountered error",
            "traceback": traceback.format_exc(),
        }


# =============================================================================
# ROADMAP INVALIDATION
# =============================================================================

def invalidate_roadmap_if_strategy_changed(
    roadmap: dict,
    current_session: dict
) -> dict:
    """
    Check if roadmap should be invalidated due to strategy change.
    
    A roadmap must be invalidated if:
    - Strategy name has changed
    - Strategy has entered RECONSIDER state
    - Strategy confidence has dropped significantly
    
    Args:
        roadmap: Existing roadmap dict
        current_session: Current agent session
        
    Returns:
        Updated roadmap dict with invalidation status
    """
    if roadmap.get("invalidated"):
        # Already invalidated
        return roadmap
    
    current_strategy = current_session.get("current_strategy", {})
    current_strategy_name = current_strategy.get("strategy")
    current_state = current_strategy.get("strategy_state")
    
    # Check if strategy changed
    if roadmap["strategy"] != current_strategy_name:
        roadmap["invalidated"] = True
        roadmap["invalidation_reason"] = f"Strategy changed from {roadmap['strategy']} to {current_strategy_name}"
        return roadmap
    
    # Check if strategy entered RECONSIDER
    if current_state == StrategyState.RECONSIDER or current_state == "reconsider":
        roadmap["invalidated"] = True
        roadmap["invalidation_reason"] = "Strategy entered RECONSIDER state (failed)"
        return roadmap
    
    # Check if strategy left EXECUTE state
    if current_state != StrategyState.EXECUTE and current_state != "execute":
        roadmap["invalidated"] = True
        roadmap["invalidation_reason"] = f"Strategy no longer in EXECUTE state (current: {current_state})"
        return roadmap
    
    return roadmap


# =============================================================================
# CLI INTERFACE
# =============================================================================

def main():
    """CLI interface for roadmap generation."""
    import traceback
    
    try:
        print(f"[roadmap_generator.py] Reading session from stdin", file=sys.stderr)
        
        stdin_data = sys.stdin.read()
        if not stdin_data:
            print(json.dumps({"error": "No input received on stdin"}))
            sys.exit(1)
        
        session_data = json.loads(stdin_data)
        print(f"[roadmap_generator.py] Parsed session successfully", file=sys.stderr)
        
        # Generate roadmap
        result = generate_roadmap(session_data)
        
        # Log eligibility check
        if result.get("eligible"):
            print(f"[roadmap_generator.py] Roadmap generation ALLOWED - state is EXECUTE", file=sys.stderr)
        else:
            print(f"[roadmap_generator.py] Roadmap generation BLOCKED - {result.get('reason')}", file=sys.stderr)
        
        print(json.dumps(result))
        
    except json.JSONDecodeError as e:
        print(f"[roadmap_generator.py] JSON decode error: {e}", file=sys.stderr)
        print(traceback.format_exc(), file=sys.stderr)
        print(json.dumps({"error": f"Invalid JSON: {str(e)}"}))
        sys.exit(1)
    except Exception as e:
        print(f"[roadmap_generator.py] ERROR: {str(e)}", file=sys.stderr)
        print(traceback.format_exc(), file=sys.stderr)
        print(json.dumps({"error": f"Error: {str(e)}"}))
        sys.exit(1)


if __name__ == "__main__":
    main()
