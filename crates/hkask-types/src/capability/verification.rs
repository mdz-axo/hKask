//! Verification logic for capability tokens
//
//! Contains `CapabilityChecker` for composition-oriented capability management
//! and `verify_delegation_token` for unified verification with structured outcomes.

// G2 Justification: This module exposes 11 public items because it defines token verification types — CapabilityChecker, VerificationOutcome, verify_delegation_token, and error constants. Each is a distinct verification concern.

use super::{DelegationAction, DelegationResource, DelegationToken};
use crate::WebID;
use ed25519_dalek::SigningKey;

// ── Token error constants (P2.8) ──────────────────────────────────────────
// Centralised here so that all MCP servers and adapters reference the same
// strings, avoiding duplication and drift.

/// Token HMAC/signature verification failed.
pub const TOKEN_ERR_INVALID_SIGNATURE: &str = "Token signature verification failed";
/// Token has expired.
pub const TOKEN_ERR_EXPIRED: &str = "Token is expired";
/// No capability checker was available to validate the token.
pub const TOKEN_ERR_NO_CHECKER: &str = "No capability checker configured";

/// Outcome of verifying a delegation token.
///
/// Provides structured, granular failure modes so call sites can map each
/// failure to a specific error response instead of a generic boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationOutcome {
    /// Token passed all verification checks.
    Valid,
    /// Token signature is invalid or tampered.
    InvalidSignature,
    /// Token has expired.
    Expired,
    /// Token does not grant the requested access.
    InsufficientAccess { resource_id: String, action: String },
    /// No capability checker was provided — access denied.
    NoChecker,
}

/// Capability checker for composition operations.
///
/// [NORMATIVE] With Ed25519 tokens, verification uses the token's own public key —
/// no shared secret is required. The checker validates structural properties
/// (expiry, resource match, holder match) against the token (P4 — Clear Boundaries).
pub struct CapabilityChecker {
    /// Optional Ed25519 signing key for token creation (grant_* methods).
    /// When absent, grant_* methods panic — token issuance requires a signing key.
    signing_key: Option<SigningKey>,
}

impl CapabilityChecker {
    /// Create a new capability checker without a signing key (verify-only).
    /// The `secret` parameter is retained for backward compatibility but unused.
    pub fn new(_secret: &[u8]) -> Self {
        Self { signing_key: None }
    }

    /// Create a capability checker with a signing key for token issuance.
    pub fn with_signing_key(signing_key: SigningKey) -> Self {
        Self {
            signing_key: Some(signing_key),
        }
    }

    /// Verify a capability token's Ed25519 signature.
    pub fn verify(&self, token: &DelegationToken) -> bool {
        token.verify()
    }

    /// Check if token is valid and not expired
    pub fn verify_with_time(&self, token: &DelegationToken, current_time: i64) -> bool {
        self.verify(token) && !token.is_expired(current_time)
    }

    /// Check if a holder has capability for a resource/action
    pub fn check(
        &self,
        token: &DelegationToken,
        holder: &WebID,
        resource: DelegationResource,
        resource_id: &str,
        action: DelegationAction,
    ) -> bool {
        self.verify(token)
            && token.delegated_to == *holder
            && token.is_valid_for(resource, resource_id, action)
    }

    /// Check if holder has any capability for a resource type
    pub fn check_resource(
        &self,
        token: &DelegationToken,
        holder: &WebID,
        resource: DelegationResource,
    ) -> bool {
        self.verify(token) && token.delegated_to == *holder && token.grants_resource(resource)
    }

    /// Create a capability token for a tool.
    /// Requires a signing key — panics if constructed via `new()` instead of `with_signing_key()`.
    pub fn grant_tool(&self, tool_name: String, from: WebID, to: WebID) -> DelegationToken {
        let sk = self.signing_key.as_ref().expect("CapabilityChecker::grant_tool requires a signing key. Use with_signing_key() to construct.");
        DelegationToken::new(
            DelegationResource::Tool,
            tool_name,
            DelegationAction::Execute,
            from,
            to,
            sk,
        )
    }

