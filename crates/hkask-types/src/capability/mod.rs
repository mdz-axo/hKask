//! Delegation tokens (OCAP) — inter-agent capability delegation
//
//! Two token kinds: **Loop authority** (ZST tokens in `tokens.rs`) prove loop-authorized operations;
//! **Delegation** (`DelegationToken`) are HMAC-signed tokens for inter-agent delegation with cryptographic attenuation.

/// Shared structural bound: capability attenuation, cascade depth, subgoal nesting.
pub const SYSTEM_MAX_RECURSION: u8 = 7;

/// Capability-domain alias for SYSTEM_MAX_RECURSION.
pub const SYSTEM_MAX_ATTENUATION: u8 = SYSTEM_MAX_RECURSION;

/// Verified authentication context — caller's identity and capability token.
/// Both API (middleware verification) and CLI (keystore resolution) produce this type.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub token: super::DelegationToken,
    pub webid: super::WebID,
}

/// Typed attenuation level (0..SYSTEM_MAX_RECURSION). New code should use this over raw `u8`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AttenuationLevel(u8);

impl AttenuationLevel {
    pub fn new(level: u8) -> Result<Self, AttenuationError> {
        if level > SYSTEM_MAX_RECURSION {
            Err(AttenuationError::ExceedsSystemMax {
                level,
                max: SYSTEM_MAX_RECURSION,
            })
        } else {
            Ok(Self(level))
        }
    }
    /// Unchecked construction — for deserialisation paths that trust the wire format.
    pub fn unchecked(level: u8) -> Self {
        Self(level)
    }
    pub fn as_u8(&self) -> u8 {
        self.0
    }
    pub const fn max() -> u8 {
        SYSTEM_MAX_RECURSION
    }
}

impl std::fmt::Display for AttenuationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AttenuationError {
    #[error("attenuation level {level} exceeds system maximum {max}")]
    ExceedsSystemMax { level: u8, max: u8 },
}

pub(crate) mod hmac_ops;
pub mod verification;

pub mod tokens;
pub use tokens::ConsolidationToken;

pub use verification::{
    CapabilityChecker, TOKEN_ERR_EXPIRED, TOKEN_ERR_INVALID_SIGNATURE, TOKEN_ERR_NO_CHECKER,
    VerificationOutcome, require_read_access, require_write_access, token_err_insufficient_access,
    token_err_tool_access_denied, verify_delegation_token, verify_delegation_token_now,
};

use crate::WebID;
use base64::Engine;
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

fn b64(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}
fn de64(s: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| e.to_string())
}
fn wid(w: &WebID) -> String {
    w.to_string()
}

/// Parsed colon-separated capability spec (e.g. `"tool:inference:call"`).
/// 2-part: `"resource:action"` → `resource_id = full string`. 3-part: `"resource:domain:action"` → `resource_id = domain`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySpec {
    pub resource: DelegationResource,
    pub resource_id: String,
    pub action: DelegationAction,
}

