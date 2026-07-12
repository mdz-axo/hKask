//! hKask Context Service — AgentService, PerAgentMemory, and loop construction.
//!
//! Extracted from `hkask-services`.

// Used via derive macros (serde/thiserror/async_trait) — invisible to unused_crate_dependencies lint
#![allow(unused_crate_dependencies)]

pub mod cns;
pub mod cns_store_slo_provider;
mod context_impl;
pub mod governance;
pub mod infra;
pub mod mcp_server_guard;
pub mod storage;
pub use context_impl::{AgentService, PerAgentMemory};
