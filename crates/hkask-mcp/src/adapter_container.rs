//! Adapter Container — Shared adapter lifecycle management
//!
//! Provides thread-safe container for adapter instances used by MCP servers.
//! Prevents per-call adapter creation and enables runtime configuration.

use hkask_types::ports::git_cas::GitCASPort;
use std::sync::{Arc, RwLock};

/// Container for shared MCP adapter instances
///
/// Holds the hexagonal GitCASPort and base path for MCP server use.
/// Thread-safe via Arc<RwLock<>> pattern.
#[derive(Clone)]
pub struct AdapterContainer {
    /// Hexagonal GitCASPort for all CAS operations.
    git_cas_port: Arc<RwLock<Option<Arc<dyn GitCASPort>>>>,
}

impl AdapterContainer {
    /// Create new empty adapter container
    pub fn new() -> Self {
        Self {
            git_cas_port: Arc::new(RwLock::new(None)),
        }
    }

    /// Configure the hexagonal GitCASPort for CAS operations.
    ///
    /// MCP servers should call `get_git_cas_port()` to obtain the shared
    /// port instance rather than constructing their own adapters.
    pub fn configure_git_cas_port(&self, port: Arc<dyn GitCASPort>) -> Result<(), String> {
        let mut lock = self.git_cas_port.write().map_err(|e| e.to_string())?;
        *lock = Some(port);

        Ok(())
    }

    /// Get the hexagonal GitCASPort instance.
    ///
    /// Returns `None` if not yet configured. MCP servers should prefer
    /// this over constructing their own `GixCasAdapter`.
    pub fn get_git_cas_port(&self) -> Result<Option<Arc<dyn GitCASPort>>, String> {
        let lock = self.git_cas_port.read().map_err(|e| e.to_string())?;
        Ok(lock.clone())
    }
}

impl Default for AdapterContainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::ports::git_cas::MockGitCas;

    /// Behavioral property: configure_git_cas_port + get_git_cas_port round-trips the same port.
    #[test]
    fn git_cas_port_round_trip() {
        let container = AdapterContainer::new();

        // Before configuration, port is None
        assert!(
            container.get_git_cas_port().unwrap().is_none(),
            "unconfigured container should return None"
        );

        // Configure a port
        let port: Arc<dyn GitCASPort> = Arc::new(MockGitCas::new());
        container.configure_git_cas_port(Arc::clone(&port)).unwrap();

        // After configuration, get returns the same port
        let retrieved = container
            .get_git_cas_port()
            .unwrap()
            .expect("port should be configured after configure_git_cas_port");

        // Both Arcs point to the same underlying object
        assert!(
            Arc::ptr_eq(&retrieved, &port),
            "get_git_cas_port must return the same Arc that was configured"
        );
    }

    /// Behavioral property: AdapterContainer is Clone and shares state.
    #[test]
    fn clone_shares_state() {
        let container = AdapterContainer::new();
        let cloned = container.clone();

        let port: Arc<dyn GitCASPort> = Arc::new(MockGitCas::new());
        container.configure_git_cas_port(port).unwrap();

        // Clone sees the port configured on the original
        assert!(
            cloned.get_git_cas_port().unwrap().is_some(),
            "cloned container must share state with original"
        );
    }
}
