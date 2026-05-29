//! Adapter Container — Shared adapter lifecycle management
//!
//! Provides thread-safe container for adapter instances used by MCP servers.
//! Prevents per-call adapter creation and enables runtime configuration.

use hkask_types::GitCASPort;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Container for Git CAS adapter
///
/// Holds shared adapter instance for MCP server use.
/// Thread-safe via Arc<RwLock<>> pattern.
#[derive(Clone)]
pub struct AdapterContainer {
    git_cas: Arc<RwLock<Option<Arc<dyn GitCASPort + Send + Sync>>>>,
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

    /// Configure Git CAS adapter with a pre-built implementation
    pub fn configure_git_cas(
        &self,
        adapter: Arc<dyn GitCASPort + Send + Sync>,
    ) -> Result<(), String> {
        let mut cas_lock = self.git_cas.write().map_err(|e| e.to_string())?;
        *cas_lock = Some(adapter);

        Ok(())
    }

    /// Set the base path for future adapter configuration
    pub fn set_base_path(&self, base_path: PathBuf) -> Result<(), String> {
        let mut path_lock = self.base_path.write().map_err(|e| e.to_string())?;
        *path_lock = Some(base_path);

        Ok(())
    }

    /// Get Git CAS adapter instance
    pub fn get_git_cas(&self) -> Result<Option<Arc<dyn GitCASPort + Send + Sync>>, String> {
        let cas_lock = self.git_cas.read().map_err(|e| e.to_string())?;
        Ok(cas_lock.clone())
    }

    /// Check if Git CAS adapter is configured
    pub fn has_git_cas(&self) -> Result<bool, String> {
        let cas_lock = self.git_cas.read().map_err(|e| e.to_string())?;
        Ok(cas_lock.is_some())
    }

    /// Get configured base path
    pub fn get_base_path(&self) -> Result<Option<PathBuf>, String> {
        let path_lock = self.base_path.read().map_err(|e| e.to_string())?;
        Ok(path_lock.clone())
    }

    /// Clear adapter configuration
    pub fn clear(&self) -> Result<(), String> {
        {
            let mut cas_lock = self.git_cas.write().map_err(|e| e.to_string())?;
            *cas_lock = None;
        }
        let mut path_lock = self.base_path.write().map_err(|e| e.to_string())?;
        *path_lock = None;
        Ok(())
    }
}

impl Default for AdapterContainer {
    fn default() -> Self {
        Self::new()
    }
}
