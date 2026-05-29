//! Filesystem adapter for RegistrySourcePort
//!
//! Concrete implementation that reads YAML files from the local filesystem.
//! This adapter lives behind the hexagonal boundary — the domain layer
//! never calls `std::fs` directly.

use crate::error::RegistryError;
use crate::ports::RegistrySourcePort;
use std::fs;

/// Filesystem-backed registry source
pub struct FilesystemRegistrySource;

impl FilesystemRegistrySource {
    pub fn new() -> Self {
        Self
    }
}

impl RegistrySourcePort for FilesystemRegistrySource {
    fn load_yaml(&self, path: &str) -> Result<String, RegistryError> {
        fs::read_to_string(path).map_err(|e| RegistryError::Io(e.to_string()))
    }

    fn list_yaml_files(&self, directory: &str) -> Result<Vec<String>, RegistryError> {
        let mut files = Vec::new();
        let entries = fs::read_dir(directory).map_err(|e| RegistryError::Io(e.to_string()))?;
        for entry in entries {
            let entry = entry.map_err(|e| RegistryError::Io(e.to_string()))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                || path.extension().and_then(|e| e.to_str()) == Some("yml")
            {
                if let Some(path_str) = path.to_str() {
                    files.push(path_str.to_string());
                }
            }
        }
        Ok(files)
    }
}
