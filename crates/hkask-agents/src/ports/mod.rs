//! Hexagonal Ports (Traits)
//!
//! Port definitions for hexagonal architecture.
//! All port traits live here so that domain code depends only on
//! these abstractions, never on concrete adapters.

pub mod acp;
pub mod acp_transport;
pub mod agent_registry;
pub mod audit_log;
pub mod audit_log_storage;
pub mod cns_query;
pub mod git_cas;
pub mod mcp_runtime;
pub mod memory_storage;
pub mod metacognition;
pub mod ocap_port;
pub mod security_port;
pub mod sovereignty;
pub mod standing_session;

pub use acp::AcpPort;
pub use acp_transport::{AcpTransport, AcpWireMessage, AcpWireResponse};
pub use agent_registry::{AgentRegistryPort, AgentRegistryPortError};
pub use audit_log::{AuditContext, AuditEntry, AuditLogPort, AuditLogPortError, AuditOutcome};
pub use audit_log_storage::AuditLogStoragePortError;
pub use cns_query::{AlertInfo, AlertLevel, CnsQueryPort, HealthStatus};
pub use git_cas::GitCASPort;
pub use mcp_runtime::MCPRuntimePort;
pub use memory_storage::MemoryStoragePort;
pub use metacognition::{
    BotDirective, BotEvaluationMetrics, BotHealthStatus as MetacognitionBotHealthStatus,
    CapabilityGap, DirectiveType, EvaluationResult, GapType, KataDirective, KataType,
    MetacognitionPort, MetacognitionPortError, RecommendedAction, StoredHealthSnapshot,
};
pub use ocap_port::{DelegationEntry, OCAPConfig, OCAPPort, OCAPResult};
pub use security_port::{RateLimitPort, ValidationError as RateLimitValidationError};
pub use sovereignty::{SovereigntyCheckResult, SovereigntyOperation, SovereigntyPort};
pub use standing_session::{
    AcpSessionMessage, BotReport, MessageRecord, SessionMessageType, SessionRecord,
    StandingSessionPort, StandingSessionPortError,
};
