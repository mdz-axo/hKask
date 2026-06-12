//! hKask MCP Condenser — Library API
//!
//! Pure domain logic for context condensation: compression algorithms,
//! engine state management, prompt formatting, and output construction.
//! Inference is handled by the centralized `InferencePort` (hkask-inference).

pub mod algorithms;
pub mod engine;
pub mod inference;
pub mod types;

pub use inference::{
    approx_token_count, build_summarization_prompt, build_summary_output, format_conversation_text,
};
