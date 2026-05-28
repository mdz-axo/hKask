//! HTTP routes — per-domain modules

mod acp;
mod bots;
mod chat;
mod cns;
mod ensemble;
mod mcp;
mod pods;
mod soap_infer;
mod sovereignty;
mod spec;
mod templates;

// Re-export router functions
pub use acp::acp_router;
pub use bots::bots_router;
pub use chat::chat_router;
pub use cns::cns_router;
pub use ensemble::ensemble_router;
pub use mcp::mcp_router;
pub use pods::pods_router;
pub use soap_infer::soap_infer_router;
pub use soap_infer::validate_soap_request;
pub use sovereignty::sovereignty_router;
pub use spec::spec_router;
pub use templates::templates_router;

// Re-export domain-local types that may be used externally
pub use ensemble::{
    CreateChatRequest, EnsembleResponse, RecordResponseRequest, RegisterBotRequest,
    SendMessageRequest,
};
pub use sovereignty::{
    AccessCheckResponse, KillZoneResponse, SovereigntyConsentResponse, SovereigntyStatusResponse,
};
