//! Git CAS adapter bundle (P2.2).
//!
//! Extracted from `ApiState::new()` to keep CAS initialization self-contained
//! and to surface the `expect("Failed to create GixCasAdapter")` failure as a
//! typed `ApiError::Internal` rather than a panic at startup (P4.1).

use std::path::PathBuf;
use std::sync::Arc;

use crate::error::ApiError;

/// Git CAS adapter bundle (P2.2).
///
/// Extracted from `ApiState::new()` to keep CAS initialization self-contained
/// and to surface the `expect("Failed to create GixCasAdapter")` failure as a
/// typed `ApiError::Internal` rather than a panic at startup (P4.1).
pub(crate) struct GitCasBundle {
    /// Concrete `GitCasAdapter` (legacy — template loading only).
    pub git_cas: Arc<hkask_mcp::GitCasAdapter>,
    /// Trait-object `GitCASPort` (hexagonal boundary) used by stores.
    pub git_cas_port: Arc<dyn hkask_types::ports::git_cas::GitCASPort>,
}

/// Initialize the Git CAS adapter and the trait-object port.
///
/// `git_cas` writes to a fixed on-disk directory; `git_cas_port` resolves
/// from `GIX_*` env vars when present and falls back to the same directory.
///
/// P4.1: Returns `Result<GitCasBundle, ApiError>` so CAS initialization
/// failures surface as typed errors instead of panics. The hard-coded
/// `/tmp/hkask-templates` fallback directory is the documented invariant
/// of this function — if even that cannot be created, returning
/// `ApiError::Internal` is the correct (non-panicking) failure mode.
pub(crate) fn init_git_cas() -> Result<GitCasBundle, ApiError> {
    let git_cas = Arc::new(hkask_mcp::GitCasAdapter::from_path(PathBuf::from(
        "/tmp/hkask-templates",
    )));
    let fallback_path = PathBuf::from("/tmp/hkask-templates");
    let git_cas_port: Arc<dyn hkask_types::ports::git_cas::GitCASPort> = Arc::new(
        hkask_mcp::GixCasAdapter::from_env()
            .or_else(|_| hkask_mcp::GixCasAdapter::new(fallback_path))
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to create GixCasAdapter: {e}"),
            })?,
    );
    Ok(GitCasBundle {
        git_cas,
        git_cas_port,
    })
}