impl CapabilitySpec {
    /// Parse `"resource:action"` (2 parts) or `"resource:domain:action"` (3 parts).
    /// Unknown actions fall back to `Execute`. `"memory"` alias → `Registry`.
    pub fn parse(capability: &str) -> Result<Self, CapabilityParseError> {
        let parts: Vec<&str> = capability.split(':').collect();
        if parts.len() < 2 || parts.len() > 3 {
            return Err(CapabilityParseError::InvalidFormat(capability.to_string()));
        }
        let resource = DelegationResource::parse_str(parts[0])
            .ok_or_else(|| CapabilityParseError::UnknownResource(parts[0].to_string()))?;
        let resource_id = if parts.len() == 3 {
            parts[1].to_string()
        } else {
            capability.to_string()
        };
        let action =
            DelegationAction::parse_str(parts.last().unwrap()).unwrap_or(DelegationAction::Execute);
        Ok(Self {
            resource,
            resource_id,
            action,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapabilityParseError {
    #[error(
        "Invalid capability format: expected 'resource:action' or 'resource:domain:action', got '{0}'"
    )]
    InvalidFormat(String),
    #[error("Unknown resource type: '{0}'. Valid types: tool, template, registry, memory")]
    UnknownResource(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationResource {
    Tool,
    Template,
    Registry,
    /// API key lifecycle management (issue, revoke, fund).
    Key,
}

impl DelegationResource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Template => "template",
            Self::Registry => "registry",
            Self::Key => "key",
        }
    }
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.split(':').next() {
            Some("tool") => Some(Self::Tool),
            Some("template") => Some(Self::Template),
            Some("registry") | Some("memory") => Some(Self::Registry),
            Some("key") => Some(Self::Key),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationAction {
    Read,
    Write,
    Execute,
}

impl DelegationAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Execute => "execute",
        }
    }
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "execute" => Some(Self::Execute),
            _ => None,
        }
    }
    /// `Write` and `Execute` grant write-level; `Read` is read-only.
    pub fn permits_write(&self) -> bool {
        !matches!(self, Self::Read)
    }
    /// All three actions grant read authority.
    pub fn permits_read(&self) -> bool {
        matches!(self, Self::Read | Self::Execute | Self::Write)
    }
}

/// Derive capability shorthand from MCP server ID: `hkask-mcp-<domain>` → `tool:<domain>:execute`. Returns `None` if not `hkask-mcp-` prefix.
pub fn capability_from_server_id(server_id: &str) -> Option<String> {
    server_id
        .strip_prefix("hkask-mcp-")
        .map(|domain| format!("tool:{}:execute", domain))
}

