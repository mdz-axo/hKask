//! hKask Communication — core Matrix transport, agent registry, and 7R7 listener.
//!
//! This is a core infrastructure crate, not an MCP server. It provides:
//! - `MatrixTransport` — matrix-sdk wrapper for Matrix protocol operations
//! - `AgentRegistry` — WebID→UserId mapping and thread watchlists
//! - `SevenR7Listener` — passive Matrix room observer, emits CNS spans
//!
//! The daemon owns the Matrix connection. The REPL, pod activation hooks,
//! and MCP tool surface all use this crate through the daemon.

pub mod agent_registration;
pub mod listener;
pub mod matrix;
