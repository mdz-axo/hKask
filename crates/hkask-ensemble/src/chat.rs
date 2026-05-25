<<<<<<< HEAD
//! Multi-agent chat coordination
//!
//! Orchestrates conversation between Curator (replicant) and expert bots
//! via template-mediated A2A communication. No swarms, no consensus mechanisms.

use hkask_agents::SovereigntyChecker;
use hkask_cns::spans::SpanEmitter;
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

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
    /// Capabilities granted to this participant (R4: Capability Intersection)
    pub capabilities: Vec<String>,
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
    /// Template registry for capability lookup (R4: Capability Intersection)
    template_registry: Option<Arc<dyn hkask_templates::RegistryIndex + Send + Sync>>,
}

impl EnsembleChat {
    /// Create new ensemble chat with curator as owner
    pub fn new(curator_webid: WebID) -> Self {
        let span_emitter = SpanEmitter::new(curator_webid);
        let sovereignty_checker = SovereigntyChecker::new(curator_webid);

        let mut participants = HashMap::new();
        participants.insert(
            curator_webid,
            ChatParticipant {
                webid: curator_webid,
                role: ParticipantRole::Curator,
                pod_id: None,
                capabilities: vec![],
            },
        );

        Self {
            curator_webid,
            participants,
            messages: Vec::new(),
            span_emitter,
            sovereignty_checker,
            template_registry: None,
        }
    }

    /// Set template registry for capability intersection checks (R4)
    pub fn with_template_registry(
        mut self,
        registry: Arc<dyn hkask_templates::RegistryIndex + Send + Sync>,
    ) -> Self {
        self.template_registry = Some(registry);
        self
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

        self.participants.insert(participant.webid, participant);
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
        let participant = match self.participants.get(bot_webid) {
            Some(p) => p,
            None => {
                self.span_emitter.emit_tool(
                    "chat_dispatch.outcome",
                    json!({"outcome": "participant_not_found"}),
                );
                return Err(EnsembleError::ParticipantNotFound(bot_webid.to_string()));
            }
        };

        // R4: Capability Intersection — check if bot has required capabilities for template
        if let Some(ref registry) = self.template_registry
            && let Ok(entry) = registry.get(template_id)
        {
            let required_caps = &entry.required_capabilities;
            if !required_caps.is_empty() {
                let bot_caps = &participant.capabilities;
                let intersection: Vec<_> = required_caps
                    .iter()
                    .filter(|cap| bot_caps.contains(cap))
                    .collect();

                if intersection.is_empty() {
                    self.span_emitter.emit_tool(
                        "chat_dispatch.outcome",
                        json!({
                            "outcome": "capability_denied",
                            "bot": bot_webid.to_string(),
                            "template": template_id,
                            "required": required_caps,
                            "granted": bot_caps,
                        }),
                    );
                    return Err(EnsembleError::CapabilityDenied(format!(
                        "Bot {} lacks required capabilities {:?} for template {}",
                        bot_webid, required_caps, template_id
                    )));
                }
            }
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

    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
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
        let chat = Arc::new(RwLock::new(EnsembleChat::new(self.curator_webid)));

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
=======
//! Multi-agent chat with Curator moderation
//!
//! This module provides:
//! - **Chat Sessions**: Multi-agent conversations with turn-taking
//! - **Curator Moderation**: Curator replicant moderates agent interactions
//! - **A2A Messaging**: Integration with ACP runtime for agent-to-agent communication
//! - **Context Management**: Conversation history and state tracking
//! - **OCAP Enforcement**: Capability-gated participation and tool access

use chrono::{DateTime, Utc};
use hkask_agents::acp::AcpRuntime;
use hkask_agents::pod::{AgentType, PodID, PodManager};
use hkask_types::WebID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};
use tracing::info;

/// Chat session unique identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChatID(pub uuid::Uuid);

impl ChatID {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for ChatID {
>>>>>>> origin/main
    fn default() -> Self {
        Self::new(WebID::new())
    }
}

impl std::fmt::Display for ChatID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Chat error types
#[derive(Debug, Error)]
pub enum ChatError {
    #[error("Chat session not found: {0}")]
    SessionNotFound(String),

    #[error("Participant not found: {0:?}")]
    ParticipantNotFound(WebID),

    #[error("Participant not authorized: {0:?}")]
    ParticipantNotAuthorized(WebID),

    #[error("Chat session not started")]
    SessionNotStarted,

    #[error("Chat session already started")]
    SessionAlreadyStarted,

    #[error("Chat session ended")]
    SessionEnded,

    #[error("Turn management error: {0}")]
    TurnManagementError(String),

    #[error("Context limit exceeded: {0}")]
    ContextLimitExceeded(String),

    #[error("ACP messaging error: {0}")]
    AcpMessagingError(String),

    #[error("Pod management error: {0}")]
    PodManagementError(String),

    #[error("Curator moderation error: {0}")]
    CuratorModerationError(String),
}

/// Chat participant information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatParticipant {
    /// Participant WebID
    pub webid: WebID,
    /// Participant name
    pub name: String,
    /// Agent type
    pub agent_type: AgentType,
    /// Is moderator
    pub is_moderator: bool,
    /// Pod ID if participant is an agent pod
    pub pod_id: Option<PodID>,
    /// Joined timestamp
    pub joined_at: DateTime<Utc>,
    /// Turn order position
    pub turn_order: u8,
    /// Participation status
    pub status: ParticipantStatus,
}

/// Participant status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantStatus {
    Active,
    Muted,
    Left,
    Removed,
}

/// Turn state for chat session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnState {
    pub turn_number: u32,
    pub current_speaker: Option<WebID>,
    pub turn_order: Vec<WebID>,
    pub last_activity: DateTime<Utc>,
    pub timeout_seconds: Option<u64>,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub from: WebID,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub is_system: bool,
    pub correlation_id: Option<String>,
    pub turn_number: u32,
}

