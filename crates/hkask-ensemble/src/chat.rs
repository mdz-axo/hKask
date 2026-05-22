//! Multi-agent chat coordination
//!
//! Orchestrates conversation between Curator (replicant) and expert bots
//! via template-mediated A2A communication. No swarms, no consensus mechanisms.

use hkask_agents::SovereigntyChecker;
use hkask_cns::spans::SpanEmitter;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Chat message in multi-agent conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub from: WebID,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub template_id: Option<String>,
}

impl ChatMessage {
    pub fn new(from: WebID, content: String) -> Self {
        Self {
            from,
            content,
            timestamp: chrono::Utc::now(),
            template_id: None,
        }
    }

    pub fn with_template(mut self, template_id: String) -> Self {
        self.template_id = Some(template_id);
        self
    }
}

/// Chat participant (Curator or expert bot)
#[derive(Debug, Clone)]
pub struct ChatParticipant {
    pub webid: WebID,
    pub role: ParticipantRole,
    pub pod_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantRole {
    Curator,
    MemoryBot,
    SpandrelBot,
    OkapiBot,
    ScholarBot,
    Custom(String),
}

/// Multi-agent chat session
pub struct EnsembleChat {
    curator_webid: WebID,
    participants: HashMap<WebID, ChatParticipant>,
    messages: Vec<ChatMessage>,
    span_emitter: SpanEmitter,
    sovereignty_checker: SovereigntyChecker,
}

impl EnsembleChat {
    /// Create new ensemble chat with curator as owner
    pub fn new(curator_webid: WebID) -> Self {
        let span_emitter = SpanEmitter::new(curator_webid.clone());
        let sovereignty_checker = SovereigntyChecker::new(curator_webid.clone());

        let mut participants = HashMap::new();
        participants.insert(
            curator_webid.clone(),
            ChatParticipant {
                webid: curator_webid.clone(),
                role: ParticipantRole::Curator,
                pod_id: None,
            },
        );

        Self {
            curator_webid,
            participants,
            messages: Vec::new(),
            span_emitter,
            sovereignty_checker,
        }
    }

    /// Register a bot participant in the chat
    pub fn register_participant(&mut self, participant: ChatParticipant) {
        self.span_emitter.emit_agent_pod(
            "chat_participant_registered",
            json!({
                "webid": participant.webid.to_string(),
                "role": format!("{:?}", participant.role),
            }),
        );

        self.participants
            .insert(participant.webid.clone(), participant.clone());
        debug!("Registered chat participant: {:?}", participant.role);
    }

    /// Add a message to the chat
    pub fn add_message(&mut self, message: ChatMessage) {
        self.span_emitter.emit_tool(
            "chat_message",
            json!({
                "from": message.from.to_string(),
                "content_length": message.content.len(),
            }),
        );

        self.messages.push(message);
    }

    /// Get chat history
    pub fn get_history(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Get participants
    pub fn get_participants(&self) -> &HashMap<WebID, ChatParticipant> {
        &self.participants
    }

    /// Dispatch a task to a specific bot via template
    pub async fn dispatch_to_bot(
        &mut self,
        bot_webid: &WebID,
        template_id: &str,
        _input: Value,
    ) -> Result<String, EnsembleError> {
        // Check sovereignty
        if !self.sovereignty_checker.can_access(
            &hkask_types::DataCategory::TemplateInvocations,
            &self.curator_webid,
        ) {
            self.span_emitter.emit_tool(
                "chat_dispatch.outcome",
                json!({"outcome": "sovereignty_denied"}),
            );
            return Err(EnsembleError::SovereigntyDenied(
                "Template dispatch requires consent".to_string(),
            ));
        }

        // Check participant exists
        if !self.participants.contains_key(bot_webid) {
            self.span_emitter.emit_tool(
                "chat_dispatch.outcome",
                json!({"outcome": "participant_not_found"}),
            );
            return Err(EnsembleError::ParticipantNotFound(bot_webid.to_string()));
        }

        self.span_emitter.emit_tool(
            "chat_dispatch",
            json!({
                "bot": bot_webid.to_string(),
                "template": template_id,
            }),
        );

        // Simulate template-mediated dispatch (actual dispatch via hkask_templates)
        let response = format!("Bot {} processed via template {}", bot_webid, template_id);

        self.span_emitter.emit_tool(
            "chat_dispatch.outcome",
            json!({
                "outcome": "success",
                "response": response
            }),
        );

        Ok(response)
    }

    /// Aggregate responses from multiple bots (no consensus, just collection)
    pub fn aggregate_responses(&self, bot_responses: HashMap<WebID, String>) -> String {
        self.span_emitter.emit_tool(
            "chat_aggregate",
            json!({
                "response_count": bot_responses.len(),
            }),
        );

        let mut aggregated = String::new();
        for (webid, response) in bot_responses {
            aggregated.push_str(&format!("[{}]: {}\n", webid, response));
        }

        aggregated
    }

    /// Emit CNS span for chat activity
    pub fn emit_chat_span(&self, event_type: &str, data: Value) {
        self.span_emitter.emit_agent_pod(event_type, data);
    }

    /// Get curator WebID
    pub fn curator(&self) -> &WebID {
        &self.curator_webid
    }

    /// Clear chat history
    pub fn clear(&mut self) {
        self.messages.clear();
        self.span_emitter.emit_agent_pod("chat_cleared", json!({}));
        info!("Chat history cleared");
    }

    /// Grant explicit consent for template invocations
    pub fn grant_consent(&mut self) {
        self.sovereignty_checker.grant_consent();
        self.span_emitter
            .emit_agent_pod("chat_consent_granted", json!({}));
    }
}

/// Ensemble chat error types
#[derive(Debug, thiserror::Error)]
pub enum EnsembleError {
    #[error("Sovereignty denied: {0}")]
    SovereigntyDenied(String),