/// Check whether a token's capability covers a required capability.
/// Action hierarchy: Execute ≥ Write ≥ Read. Different domain → no match.
/// Unknown actions fall back to `Execute`. Falls back to exact string compare on parse failure.
pub fn capabilities_match(token_capability: &str, required_capability: &str) -> bool {
    let token_spec = match CapabilitySpec::parse(token_capability) {
        Ok(s) => s,
        Err(_) => return token_capability == required_capability,
    };
    let required_spec = match CapabilitySpec::parse(required_capability) {
        Ok(s) => s,
        Err(_) => return token_capability == required_capability,
    };

    // Different resource types never match (tool ≠ registry)
    if token_spec.resource != required_spec.resource {
        return false;
    }
    // Different domains never match (cns ≠ semantic)
    if token_spec.resource_id != required_spec.resource_id {
        return false;
    }
    // Action hierarchy: Execute ≥ Write ≥ Read
    // Token's action must cover the required action
    match required_spec.action {
        DelegationAction::Read => token_spec.action.permits_read(),
        DelegationAction::Write => token_spec.action.permits_write(),
        DelegationAction::Execute => token_spec.action == DelegationAction::Execute,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: capability-parse-001 — canonical default capability always parses
    //
    // The pod constructor uses `CapabilitySpec::parse("tool:execute")` as the
    // infallible default. This test ensures that never fails.
    #[test]
    fn default_capability_always_parses() {
        assert!(CapabilitySpec::parse("tool:execute").is_ok());
    }

    // REQ: capability-parse-002 — malformed user-supplied capability does not panic
    //
    // Before fix, `AgentPod::new` called `.expect()` on the user-supplied first
    // capability, causing a panic for any malformed input. The fallback is now
    // applied instead.
    #[test]
    fn malformed_capability_parses_to_err_not_panic() {
        // These must return Err, not panic.
        assert!(CapabilitySpec::parse("").is_err());
        assert!(CapabilitySpec::parse("not-a-capability").is_err());
        assert!(CapabilitySpec::parse("::::").is_err());
    }

    // REQ: capability-parse-003 — fallback logic mirrors pod constructor
    #[test]
    fn malformed_capability_falls_back_to_default() {
        let default = "tool:execute".to_string();
        let user_supplied = "garbage:input:bad";
        let spec = CapabilitySpec::parse(user_supplied).unwrap_or_else(|_| {
            CapabilitySpec::parse(&default)
                .expect("Default capability 'tool:execute' must always parse")
        });
        // Fallback spec must be for tool:execute
        assert_eq!(spec.resource, DelegationResource::Tool);
        assert_eq!(spec.action, DelegationAction::Execute);
    }

    // ── Property tests (proptest) ───────────────────────────────────────────

    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        // Valid resource names for strategy generation
        fn valid_resource_str() -> impl Strategy<Value = String> {
            prop_oneof![
                Just("tool".to_string()),
                Just("template".to_string()),
                Just("registry".to_string()),
                Just("memory".to_string()),
                Just("key".to_string()),
            ]
        }

        fn valid_action_str() -> impl Strategy<Value = String> {
            prop_oneof![
                Just("read".to_string()),
                Just("write".to_string()),
                Just("execute".to_string()),
            ]
        }

        // ── CapabilitySpec::parse ──────────────────────────────────────────

        // REQ: cap-prop-001 — valid 2-part capabilities parse without error
        proptest! {
            #[test]
            fn parse_2part_always_succeeds(
                resource in valid_resource_str(),
                action in valid_action_str()
            ) {
                let input = format!("{resource}:{action}");
                let result = CapabilitySpec::parse(&input);
                prop_assert!(result.is_ok(), "2-part parse failed for: {input}");
                let spec = result.unwrap();
                // resource_id for 2-part should be the full input
                prop_assert_eq!(spec.resource_id, input);
            }
        }

        // REQ: cap-prop-002 — valid 3-part capabilities parse correctly
        proptest! {
            #[test]
            fn parse_3part_has_correct_resource_id(
                resource in valid_resource_str(),
                domain in "[a-z][a-z0-9_]*",
                action in valid_action_str()
            ) {
                let input = format!("{resource}:{domain}:{action}");
                let result = CapabilitySpec::parse(&input);
                prop_assert!(result.is_ok(), "3-part parse failed for: {input}");
                let spec = result.unwrap();
                // resource_id for 3-part should be the domain (middle part)
                prop_assert_eq!(spec.resource_id, domain);
            }
        }

        // REQ: cap-prop-003 — parse never panics on arbitrary input
        proptest! {
            #[test]
            fn parse_never_panics(input in "\\PC*") {
                let _ = CapabilitySpec::parse(&input);
            }
        }

        // REQ: cap-prop-004 — single-part input or 4+ parts returns error
        proptest! {
            #[test]
            fn malformed_part_count_returns_err(
                input in proptest::string::string_regex("[a-z]+").unwrap()
            ) {
                // Single part, no colon
                if !input.contains(':') {
                    prop_assert!(CapabilitySpec::parse(&input).is_err(),
                        "single-part input should fail: {input}");
                }
            }
        }

        // REQ: cap-prop-005 — 4+ colon-separated parts returns error
        proptest! {
            #[test]
            fn four_plus_parts_returns_err(
                extra in "[a-z]+:[a-z]+:[a-z]+:[a-z]+"
            ) {
                prop_assert!(CapabilitySpec::parse(&extra).is_err(),
                    "4-part input should fail: {extra}");
            }
        }

        // REQ: cap-prop-006 — unknown action falls back to Execute
        proptest! {
            #[test]
            fn unknown_action_uses_execute(
                resource in valid_resource_str(),
                unknown_action in "[a-z_]+"
            ) {
                prop_assume!(unknown_action != "read"
                    && unknown_action != "write"
                    && unknown_action != "execute");
                let input = format!("{resource}:{unknown_action}");
                let result = CapabilitySpec::parse(&input);
                prop_assert!(result.is_ok(),
                    "parse with unknown action should succeed: {input}");
                prop_assert_eq!(result.unwrap().action, DelegationAction::Execute);
            }
        }

        // ── DelegationResource::parse_str / as_str round-trip ──────────────

        // REQ: cap-prop-007 — resource parse/as_str round-trip for all variants
        proptest! {
            #[test]
            fn resource_parse_as_str_round_trip(
                resource in valid_resource_str()
            ) {
                let parsed = DelegationResource::parse_str(&resource);
                prop_assert!(parsed.is_some(), "parse_str failed for: {resource}");
                let round_tripped = parsed.unwrap().as_str();
                // "memory" aliases to Registry, so round-trip differs
                if resource == "memory" {
                    prop_assert_eq!(round_tripped, "registry");
                } else {
                    prop_assert_eq!(round_tripped, resource);
                }
            }
        }

        // ── DelegationAction::parse_str / as_str round-trip ────────────────

        // REQ: cap-prop-008 — action parse/as_str round-trip for all variants
        proptest! {
            #[test]
            fn action_parse_as_str_round_trip(
                action in valid_action_str()
            ) {
                let parsed = DelegationAction::parse_str(&action);
                prop_assert!(parsed.is_some(), "parse_str failed for: {action}");
                prop_assert_eq!(parsed.unwrap().as_str(), action);
            }
        }

        // REQ: cap-prop-009 — action hierarchy: Execute ≥ Write ≥ Read
        proptest! {
            #[test]
            fn action_hierarchy_permits_write(
                action in valid_action_str()
            ) {
                let parsed = DelegationAction::parse_str(&action).unwrap();
                if action == "read" {
                    prop_assert!(!parsed.permits_write());
                } else {
                    prop_assert!(parsed.permits_write());
                }
            }
        }

        proptest! {
            #[test]
            fn action_hierarchy_permits_read(
                action in valid_action_str()
            ) {
                let parsed = DelegationAction::parse_str(&action).unwrap();
                // All actions permit read
                prop_assert!(parsed.permits_read());
            }
        }

        // ── AttenuationLevel ────────────────────────────────────────────────

        // REQ: cap-prop-010 — valid attenuation levels (0..max) round-trip
        proptest! {
            #[test]
            fn attenuation_valid_round_trip(
                level in 0u8..=SYSTEM_MAX_RECURSION
            ) {
                let al = AttenuationLevel::new(level);
                prop_assert!(al.is_ok());
                prop_assert_eq!(al.unwrap().as_u8(), level);
            }
        }

        // REQ: cap-prop-011 — attenuation above max returns error
        proptest! {
            #[test]
            fn attenuation_above_max_is_error(
                level in (SYSTEM_MAX_RECURSION + 1)..=u8::MAX
            ) {
                let al = AttenuationLevel::new(level);
                prop_assert!(al.is_err());
            }
        }

        // ── capabilities_match ──────────────────────────────────────────────

        // REQ: cap-prop-012 — a capability always matches itself (reflexive)
        proptest! {
            #[test]
            fn capabilities_match_is_reflexive(
                resource in valid_resource_str(),
                action in valid_action_str()
            ) {
                let cap = format!("{resource}:{action}");
                prop_assert!(capabilities_match(&cap, &cap),
                    "capability should match itself: {cap}");
            }
        }

        // REQ: cap-prop-013 — action hierarchy: execute covers write covers read
        // Uses 3-part capabilities with shared domain so resource_id matches.
        // 2-part capabilities have resource_id = full input, so different
        // actions produce different resource_ids and never match.
        proptest! {
            #[test]
            fn capabilities_match_action_hierarchy(
                resource in valid_resource_str(),
                domain in "[a-z][a-z0-9_]*"
            ) {
                let exec_cap = format!("{resource}:{domain}:execute");
                let write_cap = format!("{resource}:{domain}:write");
                let read_cap = format!("{resource}:{domain}:read");

                // Execute covers write and read within same resource+domain
                prop_assert!(capabilities_match(&exec_cap, &write_cap));
                prop_assert!(capabilities_match(&exec_cap, &read_cap));
                // Write covers read
                prop_assert!(capabilities_match(&write_cap, &read_cap));
                // Read does not cover write or execute
                prop_assert!(!capabilities_match(&read_cap, &write_cap));
                prop_assert!(!capabilities_match(&read_cap, &exec_cap));
            }
        }

        // REQ: cap-prop-014 — different resources never match
        proptest! {
            #[test]
            fn different_resources_never_match(
                r1 in valid_resource_str(),
                r2 in valid_resource_str()
            ) {
                prop_assume!(r1 != r2 && r1 != "memory");
                // Skip the "memory" alias for simplicity
                if r2 == "memory" { return Ok(()); }

                let cap1 = format!("{r1}:execute");
                let cap2 = format!("{r2}:execute");
                prop_assert!(!capabilities_match(&cap1, &cap2),
                    "different resources should not match: {cap1} vs {cap2}");
            }
        }

        // ── capability_from_server_id ───────────────────────────────────────

        // REQ: cap-prop-015 — server_id with hkask-mcp- prefix produces capability
        proptest! {
            #[test]
            fn server_id_to_capability_format(
                domain in "[a-z][a-z0-9_]*"
            ) {
                let server_id = format!("hkask-mcp-{domain}");
                let cap = capability_from_server_id(&server_id);
                prop_assert!(cap.is_some());
                prop_assert_eq!(cap.unwrap(), format!("tool:{domain}:execute"));
            }
        }

        proptest! {
        #[test]
        fn non_prefixed_server_id_returns_none(
            server_id in "[a-z][a-z0-9_-]*"
        ) {
            prop_assume!(!server_id.starts_with("hkask-mcp-"));
            prop_assert!(capability_from_server_id(&server_id).is_none());
            }
        }
    }

    // ── DelegationToken Tests ────────────────────────────────────────────

    const TOKEN_SECRET: &[u8] = b"test-token-secret-32-bytes!!";

    fn test_webid(label: &str) -> WebID {
        WebID::from_persona(label.as_bytes())
    }

    // REQ: token-verify-001 — DelegationToken verifies with correct secret
    #[test]
    fn token_verifies_with_correct_secret() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            TOKEN_SECRET,
        );
        assert!(token.verify(TOKEN_SECRET));
    }

    // REQ: token-verify-002 — DelegationToken rejects wrong secret
    #[test]
    fn token_rejects_wrong_secret() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            TOKEN_SECRET,
        );
        let wrong_secret = b"wrong-secret-32-bytes-minimum!";
        assert!(!token.verify(wrong_secret));
    }

    // REQ: token-verify-003 — DelegationToken rejects tampered signature
    #[test]
    fn token_rejects_tampered_signature() {
        let mut token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            TOKEN_SECRET,
        );
        token.signature.push_str("tampered");
        assert!(!token.verify(TOKEN_SECRET));
    }

    // REQ: token-attenuation-001 — DelegationToken can_attenuate when below max
    #[test]
    fn token_can_attenuate_when_below_max() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            TOKEN_SECRET,
        );
        assert!(token.can_attenuate());
    }

    // REQ: token-attenuation-002 — DelegationToken attenuation enforced at max
    #[test]
    fn token_attenuation_enforced_at_max() {
        let root = test_webid("root");
        let agent = test_webid("agent");

        let mut current = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            root,
            agent,
            TOKEN_SECRET,
        );

        for i in 1..=7 {
            let next_agent = test_webid(&format!("agent-{}", i));
            let attenuated = current
                .attenuate(next_agent, TOKEN_SECRET, 100_000)
                .expect(&format!("Attenuation {} should succeed", i));
            assert!(attenuated.verify(TOKEN_SECRET));
            assert_eq!(attenuated.attenuation_level, i as u8);
            current = attenuated;
        }

        assert!(!current.can_attenuate());
        let next_agent = test_webid("agent-8");
        assert!(
            current
                .attenuate(next_agent, TOKEN_SECRET, 100_000)
                .is_none()
        );
    }

    // REQ: token-attenuation-003 — DelegationToken attenuation preserves signature validity
    #[test]
    fn token_attenuation_preserves_signature_validity() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            TOKEN_SECRET,
        );

        let attenuated = token
            .attenuate(test_webid("agent-2"), TOKEN_SECRET, 100_000)
            .expect("Attenuation should succeed");

        assert!(attenuated.verify(TOKEN_SECRET));
        assert_eq!(attenuated.attenuation_level, 1);
        assert_eq!(attenuated.delegated_from, token.delegated_to);
        assert_eq!(attenuated.delegated_to, test_webid("agent-2"));
    }

    // REQ: token-attenuation-004 — DelegationToken verify_attenuation_chain
    #[test]
    fn token_verify_attenuation_chain() {
        let root = test_webid("root");
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            root,
            test_webid("agent"),
            TOKEN_SECRET,
        );

        let root_nonce = token.root_context_nonce().to_string();

        let attenuated = token
            .attenuate(test_webid("agent-2"), TOKEN_SECRET, 100_000)
            .expect("Attenuation should succeed");

        assert!(attenuated.verify_attenuation_chain(&root_nonce, 1));
        assert!(!attenuated.verify_attenuation_chain("wrong-root", 1));
        assert!(!attenuated.verify_attenuation_chain(&root_nonce, 0));
    }

    // REQ: token-expiry-001 — DelegationToken is_expired when past expiry
    #[test]
    fn token_is_expired_when_past_expiry() {
        let mut token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            TOKEN_SECRET,
        );
        token.expires_at = Some(1000);
        assert!(token.is_expired(2000));
        assert!(!token.is_expired(500));
    }

    // REQ: token-expiry-002 — DelegationToken without expiry never expires
    #[test]
    fn token_without_expiry_never_expires() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            TOKEN_SECRET,
        );
        assert!(!token.is_expired(i64::MAX));
    }

    // REQ: token-serialization-001 — DelegationToken base64 round-trip
    #[test]
    fn token_base64_round_trip() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            TOKEN_SECRET,
        );

        let encoded = token.to_base64().expect("Base64 encoding should succeed");
        let decoded =
            DelegationToken::from_base64(&encoded).expect("Base64 decoding should succeed");

        assert_eq!(token.id, decoded.id);
        assert_eq!(token.resource, decoded.resource);
        assert_eq!(token.delegated_to, decoded.delegated_to);
        assert!(decoded.verify(TOKEN_SECRET));
    }
}

