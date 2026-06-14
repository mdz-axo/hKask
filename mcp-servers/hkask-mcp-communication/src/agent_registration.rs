//! Agent registration and thread routing for the Matrix-based communication server.
//!
//! On `pod activate`, each replicant auto-registers as a Matrix user on the
//! local Conduit homeserver. Threads are the unit of attention; agents can
//! monitor threads (watchlist) or be tagged into discussions (@mentions).
//!
//! CNS spans route algedonic signals for thread lifecycle events:
//!   `cns.communication.thread.{created,escalated,resolved}`
//!
//! The 7R7 listener polls Matrix rooms and emits CNS observation spans.
//! The agent layer (Curator + skills + templates) decides what action to take.

use crate::matrix::{RoomId, UserId};
use hkask_types::WebID;
use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing;

// ── Agent registry ─────────────────────────────────────────────────────────

/// Maps replicant WebIDs to their Matrix user identities.
///
/// Maintained in sync across pod activation/deactivation events.
#[derive(Debug, Default)]
pub struct AgentRegistry {
    /// Mapping from replicant WebID (string) to Matrix UserId.
    entries: RwLock<HashMap<String, UserId>>,
    /// Mapping from room ID to list of agents monitoring it.
    thread_watchlists: RwLock<HashMap<RoomId, Vec<String>>>,
}

impl AgentRegistry {
    /// Create an empty agent registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a WebID → Matrix UserId mapping.
    ///
    /// Called after `kask matrix register --agent` succeeds.
    /// Does NOT perform Matrix registration — that is done by the CLI
    /// via Conduit's admin API.
    pub async fn record_mapping(&self, webid: &WebID, user_id: &UserId) {
        self.entries
            .write()
            .await
            .insert(webid.to_string(), user_id.clone());
        tracing::info!(
            target: "cns.communication.agent.registered",
            webid = %webid.redacted_display(),
            matrix_user = %user_id.as_str(),
            "Agent Matrix mapping recorded"
        );
    }

    /// Deregister a replicant.
    pub async fn deregister(&self, webid: &WebID) -> Result<(), AgentRegistrationError> {
        let webid_str = webid.to_string();
        let removed = self.entries.write().await.remove(&webid_str);
        if removed.is_some() {
            tracing::info!(
                target: "cns.communication.agent.deregistered",
                webid = %webid.redacted_display(),
                "Agent deregistered from Matrix"
            );
        }
        Ok(())
    }

    /// Resolve a WebID to its Matrix UserId.
    pub async fn resolve(&self, webid: &WebID) -> Option<UserId> {
        self.entries.read().await.get(&webid.to_string()).cloned()
    }

    /// Add a thread to an agent's watchlist.
    pub async fn monitor_thread(
        &self,
        webid: &WebID,
        room_id: &RoomId,
    ) -> Result<(), AgentRegistrationError> {
        let webid_str = webid.to_string();
        {
            let entries = self.entries.read().await;
            if !entries.contains_key(&webid_str) {
                return Err(AgentRegistrationError::NotRegistered(webid_str));
            }
        }
        self.thread_watchlists
            .write()
            .await
            .entry(room_id.clone())
            .or_default()
            .push(webid_str);
        tracing::info!(
            target: "cns.communication.thread.monitored",
            webid = %webid.redacted_display(),
            room_id = %room_id.as_str(),
            "Agent added to thread watchlist"
        );
        Ok(())
    }

    /// Get agents monitoring a given thread.
    pub async fn get_watchers(&self, room_id: &RoomId) -> Vec<String> {
        self.thread_watchlists
            .read()
            .await
            .get(room_id)
            .cloned()
            .unwrap_or_default()
    }
}

// ── Registration errors ────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum AgentRegistrationError {
    #[error("Agent not registered: {0}")]
    NotRegistered(String),
    #[error("Lock error: {0}")]
    Lock(String),
}
