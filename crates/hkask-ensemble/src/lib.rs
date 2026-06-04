//! hKask Ensemble — Multi-agent chat coordination
//!
//! Orchestrates conversation between Curator (replicant) and expert bots
//! via template-mediated A2A communication. No swarms, no consensus mechanisms.

pub mod adapters;
pub mod chat;
pub mod confidence_router;
pub mod deliberation;
pub mod improv;
pub mod ports;
pub mod standing_session;

// Re-export commonly used types
pub use adapters::InferencePortAdapter;
pub use chat::{
    ChatMessage, ChatParticipant, DegradationLevel, EnsembleChat, EnsembleError, GasBudgetConfig,
    ParticipantRole, SessionManager,
};
pub use deliberation::{AgentResponse, DeliberationSession};
pub use improv::{ImprovError, ImprovMode, ImprovSessionConfig, ImprovTurn};
pub use ports::{GenerateOptions, GenerateRequest};
pub use standing_session::{
    GasSection, StandingSession, StandingSessionConfig, StandingSessionError,
    StandingSessionStatus, bootstrap_standing_session, bootstrap_standing_session_with_store,
    load_standing_session_config,
};
