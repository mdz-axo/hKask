//! hKask Ensemble — Multi-agent chat coordination

pub mod chat;
pub mod deliberation;

// Okapi integration modules
pub mod adapters;
pub mod ports;
pub mod confidence_router;
pub mod okapi_integration;

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
pub use capability::OkapiOperation;
pub use ports::{GenerateOptions, GenerateRequest};
