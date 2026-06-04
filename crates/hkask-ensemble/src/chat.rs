//! Multi-agent chat coordination
//!
//! Orchestrates conversation between Curator (replicant) and R7 bots
//! via template-mediated A2A communication. No swarms, no consensus mechanisms.

use hkask_types::NuEventSink;
use hkask_types::WebID;
use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
use hkask_types::ports::RegistryIndex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::improv::{ImprovError, ImprovMode, ImprovSessionConfig, ImprovTurn, improv_turn};
use crate::ports::InferenceClient;

/// Degradation level for gas budget enforcement
///
/// Maps to the degradation rules in standing-ensemble-session.yaml:
/// - 80%: reduce_memory_to_batch_only
/// - 90%: suspend_standing_session_reports
/// - 95%: curator_escalates_to_administrator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradationLevel {
    Normal,
    /// At 80%: reduce memory to batch-only writes
    BatchOnlyMemory,
    /// At 90%: suspend standing session reports
    SuspendReports,
    /// At 95%: curator escalates to administrator
    Escalate,
}

/// Gas budget configuration from standing-ensemble-session.yaml
#[derive(Debug, Clone)]
pub struct GasBudgetConfig {
    pub session_cap: u64,
    pub per_message_cost: u64,
    pub alert_threshold: f64,
    pub hard_limit: bool,
    pub per_bot_allocation: u64,
    pub curator_allocation: u64,
}

impl Default for GasBudgetConfig {
    fn default() -> Self {
        Self {
            session_cap: 150000,
            per_message_cost: 100,
            alert_threshold: 0.7,
            hard_limit: true,
            per_bot_allocation: 15000,
            curator_allocation: 25000,
        }
    }
}

impl GasBudgetConfig {
    /// Parse from the `gas` section of standing-ensemble-session.yaml
    pub fn from_yaml_gas(gas: &serde_json::Value) -> Self {
        Self {
            session_cap: gas
                .get("session_cap")
                .and_then(|v| v.as_u64())
                .unwrap_or(150000),
            per_message_cost: gas
                .get("per_message_cost")
                .and_then(|v| v.as_u64())
                .unwrap_or(100),
            alert_threshold: gas
                .get("alert_threshold")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7),
            hard_limit: gas
                .get("hard_limit")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            per_bot_allocation: gas
                .get("per_bot_allocation")
                .and_then(|v| v.as_u64())
                .unwrap_or(15000),
            curator_allocation: gas
                .get("curator_allocation")
                .and_then(|v| v.as_u64())
                .unwrap_or(25000),
        }
    }

    /// Determine degradation level based on gas usage percentage
    pub fn degradation_level(&self, gas_used: u64) -> DegradationLevel {
        let pct = gas_used as f64 / self.session_cap as f64;
        if pct >= 0.95 {
            DegradationLevel::Escalate
        } else if pct >= 0.9 {
            DegradationLevel::SuspendReports
        } else if pct >= 0.8 {
            DegradationLevel::BatchOnlyMemory
        } else {
            DegradationLevel::Normal
        }
    }
}

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
    template_registry: Option<Arc<dyn RegistryIndex + Send + Sync>>,
    improv_config: ImprovSessionConfig,
    event_sink: Option<Arc<dyn NuEventSink>>,
    gas_budget: Option<GasBudgetConfig>,
    gas_used: u64,
}