/// Additive restrictions on a capability token.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Caveat {
    pub caveat_id: String,
    pub data: String,
}

/// HMAC-signed OCAP token for inter-agent capability delegation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationToken {
    pub id: String,
    pub resource: DelegationResource,
    pub resource_id: String,
    pub action: DelegationAction,
    pub delegated_from: WebID,
    pub delegated_to: WebID,
    pub signature: String,
    pub expires_at: Option<i64>,
    /// 0 = full authority, increases with each delegation
    pub attenuation_level: u8,
    pub max_attenuation: u8,
    pub context_nonce: String,
    pub(crate) caveats: Vec<Caveat>,
}

/// Internal signing payload extracted from builder state.
struct SigningPayload {
    id: String,
    resource: DelegationResource,
    resource_id: String,
    action: DelegationAction,
    from: WebID,
    to: WebID,
    caveats: Vec<Caveat>,
}

/// Builder for constructing delegation tokens.
pub struct DelegationTokenBuilder {
    resource: DelegationResource,
    resource_id: String,
    action: DelegationAction,
    delegated_from: WebID,
    delegated_to: WebID,
    expires_at: Option<i64>,
    attenuation_level: u8,
    max_attenuation: u8,
    context_nonce: Option<String>,
    caveats: Vec<Caveat>,
}

