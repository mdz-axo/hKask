//! YAML Manifest Repository Adapter
//!
//! Implements ManifestRepository trait for YAML file system persistence.
//! Per hexagonal architecture: this is an adapter that implements the repository port.

use crate::ports::{ManifestRepository, ProcessManifest, Result, TemplateError};
use crate::rate_limiter::RateLimiter;
use std::path::PathBuf;

/// YAML file system manifest repository
///
/// Stores manifests as YAML files in a base directory.
/// File naming convention: `{manifest_id}.yaml`
pub struct FileSystemManifestRepository {
    base_path: PathBuf,
    rate_limiter: RateLimiter,
}

impl FileSystemManifestRepository {
    /// Create new repository with base path
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            rate_limiter: RateLimiter::with_defaults(),
        }
    }

    /// Create repository with custom rate limiting
    pub fn with_rate_limit(base_path: PathBuf, max_tokens: u64, refill_rate: u64) -> Self {
        Self {
            base_path,
            rate_limiter: RateLimiter::new(max_tokens, refill_rate),
        }
    }

    /// Create repository with default path (./registry/manifests)
    pub fn with_default_path() -> Self {
        Self {
            base_path: PathBuf::from("registry/manifests"),
            rate_limiter: RateLimiter::with_defaults(),
        }
    }

    /// Get path for manifest ID
    fn manifest_path(&self, id: &str) -> PathBuf {
        // Sanitize ID to prevent path traversal
        let safe_id = id.replace(['/', '\\'], "_").replace("..", "_");
        self.base_path.join(format!("{}.yaml", safe_id))
    }
}

impl ManifestRepository for FileSystemManifestRepository {
    fn load(&self, id: &str) -> Result<ProcessManifest> {
        // Check rate limit first
        if !self.rate_limiter.try_acquire() {
            return Err(TemplateError::RateLimitExceeded(
                "Manifest load rate limit exceeded".to_string(),
            ));
        }

        let path = self.manifest_path(id);

        if !path.exists() {
            return Err(TemplateError::NotFound(format!("Manifest {}", id)));
        }

        ProcessManifest::load_from_yaml(&path)
    }

