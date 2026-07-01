//! Chat service — unified inference, memory integration, and prompt composition.
//!
//! Module structure:
//! - `types` — Request/response structs, token accounting, message sources
//! - `service` — `ChatService` struct and core orchestration methods
//! - `condenser` — Auto-condensation of conversation history
//! - `improv` — Improv mode system prompt generation

mod condenser;
mod improv;
pub mod service;
#[cfg(test)]
mod tests;
pub mod types;

pub use service::ChatService;
pub use types::{
    ChatTurnRequest, ChatTurnResponse, MessageSource, PreparedChat, TokenUsage, TurnRequest,
    TurnResult,
};