/// Chat session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub max_context_messages: usize,
    pub turn_timeout_seconds: Option<u64>,
    pub curator_moderation: bool,
    pub dynamic_participation: bool,
    pub a2a_messaging: bool,
    pub cns_emission: bool,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            max_context_messages: 100,
            turn_timeout_seconds: Some(300),
            curator_moderation: true,
            dynamic_participation: true,
            a2a_messaging: true,
            cns_emission: true,
        }
    }
}

/// Chat session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatState {
    Created,
    Active,
    Paused,
    Ended,
}

/// Multi-agent chat session
pub struct ChatSession {
    pub id: ChatID,
    pub name: String,
    pub config: ChatConfig,
    pub state: Arc<RwLock<ChatState>>,
    pub participants: Arc<RwLock<HashMap<WebID, ChatParticipant>>>,
    pub context: Arc<Mutex<Vec<ChatMessage>>>,
    pub turn_state: Arc<RwLock<TurnState>>,
    pub acp_runtime: Option<Arc<AcpRuntime>>,
    pub pod_manager: Option<Arc<PodManager>>,
    pub created_at: DateTime<Utc>,
    pub last_activity: Arc<RwLock<DateTime<Utc>>>,
}

impl ChatSession {
    pub fn new(name: &str, config: ChatConfig) -> Self {
        let now = Utc::now();
        Self {
            id: ChatID::new(),
            name: name.to_string(),
            config,
            state: Arc::new(RwLock::new(ChatState::Created)),
            participants: Arc::new(RwLock::new(HashMap::new())),
            context: Arc::new(Mutex::new(Vec::new())),
            turn_state: Arc::new(RwLock::new(TurnState {
                turn_number: 0,
                current_speaker: None,
                turn_order: Vec::new(),
                last_activity: now,
                timeout_seconds: None,
            })),
            acp_runtime: None,
            pod_manager: None,
            created_at: now,
            last_activity: Arc::new(RwLock::new(now)),
        }
    }

    pub fn with_acp(name: &str, config: ChatConfig, acp_runtime: Arc<AcpRuntime>) -> Self {
        let mut session = Self::new(name, config);
        session.acp_runtime = Some(acp_runtime);
        session
    }

    pub fn with_pods(name: &str, config: ChatConfig, pod_manager: Arc<PodManager>) -> Self {
        let mut session = Self::new(name, config);
        session.pod_manager = Some(pod_manager);
        session
    }

    pub fn with_full_integration(
        name: &str,
        config: ChatConfig,
        acp_runtime: Arc<AcpRuntime>,
        pod_manager: Arc<PodManager>,
    ) -> Self {
        let mut session = Self::new(name, config);
        session.acp_runtime = Some(acp_runtime);
        session.pod_manager = Some(pod_manager);
        session
    }

