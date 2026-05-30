//! hKask Ensemble — Multi-agent chat coordination
//!
//! Orchestrates conversation between Curator (replicant) and expert bots
//! via template-mediated A2A communication. No swarms, no consensus mechanisms.

pub mod adapters;
pub mod chat;
pub mod chat_dedup;
pub mod cns_integration;
pub mod cns_spans;
pub mod confidence_router;
pub mod deliberation;
pub mod improv;
pub mod ocap_enforcement;
pub mod okapi_capability;
pub mod okapi_integration;
pub mod ports;
pub mod resilience;
pub mod standing_session;
pub mod webid_registry;

// Re-export commonly used types
#[allow(deprecated)]
pub use adapters::{ImprovClientError, OkapiImprovClient};
#[allow(deprecated)]
pub use adapters::{OkapiClient, OkapiClientError, OkapiHttpClient};
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
pub use improv::{
    ImprovMode, ImprovSessionConfig, ImprovTurn, RelevanceJudgment, SynthesisMode, improv_turn,
};
pub use okapi_capability::{
    OkapiCapabilityError, OkapiOperation, attenuate_for_template, create_okapi_capability,
    create_okapi_capability_for_template, default_system_capability, granted_operations,
    has_operation, is_expired, read_only_capability, verify_okapi_capability,
};
pub use ports::{GenerateOptions, GenerateRequest};
pub use standing_session::{
    StandingSession, StandingSessionConfig, StandingSessionError, StandingSessionStatus,
    bootstrap_standing_session, load_standing_session_config,
};