    fn save(&self, manifest: &ProcessManifest) -> Result<()> {
        // Check rate limit first
        if !self.rate_limiter.try_acquire() {
            return Err(TemplateError::RateLimitExceeded(
                "Manifest save rate limit exceeded".to_string(),
            ));
        }

        let path = self.manifest_path(&manifest.id);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                TemplateError::Manifest(format!("Failed to create directory: {}", e))
            })?;
        }

        let yaml_content = serde_yaml::to_string(manifest)
            .map_err(|e| TemplateError::Manifest(format!("Failed to serialize manifest: {}", e)))?;

        std::fs::write(&path, yaml_content).map_err(|e| {
            TemplateError::Manifest(format!("Failed to write manifest file: {}", e))
        })?;

        Ok(())
    }

    fn delete(&self, id: &str) -> Result<()> {
        // Check rate limit first
        if !self.rate_limiter.try_acquire() {
            return Err(TemplateError::RateLimitExceeded(
                "Manifest delete rate limit exceeded".to_string(),
            ));
        }

        let path = self.manifest_path(id);

        if !path.exists() {
            return Err(TemplateError::NotFound(format!("Manifest {}", id)));
        }

        std::fs::remove_file(&path).map_err(|e| {
            TemplateError::Manifest(format!("Failed to delete manifest file: {}", e))
        })?;

        Ok(())
    }

    fn list(&self) -> Result<Vec<String>> {
        // Check rate limit first
        if !self.rate_limiter.try_acquire() {
            return Err(TemplateError::RateLimitExceeded(
                "Manifest list rate limit exceeded".to_string(),
            ));
        }

        if !self.base_path.exists() {
            return Ok(vec![]);
        }

        let mut manifests = Vec::new();

        let entries = std::fs::read_dir(&self.base_path).map_err(|e| {
            TemplateError::Manifest(format!("Failed to read manifest directory: {}", e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                TemplateError::Manifest(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();

            // Only process .yaml files
            if path.extension().and_then(|s| s.to_str()) == Some("yaml")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            {
                manifests.push(stem.to_string());
            }
        }

        Ok(manifests)
    }
}

/// In-memory manifest repository for testing
pub struct InMemoryManifestRepository {
    manifests: std::sync::Mutex<std::collections::HashMap<String, ProcessManifest>>,
}

impl InMemoryManifestRepository {
    pub fn new() -> Self {
        Self {
            manifests: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn with_manifests(manifests: Vec<ProcessManifest>) -> Self {
        let mut map = std::collections::HashMap::new();
        for manifest in manifests {
            map.insert(manifest.id.clone(), manifest);
        }
        Self {
            manifests: std::sync::Mutex::new(map),
        }
    }
}

impl Default for InMemoryManifestRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl ManifestRepository for InMemoryManifestRepository {
    fn load(&self, id: &str) -> Result<ProcessManifest> {
        let manifests = self.manifests.lock().unwrap();
        manifests
            .get(id)
            .cloned()
            .ok_or_else(|| TemplateError::NotFound(format!("Manifest {}", id)))
    }

    fn save(&self, manifest: &ProcessManifest) -> Result<()> {
        let mut manifests = self.manifests.lock().unwrap();
        manifests.insert(manifest.id.clone(), manifest.clone());
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<()> {
        let mut manifests = self.manifests.lock().unwrap();
        manifests
            .remove(id)
            .ok_or_else(|| TemplateError::NotFound(format!("Manifest {}", id)))?;
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>> {
        let manifests = self.manifests.lock().unwrap();
        Ok(manifests.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{Action, ManifestStep};
    use tempfile::TempDir;

    #[test]
    fn test_file_system_repository_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FileSystemManifestRepository::new(temp_dir.path().to_path_buf());

        let manifest = ProcessManifest {
            id: "test-manifest".to_string(),
            name: "Test Manifest".to_string(),
            description: "A test manifest".to_string(),
            steps: vec![ManifestStep {
                ordinal: 1,
                action: Action::Select,
                description: "Select template".to_string(),
                template_ref: "prompt/selector".to_string(),
                model_tier: Some("fast_local".to_string()),
                mcp: Some("hkask-mcp-inference".to_string()),
                renderer: Some("minijinja".to_string()),
            }],
        };

        // Save
        repo.save(&manifest).unwrap();

        // Load
        let loaded = repo.load("test-manifest").unwrap();
        assert_eq!(loaded.id, manifest.id);
        assert_eq!(loaded.name, manifest.name);
        assert_eq!(loaded.steps.len(), 1);
    }

    #[test]
    fn test_file_system_repository_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FileSystemManifestRepository::new(temp_dir.path().to_path_buf());

        let result = repo.load("nonexistent");
        assert!(matches!(result, Err(TemplateError::NotFound(_))));
    }

    #[test]
    fn test_file_system_repository_list() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FileSystemManifestRepository::new(temp_dir.path().to_path_buf());

        // Initially empty
        assert!(repo.list().unwrap().is_empty());

        // Save some manifests
        for i in 1..=3 {
            let manifest = ProcessManifest {
                id: format!("manifest-{}", i),
                name: format!("Manifest {}", i),
                description: "Test".to_string(),
                steps: vec![],
            };
            repo.save(&manifest).unwrap();
        }

        // List should return all
        let list = repo.list().unwrap();
        assert_eq!(list.len(), 3);
        assert!(list.contains(&"manifest-1".to_string()));
        assert!(list.contains(&"manifest-2".to_string()));
        assert!(list.contains(&"manifest-3".to_string()));
    }

    #[test]
    fn test_file_system_repository_delete() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FileSystemManifestRepository::new(temp_dir.path().to_path_buf());

        let manifest = ProcessManifest {
            id: "to-delete".to_string(),
            name: "To Delete".to_string(),
            description: "Will be deleted".to_string(),
            steps: vec![],
        };

        repo.save(&manifest).unwrap();
        assert!(repo.load("to-delete").is_ok());

        repo.delete("to-delete").unwrap();
        assert!(matches!(
            repo.load("to-delete"),
            Err(TemplateError::NotFound(_))
        ));
    }

    #[test]
    fn test_in_memory_repository() {
        let manifest = ProcessManifest {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            steps: vec![],
        };

        let repo = InMemoryManifestRepository::with_manifests(vec![manifest.clone()]);

        // Load
        let loaded = repo.load("test").unwrap();
        assert_eq!(loaded.id, manifest.id);

        // Save new
        let manifest2 = ProcessManifest {
            id: "test2".to_string(),
            name: "Test2".to_string(),
            description: "Test2".to_string(),
            steps: vec![],
        };
        repo.save(&manifest2).unwrap();

        // List
        let list = repo.list().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&"test".to_string()));
        assert!(list.contains(&"test2".to_string()));

        // Delete
        repo.delete("test").unwrap();
        assert!(matches!(repo.load("test"), Err(TemplateError::NotFound(_))));
    }

    #[test]
    fn test_file_system_repository_rate_limiting() {
        let temp_dir = TempDir::new().unwrap();
        // Create repo with very low rate limit (2 tokens, no refill for test duration)
        let repo =
            FileSystemManifestRepository::with_rate_limit(temp_dir.path().to_path_buf(), 2, 0);

        // First two operations should succeed
        let manifest = ProcessManifest {
            id: "rate-test".to_string(),
            name: "Rate Test".to_string(),
            description: "Test".to_string(),
            steps: vec![],
        };

        assert!(repo.save(&manifest).is_ok());
        assert!(repo.load("rate-test").is_ok());

        // Third operation should fail due to rate limit
        let result = repo.load("rate-test");
        assert!(matches!(result, Err(TemplateError::RateLimitExceeded(_))));
    }

    #[test]
    fn test_manifest_path_sanitization() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FileSystemManifestRepository::new(temp_dir.path().to_path_buf());

        // Path traversal attempt should be sanitized
        let path = repo.manifest_path("../../../etc/passwd");
        assert!(!path.to_string_lossy().contains(".."));
        assert!(path.to_string_lossy().contains("_"));
    }
}
