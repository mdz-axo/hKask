//! hKask Chat Service — chat orchestration, memory recall, turn management.
//!
//! Extracted from `hkask-services` (ADR-040, 2026-06-27).

pub mod chat;
pub mod memory;

pub use chat::{
    ChatService, ChatTurnRequest, ChatTurnResponse, PreparedChat, TokenUsage, TurnRequest,
    TurnResult,
};
pub use memory::MemoryService;