impl EnsembleChat {
    /// Create new ensemble chat with curator as owner
    pub fn new(curator_webid: WebID) -> Self {
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
            template_registry: None,
            improv_config: ImprovSessionConfig::default(),
            event_sink: None,
            gas_budget: None,
            gas_used: 0,
        }
    }

    /// Set CNS event sink for span emission
    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Set template registry for capability intersection checks (R4)
    pub fn with_template_registry(
        mut self,
        registry: Arc<dyn RegistryIndex + Send + Sync>,
    ) -> Self {
        self.template_registry = Some(registry);
        self
    }

    /// Set gas budget configuration
    pub fn with_gas_budget(mut self, config: GasBudgetConfig) -> Self {
        self.gas_budget = Some(config);
        self
    }

    /// Check whether a gas-consuming operation of the given cost may proceed.
    ///
    /// Returns `(can_proceed, degradation_level)`. When `hard_limit` is enabled
    /// and the additional cost would exceed the session cap, `can_proceed` is `false`.
    pub fn can_proceed_with_gas(&self, additional_cost: u64) -> (bool, DegradationLevel) {
        match &self.gas_budget {
            Some(budget) => {
                let new_total = self.gas_used + additional_cost;
                let level = budget.degradation_level(new_total);
                let can_proceed = !budget.hard_limit || new_total <= budget.session_cap;
                (can_proceed, level)
            }
            None => (true, DegradationLevel::Normal),
        }
    }

    /// Record gas consumption after an operation completes.
    ///
    /// Emits a CNS span when the degradation level changes from Normal.
    pub fn consume_gas(&mut self, cost: u64) {
        self.gas_used += cost;
        let level = self
            .gas_budget
            .as_ref()
            .map(|b| b.degradation_level(self.gas_used))
            .unwrap_or(DegradationLevel::Normal);

        if level != DegradationLevel::Normal {
            if let Some(ref budget) = self.gas_budget {
                tracing::warn!(
                    target: "cns.gas",
                    gas_used = self.gas_used,
                    session_cap = budget.session_cap,
                    level = ?level,
                    "Gas budget degradation"
                );
            }
            if let Some(ref sink) = self.event_sink {
                let span = Span::new(SpanNamespace::new("cns.gas"), "ensemble.degradation");
                let event = NuEvent::new(
                    self.curator_webid,
                    span,
                    Phase::Compute,
                    serde_json::json!({
                        "gas_used": self.gas_used,
                        "session_cap": self.gas_budget.as_ref().map(|b| b.session_cap).unwrap_or(0),
                        "degradation_level": format!("{:?}", level),
                    }),
                    0,
                );
                if let Err(e) = sink.persist(&event) {
                    tracing::warn!(target: "cns.gas", error = %e, "Failed to persist gas degradation NuEvent");
                }
            }
        }
    }

    /// Get current gas usage
    pub fn gas_used(&self) -> u64 {
        self.gas_used
    }

    /// Get gas budget config if set
    pub fn gas_budget(&self) -> Option<&GasBudgetConfig> {
        self.gas_budget.as_ref()
    }

    /// Register a bot participant in the chat
    pub fn register_participant(&mut self, participant: ChatParticipant) {
        self.participants.insert(participant.webid, participant);
    }

    /// Add a message to the chat.
    ///
    /// When a gas budget is configured with `hard_limit`, messages that would
    /// exceed the session cap are silently rejected and a CNS span is emitted.
    /// When no gas budget is set, all messages are accepted (backward compatible).
    pub fn add_message(&mut self, message: ChatMessage) {
        if let Some(ref budget) = self.gas_budget {
            let cost = budget.per_message_cost;
            let (can_proceed, level) = self.can_proceed_with_gas(cost);
            if !can_proceed {
                tracing::warn!(
                    target: "cns.gas",
                    gas_used = self.gas_used,
                    session_cap = budget.session_cap,
                    "Message rejected — gas budget hard limit reached"
                );
                if let Some(ref sink) = self.event_sink {
                    let span =
                        Span::new(SpanNamespace::new("cns.gas"), "ensemble.message_rejected");
                    let event = NuEvent::new(
                        message.from,
                        span,
                        Phase::Compute,
                        serde_json::json!({
                            "gas_used": self.gas_used,
                            "session_cap": budget.session_cap,
                            "message_rejected": true,
                        }),
                        0,
                    );
                    if let Err(e) = sink.persist(&event) {
                        tracing::warn!(target: "cns.gas", error = %e, "Failed to persist message_rejected NuEvent");
                    }
                }
                return;
            }
            self.gas_used += cost;
            // Emit degradation span if threshold crossed
            if level != DegradationLevel::Normal
                && let Some(ref sink) = self.event_sink
            {
                let span = Span::new(SpanNamespace::new("cns.gas"), "ensemble.degradation");
                let event = NuEvent::new(
                    message.from,
                    span,
                    Phase::Compute,
                    serde_json::json!({
                        "gas_used": self.gas_used,
                        "session_cap": budget.session_cap,
                        "degradation_level": format!("{:?}", level),
                    }),
                    0,
                );
                if let Err(e) = sink.persist(&event) {
                    tracing::warn!(target: "cns.gas", error = %e, "Failed to persist gas degradation NuEvent");
                }
            }
        }

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
        // Check participant exists
        let participant = match self.participants.get(bot_webid) {
            Some(p) => p,
            None => {
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
                    return Err(EnsembleError::CapabilityDenied(format!(
                        "Bot {} lacks required capabilities {:?} for template {}",
                        bot_webid, required_caps, template_id
                    )));
                }
            }
        }

        // Simulate template-mediated dispatch (actual dispatch via hkask_templates)
        let response = format!("Bot {} processed via template {}", bot_webid, template_id);

        Ok(response)
    }

    /// Get curator WebID
    pub fn curator(&self) -> &WebID {
        &self.curator_webid
    }

    /// Clear chat history
    pub fn clear(&mut self) {
        self.messages.clear();
        info!("Chat history cleared");
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
    }

    /// Set improv mode
    pub fn set_improv_mode(&mut self, mode: ImprovMode) {
        self.improv_config.set_mode(mode);
    }

    /// Execute an improvisation turn using this session's config and participants
    ///
    /// Checks gas budget before proceeding. If the hard limit would be exceeded,
    /// returns `ImprovError::Ensemble(EnsembleError::CapabilityDenied)`.
    pub async fn improv_turn<C: InferenceClient>(
        &self,
        inference_client: &Arc<C>,
        user_message: &str,
    ) -> Result<ImprovTurn, ImprovError<C::Error>> {
        // Read-only gas budget check before inference
        if let Some(ref budget) = self.gas_budget {
            let (can_proceed, level) = self.can_proceed_with_gas(budget.per_message_cost);
            if !can_proceed {
                tracing::warn!(
                    target: "cns.gas",
                    gas_used = self.gas_used,
                    session_cap = budget.session_cap,
                    "Gas budget exceeded — improv turn rejected"
                );
                return Err(ImprovError::Ensemble(EnsembleError::CapabilityDenied(
                    format!(
                        "Gas budget exceeded: {}/{}",
                        self.gas_used, budget.session_cap
                    ),
                )));
            }
            if level != DegradationLevel::Normal {
                tracing::warn!(
                    target: "cns.gas",
                    gas_used = self.gas_used,
                    session_cap = budget.session_cap,
                    level = ?level,
                    "Gas budget degradation — improv turn proceeding with warning"
                );
            }
        }

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
    #[error("Participant not found: {0}")]
    ParticipantNotFound(String),

    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
}

/// Unified session manager for both chat and deliberation sessions.
///
/// Collapses the former `EnsembleChatManager` and `DeliberationCoordinator` into
/// a single manager that handles both session types.
pub struct SessionManager {
    chats: Arc<RwLock<HashMap<String, Arc<RwLock<EnsembleChat>>>>>,
    deliberations:
        Arc<RwLock<HashMap<String, Arc<RwLock<crate::deliberation::DeliberationSession>>>>>,
    curator_webid: WebID,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(curator_webid: WebID) -> Self {
        Self {
            chats: Arc::new(RwLock::new(HashMap::new())),
            deliberations: Arc::new(RwLock::new(HashMap::new())),
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

    /// Create a new deliberation session
    pub async fn create_deliberation(
        &self,
        session_id: &str,
    ) -> Arc<RwLock<crate::deliberation::DeliberationSession>> {
        let session = Arc::new(RwLock::new(crate::deliberation::DeliberationSession::new(
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
    ) -> Option<Arc<RwLock<crate::deliberation::DeliberationSession>>> {
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
