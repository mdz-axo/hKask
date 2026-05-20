//! hKask MCP Server Stub
//!
//! This server is ready for rmcp integration. See tool definitions below.

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    eprintln!("hKask MCP Server v{}", SERVER_VERSION);
    eprintln!("Status: Stub - rmcp #[tool_router] integration pending");
    eprintln!("Tools defined in source - ready for trait implementation");
}
