//! Adapter implementations for hexagonal ports

pub mod mcp_runtime;
pub mod memory_loop_adapter;

pub mod registry_source;
pub mod russell_acp;

pub use hkask_mcp::GitCasAdapter;
pub use mcp_runtime::McpRuntimeAdapter;
pub use memory_loop_adapter::MemoryLoopAdapter;
pub use registry_source::FilesystemRegistrySource;
pub use russell_acp::RussellAcpAdapter;
