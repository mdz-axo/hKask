//! Multi-agent chat coordination — Curator ↔ R7 bots via A2A templates.

use hkask_types::NuEventSink;
use hkask_types::WebID;
use hkask_types::capability::{CapabilitySpec, DelegationResource};
use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
use hkask_types::ports::{RegistryIndex, ToolInfo};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;

static GAS_GOVERNANCE_WARNED: AtomicBool = AtomicBool::new(false);

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

impl GasBudgetConfig {
    /// Default session capacity (total gas units per session).
    pub const DEFAULT_SESSION_CAP: u64 = 150_000;
    /// Default gas cost per message.
    pub const DEFAULT_PER_MESSAGE_COST: u64 = 100;
    /// Default alert threshold (70% consumed).
    pub const DEFAULT_ALERT_THRESHOLD: f64 = 0.7;
    /// Default per-bot gas allocation.
    pub const DEFAULT_PER_BOT_ALLOCATION: u64 = 15_000;
    /// Default curator gas allocation.
    pub const DEFAULT_CURATOR_ALLOCATION: u64 = 25_000;
}

impl Default for GasBudgetConfig {
    fn default() -> Self {
        Self {
            session_cap: Self::DEFAULT_SESSION_CAP,
            per_message_cost: Self::DEFAULT_PER_MESSAGE_COST,
            alert_threshold: Self::DEFAULT_ALERT_THRESHOLD,
            hard_limit: true,
            per_bot_allocation: Self::DEFAULT_PER_BOT_ALLOCATION,
            curator_allocation: Self::DEFAULT_CURATOR_ALLOCATION,
        }
    }
}

impl GasBudgetConfig {
    /// Parse from the `gas` section of standing-ensemble-session.yaml
    pub fn from_yaml_gas(gas: &serde_json::Value) -> Self {
        fn yaml_u64(gas: &Value, key: &str, d: u64) -> u64 {
            gas.get(key).and_then(|v| v.as_u64()).unwrap_or(d)
        }
        Self {
            session_cap: yaml_u64(gas, "session_cap", Self::DEFAULT_SESSION_CAP),
            per_message_cost: yaml_u64(gas, "per_message_cost", Self::DEFAULT_PER_MESSAGE_COST),
            alert_threshold: gas
                .get("alert_threshold")
                .and_then(|v| v.as_f64())
                .unwrap_or(Self::DEFAULT_ALERT_THRESHOLD),
            hard_limit: gas
                .get("hard_limit")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            per_bot_allocation: yaml_u64(
                gas,
                "per_bot_allocation",
                Self::DEFAULT_PER_BOT_ALLOCATION,
            ),
            curator_allocation: yaml_u64(
                gas,
                "curator_allocation",
                Self::DEFAULT_CURATOR_ALLOCATION,
            ),
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
    /// Available tools for intersection-based tool scoping (R4).
    /// When set, `intersection_tools()` filters this list by participant capabilities.
    available_tools: Option<Vec<ToolInfo>>,
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
            available_tools: None,
        }
    }

