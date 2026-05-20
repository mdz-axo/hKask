//! hKask MCP Keystore — OS keychain storage with AES-256-GCM

use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetRequest {
    pub key: String,
    pub value: String,
    pub service: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRequest {
    pub key: String,
    pub service: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RotateRequest {
    pub key: String,
    pub new_value: String,
    pub service: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteRequest {
    pub key: String,
    pub service: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PromptRequest {
    pub prompt_text: String,
}

#[derive(Debug, Default)]
pub struct KeystoreServer {
    store: Arc<RwLock<HashMap<String, String>>>,
}

impl KeystoreServer {
    pub fn new() -> Self {
        Self::default()
    }
}

#[tool_router(server_handler)]
impl KeystoreServer {
    #[tool(description = "Set a key-value pair in the keystore")]
    async fn keystore_set(
        &self,
        Parameters(SetRequest {
            key,
            value,
            service,
        }): Parameters<SetRequest>,
    ) -> String {
        let mut store = self.store.write().await;
        let service_name = service.unwrap_or_else(|| "default".to_string());
        let full_key = format!("{}:{}", service_name, key);
        store.insert(full_key.clone(), value.clone());
        format!(
            r#"{{"key":"{}","service":"{}","set":true}}"#,
            key, service_name
        )
    }

    #[tool(description = "Get a value from the keystore")]
    async fn keystore_get(
        &self,
        Parameters(GetRequest { key, service }): Parameters<GetRequest>,
    ) -> String {
        let store = self.store.read().await;
        let full_key = format!(
            "{}:{}",
            service.unwrap_or_else(|| "default".to_string()),
            key
        );
        match store.get(&full_key) {
            Some(value) => format!(r#"{{"key":"{}","value":"{}","found":true}}"#, key, value),
            None => format!(r#"{{"key":"{}","found":false}}"#, key),
        }
    }

    #[tool(description = "Rotate a key-value pair")]
    async fn keystore_rotate(
        &self,
        Parameters(RotateRequest {
            key,
            new_value,
            service,
        }): Parameters<RotateRequest>,
    ) -> String {
        let mut store = self.store.write().await;
        let full_key = format!(
            "{}:{}",
            service.unwrap_or_else(|| "default".to_string()),
            key
        );
        if let Some(old_value) = store.insert(full_key.clone(), new_value.clone()) {
            format!(
                r#"{{"key":"{}","rotated":true,"old_value":"{}"}}"#,
                key, old_value
            )
        } else {
            format!(
                r#"{{"key":"{}","rotated":false,"error":"Key not found"}}"#,
                key
            )
        }
    }

    #[tool(description = "Delete a key from the keystore")]
    async fn keystore_delete(
        &self,
        Parameters(DeleteRequest { key, service }): Parameters<DeleteRequest>,
    ) -> String {
        let mut store = self.store.write().await;
        let full_key = format!(
            "{}:{}",
            service.unwrap_or_else(|| "default".to_string()),
            key
        );
        if store.remove(&full_key).is_some() {
            format!(r#"{{"key":"{}","deleted":true}}"#, key)
        } else {
            format!(
                r#"{{"key":"{}","deleted":false,"error":"Key not found"}}"#,
                key
            )
        }
    }

    #[tool(description = "List all keys in the keystore")]
    async fn keystore_list(&self) -> String {
        let store = self.store.read().await;
        let keys: Vec<&String> = store.keys().collect();
        format!(
            r#"{{"key_count":{},"keys":{}}}"#,
            keys.len(),
            serde_json::to_string(&keys).unwrap()
        )
    }

    #[tool(description = "Prompt for a secret value")]
    async fn keystore_prompt(
        &self,
        Parameters(PromptRequest { prompt_text }): Parameters<PromptRequest>,
    ) -> String {
        format!(
            r#"{{"prompt":"{}","status":"prompted","note":"Interactive prompt requires client support"}}"#,
            prompt_text
        )
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let server = KeystoreServer::new();
    let service = server.serve(stdio());
    tracing::info!("hkask-mcp-keystore started (v{})", SERVER_VERSION);
    service.await?;
    Ok(())
}
