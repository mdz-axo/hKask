//! Security utilities for path sanitization and access control
//!
//! Prevents path traversal attacks in storage operations.

use hkask_types::{HkaskError, InfrastructureError};
use std::path::{Component, Path, PathBuf};

/// Sanitize a user-provided path to prevent path traversal attacks.
///
/// Returns an error if the path contains `..` components or escapes the base directory.
pub fn sanitize_path(base: &Path, input: &str) -> Result<PathBuf, HkaskError> {
    let input_path = Path::new(input);
    if input_path
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(HkaskError::from(InfrastructureError::Io(format!(
            "Path traversal detected: {}",
            input
        ))));
    }
    let joined = base.join(input_path);
    let canonical_base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
    // For non-existent paths, verify the parent is within base
    if let Some(canonical_joined) = joined.parent().and_then(|p| p.canonicalize().ok())
        && !canonical_joined.starts_with(&canonical_base)
    {
        return Err(HkaskError::from(InfrastructureError::Io(format!(
            "Path escapes base directory: {}",
            input
        ))));
    }
    Ok(joined)
}