    pub async fn add_participant(
        &self,
        webid: WebID,
        name: &str,
        agent_type: AgentType,
        is_moderator: bool,
        pod_id: Option<PodID>,
    ) -> ChatResult<()> {
        if *self.state.read().await == ChatState::Ended {
            return Err(ChatError::SessionEnded);
        }

        let mut participants = self.participants.write().await;

        if participants.contains_key(&webid) {
            return Ok(());
        }

        let participant = ChatParticipant {
            webid,
            name: name.to_string(),
            agent_type,
            is_moderator,
            pod_id,
            joined_at: Utc::now(),
            turn_order: participants.len() as u8,
            status: ParticipantStatus::Active,
        };

        participants.insert(webid, participant);

        let mut turn_state = self.turn_state.write().await;
        turn_state.turn_order.push(webid);

        self.add_system_message(format!("{} joined the chat", name))
            .await;

        info!(
            target: "hkask.ensemble.chat",
            chat_id = %self.id,
            participant = %name,
            webid = %webid,
            "Participant added"
        );

        Ok(())
    }

    pub async fn start(&self) -> ChatResult<()> {
        let current_state = *self.state.read().await;
        if current_state != ChatState::Created {
            return Err(ChatError::SessionAlreadyStarted);
        }

        let participants = self.participants.read().await;
        if participants.is_empty() {
            return Err(ChatError::TurnManagementError(
                "Cannot start without participants".to_string(),
            ));
        }

        let mut turn_state = self.turn_state.write().await;
        turn_state.current_speaker = turn_state.turn_order.first().copied();
        turn_state.timeout_seconds = self.config.turn_timeout_seconds;

        drop(participants);
        drop(turn_state);

        *self.state.write().await = ChatState::Active;
        self.add_system_message("Chat session started".to_string())
            .await;

        info!(target: "hkask.ensemble.chat", chat_id = %self.id, "Session started");

        Ok(())
    }

    pub async fn send_message(&self, from: WebID, content: String) -> ChatResult<ChatMessage> {
        let current_state = *self.state.read().await;
        if current_state != ChatState::Active {
            return Err(ChatError::SessionNotStarted);
        }

        let participants = self.participants.read().await;
        let participant = participants
            .get(&from)
            .ok_or(ChatError::ParticipantNotFound(from))?;

        if participant.status != ParticipantStatus::Active {
            return Err(ChatError::ParticipantNotAuthorized(from));
        }

        if participant.status == ParticipantStatus::Muted {
            return Err(ChatError::TurnManagementError(
                "Participant is muted".to_string(),
            ));
        }

        drop(participants);

        let turn_state = self.turn_state.read().await;
        let message = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from,
            content,
            timestamp: Utc::now(),
            is_system: false,
            correlation_id: None,
            turn_number: turn_state.turn_number,
        };
        drop(turn_state);

        let mut context = self.context.lock().await;
        context.push(message.clone());

        if context.len() > self.config.max_context_messages {
            let drain_count = context.len() - self.config.max_context_messages;
            context.drain(0..drain_count);
        }

        *self.last_activity.write().await = Utc::now();
        let mut turn_state = self.turn_state.write().await;
        turn_state.last_activity = *self.last_activity.read().await;
        self.advance_turn(from).await?;

        info!(
            target: "hkask.ensemble.chat",
            chat_id = %self.id,
            from = %from,
            turn = %message.turn_number,
            "Message sent"
        );

