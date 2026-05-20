//! hKask Ensemble — Multi-agent chat coordination

pub mod chat;
pub mod confidence_router;
pub mod deliberation;
pub mod ports;

pub use confidence_router::{
    ConfidenceConfig, ConfidenceRouter, GenerateRequest, GenerateOptions,
    OkapiClient, OkapiClientTrait, OkapiResponse, RouterError, TokenProbability, TokenProb,
    compute_confidence,
};
pub use ports::{
    CapabilityProvider, GenerateResponse, InferenceClient, MetricsSource, OkapiCapabilities,
    OkapiMetrics, TokenProb,
};
