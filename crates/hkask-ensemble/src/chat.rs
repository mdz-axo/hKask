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
    fn default() -> Self {
        Self::new()
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