impl DelegationTokenBuilder {
    pub fn new(
        resource: DelegationResource,
        resource_id: String,
        action: DelegationAction,
        delegated_from: WebID,
        delegated_to: WebID,
    ) -> Self {
        Self {
            resource,
            resource_id,
            action,
            delegated_from,
            delegated_to,
            expires_at: None,
            attenuation_level: 0,
            max_attenuation: SYSTEM_MAX_ATTENUATION,
            context_nonce: None,
            caveats: Vec::new(),
        }
    }
    pub fn expires_at(mut self, ts: i64) -> Self {
        self.expires_at = Some(ts);
        self
    }
    pub fn attenuation(mut self, level: u8, max: u8) -> Self {
        self.attenuation_level = level;
        self.max_attenuation = max;
        self
    }
    pub fn context_nonce(mut self, nonce: String) -> Self {
        self.context_nonce = Some(nonce);
        self
    }
    pub(crate) fn caveat(mut self, c: Caveat) -> Self {
        self.caveats.push(c);
        self
    }
    pub fn sign(self, secret: &[u8]) -> DelegationToken {
        let id = DelegationToken::generate_id(
            &self.resource,
            &self.resource_id,
            &self.action,
            &self.delegated_from,
            &self.delegated_to,
        );
        let payload = SigningPayload {
            id,
            resource: self.resource,
            resource_id: self.resource_id,
            action: self.action,
            from: self.delegated_from,
            to: self.delegated_to,
            caveats: self.caveats,
        };
        let signature = DelegationToken::sign_payload(&payload, secret);
        let context_nonce = self
            .context_nonce
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        DelegationToken {
            id: payload.id,
            resource: payload.resource,
            resource_id: payload.resource_id,
            action: payload.action,
            delegated_from: payload.from,
            delegated_to: payload.to,
            signature,
            expires_at: self.expires_at,
            attenuation_level: self.attenuation_level,
            max_attenuation: self.max_attenuation,
            context_nonce,
            caveats: payload.caveats,
        }
    }
}

