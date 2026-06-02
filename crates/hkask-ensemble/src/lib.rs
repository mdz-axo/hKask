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
    ChatMessage, ChatParticipant, EnsembleChat, EnsembleError, ParticipantRole, SessionManager,
};
pub use confidence_router::{ConfidenceConfig, compute_confidence};
pub use deliberation::{
    AgentResponse, DeliberationParticipant, DeliberationResult, DeliberationSession,
    DeliberationStatus,
};
pub use improv::{
    ImprovError, ImprovMode, ImprovSessionConfig, ImprovTurn, RelevanceJudgment, SynthesisMode,
    improv_turn,
};
pub use ports::{GenerateOptions, GenerateRequest, SovereigntyPort};
pub use standing_session::{
    StandingSession, StandingSessionConfig, StandingSessionError, StandingSessionStatus,
    bootstrap_standing_session, load_standing_session_config,
};