        Ok(message)
    }

    async fn advance_turn(&self, current_speaker: WebID) -> ChatResult<()> {
        let mut turn_state = self.turn_state.write().await;

        let current_index = turn_state
            .turn_order
            .iter()
            .position(|w| *w == current_speaker)
            .unwrap_or(0);

        let next_index = (current_index + 1) % turn_state.turn_order.len();
        turn_state.current_speaker = Some(turn_state.turn_order[next_index]);
        turn_state.turn_number += 1;

        Ok(())
    }

    pub async fn get_context(&self, limit: Option<usize>) -> Vec<ChatMessage> {
        let context = self.context.lock().await;

        match limit {
            Some(n) => context.iter().rev().take(n).cloned().collect(),
            None => context.clone(),
        }
    }

    pub async fn get_turn_state(&self) -> TurnState {
        self.turn_state.read().await.clone()
    }

    pub async fn get_participants(&self) -> Vec<ChatParticipant> {
        let participants = self.participants.read().await;
        participants
            .values()
            .filter(|p| p.status == ParticipantStatus::Active)
            .cloned()
            .collect()
    }

    pub async fn mute_participant(&self, moderator: WebID, target: WebID) -> ChatResult<()> {
        let mut participants = self.participants.write().await;

        let mod_participant = participants
            .get(&moderator)
            .ok_or(ChatError::ParticipantNotFound(moderator))?;

        if !mod_participant.is_moderator {
            return Err(ChatError::ParticipantNotAuthorized(moderator));
        }

        let target_participant = participants
            .get_mut(&target)
            .ok_or(ChatError::ParticipantNotFound(target))?;

        target_participant.status = ParticipantStatus::Muted;

        self.add_system_message(format!("{} was muted", target_participant.name))
            .await;

        Ok(())
    }

    pub async fn unmute_participant(&self, moderator: WebID, target: WebID) -> ChatResult<()> {
        let mut participants = self.participants.write().await;

        let mod_participant = participants
            .get(&moderator)
            .ok_or(ChatError::ParticipantNotFound(moderator))?;

        if !mod_participant.is_moderator {
            return Err(ChatError::ParticipantNotAuthorized(moderator));
        }

        let target_participant = participants
            .get_mut(&target)
            .ok_or(ChatError::ParticipantNotFound(target))?;

        target_participant.status = ParticipantStatus::Active;

        self.add_system_message(format!("{} was unmuted", target_participant.name))
            .await;

        Ok(())
    }

    async fn add_system_message(&self, content: String) {
        let mut context = self.context.lock().await;
        let message = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from: WebID::new(),
            content,
            timestamp: Utc::now(),
            is_system: true,
            correlation_id: None,
            turn_number: 0,
        };
        context.push(message);

        if context.len() > self.config.max_context_messages {
            let drain_count = context.len() - self.config.max_context_messages;
            context.drain(0..drain_count);
        }
    }

    pub async fn end(&self) -> ChatResult<()> {
        *self.state.write().await = ChatState::Ended;
        self.add_system_message("Chat session ended".to_string())
            .await;

        let mut participants = self.participants.write().await;
        for participant in participants.values_mut() {
            participant.status = ParticipantStatus::Left;
        }

        info!(target: "hkask.ensemble.chat", chat_id = %self.id, "Session ended");

        Ok(())
    }
}

pub type ChatResult<T> = Result<T, ChatError>;

impl Default for ChatSession {
    fn default() -> Self {
        Self::new("default-chat", ChatConfig::default())
    }
}

/// Chat manager for multiple sessions
pub struct ChatManager {
    sessions: Arc<RwLock<HashMap<ChatID, Arc<ChatSession>>>>,
    acp_runtime: Option<Arc<AcpRuntime>>,
    pod_manager: Option<Arc<PodManager>>,
}

impl ChatManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            acp_runtime: None,
            pod_manager: None,
        }
    }

    pub fn with_acp(acp_runtime: Arc<AcpRuntime>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            acp_runtime: Some(acp_runtime),
            pod_manager: None,
        }
    }

    pub fn with_full_integration(
        acp_runtime: Arc<AcpRuntime>,
        pod_manager: Arc<PodManager>,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            acp_runtime: Some(acp_runtime),
            pod_manager: Some(pod_manager),
        }
    }

    pub async fn create_session(
        &self,
        name: &str,
        config: ChatConfig,
    ) -> ChatResult<Arc<ChatSession>> {
        let session = match (&self.acp_runtime, &self.pod_manager) {
            (Some(acp), Some(pods)) => {
                ChatSession::with_full_integration(name, config, acp.clone(), pods.clone())
            }
            (Some(acp), None) => ChatSession::with_acp(name, config, acp.clone()),
            (None, Some(pods)) => ChatSession::with_pods(name, config, pods.clone()),
            (None, None) => ChatSession::new(name, config),
        };

        let session = Arc::new(session);
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id, session.clone());

        info!(target: "hkask.ensemble.chat", chat_id = %session.id, name = %name, "Session created");

        Ok(session)
    }

    pub async fn get_session(&self, chat_id: &ChatID) -> ChatResult<Arc<ChatSession>> {
        let sessions = self.sessions.read().await;
        sessions
            .get(chat_id)
            .cloned()
            .ok_or_else(|| ChatError::SessionNotFound(chat_id.to_string()))
    }

    pub async fn list_sessions(&self) -> Vec<ChatID> {
        let sessions = self.sessions.read().await;
        sessions.keys().copied().collect()
    }

    pub async fn remove_session(&self, chat_id: &ChatID) -> ChatResult<()> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get(chat_id)
            .ok_or_else(|| ChatError::SessionNotFound(chat_id.to_string()))?;

        session.end().await?;
        sessions.remove(chat_id);

        Ok(())
    }

    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }
}

