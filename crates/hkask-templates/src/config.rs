//! Generic YAML Configuration Loader
//!
//! Single source of truth for loading YAML manifests.
//! ℏKask v0.21.2

use crate::ports::TemplateError;
use serde::de::DeserializeOwned;

/// Load YAML configuration from file
pub fn load_yaml_config<T: DeserializeOwned>(path: &str) -> Result<T, TemplateError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        TemplateError::Validation(format!("Failed to read config '{}': {}", path, e))
    })?;

    serde_yaml::from_str(&content)
        .map_err(|e| TemplateError::Validation(format!("Failed to parse config '{}': {}", path, e)))
}
