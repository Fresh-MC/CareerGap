use crate::types::Policy;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum PolicyError {
    #[error("Failed to read YAML file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// Resolve the policy file path using standardized lookup logic:
/// 1. Check for repository copy (../../nogap_core/policies.yaml from dashboard dev runtime)
/// 2. If ./policies.yaml exists in cwd (dev mode), use that
/// 3. On Windows, use C:\ProgramData\NoGap\policies.yaml
/// 4. On Linux, use /etc/nogap/policies.yaml
pub fn resolve_policy_path() -> PathBuf {
    // FIRST: Check for repository copy (dev mode from dashboard)
    let repo_relative_path = Path::new("../../nogap_core/policies.yaml");
    if repo_relative_path.exists() {
        if let Ok(canonical) = repo_relative_path.canonicalize() {
            return canonical;
        }
        return repo_relative_path.to_path_buf();
    }
    
    // SECOND: Check current working directory
    let local_path = Path::new("./policies.yaml");
    if local_path.exists() {
        if let Ok(canonical) = local_path.canonicalize() {
            return canonical;
        }
        return local_path.to_path_buf();
    }
    
    // THIRD: Use system installation paths
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(r"C:\ProgramData\NoGap\policies.yaml")
    }
    
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/etc/nogap/policies.yaml")
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        PathBuf::from("./policies.yaml")
    }
}

pub fn load_policy(path: &str) -> Result<Vec<Policy>, PolicyError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let policies: Vec<Policy> = serde_yaml::from_reader(reader)?;
    Ok(policies)
}
