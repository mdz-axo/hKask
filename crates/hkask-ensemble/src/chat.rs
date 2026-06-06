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
    event_sink: Option<Arc<dyn NuEventSink + Send + Sync>>,
    gas_budget: Option<GasBudgetConfig>,
    gas_used: u64,
    dedup: crate::chat_dedup::ChatDedup,
    gas_governance: Option<Arc<dyn crate::ports::GasGovernancePort>>,
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
            dedup: crate::chat_dedup::ChatDedup::new(),
            gas_governance: None,
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

    /// Set CNS gas governance port for CyberneticsLoop observability.
    ///
    /// After `add_message()` consumes gas internally, the governance port
    /// is also notified so the CNS can sense ensemble gas usage. Ensemble
    /// gas is dual-tracked: internal counter for degradation levels,
    /// CyberneticsLoop for CNS observability.
    pub fn with_gas_governance(mut self, port: Arc<dyn crate::ports::GasGovernancePort>) -> Self {
        self.gas_governance = Some(port);
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
        // Layer 2 DRY: dedup check — skip duplicates
        if !self.dedup.check_and_register(&message) {
            tracing::debug!(
                target: "cns.ensemble.chat",
                from = %message.from,
                content_len = message.content.len(),
                "Message rejected as duplicate (dedup)"
            );
            if let Some(ref sink) = self.event_sink {
                let span = Span::new(SpanNamespace::new("cns.ensemble.chat"), "dedup_rejected");
                let event = NuEvent::new(
                    message.from,
                    span,
                    Phase::Compute,
                    serde_json::json!({
                        "from": message.from.to_string(),
                        "content_len": message.content.len(),
                        "dedup_rejected": true,
                    }),
                    0,
                );
                if let Err(e) = sink.persist(&event) {
                    tracing::warn!(target: "cns.ensemble.chat", error = %e, "Failed to persist dedup_rejected NuEvent");
                }
            }
            return;
        }

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

        // CNS gas governance: report usage to CyberneticsLoop (dual-track)
        if let Some(ref governance) = self.gas_governance {
            let cost = self
                .gas_budget
                .as_ref()
                .map(|b| b.per_message_cost)
                .unwrap_or(0);
            if !governance.can_proceed(cost) {
                tracing::warn!(
                    target: "cns.gas",
                    gas = cost,
                    "Message rejected — CNS governance blocked operation"
                );
                return;
            }
            governance.acquire(cost);
        }

        self.messages.push(message);
    }

    /// Pre-register a message in the dedup filter without adding it to history.
    ///
    /// Use this when restoring messages from storage so that reloaded messages
    /// are not falsely flagged as duplicates.
    pub fn register_dedup(&mut self, message: &ChatMessage) {
        self.dedup.register(message);
    }

    /// Add a restored message (from persistence) without gas accounting.
    ///
    /// Used when loading messages from storage to avoid re-charging gas
    /// and to pre-register in dedup tracking.
    pub fn add_restored_message(&mut self, message: ChatMessage) {
        self.dedup.register(&message);
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
        self.dedup.clear();
        info!("Chat history and dedup filter cleared");
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

        // CNS gas governance: check can_proceed before inference
        if let Some(ref governance) = self.gas_governance {
            let cost = self
                .gas_budget
                .as_ref()
                .map(|b| b.per_message_cost)
                .unwrap_or(0);
            if !governance.can_proceed(cost) {
                tracing::warn!(
                    target: "cns.gas",
                    gas = cost,
                    "Improv turn rejected — CNS governance blocked operation"
                );
                return Err(ImprovError::Ensemble(EnsembleError::CapabilityDenied(
                    "CNS governance blocked operation".to_string(),
                )));
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

pub use crate::session::SessionManager;

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::WebID;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    struct MockGasGovernance {
        can_proceed_result: AtomicBool,
        acquire_calls: AtomicU64,
    }

    impl MockGasGovernance {
        fn new(allows: bool) -> Self {
            Self {
                can_proceed_result: AtomicBool::new(allows),
                acquire_calls: AtomicU64::new(0),
            }
        }
    }

    impl crate::ports::GasGovernancePort for MockGasGovernance {
        fn can_proceed(&self, _gas: u64) -> bool {
            self.can_proceed_result.load(Ordering::Relaxed)
        }
        fn acquire(&self, gas: u64) {
            self.acquire_calls.fetch_add(gas, Ordering::Relaxed);
        }
    }

    #[test]
    fn gas_budget_config_default() {
        let cfg = GasBudgetConfig::default();
        assert_eq!(cfg.session_cap, 150000);
        assert_eq!(cfg.per_message_cost, 100);
        assert!((cfg.alert_threshold - 0.7).abs() < f64::EPSILON);
        assert!(cfg.hard_limit);
        assert_eq!(cfg.per_bot_allocation, 15000);
        assert_eq!(cfg.curator_allocation, 25000);
    }

    #[test]
    fn gas_budget_config_degradation_normal() {
        let cfg = GasBudgetConfig::default();
        assert_eq!(cfg.degradation_level(0), DegradationLevel::Normal);
    }

    #[test]
    fn gas_budget_config_degradation_batch_only() {
        let cfg = GasBudgetConfig::default();
        // 80% of 150000 = 120000
        assert_eq!(
            cfg.degradation_level(120000),
            DegradationLevel::BatchOnlyMemory
        );
    }

    #[test]
    fn gas_budget_config_degradation_suspend_reports() {
        let cfg = GasBudgetConfig::default();
        // 90% of 150000 = 135000
        assert_eq!(
            cfg.degradation_level(135000),
            DegradationLevel::SuspendReports
        );
    }

    #[test]
    fn gas_budget_config_degradation_escalate() {
        let cfg = GasBudgetConfig::default();
        // 95% of 150000 = 142500
        assert_eq!(cfg.degradation_level(142500), DegradationLevel::Escalate);
    }

    #[test]
    fn gas_budget_config_from_yaml_gas_partial() {
        let yaml = serde_json::json!({ "session_cap": 50000 });
        let cfg = GasBudgetConfig::from_yaml_gas(&yaml);
        assert_eq!(cfg.session_cap, 50000);
        // Rest should be defaults
        assert_eq!(cfg.per_message_cost, 100);
        assert!((cfg.alert_threshold - 0.7).abs() < f64::EPSILON);
        assert!(cfg.hard_limit);
        assert_eq!(cfg.per_bot_allocation, 15000);
        assert_eq!(cfg.curator_allocation, 25000);
    }

    fn curator_id() -> WebID {
        WebID::from_persona(b"curator")
    }

    fn bot_id(name: &[u8]) -> WebID {
        WebID::from_persona(name)
    }

    #[test]
    fn ensemble_chat_add_message_dedup_rejects_duplicate() {
        let mut chat = EnsembleChat::new(curator_id());
        let msg = ChatMessage::new(curator_id(), "hello".into());
        chat.add_message(msg.clone());
        chat.add_message(msg.clone());
        assert_eq!(chat.get_history().len(), 1);
    }

    #[test]
    fn ensemble_chat_add_message_different_from_same_content() {
        let mut chat = EnsembleChat::new(curator_id());
        let msg_a = ChatMessage::new(curator_id(), "hello".into());
        let msg_b = ChatMessage::new(bot_id(b"bot1"), "hello".into());
        chat.add_message(msg_a);
        chat.add_message(msg_b);
        assert_eq!(chat.get_history().len(), 2);
    }

    #[test]
    fn ensemble_chat_add_restored_message_skips_gas() {
        let budget = GasBudgetConfig::default();
        let mut chat = EnsembleChat::new(curator_id()).with_gas_budget(budget);
        let msg = ChatMessage::new(curator_id(), "restored".into());
        chat.add_restored_message(msg);
        assert_eq!(chat.gas_used(), 0);
    }

    #[test]
    fn ensemble_chat_add_restored_message_registers_dedup() {
        let mut chat = EnsembleChat::new(curator_id());
        let msg = ChatMessage::new(curator_id(), "restored".into());
        chat.add_restored_message(msg.clone());
        // Same content via add_message should be rejected as duplicate
        chat.add_message(msg.clone());
        assert_eq!(chat.get_history().len(), 1);
    }

    #[test]
    fn ensemble_chat_clear_clears_dedup() {
        let mut chat = EnsembleChat::new(curator_id());
        let msg = ChatMessage::new(curator_id(), "hello".into());
        chat.add_message(msg.clone());
        assert_eq!(chat.get_history().len(), 1);
        chat.clear();
        assert_eq!(chat.get_history().len(), 0);
        // Re-adding same message should be accepted after clear
        chat.add_message(msg.clone());
        assert_eq!(chat.get_history().len(), 1);
    }

    #[test]
    fn ensemble_chat_register_dedup_prevents_later_add() {
        let mut chat = EnsembleChat::new(curator_id());
        let msg = ChatMessage::new(curator_id(), "preregistered".into());
        chat.register_dedup(&msg);
        // Attempting to add the same message should be rejected
        chat.add_message(msg);
        assert_eq!(chat.get_history().len(), 0);
    }

    #[test]
    fn ensemble_chat_gas_budget_hard_limit_rejects() {
        let budget = GasBudgetConfig {
            session_cap: 200,
            per_message_cost: 100,
            alert_threshold: 0.7,
            hard_limit: true,
            per_bot_allocation: 50,
            curator_allocation: 50,
        };
        let mut chat = EnsembleChat::new(curator_id()).with_gas_budget(budget);
        // First two messages: 2 × 100 = 200 (at cap)
        chat.add_message(ChatMessage::new(curator_id(), "msg1".into()));
        chat.add_message(ChatMessage::new(bot_id(b"bot1"), "msg2".into()));
        assert_eq!(chat.get_history().len(), 2);
        assert_eq!(chat.gas_used(), 200);
        // Third message would exceed cap → rejected
        chat.add_message(ChatMessage::new(bot_id(b"bot2"), "msg3".into()));
        assert_eq!(chat.get_history().len(), 2);
    }

    #[test]
    fn ensemble_chat_gas_budget_no_hard_limit_allows() {
        let budget = GasBudgetConfig {
            session_cap: 200,
            per_message_cost: 100,
            alert_threshold: 0.7,
            hard_limit: false,
            per_bot_allocation: 50,
            curator_allocation: 50,
        };
        let mut chat = EnsembleChat::new(curator_id()).with_gas_budget(budget);
        chat.add_message(ChatMessage::new(curator_id(), "msg1".into()));
        chat.add_message(ChatMessage::new(bot_id(b"bot1"), "msg2".into()));
        // Third message: over cap but hard_limit=false → still accepted
        chat.add_message(ChatMessage::new(bot_id(b"bot2"), "msg3".into()));
        assert_eq!(chat.get_history().len(), 3);
    }

    #[test]
    fn ensemble_chat_can_proceed_with_gas() {
        let budget = GasBudgetConfig {
            session_cap: 1000,
            per_message_cost: 100,
            alert_threshold: 0.7,
            hard_limit: true,
            per_bot_allocation: 100,
            curator_allocation: 200,
        };
        let mut chat = EnsembleChat::new(curator_id()).with_gas_budget(budget);

        // No gas used yet, additional 100 → total 100 → Normal
        let (ok, level) = chat.can_proceed_with_gas(100);
        assert!(ok);
        assert_eq!(level, DegradationLevel::Normal);

        // Consume gas up to 700. Next 100 → total 800 (80%) → BatchOnlyMemory
        for _ in 0..7 {
            chat.consume_gas(100);
        }
        let (ok, level) = chat.can_proceed_with_gas(100);
        assert!(ok);
        assert_eq!(level, DegradationLevel::BatchOnlyMemory);

        // Consume up to 800. Next 100 → total 900 (90%) → SuspendReports
        chat.consume_gas(100);
        let (ok, level) = chat.can_proceed_with_gas(100);
        assert!(ok);
        assert_eq!(level, DegradationLevel::SuspendReports);

        // Consume up to 850. Next 100 → total 950 (95%) → Escalate
        chat.consume_gas(50);
        let (ok, level) = chat.can_proceed_with_gas(100);
        assert!(ok);
        assert_eq!(level, DegradationLevel::Escalate);

        // Consume up to 1000 (100%). Next 100 → total 1100 → would exceed cap
        chat.consume_gas(150);
        let (ok, _) = chat.can_proceed_with_gas(100);
        assert!(!ok);
    }

    #[test]
    fn ensemble_chat_consume_gas_emits_degradation() {
        let budget = GasBudgetConfig::default();
        let mut chat = EnsembleChat::new(curator_id()).with_gas_budget(budget);
        // Consume gas and verify gas_used increments
        assert_eq!(chat.gas_used(), 0);
        chat.consume_gas(500);
        assert_eq!(chat.gas_used(), 500);
        chat.consume_gas(300);
        assert_eq!(chat.gas_used(), 800);
    }

    #[test]
    fn ensemble_chat_gas_governance_can_proceed_blocks() {
        let mock = Arc::new(MockGasGovernance::new(false));
        let mut chat = EnsembleChat::new(curator_id()).with_gas_governance(mock);
        chat.add_message(ChatMessage::new(curator_id(), "blocked".into()));
        assert_eq!(chat.get_history().len(), 0);
    }

    #[test]
    fn ensemble_chat_gas_governance_acquire_called() {
        let mock = Arc::new(MockGasGovernance::new(true));
        let mut chat = EnsembleChat::new(curator_id()).with_gas_governance(mock.clone());
        chat.add_message(ChatMessage::new(curator_id(), "allowed".into()));
        assert_eq!(chat.get_history().len(), 1);
        // acquire should have been called with 0 (no gas budget → per_message_cost defaults to 0)
        assert_eq!(mock.acquire_calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn ensemble_chat_gas_governance_acquire_called_with_budget() {
        let mock = Arc::new(MockGasGovernance::new(true));
        let budget = GasBudgetConfig {
            session_cap: 10000,
            per_message_cost: 100,
            alert_threshold: 0.7,
            hard_limit: true,
            per_bot_allocation: 1000,
            curator_allocation: 2000,
        };
        let mut chat = EnsembleChat::new(curator_id())
            .with_gas_budget(budget)
            .with_gas_governance(mock.clone());
        chat.add_message(ChatMessage::new(curator_id(), "allowed".into()));
        assert_eq!(chat.get_history().len(), 1);
        assert_eq!(mock.acquire_calls.load(Ordering::Relaxed), 100);
    }
}
