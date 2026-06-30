//! Governance context — OCAP enforcement, consent management, tool dispatch,
//! agent registration, escalation queue, and curation signal routing.
//!
//! Extracted from `AgentService` as part of the strangler-fig decomposition
//! (Finding A — AgentService god object). Consolidates six governance
//! concerns that were previously exposed as individual pass-through accessors.

use hkask_agents::a2a::{A2AAgent, A2AError, A2ARuntime};
use hkask_agents::consent::{ConsentError, ConsentManager};
use hkask_capability::{CapabilityChecker, DelegationAction, DelegationResource, DelegationToken};
use hkask_cns::types::loops::{CurationInput, GoalTransitionEvent};
use hkask_mcp::McpDispatcher;
use hkask_storage::{EscalationEntry, EscalationError, EscalationQueue, EscalationStats};
use hkask_types::curation::DataCategory;
use hkask_types::{AgentKind, BotID, EscalationID, TemplateID, WebID};
use std::sync::Arc;

/// Consolidated governance context — OCAP, consent, dispatch, agents,
/// escalations, and curation signals.
pub struct GovernanceContext {
    pub checker: Arc<CapabilityChecker>,
    pub consent: Arc<ConsentManager>,
    pub dispatcher: Arc<McpDispatcher>,
    pub a2a: Arc<A2ARuntime>,
    pub escalations: Arc<EscalationQueue>,
    pub curation_tx: Option<tokio::sync::mpsc::UnboundedSender<CurationInput>>,
}

impl GovernanceContext {
    pub fn new(
        checker: Arc<CapabilityChecker>,
        consent: Arc<ConsentManager>,
        dispatcher: Arc<McpDispatcher>,
        a2a: Arc<A2ARuntime>,
        escalations: Arc<EscalationQueue>,
        curation_tx: Option<tokio::sync::mpsc::UnboundedSender<CurationInput>>,
    ) -> Self {
        Self {
            checker,
            consent,
            dispatcher,
            a2a,
            escalations,
            curation_tx,
        }
    }

    // ── OCAP ────────────────────────────────────────────────────────────────

    pub fn check_tool_access(&self, token: &DelegationToken, webid: &WebID) -> bool {
        self.checker
            .check_resource(token, webid, DelegationResource::Tool)
    }

    pub fn grant_registry_token(
        &self,
        action: DelegationAction,
        issuer: WebID,
        target: WebID,
    ) -> DelegationToken {
        self.checker.grant_registry(action, issuer, target)
    }

    // ── Consent ─────────────────────────────────────────────────────────────

    pub fn has_consent(
        &self,
        webid_str: &str,
        category: &DataCategory,
    ) -> Result<bool, ConsentError> {
        self.consent.has_consent(webid_str, category)
    }

    pub fn grant_consent(
        &self,
        webid_str: &str,
        category: &DataCategory,
    ) -> Result<(), ConsentError> {
        self.consent.grant_consent(webid_str, category)
    }

    pub fn revoke_consent(&self, webid_str: &str) -> Result<(), ConsentError> {
        self.consent.revoke_consent(webid_str)
    }

    pub fn get_granted_categories(&self, webid_str: &str) -> Result<Vec<String>, ConsentError> {
        self.consent.get_granted_categories(webid_str)
    }

    // ── MCP Dispatch ────────────────────────────────────────────────────────

    pub async fn invoke_tool(
        &self,
        tool_name: &str,
        input: serde_json::Value,
        token: &DelegationToken,
    ) -> Result<serde_json::Value, hkask_templates::TemplateError> {
        use hkask_templates::McpPort;
        self.dispatcher.invoke(tool_name, input, token).await
    }

    pub fn issue_capability(&self, capability: String, from: WebID, to: WebID) -> DelegationToken {
        self.dispatcher.issue_capability(capability, from, to)
    }

    pub async fn shutdown_all_mcp(&self) {
        self.dispatcher.shutdown_all().await;
    }

    // ── Escalation Queue ───────────────────────────────────────────────────

    pub fn add_escalation(
        &self,
        template_id: TemplateID,
        bot_id: BotID,
        output: String,
        confidence: f64,
        retry_count: u32,
        error_context: String,
    ) -> Result<EscalationID, EscalationError> {
        self.escalations.add(
            template_id,
            bot_id,
            output,
            confidence,
            retry_count,
            error_context,
        )
    }

    pub fn list_pending_escalations(&self) -> Result<Vec<EscalationEntry>, EscalationError> {
        self.escalations.list_pending()
    }

    pub fn resolve_escalation(&self, id: &str, resolved_by: &str) -> Result<(), EscalationError> {
        self.escalations.resolve(id, resolved_by)
    }

    pub fn dismiss_escalation(&self, id: &str, dismissed_by: &str) -> Result<(), EscalationError> {
        self.escalations.dismiss(id, dismissed_by)
    }

    pub fn escalation_stats(&self) -> Result<EscalationStats, EscalationError> {
        self.escalations.stats()
    }

    // ── A2A Runtime ────────────────────────────────────────────────────────

    pub async fn register_agent(
        &self,
        webid: WebID,
        kind: AgentKind,
        capabilities: Vec<String>,
    ) -> Result<DelegationToken, A2AError> {
        self.a2a.register_agent(webid, kind, capabilities).await
    }

    pub async fn list_agents(&self) -> Vec<A2AAgent> {
        self.a2a.list_agents().await
    }

    pub async fn unregister_agent(&self, webid: &WebID) -> Result<(), A2AError> {
        self.a2a.unregister_agent(webid).await
    }

    // ── Curation Inbox ─────────────────────────────────────────────────────

    pub fn notify_goal_transition(
        &self,
        goal_id: String,
        from_state: String,
        to_state: String,
        agent: WebID,
    ) {
        if let Some(tx) = &self.curation_tx {
            let event = CurationInput::GoalTransition(GoalTransitionEvent {
                goal_id,
                from_state,
                to_state,
                agent,
            });
            let _ = tx.send(event);
        }
    }
}
