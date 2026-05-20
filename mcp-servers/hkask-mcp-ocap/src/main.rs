//! hKask MCP OCAP — Capability-based access control and delegation

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, tool::ToolRoute},
    model::*,
    transport::stdio,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::borrow::Cow;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct OcapServer {
    tool_router: ToolRouter<Self>,
    tokens: Arc<RwLock<Vec<String>>>,
}

impl OcapServer {
    pub fn new() -> Self {
        let mut tool_router = ToolRouter::new();
        
        tool_router.add_route(ToolRoute::new(
            Tool::new(
                "ocap_delegate",
                "Create a delegated capability token",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "issuer": {"type": "string"},
                        "subject": {"type": "string"},
                        "capabilities": {"type": "string"}
                    },
                    "required": ["issuer", "subject", "capabilities"]
                })
            ),
            |server, ctx| {
                let params = ctx.arguments;
                let issuer = params.get("issuer").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let subject = params.get("subject").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let capabilities = params.get("capabilities").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let tokens = Arc::clone(&server.tokens);
                Box::pin(async move {
                    let mut tokens = tokens.write().await;
                    let token_id = format!("token_{}", tokens.len());
                    tokens.push(token_id.clone());
                    let result = format!(r#"{{"id":"{}","issuer":"{}","subject":"{}","capabilities":{}}}"#, token_id, issuer, subject, capabilities);
                    Ok(CallToolResult::success(vec![Content::text(result)])
                    )
                })
            }
        ));
        
        Self {
            tool_router,
            tokens: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl ServerHandler for OcapServer {
    fn call_tool(&self, req: CallToolRequest) -> impl std::future::Future<Output = Result<CallToolResult, ErrorData>> + Send + 'static {
        self.tool_router.call_tool(req)
    }
    fn list_tools(&self) -> impl std::future::Future<Output = Vec<Tool>> + Send + 'static {
        self.tool_router.list_tools()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let server = OcapServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-ocap started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