    /// Create a capability token for a template operation
    pub fn grant_template(
        &self,
        template_id: String,
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        let sk = self
            .signing_key
            .as_ref()
            .expect("CapabilityChecker::grant_template requires a signing key");
        DelegationToken::new(
            DelegationResource::Template,
            template_id,
            action,
            from,
            to,
            sk,
        )
    }

    /// Create a capability token for a manifest operation
    pub fn grant_manifest(
        &self,
        manifest_id: String,
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        let sk = self
            .signing_key
            .as_ref()
            .expect("CapabilityChecker::grant_manifest requires a signing key");
        DelegationToken::new(
            DelegationResource::Registry,
            manifest_id,
            action,
            from,
            to,
            sk,
        )
    }

    /// Create a capability token for registry operations
    pub fn grant_registry(
        &self,
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        let sk = self
            .signing_key
            .as_ref()
            .expect("CapabilityChecker::grant_registry requires a signing key");
        DelegationToken::new(
            DelegationResource::Registry,
            "*".to_string(),
            action,
            from,
            to,
            sk,
        )
    }

    /// Create a capability token for cascade operations
    pub fn grant_cascade(
        &self,
        cascade_id: String,
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        let sk = self
            .signing_key
            .as_ref()
            .expect("CapabilityChecker::grant_cascade requires a signing key");
        DelegationToken::new(
            DelegationResource::Registry,
            cascade_id,
            action,
            from,
            to,
            sk,
        )
    }

    /// Create a capability token for spec operations
    pub fn grant_spec(
        &self,
        spec_id: String,
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        let sk = self
            .signing_key
            .as_ref()
            .expect("CapabilityChecker::grant_spec requires a signing key");
        DelegationToken::new(DelegationResource::Registry, spec_id, action, from, to, sk)
    }

    /// Create an attenuated token for delegation
    pub fn attenuate(
        &self,
        token: &DelegationToken,
        new_to: WebID,
        current_time: i64,
    ) -> Option<DelegationToken> {
        let sk = self.signing_key.as_ref()?;
        token.attenuate(new_to, sk, current_time)
    }
}

/// Verify a delegation token against an optional capability checker.
///
/// Unified verification entry point that produces a structured
/// [`VerificationOutcome`] instead of a bare boolean. Call sites
/// in MCP servers and adapters use this to map each failure mode to
/// a specific error response.
///
/// When `checker` is `None`, returns `VerificationOutcome::NoChecker`.
/// Verify a delegation token using the current system time.
///
/// Equivalent to calling [`verify_delegation_token`] with `current_time` set to
/// the current UNIX epoch timestamp (seconds). Uses `std::time::SystemTime` so
/// no external time dependency is required.
pub fn verify_delegation_token_now(
    checker: Option<&CapabilityChecker>,
    token: &DelegationToken,
    holder: &WebID,
    resource: DelegationResource,
    resource_id: &str,
    action: DelegationAction,
) -> VerificationOutcome {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    verify_delegation_token(
        checker,
        token,
        holder,
        resource,
        resource_id,
        action,
        current_time,
    )
}

pub fn verify_delegation_token(
    checker: Option<&CapabilityChecker>,
    token: &DelegationToken,
    holder: &WebID,
    resource: DelegationResource,
    resource_id: &str,
    action: DelegationAction,
    current_time: i64,
) -> VerificationOutcome {
    let checker = match checker {
        Some(c) => c,
        None => return VerificationOutcome::NoChecker,
    };

    if !checker.verify(token) {
        return VerificationOutcome::InvalidSignature;
    }

    if token.is_expired(current_time) {
        return VerificationOutcome::Expired;
    }

    if !checker.check(token, holder, resource, resource_id, action) {
        return VerificationOutcome::InsufficientAccess {
            resource_id: resource_id.to_string(),
            action: action.as_str().to_string(),
        };
    }

    VerificationOutcome::Valid
}

