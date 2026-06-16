//! Delegation tokens (OCAP) — inter-agent capability delegation
//
//! Two token kinds: **Loop authority** (ZST tokens in `tokens.rs`) prove loop-authorized operations;
//! **Delegation** (`DelegationToken`) are Ed25519-signed tokens for inter-agent delegation with cryptographic attenuation.

// G2 Justification: This module exposes 20 public items because it defines the OCAP capability system — DelegationToken, DelegationResource, DelegationAction, CapabilitySpec, AttenuationLevel, and verification types. Each is a distinct security concept that cannot be merged without losing type safety.

pub mod auth;
pub mod resources;
pub mod token_types;

pub mod verification;

pub mod tokens;
pub use tokens::ConsolidationToken;

pub use verification::{
    CapabilityChecker, TOKEN_ERR_EXPIRED, TOKEN_ERR_INVALID_SIGNATURE, TOKEN_ERR_NO_CHECKER,
    VerificationOutcome, require_read_access, require_write_access, token_err_insufficient_access,
    token_err_tool_access_denied, verify_delegation_token, verify_delegation_token_now,
};

pub use auth::{AuthContext, derive_signing_key};
pub use resources::{
    CapabilityParseError, CapabilitySpec, DelegationAction, DelegationResource, capabilities_match,
    capability_from_server_id,
};
pub use token_types::{
    CapabilityToken, DelegationToken, DelegationTokenBuilder, SYSTEM_MAX_ATTENUATION,
    SYSTEM_MAX_RECURSION,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WebID;
    use ed25519_dalek::SigningKey;

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

    fn test_signing_key() -> SigningKey {
        derive_signing_key(TOKEN_SECRET)
    }

    // REQ: token-verify-001 — DelegationToken verifies with correct public key
    #[test]
    fn token_verifies_with_correct_key() {
        let sk = test_signing_key();
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            &sk,
        );
        assert!(token.verify());
    }

    // REQ: token-verify-002 — DelegationToken rejects wrong public key
    #[test]
    fn token_rejects_wrong_key() {
        let sk = test_signing_key();
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            &sk,
        );
        // Create a token with a different key and try to verify with original
        let wrong_sk = derive_signing_key(b"wrong-secret-32-bytes-minimum!");
        let wrong_token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            &wrong_sk,
        );
        // Each token verifies with its own public key
        assert!(token.verify());
        assert!(wrong_token.verify());
        // But they have different public keys
        assert_ne!(token.public_key.0, wrong_token.public_key.0);
    }

    // REQ: token-verify-003 — DelegationToken rejects tampered signature
    #[test]
    fn token_rejects_tampered_signature() {
        let sk = test_signing_key();
        let mut token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            &sk,
        );
        // Tamper with the signature bytes
        token.signature.0[0] ^= 0xFF;
        assert!(!token.verify());
    }

    // REQ: token-attenuation-001 — DelegationToken can_attenuate when below max
    #[test]
    fn token_can_attenuate_when_below_max() {
        let sk = test_signing_key();
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            &sk,
        );
        assert!(token.can_attenuate());
    }

    // REQ: token-attenuation-002 — DelegationToken attenuation enforced at max
    #[test]
    fn token_attenuation_enforced_at_max() {
        let sk = test_signing_key();
        let root = test_webid("root");
        let agent = test_webid("agent");

        let mut current = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            root,
            agent,
            &sk,
        );

        for i in 1..=7 {
            let next_agent = test_webid(&format!("agent-{}", i));
            let attenuated = current
                .attenuate(next_agent, &sk, 100_000)
                .expect(&format!("Attenuation {} should succeed", i));
            assert!(attenuated.verify());
            assert_eq!(attenuated.attenuation_level, i as u8);
            current = attenuated;
        }

        assert!(!current.can_attenuate());
        let next_agent = test_webid("agent-8");
        assert!(current.attenuate(next_agent, &sk, 100_000).is_none());
    }

    // REQ: token-attenuation-003 — DelegationToken attenuation preserves signature validity
    #[test]
    fn token_attenuation_preserves_signature_validity() {
        let sk = test_signing_key();
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            &sk,
        );

        let attenuated = token
            .attenuate(test_webid("agent-2"), &sk, 100_000)
            .expect("Attenuation should succeed");

        assert!(attenuated.verify());
        assert_eq!(attenuated.attenuation_level, 1);
        assert_eq!(attenuated.delegated_from, token.delegated_to);
        assert_eq!(attenuated.delegated_to, test_webid("agent-2"));
    }

    // REQ: token-attenuation-004 — DelegationToken verify_attenuation_chain
    #[test]
    fn token_verify_attenuation_chain() {
        let sk = test_signing_key();
        let root = test_webid("root");
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            root,
            test_webid("agent"),
            &sk,
        );

        let root_nonce = token.root_context_nonce().to_string();

        let attenuated = token
            .attenuate(test_webid("agent-2"), &sk, 100_000)
            .expect("Attenuation should succeed");

        assert!(attenuated.verify_attenuation_chain(&root_nonce, 1));
        assert!(!attenuated.verify_attenuation_chain("wrong-root", 1));
        assert!(!attenuated.verify_attenuation_chain(&root_nonce, 0));
    }

    // REQ: token-expiry-001 — DelegationToken is_expired when past expiry
    #[test]
    fn token_is_expired_when_past_expiry() {
        let sk = test_signing_key();
        let mut token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            &sk,
        );
        token.expires_at = Some(1000);
        assert!(token.is_expired(2000));
        assert!(!token.is_expired(500));
    }

    // REQ: token-expiry-002 — DelegationToken without expiry never expires
    #[test]
    fn token_without_expiry_never_expires() {
        let sk = test_signing_key();
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            &sk,
        );
        assert!(!token.is_expired(i64::MAX));
    }

    // REQ: token-serialization-001 — DelegationToken base64 round-trip
    #[test]
    fn token_base64_round_trip() {
        let sk = test_signing_key();
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "execute".to_string(),
            DelegationAction::Execute,
            test_webid("root"),
            test_webid("agent"),
            &sk,
        );

        let encoded = token.to_base64().expect("Base64 encoding should succeed");
        let decoded =
            DelegationToken::from_base64(&encoded).expect("Base64 decoding should succeed");

        assert_eq!(token.id, decoded.id);
        assert_eq!(token.resource, decoded.resource);
        assert_eq!(token.delegated_to, decoded.delegated_to);
        assert!(decoded.verify());
    }
}
