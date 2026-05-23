//! Adapter implementations for hexagonal ports
//!
//! This module provides concrete implementations of the port traits
//! defined in the parent module, enabling real-world integration with:
//! - ACP runtime (see crate::acp::AcpRuntime and crate::acp::AcpPort)
//! - ACP transport (stdio, loopback HTTP)
//! - MCP runtime (rmcp crate)
//! - CNS emitter (hkask-cns crate)
//! - Git CAS (gix crate)
//! - Keystore (hkask-keystore crate)
//! - Memory storage (hkask-storage crate)

pub mod cns_emitter;
pub mod git_cas;
pub mod keychain_adapter;
pub mod keystore_port;
pub mod loopback_http_transport;
pub mod mcp_runtime;
pub mod memory_storage;
pub mod stdio_transport;

pub use cns_emitter::CnsEmitterAdapter;
pub use git_cas::{GitCasAdapter, MockGitCas};
pub use keychain_adapter::KeychainAdapter;
pub use keystore_port::{KeystorePort, Secret};
pub use loopback_http_transport::LoopbackHttpTransport;
pub use mcp_runtime::McpRuntimeAdapter;
pub use memory_storage::MemoryStorageAdapter;
pub use stdio_transport::StdioTransport;
