//! Git CAS integration for versioned artifacts

use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitCasError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Object not found: {0}")]
    NotFound(String),
}

/// Git content-addressable storage wrapper (stub implementation)
pub struct GitCas {
    root_path: std::path::PathBuf,
}

impl GitCas {
    /// Initialize or open Git CAS at path
    pub fn open(path: &Path) -> Result<Self, GitCasError> {
        if !path.exists() {
            std::fs::create_dir_all(path)?;
        }

        Ok(Self {
            root_path: path.to_path_buf(),
        })
    }

    /// Store content and return its hash
    pub fn store(&self, content: &[u8]) -> Result<String, GitCasError> {
        let hash = blake3::hash(content).to_string();
        let object_path = self.root_path.join("objects").join(&hash);

        if let Some(parent) = object_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&object_path, content)?;
        Ok(hash)
    }

    /// Retrieve content by hash
    pub fn retrieve(&self, hash: &str) -> Result<Vec<u8>, GitCasError> {
        let object_path = self.root_path.join("objects").join(hash);

        if !object_path.exists() {
            return Err(GitCasError::NotFound(hash.to_string()));
        }

        Ok(std::fs::read(&object_path)?)
    }

    /// Check if content exists
    pub fn exists(&self, hash: &str) -> Result<bool, GitCasError> {
        let object_path = self.root_path.join("objects").join(hash);
        Ok(object_path.exists())
    }
}

