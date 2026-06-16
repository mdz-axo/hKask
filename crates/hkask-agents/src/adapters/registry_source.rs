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

impl Default for FilesystemRegistrySource {
    fn default() -> Self {
        Self::new()
    }
}

impl FilesystemRegistrySource {
    /// REQ: AGT-108
    /// pre:  (none).
    /// post: Returns a new `FilesystemRegistrySource` (unit struct).
    pub fn new() -> Self {
        Self
    }
}

impl RegistrySourcePort for FilesystemRegistrySource {
    fn load_yaml(&self, path: &str) -> Result<String, RegistryError> {
        fs::read_to_string(path).map_err(|e| RegistryError::Io(e.to_string()))
    }

    fn list_yaml_files(&self, directory: &str) -> Result<Vec<String>, RegistryError> {
        // If the directory doesn't exist, return an empty list (matching original behavior)
        let dir = match fs::read_dir(directory) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(RegistryError::Io(e.to_string())),
        };
        let mut files = Vec::new();
        for entry in dir {
            let entry = entry.map_err(|e| RegistryError::Io(e.to_string()))?;
            let path = entry.path();
            if (path.extension().and_then(|e| e.to_str()) == Some("yaml")
                || path.extension().and_then(|e| e.to_str()) == Some("yml"))
                && let Some(path_str) = path.to_str()
            {
                files.push(path_str.to_string());
            }
        }
        Ok(files)
    }
}