impl DelegationToken {
    pub fn new(
        resource: DelegationResource,
        resource_id: String,
        action: DelegationAction,
        delegated_from: WebID,
        delegated_to: WebID,
        secret: &[u8],
    ) -> Self {
        DelegationTokenBuilder::new(resource, resource_id, action, delegated_from, delegated_to)
            .sign(secret)
    }

    fn generate_id(
        resource: &DelegationResource,
        resource_id: &str,
        action: &DelegationAction,
        from: &WebID,
        to: &WebID,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(resource.as_str().as_bytes());
        hasher.update(resource_id.as_bytes());
        hasher.update(action.as_str().as_bytes());
        hasher.update(wid(from).as_bytes());
        hasher.update(wid(to).as_bytes());
        hex::encode(hasher.finalize())
    }

    fn sign_payload(payload: &SigningPayload, secret: &[u8]) -> String {
        let mut builder = hmac_ops::HmacBuilder::new(secret);
        builder.update(payload.id.as_bytes());
        builder.update(payload.resource.as_str().as_bytes());
        builder.update(payload.resource_id.as_bytes());
        builder.update(payload.action.as_str().as_bytes());
        builder.update(wid(&payload.from).as_bytes());
        builder.update(wid(&payload.to).as_bytes());
        // Include caveats in signature for tamper-evidence
        for caveat in &payload.caveats {
            builder.update(caveat.caveat_id.as_bytes());
            builder.update(caveat.data.as_bytes());
        }
        builder.finalize_hex()
    }

