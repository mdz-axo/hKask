//! DaemonHandler implementation — bridges the Unix socket daemon to hKask's
//! PodManager, UserStore, and memory infrastructure.
//!
//! This is the hKask-side implementation of the `DaemonHandler` trait defined
//! in `hkask-mcp`. It wires daemon queries to the live agent and memory stack.

use std::sync::Arc;

use hkask_agents::pod::PodManager;
use hkask_mcp::daemon::DaemonHandler;
use hkask_storage::user_store::UserStore;
use tracing;

/// hKask-side implementation of the daemon handler trait.
///
/// Wraps PodManager for assignment/capability/memory queries and
/// UserStore for authentication. Created during AgentService::build()
/// and passed to the DaemonListener.
pub struct ServiceDaemonHandler {
    pod_manager: Arc<PodManager>,
    user_store: Arc<std::sync::Mutex<UserStore>>,
}

impl ServiceDaemonHandler {
    pub fn new(pod_manager: Arc<PodManager>, user_store: Arc<std::sync::Mutex<UserStore>>) -> Self {
        Self {
            pod_manager,
            user_store,
        }
    }
}

#[async_trait::async_trait]
impl DaemonHandler for ServiceDaemonHandler {
    async fn check_auth(&self, replicant: &str) -> (bool, Option<String>) {
        // Collect all user-store data before any await (MutexGuard is not Send)
        let has_sessions = {
            let store = match self.user_store.lock() {
                Ok(s) => s,
                Err(_) => {
                    tracing::error!(target: "hkask.daemon", "UserStore lock poisoned");
                    return (false, None);
                }
            };
            let exists = store.get_replicant(replicant).is_ok();
            if !exists {
                tracing::warn!(target: "hkask.daemon", replicant = %replicant, "Replicant not found in user store");
                return (false, None);
            }
            let sessions = store.list_sessions(replicant).unwrap_or_default();
            !sessions.is_empty()
        };

        if !has_sessions {
            tracing::debug!(target: "hkask.daemon", replicant = %replicant, "No active sessions — needs passphrase");
            return (false, None);
        }

        tracing::debug!(target: "hkask.daemon", replicant = %replicant, "Replicant has active sessions");
        // Now safe to await — MutexGuard dropped
        if let Some(pod_id) = self.pod_manager.find_pod_by_name(replicant).await {
            let webid = self.pod_manager.get_pod_webid(&pod_id).await;
            (true, webid.map(|w| w.to_string()))
        } else {
            (false, None)
        }
    }

    async fn check_assignment(&self, replicant: &str, role: &str) -> bool {
        match self.pod_manager.find_pod_by_name(replicant).await {
            Some(pod_id) => {
                let assigned = self.pod_manager.is_assigned_to_role(&pod_id, role).await;
                tracing::debug!(
                    target: "hkask.daemon",
                    replicant = %replicant,
                    role = %role,
                    assigned = assigned,
                    "Assignment check"
                );
                assigned
            }
            None => {
                tracing::warn!(
                    target: "hkask.daemon",
                    replicant = %replicant,
                    "Pod not found for assignment check"
                );
                false
            }
        }
    }

    async fn check_capability(&self, replicant: &str, tool: &str) -> bool {
        match self.pod_manager.find_pod_by_name(replicant).await {
            Some(pod_id) => {
                let granted = self.pod_manager.has_capability(&pod_id, tool).await;
                tracing::debug!(
                    target: "hkask.daemon",
                    replicant = %replicant,
                    tool = %tool,
                    granted = granted,
                    "Capability check"
                );
                granted
            }
            None => false,
        }
    }

    async fn store_experience(
        &self,
        replicant: &str,
        entity: &str,
        attribute: &str,
        value: &serde_json::Value,
        confidence: Option<f64>,
    ) -> (bool, Option<String>, Option<String>) {
        let pod_id = match self.pod_manager.find_pod_by_name(replicant).await {
            Some(id) => id,
            None => {
                tracing::warn!(
                    target: "hkask.daemon",
                    replicant = %replicant,
                    "Pod not found for store_experience"
                );
                return (false, None, None);
            }
        };

        let ctx =
            match hkask_agents::pod::PodContext::from_manager(&self.pod_manager, &pod_id).await {
                Ok(ctx) => ctx,
                Err(e) => {
                    tracing::warn!(
                        target: "hkask.daemon",
                        replicant = %replicant,
                        error = %e,
                        "Failed to create PodContext for store_experience"
                    );
                    return (false, None, None);
                }
            };

        let conf = confidence.unwrap_or(0.85);

        // Store episodic (first-person, private, perspective-scoped)
        let episodic_result = ctx.store_episodic(
            entity,
            attribute,
            value.clone(),
            hkask_types::Confidence::new(conf),
        );

        // Store semantic (third-person, public, no perspective)
        let semantic_value = generalize_value(value);
        let semantic_result = ctx.store_semantic(
            entity,
            attribute,
            semantic_value,
            hkask_types::Confidence::new(conf),
        );

        match (episodic_result, semantic_result) {
            (Ok(ep_id), Ok(sem_id)) => {
                tracing::debug!(
                    target: "hkask.daemon",
                    replicant = %replicant,
                    episodic_id = %ep_id,
                    semantic_id = %sem_id,
                    "Dual-encoded experience"
                );
                (true, Some(ep_id), Some(sem_id))
            }
            (Ok(ep_id), Err(e)) => {
                tracing::warn!(
                    target: "hkask.daemon",
                    replicant = %replicant,
                    episodic_id = %ep_id,
                    semantic_error = %e,
                    "Episodic stored, semantic failed"
                );
                (true, Some(ep_id), None)
            }
            (Err(e), _) => {
                tracing::warn!(
                    target: "hkask.daemon",
                    replicant = %replicant,
                    error = %e,
                    "Failed to store episodic experience"
                );
                (false, None, None)
            }
        }
    }
}

/// Generalize a value for semantic memory by stripping caller-specific details
/// while preserving the generalizable pattern.
fn generalize_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut generalized = serde_json::Map::new();
            if let Some(tool) = map.get("tool") {
                generalized.insert("tool".to_string(), tool.clone());
            }
            if let Some(outcome) = map.get("outcome") {
                generalized.insert("outcome".to_string(), outcome.clone());
            }
            generalized.insert("generalized".to_string(), serde_json::Value::Bool(true));
            serde_json::Value::Object(generalized)
        }
        other => other.clone(),
    }
}
