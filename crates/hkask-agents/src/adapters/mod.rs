//! Adapter implementations for hexagonal ports

pub mod mcp_runtime;
pub mod memory_loop_adapter;

pub mod registry_source;

pub use hkask_mcp::TemplateCrateLoader;
pub use mcp_runtime::CapabilityOnlyAdapter;
pub use mcp_runtime::FullMcpAdapter;
pub use memory_loop_adapter::MemoryLoopAdapter;
pub use registry_source::FilesystemRegistrySource;
