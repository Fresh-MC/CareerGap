//! Atomic file write operations using temporary files and atomic rename.
use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

/// Atomically write content to a file.
///
/// This function writes the content to a temporary file in the same directory as the target,
/// then atomically renames it to the final path. This ensures that readers never see a
/// partially-written file.
#[allow(dead_code)]
pub fn atomic_write(path: &str, content: &str) -> Result<()> {
    let target_path = Path::new(path);

    // Get the parent directory to create temp file in same location
    let parent_dir = target_path.parent().ok_or_else(|| {
        log::error!("[atomic] Cannot determine parent directory for {}", path);
        anyhow::anyhow!("Cannot determine parent directory for {}", path)
    })?;

    // Create a temporary file in the same directory
    let mut temp_file = NamedTempFile::new_in(parent_dir).with_context(|| {
        log::error!(
            "[atomic] Failed to create temporary file in {:?}",
            parent_dir
        );
        format!("Failed to create temporary file in {:?}", parent_dir)
    })?;

    // Write content to temporary file
    temp_file.write_all(content.as_bytes()).with_context(|| {
        log::error!(
            "[atomic] Failed to write content to temporary file for {}",
            path
        );
        format!("Failed to write content to temporary file for {}", path)
    })?;

    // Flush to ensure all data is written
    temp_file.flush().with_context(|| {
        log::error!("[atomic] Failed to flush temporary file for {}", path);
        format!("Failed to flush temporary file for {}", path)
    })?;

    // Atomically rename temp file to target path
    temp_file.persist(target_path).with_context(|| {
        log::error!("[atomic] Failed to persist temporary file to {}", path);
        format!("Failed to persist temporary file to {}", path)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_atomic_write_new_file() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("nogap_atomic_test_new.txt");
        let test_path = test_file.to_str().unwrap();

        // Clean up if exists
        let _ = fs::remove_file(&test_file);

        let content = "test content\nline 2\n";
        let result = atomic_write(test_path, content);

        assert!(result.is_ok());
        assert!(test_file.exists());

        let read_content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(read_content, content);

        // Clean up
        let _ = fs::remove_file(&test_file);
    }

    #[test]
    fn test_atomic_write_overwrites_existing() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("nogap_atomic_test_overwrite.txt");
        let test_path = test_file.to_str().unwrap();

        // Create initial file
        fs::write(&test_file, "old content").unwrap();

        let new_content = "new content\nupdated\n";
        let result = atomic_write(test_path, new_content);

        assert!(result.is_ok());

        let read_content = fs::read_to_string(&test_file).unwrap();
        assert_eq!(read_content, new_content);

        // Clean up
        let _ = fs::remove_file(&test_file);
    }
}
