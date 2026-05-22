//! hKask MCP OCAP — Capability-based access control and delegation

use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DelegateRequest {
    pub issuer: String,
    pub subject: String,
    pub capabilities: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VerifyRequest {
    pub token_id: String,
    pub capability: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RevokeRequest {
    pub token_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnumerateRequest {
    pub subject: String,
}

#[derive(Debug, Default)]
pub struct OcapServer {
    tokens: Arc<RwLock<Vec<String>>>,
}

impl OcapServer {
    pub fn new() -> Self {
        Self::default()
    }
}

#[tool_router(server_handler)]
impl OcapServer {
    #[tool(description = "Create a delegated capability token")]
    async fn ocap_delegate(
        &self,
        Parameters(DelegateRequest {
            issuer,
            subject,
            capabilities,
        }): Parameters<DelegateRequest>,
    ) -> String {
        let mut tokens = self.tokens.write().await;
        let token_id = format!("token_{}", tokens.len());
        tokens.push(token_id.clone());
        format!(
            r#"{{"id":"{}","issuer":"{}","subject":"{}","capabilities":{}}}"#,
            token_id, issuer, subject, capabilities
        )
    }

    #[tool(description = "Verify a capability token")]
    async fn ocap_verify(
        &self,
        Parameters(VerifyRequest {
            token_id,
            capability,
        }): Parameters<VerifyRequest>,
    ) -> String {
        let tokens = self.tokens.read().await;
        let valid = tokens.contains(&token_id);
        format!(
            r#"{{"token_id":"{}","valid":{},"capability":"{}"}}"#,
            token_id, valid, capability
        )
    }

    #[tool(description = "Revoke a capability token")]
    async fn ocap_revoke(
        &self,
        Parameters(RevokeRequest { token_id }): Parameters<RevokeRequest>,
    ) -> String {
        let mut tokens = self.tokens.write().await;
        if let Some(pos) = tokens.iter().position(|t| t == &token_id) {
            tokens.remove(pos);
            format!(r#"{{"token_id":"{}","revoked":true}}"#, token_id)
        } else {
            format!(
                r#"{{"token_id":"{}","revoked":false,"error":"Token not found"}}"#,
                token_id
            )
        }
    }

    #[tool(description = "Enumerate capabilities for a subject")]
    async fn ocap_enumerate(
        &self,
        Parameters(EnumerateRequest { subject }): Parameters<EnumerateRequest>,
    ) -> String {
        let tokens = self.tokens.read().await;
        let count = tokens.len();
        format!(
            r#"{{"subject":"{}","token_count":{},"tokens":{}}}"#,
            subject,
            count,
            serde_json::to_string(&*tokens).unwrap()
        )
    }

    #[tool(description = "List all capability tokens")]
    async fn ocap_list_tokens(&self) -> String {
        let tokens = self.tokens.read().await;
        format!(
            r#"{{"token_count":{},"tokens":{}}}"#,
            tokens.len(),
            serde_json::to_string(&*tokens).unwrap()
        )
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = OcapServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-ocap started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
