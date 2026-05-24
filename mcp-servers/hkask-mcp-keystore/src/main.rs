//! hKask MCP Keystore — OS keychain storage with AES-256-GCM

use hkask_keystore::encryption::EncryptionService;
use hkask_keystore::Keychain;
use hkask_types::WebID;
use rmcp::{
    ServiceExt,
    handler::server::wrapper::Parameters,
    tool, tool_router, transport::stdio,
};
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
    encryption: Arc<RwLock<Option<EncryptionService>>>,
    entries: Arc<RwLock<HashMap<String, EncryptedEntry>>>,
}

impl KeystoreServer {
    pub fn new() -> Self {
        let service_name = std::env::var("HKASK_KEYSTORE_SERVICE")
            .unwrap_or_else(|_| "hkask-keystore".to_string());
        Self {
            keychain: Keychain::new(&service_name),
            encryption: Arc::new(RwLock::new(None)),
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn full_key(service: &Option<String>, key: &str) -> String {
        format!(
            "{}:{}",
            service.as_deref().unwrap_or("default"),
            key
        )
    }

    async fn get_encryption(&self) -> EncryptionService {
        let guard = self.encryption.read().await;
        if let Some(ref enc) = *guard {
            let salt = EncryptionService::generate_salt();
            EncryptionService::new("hkask-mcp-keystore", &salt)
                .expect("encryption service creation should not fail")
        } else {
            drop(guard);
            let salt = EncryptionService::generate_salt();
            let enc = EncryptionService::new("hkask-mcp-keystore", &salt)
                .expect("encryption service creation should not fail");
            let mut guard = self.encryption.write().await;
            *guard = Some(enc);
            guard.as_ref().unwrap().encrypt(b"init").expect("encryption init");
            EncryptionService::new("hkask-mcp-keystore", &salt)
                .expect("encryption service creation should not fail")
        }
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
                        Ok(()) => serde_json::json!({
                            "key": key,
                            "service": service.unwrap_or_else(|| "default".to_string()),
                            "set": true,
                            "encrypted": true,
                        })
                        .to_string(),
                        Err(e) => serde_json::json!({
                            "key": key,
                            "set": false,
                            "error": format!("keychain store failed: {}", e),
                        })
                        .to_string(),
                    }
                }
                Err(e) => serde_json::json!({
                    "key": key,
                    "set": false,
                    "error": format!("encryption failed: {}", e),
                })
                .to_string(),
            },
            Err(e) => serde_json::json!({
                "key": key,
                "set": false,
                "error": format!("encryption service failed: {}", e),
            })
            .to_string(),
        }
    }

    #[tool(description = "Get a value from the keystore (capability-gated: only owner pod can read)")]
    async fn keystore_get(
        &self,
        Parameters(GetRequest {
            key,
            service,
            caller_webid,
        }): Parameters<GetRequest>,
    ) -> String {
        let full_key = Self::full_key(&service, &key);
        let caller = caller_webid.unwrap_or_else(|| "anonymous".to_string());

        let entries = self.entries.read().await;
        match entries.get(&full_key) {
            Some(entry) => {
                if entry.owner_webid != caller && entry.owner_webid != "system" {
                    return serde_json::json!({
                        "key": key,
                        "found": false,
                        "error": format!("access denied: caller {} does not own this secret", caller),
                    })
                    .to_string();
                }

                match EncryptionService::new("hkask-mcp-keystore", &entry.salt) {
                    Ok(enc) => match enc.decrypt(&entry.encrypted) {
                        Ok(plaintext) => {
                            let value = String::from_utf8_lossy(&plaintext).to_string();
                            serde_json::json!({
                                "key": key,
                                "value": value,
                                "found": true,
                                "decrypted": true,
                            })
                            .to_string()
                        }
                        Err(e) => serde_json::json!({
                            "key": key,
                            "found": true,
                            "error": format!("decryption failed: {}", e),
                        })
                        .to_string(),
                    },
                    Err(e) => serde_json::json!({
                        "key": key,
                        "found": true,
                        "error": format!("encryption service failed: {}", e),
                    })
                    .to_string(),
                }
            }
            None => serde_json::json!({
                "key": key,
                "found": false,
            })
            .to_string(),
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
        let full_key = Self::full_key(&service, &key);
        let caller = caller_webid.unwrap_or_else(|| "anonymous".to_string());

        let mut entries = self.entries.write().await;
        match entries.get(&full_key) {
            Some(entry) => {
                if entry.owner_webid != caller && entry.owner_webid != "system" {
                    return serde_json::json!({
                        "key": key,
                        "rotated": false,
                        "error": format!("access denied: caller {} does not own this secret", caller),
                    })
                    .to_string();
                }
            }
            None => {
                return serde_json::json!({
                    "key": key,
                    "rotated": false,
                    "error": "Key not found",
                })
                .to_string();
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
                    serde_json::json!({
                        "key": key,
                        "rotated": true,
                        "re_encrypted": true,
                    })
                    .to_string()
                }
                Err(e) => serde_json::json!({
                    "key": key,
                    "rotated": false,
                    "error": format!("encryption failed: {}", e),
                })
                .to_string(),
            },
            Err(e) => serde_json::json!({
                "key": key,
                "rotated": false,
                "error": format!("encryption service failed: {}", e),
            })
            .to_string(),
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
        let full_key = Self::full_key(&service, &key);
        let caller = caller_webid.unwrap_or_else(|| "anonymous".to_string());

        let mut entries = self.entries.write().await;
        match entries.get(&full_key) {
            Some(entry) => {
                if entry.owner_webid != caller && entry.owner_webid != "system" {
                    return serde_json::json!({
                        "key": key,
                        "deleted": false,
                        "error": format!("access denied: caller {} does not own this secret", caller),
                    })
                    .to_string();
                }
            }
            None => {
                return serde_json::json!({
                    "key": key,
                    "deleted": false,
                    "error": "Key not found",
                })
                .to_string();
            }
        }

        let webid = WebID::from_string(&caller);
        let _ = self.keychain.delete(&webid);

        if entries.remove(&full_key).is_some() {
            serde_json::json!({
                "key": key,
                "deleted": true,
            })
            .to_string()
        } else {
            serde_json::json!({
                "key": key,
                "deleted": false,
                "error": "Key not found",
            })
            .to_string()
        }
    }

    #[tool(description = "List all keys in the keystore")]
    async fn keystore_list(&self) -> String {
        let entries = self.entries.read().await;
        let keys: Vec<&String> = entries.keys().collect();
        serde_json::json!({
            "key_count": keys.len(),
            "keys": keys,
        })
        .to_string()
    }

    #[tool(description = "Prompt for a secret value")]
    async fn keystore_prompt(
        &self,
        Parameters(PromptRequest { prompt_text }): Parameters<PromptRequest>,
    ) -> String {
        serde_json::json!({
            "prompt": prompt_text,
            "status": "prompted",
            "note": "Interactive prompt requires client support",
        })
        .to_string()
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
