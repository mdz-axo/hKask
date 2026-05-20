//! hKask MCP OCAP — Capability-based access control and delegation

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    schemars, tool, tool_router, tool_handler,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Capability token for OCAP delegation
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CapabilityToken {
    pub id: String,
    pub issuer: String,
    pub subject: String,
    pub capabilities: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Delegation request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DelegateRequest {
    pub issuer: String,
    pub subject: String,
    pub capabilities: Vec<String>,
    pub expires_in_seconds: Option<u64>,
}

/// Verification request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct VerifyRequest {
    pub token_id: String,
    pub capability: String,
}

/// Revocation request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RevokeRequest {
    pub token_id: String,
    pub reason: Option<String>,
}

/// OCAP server implementation
pub struct OcapServer {
    tool_router: ToolRouter<OcapServer>,
    tokens: std::sync::Arc<tokio::sync::RwLock<Vec<CapabilityToken>>>,
}

impl OcapServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            tokens: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
}

#[tool_router]
impl OcapServer {
    #[tool(description = "Create a delegated capability token")]
    async fn ocap_delegate(&self, Parameters(req): Parameters<DelegateRequest>) -> String {
        let token = CapabilityToken {
            id: Uuid::new_v4().to_string(),
            issuer: req.issuer.clone(),
            subject: req.subject.clone(),
            capabilities: req.capabilities.clone(),
            expires_at: req.expires_in_seconds.map(|secs| Utc::now() + chrono::Duration::seconds(secs as i64)),
            created_at: Utc::now(),
        };

        let mut tokens = self.tokens.write().await;
        tokens.push(token.clone());

        serde_json::to_string_pretty(&token).unwrap_or_else(|_| "error serializing token".to_string())
    }

    #[tool(description = "Verify a capability token has a specific capability")]
    async fn ocap_verify(&self, Parameters(req): Parameters<VerifyRequest>) -> String {
        let tokens = self.tokens.read().await;
        let token = tokens.iter().find(|t| t.id == req.token_id);
        
        match token {
            Some(t) => {
                if t.expires_at.map_or(true, |exp| exp > Utc::now()) {
                    let has_cap = t.capabilities.contains(&req.capability);
                    serde_json::json!({
                        "valid": has_cap,
                        "token_id": req.token_id,
                        "capability": req.capability
                    }).to_string()
                } else {
                    serde_json::json!({
                        "valid": false,
                        "reason": "token expired"
                    }).to_string()
                }
            }
            None => serde_json::json!({
                "valid": false,
                "reason": "token not found"
            }).to_string()
        }
    }

    #[tool(description = "Revoke a capability token")]
    async fn ocap_revoke(&self, Parameters(req): Parameters<RevokeRequest>) -> String {
        let mut tokens = self.tokens.write().await;
        let initial_len = tokens.len();
        tokens.retain(|t| t.id != req.token_id);
        
        if tokens.len() < initial_len {
            tracing::info!(token_id = %req.token_id, reason = ?req.reason, "revoked capability token");
            serde_json::json!({ "success": true, "token_id": req.token_id }).to_string()
        } else {
            serde_json::json!({ "success": false, "reason": "token not found" }).to_string()
        }
    }

    #[tool(description = "Enumerate all capabilities held by a subject")]
    async fn ocap_enumerate(&self, subject: String) -> String {
        let tokens = self.tokens.read().await;
        let caps: Vec<&String> = tokens
            .iter()
            .filter(|t| t.subject == subject && t.expires_at.map_or(true, |exp| exp > Utc::now()))
            .flat_map(|t| &t.capabilities)
            .collect();
        
        serde_json::json!({
            "subject": subject,
            "capabilities": caps,
            "count": caps.len()
        }).to_string()
    }

    #[tool(description = "List all active capability tokens")]
    async fn ocap_list_tokens(&self) -> String {
        let tokens = self.tokens.read().await;
        let active: Vec<&CapabilityToken> = tokens
            .iter()
            .filter(|t| t.expires_at.map_or(true, |exp| exp > Utc::now()))
            .collect();
        
        serde_json::to_string_pretty(&serde_json::json!({
            "tokens": active,
            "count": active.len()
        })).unwrap_or_else(|_| "error serializing".to_string())
    }
}

#[tool_handler]
impl ServerHandler for OcapServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = OcapServer::new();
    let service = server.serve_stdio();
    tracing::info!("hkask-mcp-ocap MCP server started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
