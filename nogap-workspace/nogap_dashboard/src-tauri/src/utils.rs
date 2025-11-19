//! Utility helpers for safe command execution and identifier validation.
use anyhow::Result;
use std::process::Command;

/// Validate an identifier (service name, etc.).
/// Allowed characters: letters, digits, underscore, dash, dot.
#[allow(dead_code)]
pub fn validate_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    for ch in s.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-' | '.' => continue,
            _ => return false,
        }
    }
    true
}

/// Run a command safely without shell interpolation.
/// Ensures `cmd` and the first arg are non-empty and uses `Command::new`.
#[allow(dead_code)]
pub fn run_command_safe(cmd: &str, args: &[&str]) -> Result<std::process::Output> {
    if cmd.trim().is_empty() {
        return Err(anyhow::anyhow!("empty command"));
    }
    if args.is_empty() || args[0].trim().is_empty() {
        return Err(anyhow::anyhow!("first argument is empty"));
    }

    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to execute '{}': {}", cmd, e))?;

    Ok(output)
}
