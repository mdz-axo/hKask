//! hKask MCP Keystore — Persistent encrypted key-value storage
//!
//! 6 tools:
//! - `keystore:set` — Set a key-value pair with AES-256-GCM encryption
//! - `keystore:get` — Get a value (capability-gated: only owner pod can read)
//! - `keystore:rotate` — Rotate a key-value pair with re-encryption
//! - `keystore:delete` — Delete a key (capability-gated)
//! - `keystore:list` — List all keys
//! - `keystore:prompt` — Prompt for a secret value
//!
//! Persistence: Entries are stored in a file-based vault under
//! `~/.hkask/keystore/` (or `HKASK_KEYSTORE_DIR`). The vault is
//! loaded at startup and saved after each mutation (atomic write).

use hkask_keystore::Keychain;
use hkask_keystore::encryption::EncryptionService;
use hkask_mcp::server::{
    CredentialRequirement, McpToolError, McpToolOutput, ServerContext, ToolSpanGuard,
    run_stdio_server,
};
use hkask_types::{McpErrorKind, WebID};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
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

/// Persistent encrypted entry stored in the vault file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedEntry {
    /// AES-256-GCM encrypted ciphertext (nonce prepended).
    encrypted: Vec<u8>,
    /// Argon2id salt used for key derivation (base64-encoded in JSON).
    #[serde(with = "serde_base64")]
    salt: Vec<u8>,
    /// Owner WebID for OCAP access control.
    owner_webid: String,
    /// Timestamp of last modification (ISO 8601).
    updated_at: String,
}

mod serde_base64 {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(data: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        s.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        base64::engine::general_purpose::STANDARD
            .decode(&s)
            .map_err(serde::de::Error::custom)
    }
}

/// Vault: the on-disk persistence format for all keystore entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Vault {
    /// Schema version for forward compatibility.
    version: u32,
    /// Encrypted entries keyed by `service:key`.
    entries: HashMap<String, EncryptedEntry>,
}

impl Vault {
    const VERSION: u32 = 1;

    #[allow(dead_code)]
    fn new() -> Self {
        Self {
            version: Self::VERSION,
            entries: HashMap::new(),
        }
    }
}

pub struct KeystoreServer {
    keychain: Keychain,
    entries: Arc<RwLock<HashMap<String, EncryptedEntry>>>,
    vault_path: PathBuf,
    webid: WebID,
}

impl KeystoreServer {
    pub fn new(service_name: &str, webid: WebID) -> Self {
        let vault_dir = std::env::var("HKASK_KEYSTORE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(home).join(".hkask").join("keystore")
            });
        let vault_path = vault_dir.join("vault.json");

        let entries = match Self::load_vault(&vault_path) {
            Ok(vault) => {
                tracing::info!(
                    path = %vault_path.display(),
                    count = vault.entries.len(),
                    "Loaded keystore vault from disk"
                );
                vault.entries
            }
            Err(e) => {
                tracing::warn!(
                    path = %vault_path.display(),
                    error = %e,
                    "No existing keystore vault found; starting with empty entries"
                );
                HashMap::new()
            }
        };

        Self {
            keychain: Keychain::new(service_name),
            entries: Arc::new(RwLock::new(entries)),
            vault_path,
            webid,
        }
    }

    fn full_key(service: &Option<String>, key: &str) -> String {
        format!("{}:{}", service.as_deref().unwrap_or("default"), key)
    }

    /// Load vault from disk.
    fn load_vault(path: &PathBuf) -> Result<Vault, String> {
        if !path.exists() {
            return Err("vault file does not exist".to_string());
        }
        let data =
            std::fs::read_to_string(path).map_err(|e| format!("failed to read vault: {}", e))?;
        serde_json::from_str(&data).map_err(|e| format!("failed to parse vault: {}", e))
    }

    /// Save vault to disk. Called after each mutation.
    async fn save_vault(&self) {
        let entries = self.entries.read().await;
        let vault = Vault {
            version: Vault::VERSION,
            entries: entries.clone(),
        };
        drop(entries); // Release read lock before I/O

        // Ensure parent directory exists
        if let Some(parent) = self.vault_path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::error!(path = %parent.display(), error = %e, "Failed to create vault directory");
                return;
            }
        }

        let json = match serde_json::to_string_pretty(&vault) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!(error = %e, "Failed to serialize vault");
                return;
            }
        };

        // Write atomically: temp file + rename
        let temp_path = self.vault_path.with_extension("json.tmp");
        if let Err(e) = std::fs::write(&temp_path, &json) {
            tracing::error!(path = %temp_path.display(), error = %e, "Failed to write vault");
            return;
        }
        if let Err(e) = std::fs::rename(&temp_path, &self.vault_path) {
            tracing::error!(
                from = %temp_path.display(),
                to = %self.vault_path.display(),
                error = %e,
                "Failed to rename vault temp file"
            );
            let _ = std::fs::remove_file(&temp_path);
            return;
        }

        tracing::debug!(path = %self.vault_path.display(), "Vault saved to disk");
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
                            salt: salt.to_vec(),
                            owner_webid: owner.clone(),
                            updated_at: chrono::Utc::now().to_rfc3339(),
                        },
                    );
                    drop(entries); // Release write lock before I/O

                    let webid = WebID::from_string(&owner);
                    match self.keychain.store(&webid, &full_key) {
                        Ok(()) => {
                            self.save_vault().await;
                            span.ok(McpToolOutput::new(json!({
                                "key": key,
                                "service": service.unwrap_or_else(|| "default".to_string()),
                                "set": true,
                                "encrypted": true,
                                "persisted": true,
                            }))
                            .to_json_string())
                        }
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
                            salt: salt.to_vec(),
                            owner_webid: owner,
                            updated_at: chrono::Utc::now().to_rfc3339(),
                        },
                    );
                    drop(entries); // Release write lock before I/O

                    self.save_vault().await;
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
            drop(entries); // Release write lock before I/O
            self.save_vault().await;

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