impl Default for ChatManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chat_session_creation() {
        let session = ChatSession::new("test-chat", ChatConfig::default());
        assert_eq!(session.name, "test-chat");
        assert_eq!(*session.state.read().await, ChatState::Created);
    }

    #[tokio::test]
    async fn test_add_participants() {
        let session = ChatSession::new("test-chat", ChatConfig::default());

        let curator = WebID::new();
        let bot = WebID::new();

        session
            .add_participant(curator, "curator", AgentType::Replicant, true, None)
            .await
            .unwrap();

        session
            .add_participant(bot, "memory-bot", AgentType::Bot, false, None)
            .await
            .unwrap();

        let participants = session.get_participants().await;
        assert_eq!(participants.len(), 2);
    }

    #[tokio::test]
    async fn test_chat_lifecycle() {
        let session = ChatSession::new("test-chat", ChatConfig::default());

        let curator = WebID::new();
        session
            .add_participant(curator, "curator", AgentType::Replicant, true, None)
            .await
            .unwrap();

        session.start().await.unwrap();
        assert_eq!(*session.state.read().await, ChatState::Active);

        session
            .send_message(curator, "Hello!".to_string())
            .await
            .unwrap();

        let context = session.get_context(None).await;
        assert!(!context.is_empty());

        session.end().await.unwrap();
        assert_eq!(*session.state.read().await, ChatState::Ended);
    }

    #[tokio::test]
    async fn test_moderator_actions() {
        let session = ChatSession::new("test-chat", ChatConfig::default());

        let curator = WebID::new();
        let bot = WebID::new();

        session
            .add_participant(curator, "curator", AgentType::Replicant, true, None)
            .await
            .unwrap();

        session
            .add_participant(bot, "memory-bot", AgentType::Bot, false, None)
            .await
            .unwrap();

        session.start().await.unwrap();

        session.mute_participant(curator, bot).await.unwrap();

        let result = session.send_message(bot, "Hello!".to_string()).await;
        assert!(result.is_err());

        session.unmute_participant(curator, bot).await.unwrap();

        let result = session.send_message(bot, "Hello!".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_turn_management() {
        let session = ChatSession::new("test-chat", ChatConfig::default());

        let curator = WebID::new();
        let bot1 = WebID::new();
        let bot2 = WebID::new();

        session
            .add_participant(curator, "curator", AgentType::Replicant, true, None)
            .await
            .unwrap();

        session
            .add_participant(bot1, "bot1", AgentType::Bot, false, None)
            .await
            .unwrap();

        session
            .add_participant(bot2, "bot2", AgentType::Bot, false, None)
            .await
            .unwrap();

        session.start().await.unwrap();

        let turn_state = session.get_turn_state().await;
        assert_eq!(turn_state.turn_number, 0);

        session
            .send_message(curator, "First".to_string())
            .await
            .unwrap();

        let turn_state = session.get_turn_state().await;
        assert_eq!(turn_state.turn_number, 1);
    }

    #[tokio::test]
    async fn test_context_limit() {
        let config = ChatConfig {
            max_context_messages: 5,
            ..ChatConfig::default()
        };

        let session = ChatSession::new("test-chat", config);

        let curator = WebID::new();
        session
            .add_participant(curator, "curator", AgentType::Replicant, true, None)
            .await
            .unwrap();

        session.start().await.unwrap();

        for i in 0..10 {
            session
                .send_message(curator, format!("Message {}", i))
                .await
                .unwrap();
        }

        let context = session.get_context(None).await;
        assert_eq!(context.len(), 5);
    }
}
