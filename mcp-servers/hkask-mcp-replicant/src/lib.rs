//! hkask-mcp-replicant — Replicant chat MCP server
//!
//! Exposes MCP tools:
//! - `replicant:chat` — Send a message to a replicant and receive a response
//! - `replicant:status` — Check replicant registration and identity
//! - `replicant:history` — List recent conversation turns in the current session

pub mod tools;

pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