    /// Constant-time HMAC verification. Also aliased as `verify_cryptographic`.
    pub fn verify(&self, secret: &[u8]) -> bool {
        let payload = SigningPayload {
            id: self.id.clone(),
            resource: self.resource,
            resource_id: self.resource_id.clone(),
            action: self.action,
            from: self.delegated_from,
            to: self.delegated_to,
            caveats: self.caveats.clone(),
        };
        let expected = Self::sign_payload(&payload, secret);

        // Constant-time comparison to prevent timing attacks
        hmac_ops::verify_hmac_constant_time(expected.as_bytes(), self.signature.as_bytes())
    }

    pub fn is_expired(&self, current_time: i64) -> bool {
        self.expires_at
            .map(|exp| current_time > exp)
            .unwrap_or(false)
    }
    pub fn holder(&self) -> WebID {
        self.delegated_to
    }
    pub fn issuer(&self) -> WebID {
        self.delegated_from
    }

    pub fn to_base64(&self) -> Result<String, serde_json::Error> {
        Ok(b64(serde_json::to_string(self)?.as_bytes()))
    }
    pub fn from_base64(encoded: &str) -> Result<Self, String> {
        serde_json::from_slice(&de64(encoded)?).map_err(|e| e.to_string())
    }
    pub fn can_attenuate(&self) -> bool {
        self.attenuation_level < self.max_attenuation
    }
    /// Attenuate with 1-hour expiry from `current_time`.
    pub fn attenuate(
        &self,
        new_to: WebID,
        secret: &[u8],
        current_time: i64,
    ) -> Option<DelegationToken> {
        self.attenuate_with_expiry(new_to, secret, Some(current_time + 3600))
    }

