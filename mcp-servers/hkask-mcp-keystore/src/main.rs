//! hKask MCP Keystore — OS keychain storage with AES-256-GCM
//!
//! 6 tools:
//! - `keystore:set` — Set a key-value pair with AES-256-GCM encryption
//! - `keystore:get` — Get a value (capability-gated: only owner pod can read)
//! - `keystore:rotate` — Rotate a key-value pair with re-encryption
//! - `keystore:delete` — Delete a key (capability-gated)
//! - `keystore:list` — List all keys
//! - `keystore:prompt` — Prompt for a secret value

use hkask_keystore::Keychain;
use hkask_keystore::encryption::EncryptionService;
use hkask_mcp::server::{
    CredentialRequirement, McpToolError, McpToolOutput, ServerContext, ToolSpanGuard,
    run_stdio_server,
};
use hkask_types::{McpErrorKind, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetRequest {
    pub key: String,
    pub value: String,
    pub service: Option<String>,
    pub owner_webid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetRequest {
    pub key: String,
    pub service: Option<String>,
    pub caller_webid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RotateRequest {
    pub key: String,
    pub new_value: String,
    pub service: Option<String>,
    pub caller_webid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteRequest {
    pub key: String,
    pub service: Option<String>,
    pub caller_webid: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PromptRequest {
    pub prompt_text: String,
}

struct EncryptedEntry {
    encrypted: Vec<u8>,
    salt: [u8; hkask_keystore::encryption::SALT_SIZE],
    owner_webid: String,
}

pub struct KeystoreServer {
    keychain: Keychain,
    entries: Arc<RwLock<HashMap<String, EncryptedEntry>>>,
    webid: WebID,
}

impl KeystoreServer {
    pub fn new(service_name: &str, webid: WebID) -> Self {
        Self {
            keychain: Keychain::new(service_name),
            entries: Arc::new(RwLock::new(HashMap::new())),
            webid,
        }
    }

    fn full_key(service: &Option<String>, key: &str) -> String {
        format!("{}:{}", service.as_deref().unwrap_or("default"), key)
    }
}

#[tool_router(server_handler)]
impl KeystoreServer {
    #[tool(description = "Set a key-value pair in the keystore with AES-256-GCM encryption")]
    async fn keystore_set(
        &self,
        Parameters(SetRequest {
            key,
            value,
            service,
            owner_webid,
        }): Parameters<SetRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("keystore:set", &self.webid);

        let full_key = Self::full_key(&service, &key);
        let owner = owner_webid.unwrap_or_else(|| "system".to_string());

        let salt = EncryptionService::generate_salt();
        match EncryptionService::new("hkask-mcp-keystore", &salt) {
            Ok(enc) => match enc.encrypt(value.as_bytes()) {
                Ok(encrypted) => {
                    let mut entries = self.entries.write().await;
                    entries.insert(
                        full_key.clone(),
                        EncryptedEntry {
                            encrypted,
                            salt,
                            owner_webid: owner.clone(),
                        },
                    );

                    let webid = WebID::from_string(&owner);
                    match self.keychain.store(&webid, &full_key) {
                        Ok(()) => span.ok(McpToolOutput::new(json!({
                            "key": key,
                            "service": service.unwrap_or_else(|| "default".to_string()),
                            "set": true,
                            "encrypted": true,
                        }))
                        .to_json_string()),
                        Err(e) => span.error(
                            McpErrorKind::Internal,
                            McpToolError::internal(format!("keychain store failed: {}", e))
                                .to_json_string(),
                        ),
                    }
                }
                Err(e) => span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("encryption failed: {}", e)).to_json_string(),
                ),
            },
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("encryption service failed: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(
        description = "Get a value from the keystore (capability-gated: only owner pod can read)"
    )]
    async fn keystore_get(
        &self,
        Parameters(GetRequest {
            key,
            service,
            caller_webid,
        }): Parameters<GetRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("keystore:get", &self.webid);

        let full_key = Self::full_key(&service, &key);
        let caller = caller_webid.unwrap_or_else(|| "anonymous".to_string());

        let entries = self.entries.read().await;
        match entries.get(&full_key) {
            Some(entry) => {
                if entry.owner_webid != caller && entry.owner_webid != "system" {
                    return span.error(
                        McpErrorKind::PermissionDenied,
                        McpToolError::permission_denied(format!(
                            "Caller {} does not own this secret",
                            caller
                        ))
                        .to_json_string(),
                    );
                }

                match EncryptionService::new("hkask-mcp-keystore", &entry.salt) {
                    Ok(enc) => match enc.decrypt(&entry.encrypted) {
                        Ok(plaintext) => {
                            let value = String::from_utf8_lossy(&plaintext).to_string();
                            span.ok(McpToolOutput::new(json!({
                                "key": key,
                                "value": value,
                                "found": true,
                                "decrypted": true,
                            }))
                            .to_json_string())
                        }
                        Err(e) => span.error(
                            McpErrorKind::Internal,
                            McpToolError::internal(format!("decryption failed: {}", e))
                                .to_json_string(),
                        ),
                    },
                    Err(e) => span.error(
                        McpErrorKind::Internal,
                        McpToolError::internal(format!("encryption service failed: {}", e))
                            .to_json_string(),
                    ),
                }
            }
            None => span.error(
                McpErrorKind::NotFound,
                McpToolOutput::new(json!({
                    "key": key,
                    "found": false,
                }))
                .to_json_string(),
            ),
        }
    }

    #[tool(description = "Rotate a key-value pair with re-encryption")]
    async fn keystore_rotate(
        &self,
        Parameters(RotateRequest {
            key,
            new_value,
            service,
            caller_webid,
        }): Parameters<RotateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("keystore:rotate", &self.webid);

        let full_key = Self::full_key(&service, &key);
        let caller = caller_webid.unwrap_or_else(|| "anonymous".to_string());

        let mut entries = self.entries.write().await;
        match entries.get(&full_key) {
            Some(entry) => {
                if entry.owner_webid != caller && entry.owner_webid != "system" {
                    return span.error(
                        McpErrorKind::PermissionDenied,
                        McpToolError::permission_denied(format!(
                            "Caller {} does not own this secret",
                            caller
                        ))
                        .to_json_string(),
                    );
                }
            }
            None => {
                return span.error(
                    McpErrorKind::NotFound,
                    McpToolError::not_found(format!("Key {} not found", key)).to_json_string(),
                );
            }
        }

        let salt = EncryptionService::generate_salt();
        match EncryptionService::new("hkask-mcp-keystore", &salt) {
            Ok(enc) => match enc.encrypt(new_value.as_bytes()) {
                Ok(encrypted) => {
                    let owner = entries.get(&full_key).unwrap().owner_webid.clone();
                    entries.insert(
                        full_key,
                        EncryptedEntry {
                            encrypted,
                            salt,
                            owner_webid: owner,
                        },
                    );
                    span.ok(McpToolOutput::new(json!({
                        "key": key,
                        "rotated": true,
                        "re_encrypted": true,
                    }))
                    .to_json_string())
                }
                Err(e) => span.error(
                    McpErrorKind::Internal,
                    McpToolError::internal(format!("encryption failed: {}", e)).to_json_string(),
                ),
            },
            Err(e) => span.error(
                McpErrorKind::Internal,
                McpToolError::internal(format!("encryption service failed: {}", e))
                    .to_json_string(),
            ),
        }
    }

    #[tool(description = "Delete a key from the keystore (capability-gated)")]
    async fn keystore_delete(
        &self,
        Parameters(DeleteRequest {
            key,
            service,
            caller_webid,
        }): Parameters<DeleteRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("keystore:delete", &self.webid);

        let full_key = Self::full_key(&service, &key);
        let caller = caller_webid.unwrap_or_else(|| "anonymous".to_string());

        let mut entries = self.entries.write().await;
        match entries.get(&full_key) {
            Some(entry) => {
                if entry.owner_webid != caller && entry.owner_webid != "system" {
                    return span.error(
                        McpErrorKind::PermissionDenied,
                        McpToolError::permission_denied(format!(
                            "Caller {} does not own this secret",
                            caller
                        ))
                        .to_json_string(),
                    );
                }
            }
            None => {
                return span.error(
                    McpErrorKind::NotFound,
                    McpToolError::not_found(format!("Key {} not found", key)).to_json_string(),
                );
            }
        }

        let webid = WebID::from_string(&caller);
        let _ = self.keychain.delete(&webid);

        if entries.remove(&full_key).is_some() {
            span.ok(McpToolOutput::new(json!({
                "key": key,
                "deleted": true,
            }))
            .to_json_string())
        } else {
            span.error(
                McpErrorKind::NotFound,
                McpToolError::not_found(format!("Key {} not found", key)).to_json_string(),
            )
        }
    }

    #[tool(description = "List all keys in the keystore")]
    async fn keystore_list(&self) -> String {
        let span = ToolSpanGuard::new("keystore:list", &self.webid);

        let entries = self.entries.read().await;
        let keys: Vec<&String> = entries.keys().collect();

        span.ok(McpToolOutput::new(json!({
            "key_count": keys.len(),
            "keys": keys,
        }))
        .to_json_string())
    }

    #[tool(description = "Prompt for a secret value")]
    async fn keystore_prompt(
        &self,
        Parameters(PromptRequest { prompt_text }): Parameters<PromptRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("keystore:prompt", &self.webid);

        span.ok(McpToolOutput::new(json!({
            "prompt": prompt_text,
            "status": "prompted",
            "note": "Interactive prompt requires client support",
        }))
        .to_json_string())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_stdio_server(
        "hkask-mcp-keystore",
        env!("CARGO_PKG_VERSION"),
        |ctx: ServerContext| {
            let service_name = "hkask-mcp-keystore".to_string();
            Ok(KeystoreServer::new(&service_name, ctx.webid))
        },
        vec![CredentialRequirement::optional(
            "HKASK_KEYSTORE_SERVICE",
            "Service name for OS keychain (default: hkask-mcp-keystore)",
        )],
    )
    .await
}
