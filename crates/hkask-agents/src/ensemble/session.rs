//! Unified session manager for both chat and deliberation sessions.

use hkask_types::WebID;
use hkask_types::ports::ToolInfo;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ensemble::chat::EnsembleChat;
use crate::ensemble::deliberation::DeliberationSession;
use crate::ensemble::ports::GasGovernancePort;

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
    available_tools: Option<Vec<ToolInfo>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(curator_webid: WebID) -> Self {
        Self {
            chats: Arc::new(RwLock::new(HashMap::new())),
            deliberations: Arc::new(RwLock::new(HashMap::new())),
            curator_webid,
            gas_governance: None,
            available_tools: None,
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

    /// Set available tools for intersection-based tool scoping (R4).
    ///
    /// When set, every `EnsembleChat` created by this manager will be
    /// wired with the available tools so that `intersection_tools()`
    /// can filter tools to only those visible across all participants.
    pub fn with_available_tools(mut self, tools: Vec<ToolInfo>) -> Self {
        self.available_tools = Some(tools);
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
        if let Some(ref tools) = self.available_tools {
            chat = chat.with_available_tools(tools.clone());
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
