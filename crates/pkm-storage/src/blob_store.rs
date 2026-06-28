//! Content-addressed blob store for binary attachments.
//!
//! Stores arbitrary binary data (PDFs, images, etc.) without duplicating content.
//! Each blob is identified by its SHA256 hash. Blobs are stored in the app data
//! directory, not in the database.

use crate::StorageError;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, StorageError>;

/// A content-addressed blob store. Blobs are identified by their SHA256 hash.
pub struct BlobStore {
    /// Root directory where blobs are stored (app data dir).
    root: PathBuf,
}

impl BlobStore {
    /// Create a blob store with the given root directory.
    /// The directory is created if it doesn't exist.
    pub fn new(root: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&root)
            .map_err(|e| StorageError::Migration(format!("failed to create blob dir: {}", e)))?;
        Ok(BlobStore { root })
    }

    /// Compute the SHA256 hash of data.
    fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    /// Store a blob and return its hash.
    /// If a blob with the same content already exists, just return its hash (dedup).
    pub fn store(&self, data: &[u8]) -> Result<String> {
        let hash = Self::compute_hash(data);
        let path = self.blob_path(&hash);

        // If it already exists, just return the hash (dedup).
        if path.exists() {
            return Ok(hash);
        }

        // Write the blob to disk.
        std::fs::write(&path, data)
            .map_err(|e| StorageError::Migration(format!("failed to write blob: {}", e)))?;

        Ok(hash)
    }

    fn is_valid_hash(hash: &str) -> bool {
        hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Fetch a blob by its hash.
    pub fn fetch(&self, hash: &str) -> Result<Vec<u8>> {
        if !Self::is_valid_hash(hash) {
            return Err(StorageError::Migration(format!("invalid hash format: {}", hash)));
        }
        let path = self.blob_path(hash);

        std::fs::read(&path)
            .map_err(|e| StorageError::Migration(format!("failed to read blob {}: {}", hash, e)))
    }

    /// Check if a blob exists.
    pub fn exists(&self, hash: &str) -> bool {
        if !Self::is_valid_hash(hash) {
            return false;
        }
        self.blob_path(hash).exists()
    }

    /// Get the filesystem path for a blob given its hash.
    fn blob_path(&self, hash: &str) -> PathBuf {
        self.root.join(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn store_and_fetch_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let store = BlobStore::new(temp_dir.path().to_path_buf()).unwrap();

        let data = b"This is a test blob";
        let hash = store.store(data).unwrap();

        let fetched = store.fetch(&hash).unwrap();
        assert_eq!(fetched, data);
    }

    #[test]
    fn dedup_same_content() {
        let temp_dir = TempDir::new().unwrap();
        let store = BlobStore::new(temp_dir.path().to_path_buf()).unwrap();

        let data = b"Test data";
        let hash1 = store.store(data).unwrap();
        let hash2 = store.store(data).unwrap();

        // Same data should produce the same hash.
        assert_eq!(hash1, hash2);

        // Only one file should exist on disk.
        let entry_count = std::fs::read_dir(temp_dir.path()).unwrap().count();
        assert_eq!(entry_count, 1, "should only store one copy of the blob");
    }

    #[test]
    fn different_content_different_hash() {
        let temp_dir = TempDir::new().unwrap();
        let store = BlobStore::new(temp_dir.path().to_path_buf()).unwrap();

        let data1 = b"First blob";
        let data2 = b"Second blob";

        let hash1 = store.store(data1).unwrap();
        let hash2 = store.store(data2).unwrap();

        // Different data should produce different hashes.
        assert_ne!(hash1, hash2);

        // Both files should exist on disk.
        let entry_count = std::fs::read_dir(temp_dir.path()).unwrap().count();
        assert_eq!(entry_count, 2, "should store both blobs");
    }

    #[test]
    fn exists_returns_true_for_stored_blobs() {
        let temp_dir = TempDir::new().unwrap();
        let store = BlobStore::new(temp_dir.path().to_path_buf()).unwrap();

        let data = b"Test";
        let hash = store.store(data).unwrap();

        assert!(store.exists(&hash));
    }

    #[test]
    fn exists_returns_false_for_nonexistent_blobs() {
        let temp_dir = TempDir::new().unwrap();
        let store = BlobStore::new(temp_dir.path().to_path_buf()).unwrap();

        // Valid SHA256 hex length (64 chars) but not stored
        let valid_but_missing = "a".repeat(64);
        assert!(!store.exists(&valid_but_missing));
        assert!(!store.exists("nonexistent_hash_value"));
    }

    #[test]
    fn path_traversal_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let store = BlobStore::new(temp_dir.path().to_path_buf()).unwrap();

        let bad_hash = "../../../etc/passwd";
        assert!(!store.exists(bad_hash));
        assert!(store.fetch(bad_hash).is_err());
    }

    #[test]
    fn fetch_nonexistent_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let store = BlobStore::new(temp_dir.path().to_path_buf()).unwrap();

        let result = store.fetch("nonexistent_hash_value");
        assert!(result.is_err());
    }
}
