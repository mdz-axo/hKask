//! hKask Ensemble — Multi-agent chat coordination
//!
//! Orchestrates conversation between Curator (replicant) and expert bots
//! via template-mediated A2A communication. No swarms, no consensus mechanisms.

pub mod adapters;
pub mod chat;
pub mod chat_dedup;
pub mod confidence_router;
pub mod deliberation;
pub mod improv;
pub mod ports;
pub mod standing_session;

// Re-export commonly used types
pub use adapters::{CircuitBreakerInferenceAdapter, InferencePortAdapter};
pub use chat::{
    ChatMessage, ChatParticipant, DegradationLevel, EnsembleChat, EnsembleError, GasBudgetConfig,
    ParticipantRole, SessionManager,
};
pub use chat_dedup::{ChatDedup, dedup_messages, message_hash};
pub use confidence_router::{ConfidenceConfig, check_and_escalate, compute_confidence};
pub use deliberation::{AgentResponse, DeliberationSession};
pub use improv::{ImprovError, ImprovMode, ImprovSessionConfig, ImprovTurn};
pub use ports::{GasGovernancePort, GenerateOptions, GenerateRequest};
pub use standing_session::{
    GasSection, StandingSession, StandingSessionConfig, StandingSessionError,
    StandingSessionStatus, bootstrap_standing_session, bootstrap_standing_session_with_store,
    load_standing_session_config,
};
