//! Registry Source Port — Abstract source for loading registry YAML content
//!
//! Hexagonal port that decouples the registry loader from filesystem I/O.
//! The domain layer depends on this trait; adapters provide concrete implementations.

use crate::error::RegistryError;

/// Port for loading registry content from a source.
pub trait RegistrySourcePort: Send + Sync {
    /// Load the content of a YAML file at the given path
    fn load_yaml(&self, path: &str) -> Result<String, RegistryError>;

    /// List all YAML files in the given directory
    fn list_yaml_files(&self, directory: &str) -> Result<Vec<String>, RegistryError>;
}
