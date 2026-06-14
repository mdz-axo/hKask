//! Security utilities for path sanitization and access control
//!
//! Prevents path traversal attacks in storage operations.

use hkask_types::InfrastructureError;
use std::path::{Component, Path, PathBuf};

/// Sanitize a user-provided path to prevent path traversal attacks.
///
/// Returns an error if the path contains `..` components or escapes the base directory.
pub fn sanitize_path(base: &Path, input: &str) -> Result<PathBuf, InfrastructureError> {
    let input_path = Path::new(input);
    if input_path
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(InfrastructureError::Io(format!(
            "Path traversal detected: {}",
            input
        )));
    }
    let joined = base.join(input_path);
    let canonical_base = match base.canonicalize() {
        Ok(cb) => cb,
        Err(_) => {
            // Base doesn't exist — construct canonical by resolving parent.
            // Fail closed: if we can't verify containment, reject.
            if let Some(parent) = base.parent().and_then(|p| p.canonicalize().ok()) {
                parent.join(base.file_name().unwrap_or_default())
            } else {
                return Err(InfrastructureError::Io(format!(
                    "Cannot resolve base directory: {}",
                    base.display()
                )));
            }
        }
    };
    // For non-existent paths, verify the parent is within base
    if let Some(canonical_joined) = joined.parent().and_then(|p| p.canonicalize().ok())
        && !canonical_joined.starts_with(&canonical_base)
    {
        return Err(InfrastructureError::Io(format!(
            "Path escapes base directory: {}",
            input
        )));
    }
    Ok(joined)
}
