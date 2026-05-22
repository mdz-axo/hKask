//! Adapter Container — Shared adapter lifecycle management
//!
//! Provides thread-safe container for adapter instances used by MCP servers.
//! Prevents per-call adapter creation and enables runtime configuration.

use hkask_agents::adapters::git_cas::GitCasAdapter;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Container for Git CAS adapter
///
/// Holds shared adapter instance for MCP server use.
/// Thread-safe via Arc<RwLock<>> pattern.
#[derive(Clone)]
pub struct AdapterContainer {
    git_cas: Arc<RwLock<Option<Arc<GitCasAdapter>>>>,
    base_path: Arc<RwLock<Option<PathBuf>>>,
}

impl AdapterContainer {
    /// Create new empty adapter container
    pub fn new() -> Self {
        Self {
            git_cas: Arc::new(RwLock::new(None)),
            base_path: Arc::new(RwLock::new(None)),
        }
    }

    /// Configure Git CAS adapter with base path
    pub fn configure_git_cas(&self, base_path: PathBuf) -> Result<(), String> {
        let adapter = GitCasAdapter::from_path(base_path.clone());

        let mut cas_lock = self.git_cas.write().map_err(|e| e.to_string())?;
        *cas_lock = Some(Arc::new(adapter));

        let mut path_lock = self.base_path.write().map_err(|e| e.to_string())?;
        *path_lock = Some(base_path);

        Ok(())
    }

    /// Get Git CAS adapter instance
    pub fn get_git_cas(&self) -> Option<Arc<GitCasAdapter>> {
        let cas_lock = self.git_cas.read().expect("Adapter lock poisoned");
        cas_lock.clone()
    }

    /// Check if Git CAS adapter is configured
    pub fn has_git_cas(&self) -> bool {
        let cas_lock = self.git_cas.read().expect("Adapter lock poisoned");
        cas_lock.is_some()
    }

    /// Get configured base path
    pub fn get_base_path(&self) -> Option<PathBuf> {
        let path_lock = self.base_path.read().expect("Base path lock poisoned");
        path_lock.clone()
    }

    /// Clear adapter configuration
    pub fn clear(&self) {
        let mut cas_lock = self.git_cas.write().expect("Adapter lock poisoned");
        *cas_lock = None;

        let mut path_lock = self.base_path.write().expect("Base path lock poisoned");
        *path_lock = None;
    }
}

impl Default for AdapterContainer {
    fn default() -> Self {
        Self::new()
    }
}
