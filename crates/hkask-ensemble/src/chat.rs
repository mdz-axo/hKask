//! Multi-agent chat coordination
//!
//! Orchestrates conversation between Curator (replicant) and R7 bots
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

use crate::improv::{ImprovError, ImprovMode, ImprovSessionConfig, ImprovTurn, improv_turn};
use crate::ports::InferenceClient;

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

/// Chat participant (Curator or R7 bot)
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
    /// Dynamic role from YAML config (e.g., "participant", "orchestrator")
    Custom(String),
}

/// Multi-agent chat session
pub struct EnsembleChat {
    curator_webid: WebID,
    participants: HashMap<WebID, ChatParticipant>,
    messages: Vec<ChatMessage>,
    span_emitter: SpanEmitter,
    sovereignty_checker: SovereigntyChecker,
    template_registry: Option<Arc<dyn hkask_templates::RegistryIndex + Send + Sync>>,
    improv_config: ImprovSessionConfig,
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
            improv_config: ImprovSessionConfig::default(),
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
        self.span_emitter.emit_with_phase(
        Span::agent_pod("chat_participant_registered"),
        Phase::Observe,
            json!({
                "webid": participant.webid.to_string(),
                "role": format!("{:?}", participant.role),
            }),
        );

        self.participants.insert(participant.webid, participant);
    }

    /// Add a message to the chat
    pub fn add_message(&mut self, message: ChatMessage) {
        self.span_emitter.emit_with_phase(
        Span::tool("chat_message"),
        Phase::Observe,
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
            self.span_emitter.emit_with_phase(
        Span::tool("chat_dispatch.outcome"),
        Phase::Observe,
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
                self.span_emitter.emit_with_phase(
        Span::tool("chat_dispatch.outcome"),
        Phase::Observe,
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
                    self.span_emitter.emit_with_phase(
        Span::tool("chat_dispatch.outcome"),
        Phase::Observe,
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

        self.span_emitter.emit_with_phase(
        Span::tool("chat_dispatch"),
        Phase::Observe,
            json!({
                "bot": bot_webid.to_string(),
                "template": template_id,
            }),
        );

        // Simulate template-mediated dispatch (actual dispatch via hkask_templates)
        let response = format!("Bot {} processed via template {}", bot_webid, template_id);

        self.span_emitter.emit_with_phase(
        Span::tool("chat_dispatch.outcome"),
        Phase::Observe,
            json!({
                "outcome": "success",
                "response": response
            }),
        );

        Ok(response)
    }

    /// Aggregate responses from multiple bots (no consensus, just collection)
    pub fn aggregate_responses(&self, bot_responses: HashMap<WebID, String>) -> String {
        self.span_emitter.emit_with_phase(
        Span::tool("chat_aggregate"),
        Phase::Observe,
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
        self.span_emitter.emit_with_phase(
        Span::agent_pod("chat_cleared"),
        Phase::Observe, json!({}));
        info!("Chat history cleared");
    }

    /// Grant explicit consent for template invocations
    pub fn grant_consent(&mut self) {
        self.sovereignty_checker.grant_consent();
        self.span_emitter
            .emit_with_phase(
        Span::agent_pod("chat_consent_granted"),
        Phase::Observe, json!({}));
    }

    /// Get improv session config
    pub fn improv_config(&self) -> &ImprovSessionConfig {
        &self.improv_config
    }

    /// Get mutable improv session config
    pub fn improv_config_mut(&mut self) -> &mut ImprovSessionConfig {
        &mut self.improv_config
    }

    /// Set participation threshold
    pub fn set_participation_threshold(&mut self, threshold: f64) {
        self.improv_config.set_threshold(threshold);
        self.span_emitter.emit_with_phase(
        Span::tool("improv_threshold_set"),
        Phase::Observe,
            json!({"threshold": self.improv_config.participation_threshold}),
        );
    }

    /// Set improv mode
    pub fn set_improv_mode(&mut self, mode: ImprovMode) {
        let mode_str = mode.as_str().to_string();
        self.improv_config.set_mode(mode);
        self.span_emitter
            .emit_with_phase(
        Span::tool("improv_mode_set"),
        Phase::Observe, json!({"mode": mode_str}));
    }

    /// Execute an improvisation turn using this session's config and participants
    pub async fn improv_turn<C: InferenceClient>(
        &self,
        inference_client: &Arc<C>,
        user_message: &str,
    ) -> Result<ImprovTurn, ImprovError<C::Error>> {
        let participants: Vec<(WebID, String, String)> = self
            .participants
            .values()
            .filter(|p| !matches!(p.role, ParticipantRole::Curator))
            .map(|p| {
                let name = format!("{:?}", p.role);
                let desc = format!("Agent with role {:?}", p.role);
                (p.webid, name, desc)
            })
            .collect();

        let chat_history: Vec<(WebID, String)> = self
            .messages
            .iter()
            .map(|msg| (msg.from, msg.content.clone()))
            .collect();

        improv_turn(
            &self.improv_config,
            inference_client,
            user_message,
            &participants,
            &chat_history,
        )
        .await
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
    fn default() -> Self {
        Self::new(WebID::new())
    }
}
