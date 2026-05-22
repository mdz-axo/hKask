//! hKask Ensemble — Multi-agent chat coordination
//!
//! Orchestrates conversation between Curator (replicant) and expert bots
//! via template-mediated A2A communication. No swarms, no consensus mechanisms.

pub mod chat;
pub mod deliberation;

// Okapi integration modules
pub mod adapters;
pub mod confidence_router;
pub mod okapi_integration;
pub mod ports;

// Additional modules
pub mod capability;
pub mod cns_spans;
pub mod macaroon;
pub mod metrics;
pub mod multi_okapi;
pub mod ocap_enforcement;
pub mod resilience;
pub mod webid_registry;

// Re-export commonly used types
pub use chat::{
    ChatMessage, ChatParticipant, EnsembleChat, EnsembleChatManager, EnsembleError, ParticipantRole,
};
pub use deliberation::{
    AgentResponse, DeliberationCoordinator, DeliberationRequest, DeliberationResult,
    DeliberationSession, DeliberationStatus,
};
pub use capability::OkapiOperation;
pub use ports::{GenerateOptions, GenerateRequest};
