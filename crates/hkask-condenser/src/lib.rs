//! hKask Condenser — Domain logic for context condensation
//!
//! Pure domain crate: compression algorithms, engine state management,
//! prompt formatting, and output construction. No MCP, no HTTP, no async.
//!
//! This crate provides the domain primitives consumed by:
//! - `hkask-services` (ChatService::condense_history — auto-condense)
//! - `hkask-mcp-condenser` (MCP server — thin wrapper exposing tools)

pub mod algorithms;
pub mod engine;
pub mod inference;
pub mod types;

pub use inference::{
    approx_token_count, build_summarization_prompt, build_summary_output, format_conversation_text,
};
