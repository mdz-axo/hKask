//! Adapter implementations for hexagonal ports

pub mod git_cas;
pub mod loopback_http_transport;
pub mod mcp_runtime;
pub mod memory_storage;

pub mod registry_source;
pub mod russell_acp;
pub mod standing_session_store;
pub mod stdio_transport;

pub use git_cas::GitCasAdapter;
pub use loopback_http_transport::LoopbackHttpTransport;
pub use mcp_runtime::McpRuntimeAdapter;
pub use memory_storage::MemoryStorageAdapter;
pub use registry_source::FilesystemRegistrySource;
pub use russell_acp::RussellAcpAdapter;
pub use standing_session_store::StandingSessionStoreAdapter;
pub use stdio_transport::StdioTransport;
