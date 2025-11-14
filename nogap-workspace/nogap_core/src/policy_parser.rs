use crate::types::Policy;
use std::fs::File;
use std::io::BufReader;

#[derive(thiserror::Error, Debug)]
pub enum PolicyError {
    #[error("Failed to read YAML file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

pub fn load_policy(path: &str) -> Result<Vec<Policy>, PolicyError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let policies: Vec<Policy> = serde_yaml::from_reader(reader)?;
    Ok(policies)
}
