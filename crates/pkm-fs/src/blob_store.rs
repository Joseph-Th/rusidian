use sha2::{Digest, Sha256};
use std::path::PathBuf;
use pkm_core::Result;

pub struct BlobStore {
    root: PathBuf,
}

impl BlobStore {
    pub fn new(root: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&root)
            .map_err(|e| pkm_core::CoreError::Invariant(format!("failed to create blob dir: {}", e)))?;
        Ok(BlobStore { root })
    }

    fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }

    pub fn store(&self, data: &[u8]) -> Result<String> {
        let hash = Self::compute_hash(data);
        let path = self.blob_path(&hash);

        if path.exists() {
            return Ok(hash);
        }

        std::fs::write(&path, data)
            .map_err(|e| pkm_core::CoreError::Invariant(format!("failed to write blob: {}", e)))?;

        Ok(hash)
    }

    fn is_valid_hash(hash: &str) -> bool {
        hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())
    }

    pub fn fetch(&self, hash: &str) -> Result<Vec<u8>> {
        if !Self::is_valid_hash(hash) {
            return Err(pkm_core::CoreError::Invariant(format!("invalid hash format: {}", hash)));
        }
        let path = self.blob_path(hash);

        std::fs::read(&path)
            .map_err(|e| pkm_core::CoreError::Invariant(format!("failed to read blob {}: {}", hash, e)))
    }

    pub fn exists(&self, hash: &str) -> bool {
        if !Self::is_valid_hash(hash) {
            return false;
        }
        self.blob_path(hash).exists()
    }

    fn blob_path(&self, hash: &str) -> PathBuf {
        self.root.join(hash)
    }
}
