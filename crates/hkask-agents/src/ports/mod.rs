//! Hexagonal Ports (Traits)
//!
//! Port definitions for hexagonal architecture.
//! All port traits live here so that domain code depends only on
//! these abstractions, never on concrete adapters.

pub mod acp;
pub mod acp_transport;
pub mod audit_log;
pub mod git_cas;
pub mod mcp_runtime;
pub mod memory_storage;
pub mod metacognition;
pub mod registry_source;
pub mod sovereignty;
pub mod standing_session;

pub use acp::AcpPort;
pub use acp_transport::{AcpTransport, AcpWireMessage, AcpWireResponse};
pub use audit_log::{AuditContext, AuditEntry, AuditLogPort, AuditOutcome};
pub use git_cas::GitCASPort;
pub use mcp_runtime::MCPRuntimePort;
pub use memory_storage::{EpisodicStoragePort, SemanticStoragePort};
pub use metacognition::{
    BotEvaluationMetrics, BotHealthStatus as MetacognitionBotHealthStatus, CapabilityGap,
    EvaluationResult, GapType, KataDirective, KataType, RecommendedAction,
};
pub use registry_source::RegistrySourcePort;
pub use sovereignty::{SovereigntyCheckResult, SovereigntyOperation};
pub use standing_session::{
    AcpSessionMessage, BotReport, MessageRecord, SessionMessageType, SessionRecord,
    StandingSessionPort, StandingSessionPortError,
};
