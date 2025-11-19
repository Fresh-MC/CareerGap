use std::fs;
use std::io;
use std::path::PathBuf;
use tempfile::TempDir;

/// Prepares a secure isolated workspace for executing .aegispack files
///
/// Creates a temporary directory and copies the source file into it.
/// The temporary directory will be automatically cleaned up when dropped.
///
/// # Arguments
/// * `src` - Path to the source .aegispack file
///
/// # Returns
/// * `Ok(TempDir)` - Temporary directory containing the copied file
/// * `Err` - IO error if copy fails
pub fn prepare_secure_workspace(src: &str) -> io::Result<TempDir> {
    let dir = TempDir::new()?;
    let dest = dir.path().join("policy.aegispack");
    fs::copy(src, &dest)?;
    println!("✅ Secure workspace ready at {:?}", dir.path());
    Ok(dir)
}

/// Gets the path to the policy file within the secure workspace
pub fn get_workspace_policy_path(workspace: &TempDir) -> PathBuf {
    workspace.path().join("policy.aegispack")
}

/// Verifies that the workspace directory exists and contains the expected file
pub fn verify_workspace(workspace: &TempDir) -> Result<(), String> {
    let policy_path = get_workspace_policy_path(workspace);

    if !policy_path.exists() {
        return Err("Policy file not found in workspace".into());
    }

    if !policy_path.is_file() {
        return Err("Policy path is not a file".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_prepare_secure_workspace() {
        // Create a temporary source file
        let mut src_file = NamedTempFile::new().unwrap();
        src_file.write_all(b"test content").unwrap();

        let workspace = prepare_secure_workspace(src_file.path().to_str().unwrap());
        assert!(workspace.is_ok());

        let workspace = workspace.unwrap();
        let policy_path = get_workspace_policy_path(&workspace);
        assert!(policy_path.exists());

        let content = fs::read_to_string(&policy_path).unwrap();
        assert_eq!(content, "test content");
    }

    #[test]
    fn test_verify_workspace() {
        let mut src_file = NamedTempFile::new().unwrap();
        src_file.write_all(b"test").unwrap();

        let workspace = prepare_secure_workspace(src_file.path().to_str().unwrap()).unwrap();
        assert!(verify_workspace(&workspace).is_ok());
    }
}