    /// Create attenuated child token with custom expiry.
    pub fn attenuate_with_expiry(
        &self,
        new_to: WebID,
        secret: &[u8],
        expires_at: Option<i64>,
    ) -> Option<DelegationToken> {
        if !self.can_attenuate() {
            return None;
        }

        let mut builder = DelegationTokenBuilder::new(
            self.resource,
            self.resource_id.clone(),
            self.action,
            self.delegated_to,
            new_to,
        )
        .attenuation(self.attenuation_level + 1, self.max_attenuation)
        .context_nonce(format!(
            "{}-attenuated-{}",
            self.context_nonce,
            uuid::Uuid::new_v4()
        ));

        if let Some(ts) = expires_at {
            builder = builder.expires_at(ts);
        }

        // Preserve parent's caveats
        for caveat in &self.caveats {
            builder = builder.caveat(caveat.clone());
        }

        Some(builder.sign(secret))
    }

    pub fn is_valid_for(
        &self,
        resource: DelegationResource,
        resource_id: &str,
        action: DelegationAction,
    ) -> bool {
        self.resource == resource && self.resource_id == resource_id && self.action == action
    }
    pub fn grants_resource(&self, resource: DelegationResource) -> bool {
        self.resource == resource
    }
    pub fn validate_context_nonce(&self, expected_context: &str) -> bool {
        self.context_nonce.starts_with(expected_context)
    }
    /// Extract root nonce from attenuation chain (`"root-attenuated-uuid-..."`).
    pub fn root_context_nonce(&self) -> &str {
        self.context_nonce
            .split("-attenuated-")
            .next()
            .unwrap_or(&self.context_nonce)
    }

    /// Verify attenuation chain: root nonce matches, level ≤ expected, max ≤ SYSTEM_MAX_ATTENUATION.
    pub fn verify_attenuation_chain(&self, expected_root: &str, expected_level: u8) -> bool {
        if self.max_attenuation > SYSTEM_MAX_ATTENUATION {
            return false;
        }

        let root = self.root_context_nonce();
        if root != expected_root {
            return false;
        }

        // Count attenuation levels in nonce
        let actual_level = self.context_nonce.matches("-attenuated-").count() as u8;
        if actual_level != self.attenuation_level {
            return false; // Nonce doesn't match attenuation level
        }

        self.attenuation_level <= expected_level
    }

    /// Cryptographic verification for distributed/Paxos use.
    pub fn verify_cryptographic(&self, secret: &[u8]) -> bool {
        self.verify(secret)
    }
    pub fn caveat_ids(&self) -> Vec<&str> {
        self.caveats.iter().map(|c| c.caveat_id.as_str()).collect()
    }
    pub fn has_caveat_type(&self, caveat_type: &str) -> bool {
        self.caveats.iter().any(|c| c.caveat_id == caveat_type)
    }
    pub fn get_caveat_data(&self, caveat_type: &str) -> Option<&str> {
        self.caveats
            .iter()
            .find(|c| c.caveat_id == caveat_type)
            .map(|c| c.data.as_str())
    }
    pub fn fingerprint(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.id,
            self.resource.as_str(),
            self.resource_id,
            self.action.as_str(),
            self.delegated_to,
            self.attenuation_level
        )
    }
    pub fn allows_write(&self) -> bool {
        self.action.permits_write()
    }
    pub fn allows_read(&self) -> bool {
        self.action.permits_read()
    }
    pub fn is_compatible_with(&self, other: &DelegationToken) -> bool {
        self.resource == other.resource
            && self.resource_id == other.resource_id
            && self.action == other.action
            && self.delegated_to == other.delegated_to
    }
}

/// Type alias for spec-code alignment (`CapabilityToken`). Prefer `DelegationToken` directly.
pub type CapabilityToken = DelegationToken;
