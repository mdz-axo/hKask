//! hkask-mcp-inference — Okapi-backed LLM inference MCP server
//!
//! Exposes 3 MCP tools:
//! - `inference_generate` — Generate text via Okapi LLM
//! - `inference_metrics` — Get current inference metrics
//! - `inference_models` — List available model tiers

pub mod tools;

pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
