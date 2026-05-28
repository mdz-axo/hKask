//! hKask MCP OCAP — Capability-based access control and delegation
//!
//! 5 tools:
//! - `ocap:delegate` — Create a delegated capability token with HMAC signature
//! - `ocap:verify` — Verify a capability token with cryptographic HMAC verification
//! - `ocap:revoke` — Revoke a capability token
//! - `ocap:enumerate` — Enumerate capabilities for a subject
//! - `ocap:list_tokens` — List all capability tokens

use hkask_mcp::server::{
    McpToolError, McpToolOutput, ToolSpanGuard,
    validate_identifier,
};
use hkask_types::{
    CapabilityAction, CapabilityChecker, CapabilityResource, CapabilityToken, McpErrorKind, WebID,
};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use zeroize::Zeroizing;

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
    secret: Zeroizing<Vec<u8>>,
    webid: WebID,
}

impl OcapServer {
    pub fn new(secret: Vec<u8>, webid: WebID) -> Self {
        let checked_secret = Zeroizing::new(secret);
        let checker = CapabilityChecker::new(&checked_secret);
        Self {
            checker,
            tokens: Arc::new(RwLock::new(HashMap::new())),
            revoked: Arc::new(RwLock::new(HashSet::new())),
            secret: checked_secret,
            webid,
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
        let span = ToolSpanGuard::new("ocap:delegate", &self.webid);

        if let Err(e) = validate_identifier("issuer", &issuer, 256) {
            return span.error(e.kind, e.to_json_string());
        }
        if let Err(e) = validate_identifier("subject", &subject, 256) {
            return span.error(e.kind, e.to_json_string());
        }

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

        span.ok(McpToolOutput::new(json!({
            "id": token_id,
            "issuer": issuer_str,
            "subject": holder,
            "capabilities": capabilities,
            "signature_valid": sig_valid,
        }))
        .to_json_string())
    }

    #[tool(description = "Verify a capability token with real cryptographic HMAC verification")]
    async fn ocap_verify(
        &self,
        Parameters(VerifyRequest {
            token_id,
            capability,
        }): Parameters<VerifyRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("ocap:verify", &self.webid);

        if let Err(e) = validate_identifier("token_id", &token_id, 256) {
            return span.error(e.kind, e.to_json_string());
        }

        let tokens = self.tokens.read().await;
        let revoked = self.revoked.read().await;

        match tokens.get(&token_id) {
            Some(token) => {
                if revoked.contains(&token_id) {
                    return span.error(
                        McpErrorKind::FailedPrecondition,
                        McpToolError::failed_precondition(format!(
                            "Token {} has been revoked",
                            token_id
                        ))
                        .to_json_string(),
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
                span.ok(McpToolOutput::new(json!({
                    "token_id": token_id,
                    "valid": valid,
                    "capability": capability,
                    "signature_valid": sig_valid,
                    "not_expired": not_expired,
                    "matches_capability": matches_cap,
                }))
                .to_json_string())
            }
            None => span.error(
                McpErrorKind::NotFound,
                McpToolError::not_found(format!("Token {} not found", token_id)).to_json_string(),
            ),
        }
    }

    #[tool(description = "Revoke a capability token by adding to revocation set")]
    async fn ocap_revoke(
        &self,
        Parameters(RevokeRequest { token_id }): Parameters<RevokeRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("ocap:revoke", &self.webid);

        if let Err(e) = validate_identifier("token_id", &token_id, 256) {
            return span.error(e.kind, e.to_json_string());
        }

        let mut revoked = self.revoked.write().await;
        let tokens = self.tokens.read().await;

        if tokens.contains_key(&token_id) || revoked.contains(&token_id) {
            revoked.insert(token_id.clone());
            span.ok(McpToolOutput::new(json!({
                "token_id": token_id,
                "revoked": true,
            }))
            .to_json_string())
        } else {
            span.error(
                McpErrorKind::NotFound,
                McpToolError::not_found(format!("Token {} not found", token_id)).to_json_string(),
            )
        }
    }

    #[tool(description = "Enumerate capabilities for a subject")]
    async fn ocap_enumerate(
        &self,
        Parameters(EnumerateRequest { subject }): Parameters<EnumerateRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("ocap:enumerate", &self.webid);

        if let Err(e) = validate_identifier("subject", &subject, 256) {
            return span.error(e.kind, e.to_json_string());
        }

        let tokens = self.tokens.read().await;
        let revoked = self.revoked.read().await;
        let subject_webid = WebID::from_string(&subject);

        let matching: Vec<serde_json::Value> = tokens
            .values()
            .filter(|t| t.delegated_to == subject_webid)
            .filter(|t| !revoked.contains(&t.id))
            .map(|t| {
                json!({
                    "id": t.id,
                    "resource": t.resource.as_str(),
                    "resource_id": t.resource_id,
                    "action": t.action.as_str(),
                    "attenuation_level": t.attenuation_level,
                })
            })
            .collect();

        span.ok(McpToolOutput::new(json!({
            "subject": subject,
            "token_count": matching.len(),
            "tokens": matching,
        }))
        .to_json_string())
    }

    #[tool(description = "List all capability tokens")]
    async fn ocap_list_tokens(&self) -> String {
        let span = ToolSpanGuard::new("ocap:list_tokens", &self.webid);

        let tokens = self.tokens.read().await;
        let revoked = self.revoked.read().await;

        let token_list: Vec<serde_json::Value> = tokens
            .values()
            .map(|t| {
                json!({
                    "id": t.id,
                    "resource": t.resource.as_str(),
                    "action": t.action.as_str(),
                    "holder": t.delegated_to.to_string(),
                    "revoked": revoked.contains(&t.id),
                })
            })
            .collect();

        span.ok(McpToolOutput::new(json!({
            "token_count": token_list.len(),
            "tokens": token_list,
        }))
        .to_json_string())
    }
}

hkask_mcp::mcp_server_main!(
    "hkask-mcp-ocap",
    factory: |ctx: hkask_mcp::ServerContext| {
        let secret = ctx
            .credentials
            .get("HKASK_OCAP_SECRET")
            .ok_or_else(|| anyhow::anyhow!(
                "Missing required credential HKASK_OCAP_SECRET. Set it via environment variable or keystore."
            ))?
            .as_bytes()
            .to_vec();
        Ok(OcapServer::new(secret, ctx.webid))
    },
    credentials: vec![hkask_mcp::CredentialRequirement::required(
        "HKASK_OCAP_SECRET",
        "OCAP signing secret for capability token HMAC",
    )]
);
