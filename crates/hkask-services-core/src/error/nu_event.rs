//! CNS ν-event emission for `ServiceError`.
//!
//! Only system-level errors (infrastructure, inference, CNS, storage)
//! emit ν-events. User-input errors (NotFound, InvalidInput, LoginFailed)
//! are not system conditions — they don't need CNS observability.

use super::{DomainKind, ServiceError};

impl ServiceError {
    /// Emit a ν-event for CNS-observable errors.
    ///
    /// Returns `None` for user-input errors that don't represent system
    /// conditions. Returns `Some(RegulationRecord)` for infrastructure, inference,
    /// CNS, storage, and security errors the CNS can act on.
    ///
    /// The observer WebID is freshly generated per event — these are
    /// system-level observations, not agent-specific.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  self must be a valid ServiceError variant
    /// post: returns Some(RegulationRecord) for system-level errors (inference, CNS, storage, infra); None for user-input errors (not-found, validation)
    #[must_use]
    pub fn nu_event(&self) -> Option<hkask_types::event::RegulationRecord> {
        use hkask_types::event::{CyclePhase, RegulationRecord, Span, SpanNamespace};
        use hkask_types::id::WebID;

        let (namespace, path_suffix, observation) = match self {
            // ── Domain (consolidated) ──────────────────────────────────
            ServiceError::Domain {
                domain, message, ..
            } => match domain {
                DomainKind::Agent => (
                    "cns.agent_pod",
                    "error",
                    serde_json::json!({ "message": message }),
                ),
                DomainKind::Consent => (
                    "cns.sovereignty",
                    "error.consent",
                    serde_json::json!({ "message": message }),
                ),
                DomainKind::Curator => (
                    "cns.curation",
                    "error",
                    serde_json::json!({ "message": message }),
                ),
                DomainKind::Federation => (
                    "cns.federation.sync",
                    "error",
                    serde_json::json!({ "message": message }),
                ),
                DomainKind::Inference => (
                    "cns.inference",
                    "error",
                    serde_json::json!({ "message": message }),
                ),
                DomainKind::Infrastructure => (
                    "cns.cybernetics",
                    "error",
                    serde_json::json!({ "message": message }),
                ),
                DomainKind::Memory => (
                    "cns.memory.encode",
                    "error",
                    serde_json::json!({ "message": message }),
                ),
                DomainKind::Pod => (
                    "cns.agent_pod",
                    "error",
                    serde_json::json!({ "message": message }),
                ),
                DomainKind::Storage => (
                    "cns.cybernetics",
                    "error.storage",
                    serde_json::json!({ "message": message }),
                ),
                DomainKind::User => (
                    "cns.sovereignty",
                    "error.user",
                    serde_json::json!({ "message": message }),
                ),
                DomainKind::Wallet => (
                    "cns.wallet.balance",
                    "error",
                    serde_json::json!({ "message": message }),
                ),
                DomainKind::Mcp => (
                    "cns.tool",
                    "error",
                    serde_json::json!({ "message": message }),
                ),
                DomainKind::Skill => (
                    "cns.skill",
                    "error",
                    serde_json::json!({ "message": message }),
                ),
            },
            ServiceError::ModelService { message, .. } => (
                "cns.inference",
                "error.model_service",
                serde_json::json!({ "message": message }),
            ),
            ServiceError::McpTool {
                kind,
                server,
                tool,
                message,
            } => (
                "cns.tool",
                "error",
                serde_json::json!({
                    "kind": kind.to_string(),
                    "server": server,
                    "tool": tool,
                    "message": message,
                }),
            ),
            ServiceError::Infra(e) => (
                "cns.cybernetics",
                "error.infra",
                serde_json::json!({ "error": e.to_string() }),
            ),
            // User-input error — not a system condition
            ServiceError::InvalidWebID { .. } => return None,
        };

        let span = Span::new(
            SpanNamespace::new(namespace).expect("canonical namespace"),
            path_suffix,
        );
        Some(RegulationRecord::new(
            WebID::new(),
            span,
            CyclePhase::Sense,
            observation,
            0,
        ))
    }
}
