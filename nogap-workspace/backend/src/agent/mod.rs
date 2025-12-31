//! Agent Module - Core of the Agentic Career Assistant
//!
//! This module implements the Sense → Plan → Learn loop for career development.
//! 
//! Architecture:
//! - Memory: Persistent timeline of all agent actions
//! - Planner: Goal-driven career roadmap generation
//! - Reflection: Weekly analysis and adaptation
//! - Resume Parser: External Python integration for PDF/DOCX parsing

pub mod memory;
pub mod planner;
pub mod reflection;
pub mod resume_parser;
pub mod types;

pub use memory::*;
pub use planner::*;
pub use reflection::*;
pub use resume_parser::*;
pub use types::*;
