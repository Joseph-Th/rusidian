//! Transactional file operations with write-ahead logging pattern.
//!
//! Ensures atomic file updates: write to temp file, update database,
//! then rename (atomic at OS level). If database fails, temp file is cleaned up.

use std::path::Path;
use std::fs;
use std::io;

/// Write file atomically: temp file -> database -> rename.
/// Returns path to temp file; caller must rename after DB succeeds or delete on failure.
pub fn write_to_temp_file(vault_path: &Path, note_id: &str, content: &str) -> io::Result<std::path::PathBuf> {
    let temp_path = vault_path.join(format!("{}.tmp", note_id));
    fs::write(&temp_path, content)?;
    Ok(temp_path)
}

/// Atomically replace old file with temp file (OS-level atomic rename).
pub fn commit_temp_file(temp_path: &Path, target_path: &Path) -> io::Result<()> {
    fs::rename(temp_path, target_path)
}

/// Clean up temp file if database operation failed.
pub fn abort_temp_file(temp_path: &Path) -> io::Result<()> {
    if temp_path.exists() {
        fs::remove_file(temp_path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_write_and_commit_temp_file() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path();
        let target = vault_path.join("test.md");

        // Write to temp
        let temp = write_to_temp_file(vault_path, "test", "content").unwrap();
        assert!(temp.exists());

        // Commit (rename)
        commit_temp_file(&temp, &target).unwrap();
        assert!(!temp.exists());
        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "content");
    }

    #[test]
    fn test_abort_temp_file() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path();

        let temp = write_to_temp_file(vault_path, "test", "content").unwrap();
        assert!(temp.exists());

        abort_temp_file(&temp).unwrap();
        assert!(!temp.exists());
    }
}
