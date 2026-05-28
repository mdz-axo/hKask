//! hKask MCP GML — Allosteric Thinking with MWC model and OCAP enforcement

mod capability;
mod engine;
mod server;
mod types;

pub use capability::CapabilityManager;
pub use engine::MwcEngine;
pub use server::GmlServer;
pub use types::*;

hkask_mcp::mcp_server_main!("hkask-mcp-gml", GmlServer);