    #[error("Participant not found: {0}")]
    ParticipantNotFound(String),

    #[error("Template dispatch failed: {0}")]
    TemplateDispatchFailed(String),

    #[error("Chat session error: {0}")]
    ChatError(String),
}

/// Ensemble chat manager (handles multiple chat sessions)
pub struct EnsembleChatManager {
    chats: Arc<RwLock<HashMap<String, Arc<RwLock<EnsembleChat>>>>>,
    curator_webid: WebID,
}

impl EnsembleChatManager {
    /// Create new chat manager
    pub fn new(curator_webid: WebID) -> Self {
        Self {
            chats: Arc::new(RwLock::new(HashMap::new())),
            curator_webid,
        }
    }

    /// Create a new chat session
    pub async fn create_chat(&self, session_id: &str) -> Arc<RwLock<EnsembleChat>> {
        let chat = Arc::new(RwLock::new(EnsembleChat::new(self.curator_webid.clone())));

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

    /// List active chat sessions
    pub async fn list_sessions(&self) -> Vec<String> {
        let chats = self.chats.read().await;
        chats.keys().cloned().collect()
    }
}

impl Default for EnsembleChatManager {
    fn default() -> Self {
        Self::new(WebID::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensemble_chat_new() {
        let curator = WebID::new();
        let chat = EnsembleChat::new(curator.clone());

        assert_eq!(chat.curator(), &curator);
        assert_eq!(chat.get_participants().len(), 1);
        assert_eq!(chat.get_history().len(), 0);
    }

    #[test]
    fn test_register_participant() {
        let curator = WebID::new();
        let mut chat = EnsembleChat::new(curator.clone());

        let bot_participant = ChatParticipant {
            webid: WebID::new(),
            role: ParticipantRole::MemoryBot,
            pod_id: None,
        };

        chat.register_participant(bot_participant);
        assert_eq!(chat.get_participants().len(), 2);
    }

    #[test]
    fn test_add_message() {
        let curator = WebID::new();
        let mut chat = EnsembleChat::new(curator.clone());

        let message = ChatMessage::new(curator.clone(), "Hello, bots!".to_string());
        chat.add_message(message);

        assert_eq!(chat.get_history().len(), 1);
        assert_eq!(chat.get_history()[0].content, "Hello, bots!");
    }

    #[test]
    fn test_message_with_template() {
        let curator = WebID::new();
        let message = ChatMessage::new(curator.clone(), "Process this".to_string())
            .with_template("test_template".to_string());

        assert_eq!(message.template_id, Some("test_template".to_string()));
    }

    #[test]
    fn test_aggregate_responses() {
        let curator = WebID::new();
        let chat = EnsembleChat::new(curator.clone());

        let responses = HashMap::from([
            (WebID::new(), "Response from bot 1".to_string()),
            (WebID::new(), "Response from bot 2".to_string()),
        ]);

        let aggregated = chat.aggregate_responses(responses);
        assert!(aggregated.contains("Response from bot 1"));
        assert!(aggregated.contains("Response from bot 2"));
    }

    #[test]
    fn test_clear_chat() {
        let curator = WebID::new();
        let mut chat = EnsembleChat::new(curator.clone());

        chat.add_message(ChatMessage::new(curator.clone(), "Message 1".to_string()));
        chat.add_message(ChatMessage::new(curator.clone(), "Message 2".to_string()));

        assert_eq!(chat.get_history().len(), 2);

        chat.clear();
        assert_eq!(chat.get_history().len(), 0);
    }

    #[tokio::test]
    async fn test_chat_manager_create() {
        let curator = WebID::new();
        let manager = EnsembleChatManager::new(curator.clone());

        let chat = manager.create_chat("session1").await;
        let chat_read = chat.read().await;
        assert_eq!(chat_read.curator(), &curator);
        drop(chat_read);

        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 1);
        assert!(sessions.contains(&"session1".to_string()));
    }

    #[tokio::test]
    async fn test_chat_manager_delete() {
        let curator = WebID::new();
        let manager = EnsembleChatManager::new(curator.clone());

        manager.create_chat("session1").await;
        assert_eq!(manager.list_sessions().await.len(), 1);

        let deleted = manager.delete_chat("session1").await;
        assert!(deleted);

        assert_eq!(manager.list_sessions().await.len(), 0);
    }

    #[tokio::test]
    async fn test_dispatch_to_bot_success() {
        let curator = WebID::new();
        let mut chat = EnsembleChat::new(curator.clone());

        // Grant consent for template invocations
        chat.grant_consent();

        let bot_webid = WebID::new();
        chat.register_participant(ChatParticipant {
            webid: bot_webid.clone(),
            role: ParticipantRole::MemoryBot,
            pod_id: None,
        });

        let result = chat
            .dispatch_to_bot(&bot_webid, "test_template", json!({}))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dispatch_to_bot_not_found() {
        let curator = WebID::new();
        let mut chat = EnsembleChat::new(curator.clone());

        // Grant consent for template invocations
        chat.grant_consent();

        let unknown_bot = WebID::new();
        let result = chat
            .dispatch_to_bot(&unknown_bot, "test_template", json!({}))
            .await;

        assert!(matches!(result, Err(EnsembleError::ParticipantNotFound(_))));
    }
}