/// Require write-level access from a delegation token.
///
/// Returns an error string if the token only grants read access.
/// Consolidates the repeated `if token.action == DelegationAction::Read` guard
/// that appeared in `memory_loop_adapter.rs` (4 occurrences) and `pod/context.rs`.
///
/// # Arguments
/// * `token` — The delegation token to check.
/// * `store_type` — Human-readable name of the store being accessed ("episodic" or "semantic").
///   Used in the error message for traceability.
///
/// # Returns
/// * `Ok(())` — Token grants write access.
/// * `Err(String)` — Token is read-only; the error message explains which store was denied.
pub fn require_write_access(token: &DelegationToken, store_type: &str) -> Result<(), String> {
    if token.allows_write() {
        Ok(())
    } else {
        Err(format!(
            "read-only token cannot write to {} storage",
            store_type
        ))
    }
}

/// Require read-level access from a delegation token.
///
/// Returns an error string if the token doesn't grant any read-capable action.
///
/// # Arguments
/// * `token` — The delegation token to check.
/// * `store_type` — Human-readable name of the store being accessed.
pub fn require_read_access(token: &DelegationToken, store_type: &str) -> Result<(), String> {
    if token.allows_read() {
        Ok(())
    } else {
        Err(format!(
            "token does not grant read access for {} recall",
            store_type
        ))
    }
}

// ── Token error message helpers (P2.8) ──────────────────────────────────────
// Thin wrappers around the constants that produce the correct error type
// for each consumer, keeping message text in one place.

/// Format an "insufficient access" error message.
pub fn token_err_insufficient_access(resource_id: &str, action: &str) -> String {
    format!("Token does not authorize access to {resource_id} ({action})")
}

/// Format an "insufficient access for tool" error message.
pub fn token_err_tool_access_denied(tool_name: &str) -> String {
    format!("Token does not authorize tool: {tool_name}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::derive_signing_key;
    use crate::id::WebID;

    // REQ: types-cap-verify-001 — CapabilityChecker::with_signing_key() creates checker with signing key
    #[test]
    fn capability_checker_new_creates_checker() {
        let secret = b"test-secret-32-bytes-long!!";
        let sk = derive_signing_key(secret);
        let checker = CapabilityChecker::with_signing_key(sk);
        // Verify it can verify a token it created
        let from = WebID::from_persona(b"issuer");
        let to = WebID::from_persona(b"holder");
        let token = checker.grant_tool("test_tool".into(), from, to.clone());
        assert!(checker.verify(&token));
    }

    // REQ: types-cap-verify-002 — verify_delegation_token returns NoChecker when checker is None
    #[test]
    fn verify_delegation_token_returns_no_checker_when_none() {
        let from = WebID::from_persona(b"issuer");
        let to = WebID::from_persona(b"holder");
        let sk = SigningKey::from_bytes(&[0x42u8; 32]);
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "test_tool".into(),
            DelegationAction::Execute,
            from,
            to.clone(),
            &sk,
        );
        let outcome = verify_delegation_token(
            None,
            &token,
            &to,
            DelegationResource::Tool,
            "test_tool",
            DelegationAction::Execute,
            i64::MAX, // far future — not expired
        );
        assert_eq!(outcome, VerificationOutcome::NoChecker);
    }

    // REQ: types-cap-verify-003 — require_write_access returns Ok for write tokens
    #[test]
    fn require_write_access_accepts_write_token() {
        let from = WebID::from_persona(b"issuer");
        let to = WebID::from_persona(b"holder");
        let sk = SigningKey::from_bytes(&[0x42u8; 32]);
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "episodic".into(),
            DelegationAction::Write,
            from,
            to,
            &sk,
        );
        assert!(require_write_access(&token, "episodic").is_ok());
    }

    // REQ: types-cap-verify-004 — require_write_access returns Err for read-only tokens
    #[test]
    fn require_write_access_rejects_read_only_token() {
        let from = WebID::from_persona(b"issuer");
        let to = WebID::from_persona(b"holder");
        let sk = SigningKey::from_bytes(&[0x42u8; 32]);
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "episodic".into(),
            DelegationAction::Read,
            from,
            to,
            &sk,
        );
        let result = require_write_access(&token, "episodic");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read-only"));
    }
}
