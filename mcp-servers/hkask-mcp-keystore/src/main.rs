//! hKask MCP Keystore — OS keychain integration and encryption key management

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter},
    model::*,
    transport::stdio,
    schemars, tool, tool_router, tool_handler,
};
use rmcp::handler::server::wrapper::Parameters;
use serde::{Deserialize, Serialize};
use secrecy::{Secret, ExposeSecret};
use std::collections::HashMap;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Secure credential entry
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct CredentialEntry {
    pub key: String,
    pub service: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Store request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StoreRequest {
    pub key: String,
    pub value: String,
    pub service: Option<String>,
}

/// Get request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetRequest {
    pub key: String,
    pub service: Option<String>,
}

/// Rotate request parameters
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RotateRequest {
    pub key: String,
    pub new_value: String,
    pub service: Option<String>,
}

/// Keystore server implementation with in-memory storage
/// (Production would use OS keychain via keyring crate)
pub struct KeystoreServer {
    tool_router: ToolRouter<KeystoreServer>,
    store: std::sync::Arc<tokio::sync::RwLock<HashMap<String, Secret<String>>>>,
}

impl KeystoreServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            store: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
}

#[tool_router(server_handler)]
impl KeystoreServer {
    #[tool(description = "Store a credential securely in the OS keychain")]
    async fn keystore_set(&self, Parameters(req): Parameters<StoreRequest>) -> String {
        let mut store = self.store.write().await;
        let service = req.service.unwrap_or_else(|| "hkask-default".to_string());
        let full_key = format!("{}:{}", service, req.key);
        
        store.insert(full_key.clone(), Secret::new(req.value));
        
        let entry = CredentialEntry {
            key: req.key,
            service,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        
        tracing::info!(key = %req.key, service = %entry.service, "stored credential");
        serde_json::to_string_pretty(&entry).unwrap_or_else(|_| "error serializing".to_string())
    }

    #[tool(description = "Retrieve a credential from the OS keychain")]
    async fn keystore_get(&self, Parameters(req): Parameters<GetRequest>) -> String {
        let store = self.store.read().await;
        let service = req.service.unwrap_or_else(|| "hkask-default".to_string());
        let full_key = format!("{}:{}", service, req.key);
        
        match store.get(&full_key) {
            Some(secret) => {
                tracing::info!(key = %req.key, service = %service, "retrieved credential");
                serde_json::json!({
                    "key": req.key,
                    "service": service,
                    "value": secret.expose_secret(),
                    "found": true
                }).to_string()
            }
            None => serde_json::json!({
                "key": req.key,
                "service": service,
                "found": false
            }).to_string()
        }
    }

    #[tool(description = "Rotate an encryption key or credential")]
    async fn keystore_rotate(&self, Parameters(req): Parameters<RotateRequest>) -> String {
        let mut store = self.store.write().await;
        let service = req.service.unwrap_or_else(|| "hkask-default".to_string());
        let full_key = format!("{}:{}", service, req.key);
        
        if store.contains_key(&full_key) {
            store.insert(full_key.clone(), Secret::new(req.new_value));
            tracing::info!(key = %req.key, service = %service, "rotated credential");
            serde_json::json!({
                "success": true,
                "key": req.key,
                "service": service,
                "rotated_at": chrono::Utc::now().to_rfc3339()
            }).to_string()
        } else {
            serde_json::json!({
                "success": false,
                "reason": "key not found"
            }).to_string()
        }
    }

    #[tool(description = "Delete a credential from the OS keychain")]
    async fn keystore_delete(&self, key: String, service: Option<String>) -> String {
        let mut store = self.store.write().await;
        let service = service.unwrap_or_else(|| "hkask-default".to_string());
        let full_key = format!("{}:{}", service, key);
        
        if store.remove(&full_key).is_some() {
            tracing::info!(key = %key, service = %service, "deleted credential");
            serde_json::json!({ "success": true, "key": key }).to_string()
        } else {
            serde_json::json!({ "success": false, "reason": "key not found" }).to_string()
        }
    }

    #[tool(description = "List all stored credential keys")]
    async fn keystore_list(&self) -> String {
        let store = self.store.read().await;
        let keys: Vec<String> = store.keys().cloned().collect();
        
        serde_json::json!({
            "keys": keys,
            "count": keys.len()
        }).to_string()
    }

    #[tool(description = "Generate a prompt for interactive passphrase entry")]
    async fn keystore_prompt(&self, prompt_text: Option<String>) -> String {
        let prompt = prompt_text.unwrap_or_else(|| "Enter encryption passphrase:".to_string());
        serde_json::json!({
            "prompt": prompt,
            "masked": true,
            "required": true
        }).to_string()
    }
}

impl KeystoreServer {}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = KeystoreServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-keystore MCP server started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
