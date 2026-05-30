//! Hexagonal Ports (Traits)
//!
//! Port definitions for hexagonal architecture.
//! All port traits live here so that domain code depends only on
//! these abstractions, never on concrete adapters.

pub mod acp;
pub mod acp_transport;
pub mod audit_log;
pub mod cns_query;
pub mod git_cas;
pub mod mcp_runtime;
pub mod memory_storage;
pub mod metacognition;
pub mod registry_source;
pub mod security_port;
pub mod sovereignty;
pub mod standing_session;

pub use acp::AcpPort;
pub use acp_transport::{AcpTransport, AcpWireMessage, AcpWireResponse};
pub use audit_log::{AuditContext, AuditEntry, AuditLogPort, AuditLogPortError, AuditOutcome};
pub use cns_query::{AlertInfo, AlertLevel, HealthStatus};
pub use git_cas::GitCASPort;
pub use mcp_runtime::MCPRuntimePort;
#[allow(deprecated)]
pub use memory_storage::{EpisodicStoragePort, MemoryStoragePort, SemanticStoragePort};
#[allow(deprecated)]
pub use metacognition::{
    BotDirective, BotEvaluationMetrics, BotHealthStatus as MetacognitionBotHealthStatus,
    CapabilityGap, DirectiveType, EvaluationResult, GapType, KataDirective, KataType,
    MetacognitionPortError, RecommendedAction, StoredHealthSnapshot,
};
pub use registry_source::RegistrySourcePort;
pub use security_port::ValidationError as RateLimitValidationError;
pub use sovereignty::{SovereigntyCheckResult, SovereigntyOperation};
pub use standing_session::{
    AcpSessionMessage, BotReport, MessageRecord, SessionMessageType, SessionRecord,
    StandingSessionPort, StandingSessionPortError,
};
