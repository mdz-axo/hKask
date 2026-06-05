//! Unified session manager for both chat and deliberation sessions.

use hkask_types::WebID;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::chat::EnsembleChat;
use crate::deliberation::DeliberationSession;
use crate::ports::GasGovernancePort;

/// Unified session manager for both chat and deliberation sessions.
///
/// Collapses the former `EnsembleChatManager` and `DeliberationCoordinator` into
/// a single manager that handles both session types.
///
/// Since internal maps are `Arc<RwLock<...>>`, cloning shares the same data.
/// Use `clone_shared()` to get a handle that shares session state with the original.
#[derive(Clone)]
pub struct SessionManager {
    chats: Arc<RwLock<HashMap<String, Arc<RwLock<EnsembleChat>>>>>,
    deliberations: Arc<RwLock<HashMap<String, Arc<RwLock<DeliberationSession>>>>>,
    curator_webid: WebID,
    gas_governance: Option<Arc<dyn GasGovernancePort>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(curator_webid: WebID) -> Self {
        Self {
            chats: Arc::new(RwLock::new(HashMap::new())),
            deliberations: Arc::new(RwLock::new(HashMap::new())),
            curator_webid,
            gas_governance: None,
        }
    }

    /// Set the CNS gas governance port for all future chat sessions.
    ///
    /// When set, every `EnsembleChat` created by this manager will be
    /// wired with the governance port so the CNS can observe ensemble
    /// gas usage via the CyberneticsLoop.
    pub fn with_gas_governance(mut self, port: Arc<dyn GasGovernancePort>) -> Self {
        self.gas_governance = Some(port);
        self
    }

    /// Get a shared handle to this session manager.
    ///
    /// Equivalent to `.clone()` — since internal maps are `Arc<RwLock<...>>`,
    /// the returned handle shares all session state with the original.
    /// Use this when passing the session manager to the API server so that
    /// CLI and API share the same sessions.
    pub fn clone_shared(&self) -> Self {
        self.clone()
    }

    /// Create a new chat session
    pub async fn create_chat(&self, session_id: &str) -> Arc<RwLock<EnsembleChat>> {
        let mut chat = EnsembleChat::new(self.curator_webid);
        if let Some(ref governance) = self.gas_governance {
            chat = chat.with_gas_governance(Arc::clone(governance));
        }
        let chat = Arc::new(RwLock::new(chat));

        let mut chats = self.chats.write().await;
        chats.insert(session_id.to_string(), chat.clone());

        chat
    }

    /// Get a chat session
    pub async fn get_chat(&self, session_id: &str) -> Option<Arc<RwLock<EnsembleChat>>> {
        let chats = self.chats.read().await;
        chats.get(session_id).cloned()
    }

    /// Delete a chat session
    pub async fn delete_chat(&self, session_id: &str) -> bool {
        let mut chats = self.chats.write().await;
        chats.remove(session_id).is_some()
    }

    /// Create a new deliberation session
    pub async fn create_deliberation(&self, session_id: &str) -> Arc<RwLock<DeliberationSession>> {
        let session = Arc::new(RwLock::new(DeliberationSession::new(
            session_id.to_string(),
            self.curator_webid,
        )));

        let mut deliberations = self.deliberations.write().await;
        deliberations.insert(session_id.to_string(), session.clone());

        session
    }

    /// Get a deliberation session
    pub async fn get_deliberation(
        &self,
        session_id: &str,
    ) -> Option<Arc<RwLock<DeliberationSession>>> {
        let deliberations = self.deliberations.read().await;
        deliberations.get(session_id).cloned()
    }

    /// Remove a deliberation session
    pub async fn remove_deliberation(&self, session_id: &str) -> bool {
        let mut deliberations = self.deliberations.write().await;
        deliberations.remove(session_id).is_some()
    }

    /// List all active chat sessions
    pub async fn list_chat_sessions(&self) -> Vec<String> {
        let chats = self.chats.read().await;
        chats.keys().cloned().collect()
    }

    /// List all active deliberation sessions
    pub async fn list_deliberation_sessions(&self) -> Vec<String> {
        let deliberations = self.deliberations.read().await;
        deliberations.keys().cloned().collect()
    }

    /// List all session IDs (both chat and deliberation)
    pub async fn list_all_sessions(&self) -> Vec<String> {
        let chats = self.chats.read().await;
        let deliberations = self.deliberations.read().await;
        let mut all = Vec::with_capacity(chats.len() + deliberations.len());
        all.extend(chats.keys().cloned());
        all.extend(deliberations.keys().cloned());
        all
    }

    /// Get curator WebID
    pub fn curator_webid(&self) -> WebID {
        self.curator_webid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::{ChatMessage, GasBudgetConfig};
    use hkask_types::WebID;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn curator_id() -> WebID {
        WebID::from_persona(b"curator")
    }

    struct MockGasGovernance {
        can_proceed_result: bool,
        acquire_calls: AtomicUsize,
    }

    impl MockGasGovernance {
        fn new(can_proceed: bool) -> Self {
            Self {
                can_proceed_result: can_proceed,
                acquire_calls: AtomicUsize::new(0),
            }
        }
    }

    impl GasGovernancePort for MockGasGovernance {
        fn can_proceed(&self, _gas: u64) -> bool {
            self.can_proceed_result
        }
        fn acquire(&self, _gas: u64) {
            self.acquire_calls.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn session_manager_create_and_get_chat() {
        let mgr = SessionManager::new(curator_id());
        let chat = mgr.create_chat("s1").await;
        let retrieved = mgr.get_chat("s1").await;
        assert!(retrieved.is_some());
        assert!(Arc::ptr_eq(&chat, &retrieved.unwrap()));
    }

    #[tokio::test]
    async fn session_manager_delete_chat() {
        let mgr = SessionManager::new(curator_id());
        mgr.create_chat("s1").await;
        assert!(mgr.delete_chat("s1").await);
        assert!(mgr.get_chat("s1").await.is_none());
    }

    #[tokio::test]
    async fn session_manager_clone_shared_shares_state() {
        let mgr = SessionManager::new(curator_id());
        let mgr2 = mgr.clone_shared();
        mgr.create_chat("shared").await;
        assert!(mgr2.get_chat("shared").await.is_some());
    }

    #[tokio::test]
    async fn session_manager_gas_governance_wired_into_chat() {
        let mock = Arc::new(MockGasGovernance::new(false));
        let mgr = SessionManager::new(curator_id()).with_gas_governance(mock);
        let chat = mgr.create_chat("gov").await;
        // Governance blocks → message rejected
        chat.write()
            .await
            .add_message(ChatMessage::new(curator_id(), "blocked".into()));
        assert_eq!(chat.read().await.get_history().len(), 0);
    }
}
