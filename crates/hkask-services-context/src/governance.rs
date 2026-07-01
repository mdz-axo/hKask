//! Governance context — OCAP enforcement, consent management, tool dispatch,
//! agent registration, escalation queue, and curation signal routing.
//!
//! Extracted from `AgentService` as part of the strangler-fig decomposition.
//! Public fields provide direct access to each subsystem; the single behavioral
//! method (`notify_goal_transition`) encapsulates `GoalTransitionEvent` construction.

use hkask_agents::a2a::A2ARuntime;
use hkask_agents::consent::ConsentManager;
use hkask_capability::CapabilityChecker;
use hkask_cns::types::loops::{CurationInput, GoalTransitionEvent};
use hkask_mcp::McpDispatcher;
use hkask_storage::EscalationQueue;
use hkask_types::WebID;
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

    /// Notify the curation loop of a goal state transition.
    ///
    /// Constructs and sends a `GoalTransitionEvent` through the curation
    /// channel. Silently drops the notification if no channel is configured.
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
