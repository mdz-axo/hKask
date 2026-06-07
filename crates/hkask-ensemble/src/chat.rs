//! Multi-agent chat coordination
//!
//! Orchestrates conversation between Curator (replicant) and R7 bots
//! via template-mediated A2A communication. No swarms, no consensus mechanisms.

use hkask_types::NuEventSink;
use hkask_types::WebID;
use hkask_types::capability::{CapabilitySpec, DelegationResource};
use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
use hkask_types::ports::{RegistryIndex, ToolInfo};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::info;

/// Tracks whether the missing-gas-governance warning has been emitted,
/// so we warn once per `EnsembleChat` lifetime rather than per message.
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
        Self {
            session_cap: gas
                .get("session_cap")
                .and_then(|v| v.as_u64())
                .unwrap_or(Self::DEFAULT_SESSION_CAP),
            per_message_cost: gas
                .get("per_message_cost")
                .and_then(|v| v.as_u64())
                .unwrap_or(Self::DEFAULT_PER_MESSAGE_COST),
            alert_threshold: gas
                .get("alert_threshold")
                .and_then(|v| v.as_f64())
                .unwrap_or(Self::DEFAULT_ALERT_THRESHOLD),
            hard_limit: gas
                .get("hard_limit")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            per_bot_allocation: gas
                .get("per_bot_allocation")
                .and_then(|v| v.as_u64())
                .unwrap_or(Self::DEFAULT_PER_BOT_ALLOCATION),
            curator_allocation: gas
                .get("curator_allocation")
                .and_then(|v| v.as_u64())
                .unwrap_or(Self::DEFAULT_CURATOR_ALLOCATION),
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

    /// Set available tools for intersection-based tool scoping (R4).
    ///
    /// When set, `intersection_tools()` returns only the tools whose
    /// required_capability domains intersect across all participants.
    pub fn with_available_tools(mut self, tools: Vec<ToolInfo>) -> Self {
        self.available_tools = Some(tools);
        self
    }

    /// Compute the tools visible to all participants (intersection).
    ///
    /// Each participant can only use tools matching their capabilities.
    /// The shared tool section lists only tools that ALL participants can see.
    ///
    /// A tool is visible to a participant if:
    /// - The participant has a capability whose domain matches the tool's `required_capability`
    /// - Or the tool has no `required_capability` (always visible)
    ///
    /// ## Design note: visibility vs. authority
    ///
    /// The intersection uses **domain matching only**, not `capabilities_match()`
    /// with its action hierarchy. A participant with `tool:cns:read` will cause
    /// CNS tools to appear in the intersection (domain "cns" matches), even
    /// though that participant cannot *invoke* those tools (read ≱ execute).
    ///
    /// This is intentional: the intersection determines **visibility** (which tools
    /// appear in the shared context), while the `GovernedTool` membrane enforces
    /// **authority** (whether invocation is permitted). Showing tools you can see
    /// but not invoke is acceptable; hiding tools you can't invoke is also valid
    /// but produces a more conservative (smaller) intersection.
    ///
    /// If a stricter model is desired, replace the domain-string comparison
    /// with `capabilities_match()` and check action levels.
    ///
    /// Returns `None` if no available tools have been set.
    /// Returns an empty Vec if the intersection is empty (no common tools).
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
        } else if GAS_GOVERNANCE_WARNED
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            // No gas governance wired — ensemble sessions in API mode bypass all gas tracking.
            // This is a known gap: https://github.com/mdz-axo/hKask/issues/XXX
            // The CLI wires CyberneticsLoopGasAdapter, but API does not.
            tracing::warn!(
                target: "cns.gas",
                "No GasGovernancePort wired — ensemble session running without gas governance. \
                 This is expected in API mode. CLI mode wires gas governance automatically."
            );
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

    // --- Intersection-based tool scoping tests (R4) ---

    fn cns_tool_info() -> ToolInfo {
        ToolInfo {
            name: "cns_health".to_string(),
            description: "CNS health check".to_string(),
            input_schema: serde_json::json!({}),
            server_id: "hkask-mcp-cns".to_string(),
            required_capability: Some("tool:cns:execute".to_string()),
        }
    }

    fn semantic_tool_info() -> ToolInfo {
        ToolInfo {
            name: "semantic_search".to_string(),
            description: "Semantic search".to_string(),
            input_schema: serde_json::json!({}),
            server_id: "hkask-mcp-semantic".to_string(),
            required_capability: Some("tool:semantic:execute".to_string()),
        }
    }

    fn inference_tool_info() -> ToolInfo {
        ToolInfo {
            name: "generate".to_string(),
            description: "Generate text".to_string(),
            input_schema: serde_json::json!({}),
            server_id: "hkask-mcp-inference".to_string(),
            required_capability: Some("tool:inference:execute".to_string()),
        }
    }

    fn unscoped_tool_info() -> ToolInfo {
        ToolInfo {
            name: "custom_tool".to_string(),
            description: "Custom tool with no capability requirement".to_string(),
            input_schema: serde_json::json!({}),
            server_id: "custom-server".to_string(),
            required_capability: None,
        }
    }

    #[test]
    fn intersection_tools_overlapping_capabilities() {
        // Two participants with overlapping CNS capability → CNS tool is visible
        let mut chat = EnsembleChat::new(curator_id());
        let bot1 = WebID::new();
        let bot2 = WebID::new();
        chat.register_participant(ChatParticipant {
            webid: bot1,
            role: ParticipantRole::Custom("analyst".to_string()),
            pod_id: None,
            capabilities: vec!["tool:cns:execute".to_string()],
        });
        chat.register_participant(ChatParticipant {
            webid: bot2,
            role: ParticipantRole::Custom("researcher".to_string()),
            pod_id: None,
            capabilities: vec![
                "tool:cns:execute".to_string(),
                "tool:semantic:execute".to_string(),
            ],
        });

        let chat = chat.with_available_tools(vec![cns_tool_info(), semantic_tool_info()]);
        let visible = chat.intersection_tools().unwrap();
        assert_eq!(visible.len(), 1, "Only CNS tool should be visible to both");
        assert_eq!(visible[0].name, "cns_health");
    }

    #[test]
    fn intersection_tools_non_overlapping_capabilities() {
        // Two participants with non-overlapping capabilities → empty intersection
        let mut chat = EnsembleChat::new(curator_id());
        let bot1 = WebID::new();
        let bot2 = WebID::new();
        chat.register_participant(ChatParticipant {
            webid: bot1,
            role: ParticipantRole::Custom("analyst".to_string()),
            pod_id: None,
            capabilities: vec!["tool:cns:execute".to_string()],
        });
        chat.register_participant(ChatParticipant {
            webid: bot2,
            role: ParticipantRole::Custom("researcher".to_string()),
            pod_id: None,
            capabilities: vec!["tool:semantic:execute".to_string()],
        });

        let chat = chat.with_available_tools(vec![cns_tool_info(), semantic_tool_info()]);
        let visible = chat.intersection_tools().unwrap();
        assert!(visible.is_empty(), "No tools in common");
    }

    #[test]
    fn intersection_tools_no_capabilities_shows_all() {
        // One participant with no capabilities → all tools visible (backward compat)
        let mut chat = EnsembleChat::new(curator_id());
        let bot1 = WebID::new();
        chat.register_participant(ChatParticipant {
            webid: bot1,
            role: ParticipantRole::Custom("assistant".to_string()),
            pod_id: None,
            capabilities: vec![],
        });

        let chat = chat.with_available_tools(vec![cns_tool_info(), semantic_tool_info()]);
        let visible = chat.intersection_tools().unwrap();
        assert_eq!(
            visible.len(),
            2,
            "All tools visible when no capabilities declared"
        );
    }

    #[test]
    fn intersection_tools_three_participants_smallest_common() {
        // Three participants: A has cns, B has cns+semantic, C has cns+inference
        // Intersection = {cns} only
        let mut chat = EnsembleChat::new(curator_id());
        let bot1 = WebID::new();
        let bot2 = WebID::new();
        let bot3 = WebID::new();
        chat.register_participant(ChatParticipant {
            webid: bot1,
            role: ParticipantRole::Custom("a".to_string()),
            pod_id: None,
            capabilities: vec!["tool:cns:execute".to_string()],
        });
        chat.register_participant(ChatParticipant {
            webid: bot2,
            role: ParticipantRole::Custom("b".to_string()),
            pod_id: None,
            capabilities: vec![
                "tool:cns:execute".to_string(),
                "tool:semantic:execute".to_string(),
            ],
        });
        chat.register_participant(ChatParticipant {
            webid: bot3,
            role: ParticipantRole::Custom("c".to_string()),
            pod_id: None,
            capabilities: vec![
                "tool:cns:execute".to_string(),
                "tool:inference:execute".to_string(),
            ],
        });

        let chat = chat.with_available_tools(vec![
            cns_tool_info(),
            semantic_tool_info(),
            inference_tool_info(),
        ]);
        let visible = chat.intersection_tools().unwrap();
        assert_eq!(visible.len(), 1, "Only CNS tool is in the intersection");
        assert_eq!(visible[0].name, "cns_health");
    }

    #[test]
    fn intersection_tools_unscoped_tools_always_visible() {
        // Tools with no required_capability are always visible to everyone
        let mut chat = EnsembleChat::new(curator_id());
        let bot1 = WebID::new();
        chat.register_participant(ChatParticipant {
            webid: bot1,
            role: ParticipantRole::Custom("analyst".to_string()),
            pod_id: None,
            capabilities: vec!["tool:cns:execute".to_string()],
        });

        let chat = chat.with_available_tools(vec![cns_tool_info(), unscoped_tool_info()]);
        let visible = chat.intersection_tools().unwrap();
        assert_eq!(visible.len(), 2, "Unscoped tool always visible");
    }

    #[test]
    fn intersection_tools_none_when_no_tools_set() {
        // Without setting available_tools, intersection_tools returns None
        let chat = EnsembleChat::new(curator_id());
        assert!(chat.intersection_tools().is_none());
    }
}
