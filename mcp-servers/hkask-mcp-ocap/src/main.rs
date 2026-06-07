//! hKask MCP OCAP — Capability-based access control and delegation
//!
//! Loop: Cybernetics (Loop 6) — OCAP enforcement is authority governance.
//! Curation *uses* OCAP tokens; this server *enforces* the capability membrane.
//!
//! 5 tools:
//! - `ocap:delegate` — Create a delegated capability token with HMAC signature
//! - `ocap:verify` — Verify a capability token with cryptographic HMAC verification
//! - `ocap:revoke` — Revoke a capability token
//! - `ocap:enumerate` — Enumerate capabilities for a subject
//! - `ocap:list_tokens` — List all capability tokens

use hkask_mcp::server::{McpToolError, ToolSpanGuard};
use hkask_mcp::validate_field;
use hkask_types::{
    CapabilityChecker, CapabilitySpec, DelegationAction, DelegationResource, DelegationToken,
    McpErrorKind, WebID,
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
    tokens: Arc<RwLock<HashMap<String, DelegationToken>>>,
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

    /// Parse a capability string using the canonical [`CapabilitySpec`] parser.
    ///
    /// Returns a tuple of (resource, resource_id, action) suitable for
    /// constructing or verifying `DelegationToken`s.
    ///
    /// This replaces the previous ad-hoc `parse_resource`/`parse_action` pair
    /// with a single canonical parser shared across the entire codebase.
    fn parse_capability(
        cap: &str,
    ) -> Result<(DelegationResource, String, DelegationAction), String> {
        let spec = CapabilitySpec::parse(cap).map_err(|e| e.to_string())?;
        Ok((spec.resource, spec.resource_id, spec.action))
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
        let span = ToolSpanGuard::new("ocap_delegate", &self.webid);

        validate_field!(span, "issuer", &issuer, 256);
        validate_field!(span, "subject", &subject, 256);

        let issuer_webid: WebID = issuer.parse().unwrap_or_else(|_| WebID::new());
        let subject_webid: WebID = subject.parse().unwrap_or_else(|_| WebID::new());

        let (resource, resource_id, action) = match Self::parse_capability(&capabilities) {
            Ok(spec) => spec,
            Err(e) => {
                return span.error(
                    McpErrorKind::InvalidArgument,
                    McpToolError::invalid_argument(e).to_json_string(),
                );
            }
        };

        let token = DelegationToken::new(
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
        let revoked = self.revoked.read().await;

        // F-SYN-006: refuse to (re-)mint a token whose id is already
        // in the revocation set. Because the token id is a
        // *deterministic hash* of (resource, resource_id, action,
        // issuer, subject), re-minting with the same parameters
        // would produce the same id; the revocation log must
        // take precedence over the issuance path, otherwise an
        // attacker can trivially bypass revocation by re-issuing
        // identical tokens.
        if revoked.contains(&token_id) {
            return span.error(
                McpErrorKind::FailedPrecondition,
                McpToolError::failed_precondition(format!(
                    "Token {token_id} has been revoked; re-issuance is not permitted"
                ))
                .to_json_string(),
            );
        }

        tokens.insert(token_id.clone(), token);

        span.ok_json(json!({
            "id": token_id,
            "issuer": issuer_str,
            "subject": holder,
            "capabilities": capabilities,
            "signature_valid": sig_valid,
        }))
    }

    #[tool(description = "Verify a capability token with real cryptographic HMAC verification")]
    async fn ocap_verify(
        &self,
        Parameters(VerifyRequest {
            token_id,
            capability,
        }): Parameters<VerifyRequest>,
    ) -> String {
        let span = ToolSpanGuard::new("ocap_verify", &self.webid);

        validate_field!(span, "token_id", &token_id, 256);

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

                let (resource, resource_id, action) = match Self::parse_capability(&capability) {
                    Ok(spec) => spec,
                    Err(e) => {
                        return span.error(
                            McpErrorKind::InvalidArgument,
                            McpToolError::invalid_argument(e).to_json_string(),
                        );
                    }
                };
                let matches_cap = token.is_valid_for(resource, &resource_id, action);

                let valid = sig_valid && not_expired && matches_cap;
                span.ok_json(json!({
                    "token_id": token_id,
                    "valid": valid,
                    "capability": capability,
                    "signature_valid": sig_valid,
                    "not_expired": not_expired,
                    "matches_capability": matches_cap,
                }))
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
        let span = ToolSpanGuard::new("ocap_revoke", &self.webid);

        validate_field!(span, "token_id", &token_id, 256);

        let mut revoked = self.revoked.write().await;
        let tokens = self.tokens.read().await;

        if tokens.contains_key(&token_id) || revoked.contains(&token_id) {
            revoked.insert(token_id.clone());
            span.ok_json(json!({
                "token_id": token_id,
                "revoked": true,
            }))
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
        let span = ToolSpanGuard::new("ocap_enumerate", &self.webid);

        validate_field!(span, "subject", &subject, 256);

        let tokens = self.tokens.read().await;
        let revoked = self.revoked.read().await;
        let subject_webid: WebID = subject.parse().unwrap_or_else(|_| WebID::new());

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

        span.ok_json(json!({
            "subject": subject,
            "token_count": matching.len(),
            "tokens": matching,
        }))
    }

    #[tool(description = "List all capability tokens")]
    async fn ocap_list_tokens(&self) -> String {
        let span = ToolSpanGuard::new("ocap_list_tokens", &self.webid);

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

        span.ok_json(json!({
            "token_count": token_list.len(),
            "tokens": token_list,
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    hkask_mcp::run_server(
        "hkask-mcp-ocap",
        env!("CARGO_PKG_VERSION"),
        |ctx: hkask_mcp::ServerContext| {
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
        vec![hkask_mcp::CredentialRequirement::required(
            "HKASK_OCAP_SECRET",
            "OCAP signing secret for capability token HMAC",
        )],
    )
    .await
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::DelegationAction;

    fn test_server() -> OcapServer {
        let secret = b"test-ocap-secret-key-for-testing!!";
        let webid = WebID::new();
        OcapServer::new(secret.to_vec(), webid)
    }

    // ── parse_capability ──────────────────────────────────────────────

    // P8 invariant: parse_capability parses 2-part capability strings
    #[test]
    fn parse_capability_two_parts() {
        let (resource, resource_id, action) =
            OcapServer::parse_capability("tool:call").expect("parse tool:call");
        assert_eq!(resource, DelegationResource::Tool);
        assert_eq!(resource_id, "tool:call");
        assert_eq!(action, DelegationAction::Execute);
    }

    // P8 invariant: parse_capability parses 3-part capability strings
    #[test]
    fn parse_capability_three_parts() {
        let (resource, resource_id, action) =
            OcapServer::parse_capability("registry:my-domain:write")
                .expect("parse registry:my-domain:write");
        assert_eq!(resource, DelegationResource::Registry);
        assert_eq!(resource_id, "my-domain");
        assert_eq!(action, DelegationAction::Write);
    }

    // P8 invariant: parse_capability rejects single-part strings
    #[test]
    fn parse_capability_rejects_single_part() {
        let result = OcapServer::parse_capability("invalid");
        assert!(result.is_err(), "single-part string must fail");
    }

    // P8 invariant: parse_capability rejects unknown resource types
    #[test]
    fn parse_capability_rejects_unknown_resource() {
        let result = OcapServer::parse_capability("unknown:action");
        assert!(result.is_err(), "unknown resource type must fail");
    }

    // P8 invariant: parse_capability accepts 'memory' as alias for registry
    #[test]
    fn parse_capability_memory_alias() {
        let (resource, _, _) =
            OcapServer::parse_capability("memory:read").expect("parse memory:read");
        assert_eq!(
            resource,
            DelegationResource::Registry,
            "'memory' prefix must map to Registry"
        );
    }

    // P8 invariant: parse_capability maps known actions
    #[test]
    fn parse_capability_known_actions() {
        let (_, _, action) = OcapServer::parse_capability("tool:read").expect("read");
        assert_eq!(action, DelegationAction::Read);

        let (_, _, action) = OcapServer::parse_capability("tool:write").expect("write");
        assert_eq!(action, DelegationAction::Write);

        let (_, _, action) = OcapServer::parse_capability("tool:execute").expect("execute");
        assert_eq!(action, DelegationAction::Execute);
    }

    // ── ocap_delegate ─────────────────────────────────────────────────

    // P8 invariant: ocap_delegate creates a valid signed token
    #[tokio::test]
    async fn delegate_creates_valid_signed_token() {
        let server = test_server();
        let issuer = WebID::new().to_string();
        let subject = WebID::new().to_string();

        let result = server
            .ocap_delegate(Parameters(DelegateRequest {
                issuer: issuer.clone(),
                subject: subject.clone(),
                capabilities: "tool:inference:call".to_string(),
            }))
            .await;

        assert!(
            result.contains("signature_valid") && result.contains("true"),
            "delegated token must have valid signature, got: {result}"
        );
        assert!(
            result.contains("id"),
            "result must contain token id, got: {result}"
        );
        assert!(
            result.contains("issuer"),
            "result must contain issuer, got: {result}"
        );
    }

    // P8 invariant: ocap_delegate rejects empty issuer
    #[tokio::test]
    async fn delegate_rejects_empty_issuer() {
        let server = test_server();

        let result = server
            .ocap_delegate(Parameters(DelegateRequest {
                issuer: "".to_string(),
                subject: WebID::new().to_string(),
                capabilities: "tool:call".to_string(),
            }))
            .await;

        assert!(
            result.contains("invalid_argument") || result.contains("InvalidArgument"),
            "empty issuer must produce invalid_argument, got: {result}"
        );
    }

    // P8 invariant: ocap_delegate rejects invalid capability string
    #[tokio::test]
    async fn delegate_rejects_invalid_capability() {
        let server = test_server();

        let result = server
            .ocap_delegate(Parameters(DelegateRequest {
                issuer: WebID::new().to_string(),
                subject: WebID::new().to_string(),
                capabilities: "invalid".to_string(),
            }))
            .await;

        assert!(
            result.contains("invalid_argument") || result.contains("InvalidFormat"),
            "invalid capability must produce invalid_argument, got: {result}"
        );
    }

    // ── ocap_verify ───────────────────────────────────────────────────

    // P8 invariant: ocap_verify validates a valid token
    #[tokio::test]
    async fn verify_valid_token() {
        let server = test_server();
        let subject = WebID::new();
        let subject_str = subject.to_string();

        // First, delegate a token
        let delegate_result = server
            .ocap_delegate(Parameters(DelegateRequest {
                issuer: WebID::new().to_string(),
                subject: subject_str.clone(),
                capabilities: "tool:my-tool:execute".to_string(),
            }))
            .await;

        // Extract token_id from result
        let token_id = extract_json_string(&delegate_result, "id");
        assert!(token_id.is_some(), "delegate must return token id");

        let verify_result = server
            .ocap_verify(Parameters(VerifyRequest {
                token_id: token_id.unwrap(),
                capability: "tool:my-tool:execute".to_string(),
            }))
            .await;

        assert!(
            verify_result.contains("signature_valid") && verify_result.contains("true"),
            "verify must confirm valid signature, got: {verify_result}"
        );
        assert!(
            verify_result.contains("valid") && verify_result.contains("true"),
            "valid token must verify as true, got: {verify_result}"
        );
    }

    // P8 invariant: ocap_verify returns not_found for unknown token
    #[tokio::test]
    async fn verify_returns_not_found_for_unknown_token() {
        let server = test_server();

        let result = server
            .ocap_verify(Parameters(VerifyRequest {
                token_id: "nonexistent-token".to_string(),
                capability: "tool:call".to_string(),
            }))
            .await;

        assert!(
            result.contains("not_found") || result.contains("not found"),
            "unknown token must return not_found, got: {result}"
        );
    }

    // ── ocap_revoke ────────────────────────────────────────────────────

    // P8 invariant: ocap_revoke revokes a previously created token
    #[tokio::test]
    async fn revoke_marks_token_as_revoked() {
        let server = test_server();

        let delegate_result = server
            .ocap_delegate(Parameters(DelegateRequest {
                issuer: WebID::new().to_string(),
                subject: WebID::new().to_string(),
                capabilities: "tool:call".to_string(),
            }))
            .await;

        let token_id = extract_json_string(&delegate_result, "id").unwrap();

        let revoke_result = server
            .ocap_revoke(Parameters(RevokeRequest {
                token_id: token_id.clone(),
            }))
            .await;

        assert!(
            revoke_result.contains("revoked") && revoke_result.contains("true"),
            "revoke must confirm revocation, got: {revoke_result}"
        );

        // Verify the revoked token is rejected
        let verify_result = server
            .ocap_verify(Parameters(VerifyRequest {
                token_id,
                capability: "tool:call".to_string(),
            }))
            .await;

        assert!(
            verify_result.contains("revoked") || verify_result.contains("FailedPrecondition"),
            "revoked token must be rejected, got: {verify_result}"
        );
    }

    // F-SYN-006 red→green: a revoked token cannot be re-issued.
    //
    // Because the token id is a *deterministic hash* of the
    // (resource, resource_id, action, issuer, subject) tuple,
    // re-minting with identical parameters produces the same id.
    // Without the F-SYN-006 check, the re-mint would silently
    // overwrite the revoked entry in the tokens map, bypassing
    // revocation. With the check, the re-mint returns
    // `FailedPrecondition`.
    #[tokio::test]
    async fn re_mint_after_revoke_is_rejected() {
        let server = test_server();
        let issuer = WebID::new().to_string();
        let subject = WebID::new().to_string();
        let capability = "tool:call".to_string();

        // Mint a token.
        let delegate_result = server
            .ocap_delegate(Parameters(DelegateRequest {
                issuer: issuer.clone(),
                subject: subject.clone(),
                capabilities: capability.clone(),
            }))
            .await;
        let token_id = extract_json_string(&delegate_result, "id").unwrap();

        // Revoke it.
        let revoke_result = server
            .ocap_revoke(Parameters(RevokeRequest {
                token_id: token_id.clone(),
            }))
            .await;
        assert!(
            revoke_result.contains("revoked") && revoke_result.contains("true"),
            "first revoke must succeed, got: {revoke_result}"
        );

        // Re-mint with identical parameters. The id is the same
        // (deterministic hash), so this would silently overwrite
        // the revoked entry without the F-SYN-006 check.
        let re_mint_result = server
            .ocap_delegate(Parameters(DelegateRequest {
                issuer: issuer.clone(),
                subject: subject.clone(),
                capabilities: capability.clone(),
            }))
            .await;
        assert!(
            re_mint_result.contains("FailedPrecondition")
                || re_mint_result.contains("revoked")
                || re_mint_result.contains("re-issuance is not permitted"),
            "F-SYN-006: re-mint after revoke must be rejected, got: {re_mint_result}"
        );

        // The verify path still rejects the revoked id.
        let verify_result = server
            .ocap_verify(Parameters(VerifyRequest {
                token_id: token_id.clone(),
                capability: capability.clone(),
            }))
            .await;
        assert!(
            verify_result.contains("revoked") || verify_result.contains("FailedPrecondition"),
            "F-SYN-006: verify still rejects the revoked id after re-mint attempt, got: {verify_result}"
        );
    }

    // P8 invariant: ocap_revoke returns not_found for unknown token
    #[tokio::test]
    async fn revoke_returns_not_found_for_unknown_token() {
        let server = test_server();

        let result = server
            .ocap_revoke(Parameters(RevokeRequest {
                token_id: "nonexistent".to_string(),
            }))
            .await;

        assert!(
            result.contains("not_found") || result.contains("not found"),
            "revoking unknown token must return not_found, got: {result}"
        );
    }

    // ── ocap_enumerate ────────────────────────────────────────────────

    // P8 invariant: ocap_enumerate returns tokens for a subject
    #[tokio::test]
    async fn enumerate_returns_tokens_for_subject() {
        let server = test_server();
        let subject = WebID::new().to_string();

        // Create two tokens for the same subject
        server
            .ocap_delegate(Parameters(DelegateRequest {
                issuer: WebID::new().to_string(),
                subject: subject.clone(),
                capabilities: "tool:call".to_string(),
            }))
            .await;
        server
            .ocap_delegate(Parameters(DelegateRequest {
                issuer: WebID::new().to_string(),
                subject: subject.clone(),
                capabilities: "template:render".to_string(),
            }))
            .await;

        let result = server
            .ocap_enumerate(Parameters(EnumerateRequest {
                subject: subject.clone(),
            }))
            .await;

        assert!(
            result.contains("token_count") && result.contains("2"),
            "enumerate must return 2 tokens, got: {result}"
        );
    }

    // P8 invariant: ocap_enumerate excludes revoked tokens
    #[tokio::test]
    async fn enumerate_excludes_revoked_tokens() {
        let server = test_server();
        let subject = WebID::new().to_string();

        let delegate1 = server
            .ocap_delegate(Parameters(DelegateRequest {
                issuer: WebID::new().to_string(),
                subject: subject.clone(),
                capabilities: "tool:call".to_string(),
            }))
            .await;
        let _delegate2 = server
            .ocap_delegate(Parameters(DelegateRequest {
                issuer: WebID::new().to_string(),
                subject: subject.clone(),
                capabilities: "tool:execute".to_string(),
            }))
            .await;

        // Revoke the first token
        let token_id = extract_json_string(&delegate1, "id").unwrap();
        server
            .ocap_revoke(Parameters(RevokeRequest { token_id }))
            .await;

        let result = server
            .ocap_enumerate(Parameters(EnumerateRequest {
                subject: subject.clone(),
            }))
            .await;

        // After revoking one token, enumerate should return 1 active token
        assert!(
            result.contains("token_count") && result.contains("1"),
            "enumerate must exclude revoked tokens, got: {result}"
        );
    }

    // ── ocap_list_tokens ──────────────────────────────────────────────

    // P8 invariant: ocap_list_tokens returns all tokens with revocation status
    #[tokio::test]
    async fn list_tokens_returns_all_with_status() {
        let server = test_server();

        let delegate1 = server
            .ocap_delegate(Parameters(DelegateRequest {
                issuer: WebID::new().to_string(),
                subject: WebID::new().to_string(),
                capabilities: "tool:call".to_string(),
            }))
            .await;

        let token_id = extract_json_string(&delegate1, "id").unwrap();

        // List before revoke — should show revoked: false
        let list_before = server.ocap_list_tokens().await;
        assert!(
            list_before.contains("token_count") && list_before.contains("1"),
            "list must show 1 token before revoke, got: {list_before}"
        );

        // Revoke the token
        server
            .ocap_revoke(Parameters(RevokeRequest { token_id }))
            .await;

        // List after revoke — should still show the token but with revoked: true
        let list_after = server.ocap_list_tokens().await;
        assert!(
            list_after.contains("token_count") && list_after.contains("1"),
            "list must still show the token after revoke, got: {list_after}"
        );
        assert!(
            list_after.contains("revoked") && list_after.contains("true"),
            "revoked token must show revoked:true, got: {list_after}"
        );
    }

    // ── Helper ────────────────────────────────────────────────────────

    /// Extract a string value from a JSON response for a given key.
    /// Handles both `"key": "value"` and `"key":"value"` formats.
    fn extract_json_string(response: &str, key: &str) -> Option<String> {
        // Try with space after colon first, then without
        for sep in [": \"", ":\""] {
            let search = format!("\"{}\"{}", key, sep);
            if let Some(start) = response.find(&search) {
                let value_start = start + search.len();
                if let Some(end) = response[value_start..].find('\"') {
                    return Some(response[value_start..value_start + end].to_string());
                }
            }
        }
        None
    }
}