    /// Set CNS event sink for span emission
    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Emit a CNS span event (if sink is wired).
    fn emit_span(&self, from: WebID, ns: &str, name: &str, phase: Phase, payload: Value) {
        if let Some(ref sink) = self.event_sink {
            let event = NuEvent::new(
                from,
                Span::new(SpanNamespace::new(ns), name),
                phase,
                payload,
                0,
            );
            if let Err(e) = sink.persist(&event) {
                tracing::warn!(target: "cns.gas", error = %e, "Failed to persist NuEvent: {ns}.{name}");
            }
        }
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
    pub fn with_gas_governance(mut self, port: Arc<dyn crate::ports::GasGovernancePort>) -> Self {
        self.gas_governance = Some(port);
        self
    }

    /// Set available tools for intersection-based tool scoping (R4).
    pub fn with_available_tools(mut self, tools: Vec<ToolInfo>) -> Self {
        self.available_tools = Some(tools);
        self
    }

    /// Compute tools visible to all participants (domain intersection).
    ///
    /// A tool is visible if every participant has a capability domain matching
    /// the tool's required_capability domain. Tools with no required_capability
    /// are always visible. If no avaialbe tools are set, returns None.
    ///
    /// Visibility ≠ authority — the GovernedTool membrane enforces invocation
    /// permissions separately.
    pub fn intersection_tools(&self) -> Option<Vec<ToolInfo>> {
        let all_tools = self.available_tools.as_ref()?;

        // Parse each participant's capabilities into (resource, domain) pairs.
        // Only Tool-type capabilities contribute to tool visibility.
        let participant_domains: Vec<Vec<String>> = self
            .participants
            .values()
            .filter(|p| !matches!(p.role, ParticipantRole::Curator))
            .map(|p| {
                p.capabilities
                    .iter()
                    .filter_map(|c| CapabilitySpec::parse(c).ok())
                    .filter(|s| s.resource == DelegationResource::Tool)
                    .map(|s| s.resource_id.clone())
                    .collect()
            })
            .collect();

        // If no non-Curator participants have declared capabilities,
        // all tools are visible (backward compat).
        let all_empty = participant_domains.iter().all(|d| d.is_empty());
        if all_empty {
            return Some(all_tools.clone());
        }

        // A tool is visible to ALL if every participant has at least one
        // capability domain that covers the tool's required_capability domain.
        // Tools with no required_capability are always visible.
        let visible_tools: Vec<ToolInfo> = all_tools
            .iter()
            .filter(|t| {
                // Tools with no required_capability are always visible
                if t.required_capability.is_none() {
                    return true;
                }

                let tool_domain = t
                    .required_capability
                    .as_ref()
                    .and_then(|c| CapabilitySpec::parse(c).ok())
                    .map(|s| s.resource_id.clone())
                    .unwrap_or_else(|| {
                        // Fallback: derive domain from server_id
                        t.server_id
                            .strip_prefix("hkask-mcp-")
                            .unwrap_or(&t.server_id)
                            .to_string()
                    });

                // Tool is visible to ALL participants if every participant
                // has at least one capability domain that matches the tool's domain.
                participant_domains
                    .iter()
                    .all(|domains| domains.iter().any(|d| d == &tool_domain))
            })
            .cloned()
            .collect();

        Some(visible_tools)
    }

    /// Returns `(can_proceed, level)`. Hard-limit checks if cost exceeds cap.
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

    /// Record gas consumption; emits CNS span on degradation.
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
            self.emit_span(
                self.curator_webid,
                "cns.gas",
                "ensemble.degradation",
                Phase::Compute,
                json!({
                    "gas_used": self.gas_used,
                    "session_cap": self.gas_budget.as_ref().map(|b| b.session_cap).unwrap_or(0),
                    "degradation_level": format!("{:?}", level),
                }),
            );
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

    /// Add a message. Dedup checks, gas enforcement, CNS observability.
    pub fn add_message(&mut self, message: ChatMessage) {
        // Layer 2 DRY: dedup check — skip duplicates
        if !self.dedup.check_and_register(&message) {
            tracing::debug!(
                target: "cns.ensemble.chat",
                from = %message.from,
                content_len = message.content.len(),
                "Message rejected as duplicate (dedup)"
            );
            self.emit_span(
                message.from,
                "cns.ensemble.chat",
                "dedup_rejected",
                Phase::Compute,
                json!({
                    "from": message.from.to_string(),
                    "content_len": message.content.len(),
                    "dedup_rejected": true,
                }),
            );
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
                self.emit_span(
                    message.from,
                    "cns.gas",
                    "ensemble.message_rejected",
                    Phase::Compute,
                    json!({
                        "gas_used": self.gas_used,
                        "session_cap": budget.session_cap,
                        "message_rejected": true,
                    }),
                );
                return;
            }
            self.gas_used += cost;
            if level != DegradationLevel::Normal {
                self.emit_span(
                    message.from,
                    "cns.gas",
                    "ensemble.degradation",
                    Phase::Compute,
                    json!({
                        "gas_used": self.gas_used,
                        "session_cap": budget.session_cap,
                        "degradation_level": format!("{:?}", level),
                    }),
                );
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
        } else if GAS_GOVERNANCE_WARNED
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            tracing::warn!(
                target: "cns.gas",
                "No GasGovernancePort wired — expected in API mode. CLI wires gas governance automatically."
            );
        }

        self.messages.push(message);
    }

    /// Pre-register message in dedup filter (for restoring from storage).
    pub fn register_dedup(&mut self, message: &ChatMessage) {
        self.dedup.register(message);
    }

    /// Add a restored message (from persistence) without gas accounting.
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
        // Gas budget check before inference
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
        } else if GAS_GOVERNANCE_WARNED
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            tracing::warn!(
                target: "cns.gas",
                "No GasGovernancePort wired — ensemble improv turn running without gas governance. \
                 This is expected in API mode. CLI mode wires gas governance automatically."
            );
        }

        let participants: Vec<(WebID, String, String)> = self
            .participants
            .values()
            .filter(|p| !matches!(p.role, ParticipantRole::Curator))
            .map(|p| {
                let s = format!("{:?}", p.role);
                (p.webid, s.clone(), format!("Agent with role {s}"))
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
