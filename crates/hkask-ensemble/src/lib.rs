//! hKask Ensemble — Multi-agent chat coordination
//!
//! Orchestrates conversation between Curator (replicant) and expert bots
//! via template-mediated A2A communication. No swarms, no consensus mechanisms.

pub mod adapters;
pub mod capability;
pub mod chat;
pub mod chat_dedup;
pub mod cns_integration;
pub mod cns_spans;
pub mod confidence_router;
pub mod deliberation;
pub mod macaroon;
pub mod metrics;
pub mod multi_okapi;
pub mod ocap_enforcement;
pub mod okapi_integration;
pub mod ports;
pub mod resilience;
pub mod webid_registry;

// Re-export commonly used types
pub use chat::{
    ChatMessage, ChatParticipant, EnsembleChat, EnsembleChatManager, EnsembleError, ParticipantRole,
};
pub use chat_dedup::{DedupStats, SessionDedup, extract_context_window};
pub use cns_integration::{CnsIntegration, CnsIntegrationBuilder};
pub use cns_spans::{OkapiCnsSpan, ValidationResult};
pub use confidence_router::{ConfidenceConfig, ConfidenceRouter, RouterError, compute_confidence};
pub use deliberation::{
    AgentResponse, DeliberationCoordinator, DeliberationRequest, DeliberationResult,
    DeliberationSession, DeliberationStatus,
};
pub use ports::{GenerateOptions, GenerateRequest};
