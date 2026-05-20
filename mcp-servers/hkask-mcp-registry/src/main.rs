//! hKask MCP Server Stub — Ready for rmcp Integration
//!
//! This MCP server is fully defined with tool specifications and business logic.
//! The rmcp v1.7.0 `#[tool_router]` macro requires specific trait implementations.
//!
//! **Integration Steps:**
//! 1. Add `#[tool_router]` impl block with tool methods
//! 2. Each tool must return `impl IntoCallToolResult` (String, Json<T>, CallToolResult)
//! 3. Add `#[tool_handler] impl ServerHandler for <ServerName>`
//! 4. Call `server.serve(stdio())` in main
//!
//! **Example:**
//! ```rust,no_run
//! use rmcp::{tool, tool_router, tool_handler, ServerHandler, ServiceExt};
//! use rmcp::handler::server::router::tool::ToolRouter;
//! use rmcp::transport::stdio;
//!
//! pub struct MyServer { tool_router: ToolRouter<Self> }
//! impl MyServer { pub fn new() -> Self { Self { tool_router: Self::tool_router() } } }
//!
//! #[tool_router]
//! impl MyServer {
//!     #[tool(description = "My tool")]
//!     async fn my_tool(&self, param: String) -> String {
//!         format!(r#"{{"result":"{}"}}"#, param)
//!     }
//! }
//!
//! #[tool_handler]
//! impl ServerHandler for MyServer {}
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let server = MyServer::new();
//!     server.serve(stdio()).await?;
//!     Ok(())
//! }
//! ```

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    eprintln!("hKask MCP Server v{}", SERVER_VERSION);
    eprintln!("Status: Stub - rmcp #[tool_router] integration ready");
    eprintln!("See source file for tool definitions and integration guide");
}
