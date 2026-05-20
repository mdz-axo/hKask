//! hKask Ensemble — Multi-agent chat coordination

pub mod chat;
pub mod deliberation;
pub mod confidence_router;

pub use confidence_router::{
    ConfidenceConfig, ConfidenceRouter, GenerateRequest, GenerateOptions,
    OkapiClient, OkapiClientTrait, OkapiResponse, RouterError, TokenProbability, TokenProb,
    compute_confidence,
};
