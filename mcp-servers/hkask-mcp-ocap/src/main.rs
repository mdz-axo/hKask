//! hKask MCP OCAP — Capability-based access control and delegation

use hkask_types::{
    CapabilityAction, CapabilityChecker, CapabilityResource, CapabilityToken, WebID,
};
use rmcp::{ServiceExt, handler::server::wrapper::Parameters, tool, tool_router, transport::stdio};
use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
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

pub struct OcapServer {
    checker: CapabilityChecker,
    tokens: Arc<RwLock<HashMap<String, CapabilityToken>>>,
    revoked: Arc<RwLock<HashSet<String>>>,
    secret: Vec<u8>,
}

impl Default for OcapServer {
    fn default() -> Self {
        Self::new()
    }
}

impl OcapServer {
    pub fn new() -> Self {
        let secret = std::env::var("HKASK_OCAP_SECRET")
            .unwrap_or_else(|_| "hkask-default-ocap-secret-change-me".to_string());
        let secret_bytes = secret.as_bytes().to_vec();
        let checker = CapabilityChecker::new(&secret_bytes);

        Self {
            checker,
            tokens: Arc::new(RwLock::new(HashMap::new())),
            revoked: Arc::new(RwLock::new(HashSet::new())),
            secret: secret_bytes,
        }
    }

    fn parse_resource(cap: &str) -> CapabilityResource {
        match cap.split(':').next() {
            Some("tool") => CapabilityResource::Tool,
            Some("template") => CapabilityResource::Template,
            Some("manifest") => CapabilityResource::Manifest,
            Some("registry") => CapabilityResource::Registry,
            Some("cascade") => CapabilityResource::Cascade,
            Some("spec") => CapabilityResource::Spec,
            _ => CapabilityResource::Tool,
        }
    }

    fn parse_action(cap: &str) -> CapabilityAction {
        let parts: Vec<&str> = cap.split(':').collect();
        match parts.get(1).copied() {
            Some("read") => CapabilityAction::Read,
            Some("write") => CapabilityAction::Write,
            Some("execute") => CapabilityAction::Execute,
            Some("render") => CapabilityAction::Render,
            Some("compose") => CapabilityAction::Compose,
            Some("attenuate") => CapabilityAction::Attenuate,
            Some("validate") => CapabilityAction::Validate,
            _ => CapabilityAction::Execute,
        }
    }
}

#[tool_router(server_handler)]
impl OcapServer {
    #[tool(description = "Create a delegated capability token with real HMAC signature")]
    async fn ocap_delegate(
        &self,
        Parameters(DelegateRequest {
            issuer,
            subject,
            capabilities,
        }): Parameters<DelegateRequest>,
    ) -> String {
        let issuer_webid = WebID::from_string(&issuer);
        let subject_webid = WebID::from_string(&subject);

        let resource = Self::parse_resource(&capabilities);
        let action = Self::parse_action(&capabilities);
        let resource_id = capabilities.clone();

        let token = CapabilityToken::new(
            resource,
            resource_id,
            action,
            issuer_webid,
            subject_webid,
            &self.secret,
        );

        let token_id = token.id.clone();
        let holder = token.delegated_to.to_string();
        let issuer_str = token.delegated_from.to_string();
        let sig_valid = token.verify(&self.secret);

        let mut tokens = self.tokens.write().await;
        tokens.insert(token_id.clone(), token);

        format!(
            r#"{{"id":"{}","issuer":"{}","subject":"{}","capabilities":"{}","signature_valid":{}}}"#,
            token_id, issuer_str, holder, capabilities, sig_valid
        )
    }

    #[tool(description = "Verify a capability token with real cryptographic HMAC verification")]
    async fn ocap_verify(
        &self,
        Parameters(VerifyRequest {
            token_id,
            capability,
        }): Parameters<VerifyRequest>,
    ) -> String {
        let tokens = self.tokens.read().await;
        let revoked = self.revoked.read().await;

        match tokens.get(&token_id) {
            Some(token) => {
                if revoked.contains(&token_id) {
                    return format!(
                        r#"{{"token_id":"{}","valid":false,"capability":"{}","error":"token revoked"}}"#,
                        token_id, capability
                    );
                }

                let sig_valid = self.checker.verify(token);
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                let not_expired = !token.is_expired(current_time);

                let resource = Self::parse_resource(&capability);
                let action = Self::parse_action(&capability);
                let matches_cap = token.is_valid_for(resource, &capability, action);

                let valid = sig_valid && not_expired && matches_cap;
                format!(
                    r#"{{"token_id":"{}","valid":{},"capability":"{}","signature_valid":{},"not_expired":{},"matches_capability":{}}}"#,
                    token_id, valid, capability, sig_valid, not_expired, matches_cap
                )
            }
            None => format!(
                r#"{{"token_id":"{}","valid":false,"capability":"{}","error":"token not found"}}"#,
                token_id, capability
            ),
        }
    }

    #[tool(description = "Revoke a capability token by adding to revocation set")]
    async fn ocap_revoke(
        &self,
        Parameters(RevokeRequest { token_id }): Parameters<RevokeRequest>,
    ) -> String {
        let mut revoked = self.revoked.write().await;
        let tokens = self.tokens.read().await;

        if tokens.contains_key(&token_id) || revoked.contains(&token_id) {
            revoked.insert(token_id.clone());
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
        let revoked = self.revoked.read().await;
        let subject_webid = WebID::from_string(&subject);

        let matching: Vec<serde_json::Value> = tokens
            .values()
            .filter(|t| t.delegated_to == subject_webid)
            .filter(|t| !revoked.contains(&t.id))
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "resource": t.resource.as_str(),
                    "resource_id": t.resource_id,
                    "action": t.action.as_str(),
                    "attenuation_level": t.attenuation_level,
                })
            })
            .collect();

        format!(
            r#"{{"subject":"{}","token_count":{},"tokens":{}}}"#,
            subject,
            matching.len(),
            serde_json::to_string(&matching).unwrap()
        )
    }

    #[tool(description = "List all capability tokens")]
    async fn ocap_list_tokens(&self) -> String {
        let tokens = self.tokens.read().await;
        let revoked = self.revoked.read().await;

        let token_list: Vec<serde_json::Value> = tokens
            .values()
            .map(|t| {
                serde_json::json!({
                    "id": t.id,
                    "resource": t.resource.as_str(),
                    "action": t.action.as_str(),
                    "holder": t.delegated_to.to_string(),
                    "revoked": revoked.contains(&t.id),
                })
            })
            .collect();

        format!(
            r#"{{"token_count":{},"tokens":{}}}"#,
            token_list.len(),
            serde_json::to_string(&token_list).unwrap()
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
