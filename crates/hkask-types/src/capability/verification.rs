//! Verification logic for capability tokens
//
//! Contains `CapabilityChecker` for composition-oriented capability management
//! and `verify_delegation_token` for unified verification with structured outcomes.

use super::{DelegationAction, DelegationResource, DelegationToken};
use crate::WebID;
use zeroize::Zeroizing;

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

/// Capability checker for composition operations
pub struct CapabilityChecker {
    secret: Zeroizing<Vec<u8>>,
}

impl CapabilityChecker {
    /// Create a new capability checker with the given secret
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: Zeroizing::new(secret.to_vec()),
        }
    }

    /// Verify a capability token
    pub fn verify(&self, token: &DelegationToken) -> bool {
        token.verify(&self.secret)
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

    /// Create a capability token for a tool
    pub fn grant_tool(&self, tool_name: String, from: WebID, to: WebID) -> DelegationToken {
        DelegationToken::new(
            DelegationResource::Tool,
            tool_name,
            DelegationAction::Execute,
            from,
            to,
            &self.secret,
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
        DelegationToken::new(
            DelegationResource::Template,
            template_id,
            action,
            from,
            to,
            &self.secret,
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
        DelegationToken::new(
            DelegationResource::Registry,
            manifest_id,
            action,
            from,
            to,
            &self.secret,
        )
    }

    /// Create a capability token for registry operations
    pub fn grant_registry(
        &self,
        action: DelegationAction,
        from: WebID,
        to: WebID,
    ) -> DelegationToken {
        DelegationToken::new(
            DelegationResource::Registry,
            "*".to_string(),
            action,
            from,
            to,
            &self.secret,
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
        DelegationToken::new(
            DelegationResource::Registry,
            cascade_id,
            action,
            from,
            to,
            &self.secret,
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
        DelegationToken::new(
            DelegationResource::Registry,
            spec_id,
            action,
            from,
            to,
            &self.secret,
        )
    }

    /// Create an attenuated token for delegation
    pub fn attenuate(
        &self,
        token: &DelegationToken,
        new_to: WebID,
        current_time: i64,
    ) -> Option<DelegationToken> {
        token.attenuate(new_to, &self.secret, current_time)
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
    use crate::{DelegationAction, DelegationResource};

    const SECRET: &[u8] = b"test-secret-for-verification-tests";

    fn alice() -> WebID {
        WebID::from_persona(b"alice")
    }

    fn bob() -> WebID {
        WebID::from_persona(b"bob")
    }

    // ── CapabilityChecker ────────────────────────────────────────────────

    #[test]
    fn checker_new_creates_checker() {
        let _checker = CapabilityChecker::new(SECRET);
        // Construction itself is the invariant: no panic
    }

    #[test]
    fn checker_verify_true_with_same_secret() {
        let checker = CapabilityChecker::new(SECRET);
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );
        assert!(
            checker.verify(&token),
            "checker must verify token created with same secret"
        );
    }

    #[test]
    fn checker_verify_false_with_different_secret() {
        let checker = CapabilityChecker::new(b"different-secret");
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );
        assert!(
            !checker.verify(&token),
            "checker must reject token created with different secret"
        );
    }

    #[test]
    fn checker_verify_with_time_false_for_expired_token() {
        let checker = CapabilityChecker::new(SECRET);
        let token = crate::DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .expires_at(1000)
        .sign(SECRET);

        assert!(
            !checker.verify_with_time(&token, 1001),
            "verify_with_time must reject expired token"
        );
    }

    #[test]
    fn checker_verify_with_time_true_for_unexpired_token() {
        let checker = CapabilityChecker::new(SECRET);
        let token = crate::DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .expires_at(1000)
        .sign(SECRET);

        assert!(
            checker.verify_with_time(&token, 999),
            "verify_with_time must accept unexpired token"
        );
    }

    #[test]
    fn checker_check_true_when_all_fields_match() {
        let checker = CapabilityChecker::new(SECRET);
        let token = checker.grant_tool("inference".to_string(), alice(), bob());

        assert!(
            checker.check(
                &token,
                &bob(),
                DelegationResource::Tool,
                "inference",
                DelegationAction::Execute,
            ),
            "check must return true when holder, resource, resource_id, action all match"
        );
    }

    #[test]
    fn checker_check_false_when_holder_mismatches() {
        let checker = CapabilityChecker::new(SECRET);
        let token = checker.grant_tool("inference".to_string(), alice(), bob());

        assert!(
            !checker.check(
                &token,
                &alice(), // holder ≠ delegated_to
                DelegationResource::Tool,
                "inference",
                DelegationAction::Execute,
            ),
            "check must return false when holder doesn't match delegated_to"
        );
    }

    #[test]
    fn checker_check_false_when_resource_mismatches() {
        let checker = CapabilityChecker::new(SECRET);
        let token = checker.grant_tool("inference".to_string(), alice(), bob());

        assert!(
            !checker.check(
                &token,
                &bob(),
                DelegationResource::Template, // wrong resource
                "inference",
                DelegationAction::Execute,
            ),
            "check must return false when resource doesn't match"
        );
    }

    #[test]
    fn checker_check_resource_true_when_resource_matches() {
        let checker = CapabilityChecker::new(SECRET);
        let token = checker.grant_tool("inference".to_string(), alice(), bob());

        assert!(
            checker.check_resource(&token, &bob(), DelegationResource::Tool),
            "check_resource must return true when resource type matches"
        );
    }

    #[test]
    fn checker_check_resource_false_when_resource_mismatches() {
        let checker = CapabilityChecker::new(SECRET);
        let token = checker.grant_tool("inference".to_string(), alice(), bob());

        assert!(
            !checker.check_resource(&token, &bob(), DelegationResource::Registry),
            "check_resource must return false when resource type doesn't match"
        );
    }

    // ── verify_delegation_token ───────────────────────────────────────────

    #[test]
    fn verify_delegation_token_now_produces_same_result_as_verify_delegation_token() {
        let checker = CapabilityChecker::new(SECRET);
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );

        // Compute current_time the same way the _now function does
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let explicit = verify_delegation_token(
            Some(&checker),
            &token,
            &bob(),
            DelegationResource::Tool,
            "inference",
            DelegationAction::Execute,
            current_time,
        );
        let convenience = verify_delegation_token_now(
            Some(&checker),
            &token,
            &bob(),
            DelegationResource::Tool,
            "inference",
            DelegationAction::Execute,
        );

        assert_eq!(explicit, convenience);
    }

    #[test]
    fn verify_delegation_token_returns_valid_when_all_checks_pass() {
        let checker = CapabilityChecker::new(SECRET);
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );

        let outcome = verify_delegation_token(
            Some(&checker),
            &token,
            &bob(),
            DelegationResource::Tool,
            "inference",
            DelegationAction::Execute,
            0,
        );

        assert_eq!(outcome, VerificationOutcome::Valid);
    }

    #[test]
    fn verify_delegation_token_returns_invalid_signature() {
        let checker = CapabilityChecker::new(b"wrong-secret");
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );

        let outcome = verify_delegation_token(
            Some(&checker),
            &token,
            &bob(),
            DelegationResource::Tool,
            "inference",
            DelegationAction::Execute,
            0,
        );

        assert_eq!(outcome, VerificationOutcome::InvalidSignature);
    }

    #[test]
    fn verify_delegation_token_returns_expired() {
        let checker = CapabilityChecker::new(SECRET);
        let token = crate::DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .expires_at(1000)
        .sign(SECRET);

        let outcome = verify_delegation_token(
            Some(&checker),
            &token,
            &bob(),
            DelegationResource::Tool,
            "inference",
            DelegationAction::Execute,
            1001, // past expiry
        );

        assert_eq!(outcome, VerificationOutcome::Expired);
    }

    #[test]
    fn verify_delegation_token_returns_insufficient_access() {
        let checker = CapabilityChecker::new(SECRET);
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Read,
            alice(),
            bob(),
            SECRET,
        );

        let outcome = verify_delegation_token(
            Some(&checker),
            &token,
            &bob(),
            DelegationResource::Tool,
            "inference",
            DelegationAction::Write, // token only has Read
            0,                       // current_time
        );

        assert_eq!(
            outcome,
            VerificationOutcome::InsufficientAccess {
                resource_id: "inference".to_string(),
                action: "write".to_string(),
            }
        );
    }

    #[test]
    fn verify_delegation_token_returns_no_checker() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );

        let outcome = verify_delegation_token(
            None, // no checker
            &token,
            &bob(),
            DelegationResource::Tool,
            "inference",
            DelegationAction::Execute,
            0,
        );

        assert_eq!(outcome, VerificationOutcome::NoChecker);
    }

    // ── require_write_access ─────────────────────────────────────────────

    #[test]
    fn require_write_access_ok_for_write() {
        let token = DelegationToken::new(
            DelegationResource::Registry,
            "episodic_memory".to_string(),
            DelegationAction::Write,
            alice(),
            bob(),
            SECRET,
        );
        assert!(
            require_write_access(&token, "episodic").is_ok(),
            "Write token must pass require_write_access"
        );
    }

    #[test]
    fn require_write_access_ok_for_execute() {
        let token = DelegationToken::new(
            DelegationResource::Registry,
            "episodic_memory".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );
        assert!(
            require_write_access(&token, "episodic").is_ok(),
            "Execute token must pass require_write_access"
        );
    }

    #[test]
    fn require_write_access_err_for_read() {
        let token = DelegationToken::new(
            DelegationResource::Registry,
            "episodic_memory".to_string(),
            DelegationAction::Read,
            alice(),
            bob(),
            SECRET,
        );
        let err = require_write_access(&token, "episodic").unwrap_err();
        assert!(
            err.contains("episodic"),
            "error must include store_type, got: {err}"
        );
        assert!(
            err.contains("read-only"),
            "error must mention read-only, got: {err}"
        );
    }

    // ── require_read_access ──────────────────────────────────────────────

    #[test]
    fn require_read_access_ok_for_all_actions() {
        for (action, label) in [
            (DelegationAction::Read, "Read"),
            (DelegationAction::Write, "Write"),
            (DelegationAction::Execute, "Execute"),
        ] {
            let token = DelegationToken::new(
                DelegationResource::Registry,
                "episodic_memory".to_string(),
                action,
                alice(),
                bob(),
                SECRET,
            );
            assert!(
                require_read_access(&token, "episodic").is_ok(),
                "{} token must pass require_read_access",
                label
            );
        }
    }
}
