//! Delegation tokens (OCAP) — inter-agent capability delegation
//
//! Two token kinds: **Loop authority** (ZST tokens in `tokens.rs`) prove loop-authorized operations;
//! **Delegation** (`DelegationToken`) are HMAC-signed tokens for inter-agent delegation with cryptographic attenuation.

/// Shared structural bound: capability attenuation, cascade depth, subgoal nesting.
pub const SYSTEM_MAX_RECURSION: u8 = 7;

/// Capability-domain alias for SYSTEM_MAX_RECURSION.
pub const SYSTEM_MAX_ATTENUATION: u8 = SYSTEM_MAX_RECURSION;

pub(crate) mod hmac_ops;
pub mod verification;

pub mod tokens;
pub use tokens::{ConsolidationToken, IssuerVerification};

pub use verification::{
    CapabilityChecker, TOKEN_ERR_EXPIRED, TOKEN_ERR_INVALID_SIGNATURE, TOKEN_ERR_NO_CHECKER,
    VerificationOutcome, require_read_access, require_write_access, token_err_insufficient_access,
    token_err_tool_access_denied, verify_delegation_token,
};

use crate::WebID;
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

fn to_string(webid: &WebID) -> String {
    webid.to_string()
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| e.to_string())
}

/// Parsed colon-separated capability string (e.g. `"tool:inference:call"`).
/// Single source of truth — all parsing must use [`CapabilitySpec::parse`].
/// 2-part: `"resource:action"` → `resource_id = full string`
/// 3-part: `"resource:domain:action"` → `resource_id = domain`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySpec {
    pub resource: DelegationResource,
    pub resource_id: String,
    pub action: DelegationAction,
}

impl CapabilitySpec {
    /// Parse a colon-separated capability string into its typed components.
    ///
    /// Accepted formats:
    /// - `"resource:action"` (2 parts) — `resource_id` is the full string
    /// - `"resource:domain:action"` (3 parts) — `resource_id` is the domain
    ///
    /// Unknown action names fall back to `DelegationAction::Execute`.
    /// The `"memory"` prefix is accepted as an alias for `DelegationResource::Registry`.
    pub fn parse(capability: &str) -> Result<Self, CapabilityParseError> {
        let parts: Vec<&str> = capability.split(':').collect();

        if parts.len() < 2 || parts.len() > 3 {
            return Err(CapabilityParseError::InvalidFormat(capability.to_string()));
        }

        let resource = DelegationResource::parse_str(parts[0])
            .ok_or_else(|| CapabilityParseError::UnknownResource(parts[0].to_string()))?;

        // For 3-part capabilities (resource:domain:action), the resource_id is the domain.
        // For 2-part capabilities (resource:action), the resource_id is the full string.
        let resource_id = if parts.len() == 3 {
            parts[1].to_string()
        } else {
            capability.to_string()
        };

        let action_str = parts
            .last()
            .expect("split always produces at least one element");
        let action = DelegationAction::parse_str(action_str).unwrap_or(DelegationAction::Execute);

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
}

impl DelegationResource {
    pub fn as_str(&self) -> &'static str {
        match self {
            DelegationResource::Tool => "tool",
            DelegationResource::Template => "template",
            DelegationResource::Registry => "registry",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s.split(':').next() {
            Some("tool") => Some(DelegationResource::Tool),
            Some("template") => Some(DelegationResource::Template),
            Some("registry") | Some("memory") => Some(DelegationResource::Registry),
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
            DelegationAction::Read => "read",
            DelegationAction::Write => "write",
            DelegationAction::Execute => "execute",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "read" => Some(DelegationAction::Read),
            "write" => Some(DelegationAction::Write),
            "execute" => Some(DelegationAction::Execute),
            _ => None,
        }
    }

    /// Whether this action permits write-level access.
    ///
    /// Only `Write` and `Execute` grant write-level authority.
    /// `Read` tokens are read-only and must be rejected for store operations.
    pub fn permits_write(&self) -> bool {
        !matches!(self, DelegationAction::Read)
    }

    /// Whether this action permits read-level access.
    ///
    /// `Read` and `Execute` grant read authority; `Write` also implies read.
    pub fn permits_read(&self) -> bool {
        matches!(
            self,
            DelegationAction::Read | DelegationAction::Execute | DelegationAction::Write
        )
    }
}

/// Derive the capability shorthand for an MCP server ID.
///
/// Maps `hkask-mcp-<domain>` to `tool:<domain>:execute`.
/// For example, `hkask-mcp-cns` → `tool:cns:execute`.
///
/// This bridges the MCP namespace (server IDs) and the OCAP capability namespace.
/// Agent definitions declare capabilities like `tool:cns:emit`, and this function
/// derives what capability is required to use tools from a given server.
///
/// Returns `None` if the server ID doesn't follow the `hkask-mcp-` convention.
pub fn capability_from_server_id(server_id: &str) -> Option<String> {
    server_id
        .strip_prefix("hkask-mcp-")
        .map(|domain| format!("tool:{}:execute", domain))
}

/// Check whether a token's capability covers a required capability.
///
/// Two capabilities match if they share the same resource type and domain,
/// and the token's action level is sufficient for the required action:
///
/// - `tool:cns:execute` covers `tool:cns:execute` (exact match)
/// - `tool:cns:execute` covers `tool:cns:emit` (same domain, execute ≥ any action)
/// - `tool:cns:execute` covers `tool:cns:read` (execute ≥ read)
/// - `tool:cns:read` does **not** cover `tool:cns:execute` (read ≱ execute)
/// - `tool:cns:write` covers `tool:cns:read` (write ≥ read) but not `tool:cns:execute`
/// - `tool:cns:execute` does **not** cover `tool:semantic:execute` (different domain)
///
/// If either string cannot be parsed as a capability spec, falls back to exact
/// string comparison.
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

/// Builder for constructing delegation tokens. Each method returns Self.
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

    pub(crate) fn caveat(mut self, caveat: Caveat) -> Self {
        self.caveats.push(caveat);
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
        hasher.update(to_string(from).as_bytes());
        hasher.update(to_string(to).as_bytes());
        hex::encode(hasher.finalize())
    }

    /// HMAC-SHA256.
    fn sign_payload(payload: &SigningPayload, secret: &[u8]) -> String {
        let mut builder = hmac_ops::HmacBuilder::new(secret);
        builder.update(payload.id.as_bytes());
        builder.update(payload.resource.as_str().as_bytes());
        builder.update(payload.resource_id.as_bytes());
        builder.update(payload.action.as_str().as_bytes());
        builder.update(to_string(&payload.from).as_bytes());
        builder.update(to_string(&payload.to).as_bytes());
        // Include caveats in signature for tamper-evidence
        for caveat in &payload.caveats {
            builder.update(caveat.caveat_id.as_bytes());
            builder.update(caveat.data.as_bytes());
        }
        builder.finalize_hex()
    }

    /// Constant-time comparison to prevent timing attacks.
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
        let json = serde_json::to_string(self)?;
        Ok(base64_encode(json.as_bytes()))
    }

    pub fn from_base64(encoded: &str) -> Result<Self, String> {
        let json = base64_decode(encoded)?;
        serde_json::from_slice(&json).map_err(|e| e.to_string())
    }

    pub fn can_attenuate(&self) -> bool {
        self.attenuation_level < self.max_attenuation
    }

    /// 1-hour expiry from `current_time`.
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

    /// Check if this token is valid for a given resource and action
    pub fn is_valid_for(
        &self,
        resource: DelegationResource,
        resource_id: &str,
        action: DelegationAction,
    ) -> bool {
        self.resource == resource && self.resource_id == resource_id && self.action == action
    }

    /// Check if this token grants access to a resource type (regardless of specific ID)
    pub fn grants_resource(&self, resource: DelegationResource) -> bool {
        self.resource == resource
    }

    /// Validate context nonce matches expected execution context
    pub fn validate_context_nonce(&self, expected_context: &str) -> bool {
        // Context nonce must start with expected context (allows attenuation chain)
        self.context_nonce.starts_with(expected_context)
    }

    /// Get the root context nonce (before any attenuation)
    pub fn root_context_nonce(&self) -> &str {
        // Extract root nonce from attenuation chain (format: "root-attenuated-uuid-attenuated-uuid...")
        self.context_nonce
            .split("-attenuated-")
            .next()
            .unwrap_or(&self.context_nonce)
    }

    /// Verify attenuation chain from root nonce to expected level
    ///
    /// Returns true if:
    /// - Root nonce matches expected_root
    /// - attenuation_level <= expected_level
    /// - max_attenuation does not exceed SYSTEM_MAX_ATTENUATION
    /// - Nonce format is valid (root-attenuated-uuid-attenuated-uuid...)
    pub fn verify_attenuation_chain(&self, expected_root: &str, expected_level: u8) -> bool {
        // Reject tokens whose self-attested max_attenuation exceeds the system limit
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

    /// Verify capability cryptographically (for distributed/Paxos verification)
    ///
    /// This method enables cross-machine capability verification without a central authority.
    /// Each machine can verify capabilities independently using the shared secret.
    ///
    /// # Arguments
    /// * `secret` — Shared HMAC secret (distributed via secure channel)
    ///
    /// # Returns
    /// * `true` — Signature is valid
    /// * `false` — Signature invalid or tampered
    pub fn verify_cryptographic(&self, secret: &[u8]) -> bool {
        self.verify(secret)
    }

    /// Add a caveat to this capability token
    ///
    /// Caveats are additive restrictions on the capability. Each caveat
    /// adds a new constraint that must be satisfied for the capability to be valid.
    ///
    /// # Arguments
    /// * `caveat` — The caveat to add
    /// * `secret` — Secret key for re-signing the token
    ///
    /// # Returns
    /// A new `DelegationToken` with the caveat added and re-signed
    pub fn caveat_ids(&self) -> Vec<&str> {
        self.caveats.iter().map(|c| c.caveat_id.as_str()).collect()
    }

    /// Check if token has a specific caveat type
    pub fn has_caveat_type(&self, caveat_type: &str) -> bool {
        self.caveats.iter().any(|c| c.caveat_id == caveat_type)
    }

    /// Get caveat data for a specific caveat type
    pub fn get_caveat_data(&self, caveat_type: &str) -> Option<&str> {
        self.caveats
            .iter()
            .find(|c| c.caveat_id == caveat_type)
            .map(|c| c.data.as_str())
    }

    /// Get capability fingerprint for CRDT merge operations
    ///
    /// Returns a unique fingerprint that can be used for CRDT conflict resolution
    /// when capabilities are replicated across machines.
    ///
    /// # Returns
    /// Fingerprint string suitable for CRDT merge comparison
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

    /// Whether this token authorizes write-level operations.
    ///
    /// Convenience wrapper that delegates to `DelegationAction::permits_write()`.
    /// Use this instead of directly inspecting `token.action` — it encapsulates
    /// the OCAP policy that `Read` tokens cannot mutate state.
    pub fn allows_write(&self) -> bool {
        self.action.permits_write()
    }

    /// Whether this token authorizes read-level operations.
    ///
    /// Convenience wrapper that delegates to `DelegationAction::permits_read()`.
    pub fn allows_read(&self) -> bool {
        self.action.permits_read()
    }

    /// Check if this capability is compatible with another (for CRDT merge)
    ///
    /// Two capabilities are compatible if they have the same resource, action,
    /// and delegated_to, regardless of signature or attenuation level.
    ///
    /// # Arguments
    /// * `other` — Other capability to compare
    ///
    /// # Returns
    /// * `true` — Capabilities are compatible (can be merged in CRDT)
    /// * `false` — Capabilities are incompatible
    pub fn is_compatible_with(&self, other: &DelegationToken) -> bool {
        self.resource == other.resource
            && self.resource_id == other.resource_id
            && self.action == other.action
            && self.delegated_to == other.delegated_to
    }
}

#[cfg(test)]
mod capability_spec_tests {
    use super::{CapabilityParseError, CapabilitySpec, DelegationAction, DelegationResource};

    #[test]
    fn parse_3_part_registry() {
        let spec = CapabilitySpec::parse("registry:episodic_memory:read").unwrap();
        assert_eq!(spec.resource, DelegationResource::Registry);
        assert_eq!(spec.resource_id, "episodic_memory");
        assert_eq!(spec.action, DelegationAction::Read);
    }

    #[test]
    fn parse_3_part_tool() {
        let spec = CapabilitySpec::parse("tool:inference:call").unwrap();
        assert_eq!(spec.resource, DelegationResource::Tool);
        assert_eq!(spec.resource_id, "inference");
        assert_eq!(spec.action, DelegationAction::Execute); // "call" defaults to Execute
    }

    #[test]
    fn parse_2_part_tool() {
        let spec = CapabilitySpec::parse("tool:execute").unwrap();
        assert_eq!(spec.resource, DelegationResource::Tool);
        assert_eq!(spec.resource_id, "tool:execute"); // 2-part: full string as resource_id
        assert_eq!(spec.action, DelegationAction::Execute);
    }

    #[test]
    fn parse_memory_alias() {
        let spec = CapabilitySpec::parse("memory:episodic:read").unwrap();
        assert_eq!(spec.resource, DelegationResource::Registry);
        assert_eq!(spec.resource_id, "episodic");
        assert_eq!(spec.action, DelegationAction::Read);
    }

    #[test]
    fn parse_invalid_format() {
        assert!(CapabilitySpec::parse("justonepart").is_err());
        assert!(CapabilitySpec::parse("a:b:c:d").is_err());
        assert!(CapabilitySpec::parse("").is_err());
    }

    #[test]
    fn parse_unknown_resource() {
        let err = CapabilitySpec::parse("unknown:foo:read").unwrap_err();
        assert!(matches!(err, CapabilityParseError::UnknownResource(_)));
    }

    #[test]
    fn capability_from_server_id_derives_domain() {
        use super::capability_from_server_id;
        assert_eq!(
            capability_from_server_id("hkask-mcp-cns"),
            Some("tool:cns:execute".to_string())
        );
        assert_eq!(
            capability_from_server_id("hkask-mcp-semantic"),
            Some("tool:semantic:execute".to_string())
        );
        assert_eq!(
            capability_from_server_id("hkask-mcp-inference"),
            Some("tool:inference:execute".to_string())
        );
    }

    #[test]
    fn capability_from_server_id_returns_none_for_unknown_prefix() {
        use super::capability_from_server_id;
        assert_eq!(capability_from_server_id("custom-server"), None);
        assert_eq!(capability_from_server_id(""), None);
    }
}

#[cfg(test)]
mod capabilities_match_tests {
    use super::capabilities_match;

    #[test]
    fn exact_match_same_domain_same_action() {
        // token:cns:execute covers tool:cns:execute — exact match
        assert!(capabilities_match("tool:cns:execute", "tool:cns:execute"));
    }

    #[test]
    fn execute_covers_any_action_in_same_domain() {
        // Execute authority on domain "cns" covers any action in that domain
        assert!(capabilities_match("tool:cns:execute", "tool:cns:emit"));
        assert!(capabilities_match("tool:cns:execute", "tool:cns:read"));
        assert!(capabilities_match("tool:cns:execute", "tool:cns:write"));
    }

    #[test]
    fn read_does_not_cover_execute_or_write() {
        // Read authority does not cover execute or write
        assert!(!capabilities_match("tool:cns:read", "tool:cns:execute"));
        assert!(!capabilities_match("tool:cns:read", "tool:cns:write"));
    }

    #[test]
    fn read_covers_read() {
        assert!(capabilities_match("tool:cns:read", "tool:cns:read"));
    }

    #[test]
    fn write_covers_read_and_write_but_not_execute() {
        assert!(capabilities_match("tool:cns:write", "tool:cns:read"));
        assert!(capabilities_match("tool:cns:write", "tool:cns:write"));
        assert!(!capabilities_match("tool:cns:write", "tool:cns:execute"));
    }

    #[test]
    fn different_domain_does_not_match() {
        // tool:cns:execute does NOT cover tool:semantic:execute
        assert!(!capabilities_match(
            "tool:cns:execute",
            "tool:semantic:execute"
        ));
    }

    #[test]
    fn different_resource_type_does_not_match() {
        // tool:cns:execute does NOT cover registry:cns:execute
        assert!(!capabilities_match(
            "tool:cns:execute",
            "registry:cns:execute"
        ));
    }

    #[test]
    fn unparseable_falls_back_to_exact_match() {
        // If either side can't be parsed, fall back to exact string comparison
        assert!(capabilities_match("exact-match-token", "exact-match-token"));
        assert!(!capabilities_match("exact-match-token", "different-token"));
    }

    #[test]
    fn execute_covers_all_in_domain() {
        // A single execute capability covers read, write, and execute in that domain
        assert!(capabilities_match(
            "tool:inference:execute",
            "tool:inference:read"
        ));
        assert!(capabilities_match(
            "tool:inference:execute",
            "tool:inference:write"
        ));
        assert!(capabilities_match(
            "tool:inference:execute",
            "tool:inference:execute"
        ));
    }
}

#[cfg(test)]
mod delegation_token_tests {
    use super::{
        DelegationAction, DelegationResource, DelegationToken, DelegationTokenBuilder,
        SYSTEM_MAX_ATTENUATION,
    };
    use crate::WebID;

    const SECRET: &[u8] = b"test-secret-for-delegation-token-tests";

    fn alice() -> WebID {
        WebID::from_persona(b"alice")
    }

    fn bob() -> WebID {
        WebID::from_persona(b"bob")
    }

    fn carol() -> WebID {
        WebID::from_persona(b"carol")
    }

    // ── DelegationAction ──────────────────────────────────────────────────

    #[test]
    fn delegation_action_as_str_roundtrips() {
        for action in [
            DelegationAction::Read,
            DelegationAction::Write,
            DelegationAction::Execute,
        ] {
            assert_eq!(
                DelegationAction::parse_str(action.as_str()),
                Some(action),
                "action {:?} should roundtrip through as_str/parse_str",
                action
            );
        }
    }

    #[test]
    fn delegation_action_permits_write_only_for_write_and_execute() {
        assert!(
            !DelegationAction::Read.permits_write(),
            "Read must not permit write"
        );
        assert!(
            DelegationAction::Write.permits_write(),
            "Write must permit write"
        );
        assert!(
            DelegationAction::Execute.permits_write(),
            "Execute must permit write"
        );
    }

    #[test]
    fn delegation_action_permits_read_for_all_variants() {
        assert!(DelegationAction::Read.permits_read());
        assert!(DelegationAction::Write.permits_read());
        assert!(DelegationAction::Execute.permits_read());
    }

    // ── DelegationResource ──────────────────────────────────────────────

    #[test]
    fn delegation_resource_as_str_roundtrips() {
        for resource in [
            DelegationResource::Tool,
            DelegationResource::Template,
            DelegationResource::Registry,
        ] {
            assert_eq!(
                DelegationResource::parse_str(resource.as_str()),
                Some(resource),
                "resource {:?} should roundtrip through as_str/parse_str",
                resource
            );
        }
    }

    #[test]
    fn delegation_resource_parse_str_memory_alias() {
        assert_eq!(
            DelegationResource::parse_str("memory"),
            Some(DelegationResource::Registry),
            "'memory' must alias to Registry"
        );
    }

    #[test]
    fn delegation_resource_parse_str_unknown_returns_none() {
        assert_eq!(
            DelegationResource::parse_str("unknown"),
            None,
            "unknown resource string must return None"
        );
    }

    // ── DelegationToken construction & verification ──────────────────────

    #[test]
    fn new_creates_token_with_correct_fields() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );

        assert_eq!(token.resource, DelegationResource::Tool);
        assert_eq!(token.resource_id, "inference");
        assert_eq!(token.action, DelegationAction::Execute);
        assert_eq!(token.delegated_from, alice());
        assert_eq!(token.delegated_to, bob());
        assert_eq!(token.attenuation_level, 0);
        assert_eq!(token.max_attenuation, SYSTEM_MAX_ATTENUATION);
    }

    #[test]
    fn verify_returns_true_with_same_secret() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );
        assert!(
            token.verify(SECRET),
            "token must verify with the same secret"
        );
    }

    #[test]
    fn verify_returns_false_with_different_secret() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );
        assert!(
            !token.verify(b"wrong-secret"),
            "token must not verify with a different secret"
        );
    }

    // ── is_expired ───────────────────────────────────────────────────────

    #[test]
    fn is_expired_true_when_past_expiry() {
        let token = DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .expires_at(1000)
        .sign(SECRET);

        assert!(
            token.is_expired(1001),
            "token must be expired when current_time > expires_at"
        );
    }

    #[test]
    fn is_expired_false_when_before_expiry() {
        let token = DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .expires_at(1000)
        .sign(SECRET);

        assert!(
            !token.is_expired(999),
            "token must not be expired when current_time < expires_at"
        );
        assert!(
            !token.is_expired(1000),
            "token must not be expired when current_time == expires_at"
        );
    }

    #[test]
    fn is_expired_false_when_no_expiry() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );

        assert!(
            !token.is_expired(999999),
            "token with no expiry must never be expired"
        );
    }

    // ── holder / issuer ──────────────────────────────────────────────────

    #[test]
    fn holder_returns_delegated_to() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );
        assert_eq!(token.holder(), bob());
    }

    #[test]
    fn issuer_returns_delegated_from() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );
        assert_eq!(token.issuer(), alice());
    }

    // ── is_valid_for / grants_resource ────────────────────────────────────

    #[test]
    fn is_valid_for_matches_resource_id_action() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );

        assert!(token.is_valid_for(
            DelegationResource::Tool,
            "inference",
            DelegationAction::Execute
        ));
    }

    #[test]
    fn is_valid_for_rejects_wrong_resource() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );
        assert!(!token.is_valid_for(
            DelegationResource::Registry,
            "inference",
            DelegationAction::Execute
        ));
    }

    #[test]
    fn is_valid_for_rejects_wrong_resource_id() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );
        assert!(!token.is_valid_for(DelegationResource::Tool, "other", DelegationAction::Execute));
    }

    #[test]
    fn is_valid_for_rejects_wrong_action() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );
        assert!(!token.is_valid_for(
            DelegationResource::Tool,
            "inference",
            DelegationAction::Read
        ));
    }

    #[test]
    fn grants_resource_matches_type_only() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );

        assert!(token.grants_resource(DelegationResource::Tool));
        assert!(!token.grants_resource(DelegationResource::Registry));
        assert!(!token.grants_resource(DelegationResource::Template));
    }

    // ── allows_write / allows_read ───────────────────────────────────────

    #[test]
    fn allows_write_true_for_write_and_execute() {
        for (action, label) in [
            (DelegationAction::Write, "Write"),
            (DelegationAction::Execute, "Execute"),
        ] {
            let token = DelegationToken::new(
                DelegationResource::Tool,
                "inference".to_string(),
                action,
                alice(),
                bob(),
                SECRET,
            );
            assert!(token.allows_write(), "{} must allow write", label);
        }
    }

    #[test]
    fn allows_write_false_for_read() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Read,
            alice(),
            bob(),
            SECRET,
        );
        assert!(!token.allows_write(), "Read must not allow write");
    }

    #[test]
    fn allows_read_true_for_all_actions() {
        for (action, label) in [
            (DelegationAction::Read, "Read"),
            (DelegationAction::Write, "Write"),
            (DelegationAction::Execute, "Execute"),
        ] {
            let token = DelegationToken::new(
                DelegationResource::Tool,
                "inference".to_string(),
                action,
                alice(),
                bob(),
                SECRET,
            );
            assert!(token.allows_read(), "{} must allow read", label);
        }
    }

    // ── fingerprint ──────────────────────────────────────────────────────

    #[test]
    fn fingerprint_includes_key_fields() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );

        let fp = token.fingerprint();
        assert!(fp.contains(&token.id), "fingerprint must contain id");
        assert!(
            fp.contains("tool"),
            "fingerprint must contain resource as_str"
        );
        assert!(
            fp.contains("inference"),
            "fingerprint must contain resource_id"
        );
        assert!(
            fp.contains("execute"),
            "fingerprint must contain action as_str"
        );
        assert!(
            fp.contains(&format!("{}", token.delegated_to)),
            "fingerprint must contain delegated_to"
        );
        assert!(
            fp.contains(&token.attenuation_level.to_string()),
            "fingerprint must contain attenuation_level"
        );
    }

    // ── can_attenuate / attenuate ─────────────────────────────────────────

    #[test]
    fn can_attenuate_true_when_below_max() {
        let token = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );
        assert!(
            token.can_attenuate(),
            "new token (level 0 < max) must be attenuatable"
        );
    }

    #[test]
    fn can_attenuate_false_at_max() {
        let token = DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .attenuation(SYSTEM_MAX_ATTENUATION, SYSTEM_MAX_ATTENUATION)
        .sign(SECRET);

        assert!(
            !token.can_attenuate(),
            "token at max attenuation must not be attenuatable"
        );
    }

    #[test]
    fn attenuate_returns_child_with_incremented_level() {
        let parent = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );

        let child = parent
            .attenuate(carol(), SECRET, 100)
            .expect("parent at level 0 must attenuate");

        assert_eq!(
            child.attenuation_level,
            parent.attenuation_level + 1,
            "child attenuation_level must be parent_level + 1"
        );
    }

    #[test]
    fn attenuate_child_delegated_from_is_parent_delegated_to() {
        let parent = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );

        let child = parent
            .attenuate(carol(), SECRET, 100)
            .expect("attenuate must succeed");

        assert_eq!(
            child.delegated_from, parent.delegated_to,
            "child's delegated_from must be parent's delegated_to"
        );
        assert_eq!(child.delegated_to, carol());
    }

    #[test]
    fn attenuate_child_verifies_with_same_secret() {
        let parent = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );

        let child = parent
            .attenuate(carol(), SECRET, 100)
            .expect("attenuate must succeed");

        assert!(
            child.verify(SECRET),
            "attenuated child must verify with the same secret"
        );
    }

    #[test]
    fn attenuate_returns_none_at_max() {
        let token = DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .attenuation(SYSTEM_MAX_ATTENUATION, SYSTEM_MAX_ATTENUATION)
        .sign(SECRET);

        assert!(
            token.attenuate(carol(), SECRET, 100).is_none(),
            "attenuate at max must return None"
        );
    }

    // ── to_base64 / from_base64 roundtrip ────────────────────────────────

    #[test]
    fn base64_roundtrip() {
        let token = DelegationToken::new(
            DelegationResource::Template,
            "my-template".to_string(),
            DelegationAction::Read,
            alice(),
            bob(),
            SECRET,
        );

        let encoded = token.to_base64().expect("encoding must succeed");
        let decoded = DelegationToken::from_base64(&encoded).expect("decoding must succeed");

        assert_eq!(decoded.id, token.id);
        assert_eq!(decoded.resource, token.resource);
        assert_eq!(decoded.resource_id, token.resource_id);
        assert_eq!(decoded.action, token.action);
        assert_eq!(decoded.delegated_from, token.delegated_from);
        assert_eq!(decoded.delegated_to, token.delegated_to);
        assert_eq!(decoded.signature, token.signature);
        assert_eq!(decoded.attenuation_level, token.attenuation_level);
        assert_eq!(decoded.max_attenuation, token.max_attenuation);
    }

    // ── is_compatible_with ───────────────────────────────────────────────

    #[test]
    fn is_compatible_with_same_resource_action_holder() {
        let t1 = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );
        let t2 = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );

        assert!(
            t1.is_compatible_with(&t2),
            "tokens with same resource/id/action/holder must be compatible"
        );
    }

    #[test]
    fn is_compatible_with_rejects_different_holder() {
        let t1 = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
            SECRET,
        );
        let t2 = DelegationToken::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            carol(),
            SECRET,
        );

        assert!(
            !t1.is_compatible_with(&t2),
            "tokens with different holders must not be compatible"
        );
    }

    // ── verify_attenuation_chain ─────────────────────────────────────────

    #[test]
    fn verify_attenuation_chain_accepts_valid_root() {
        let nonce = "test-root-nonce";
        let token = DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .context_nonce(nonce.to_string())
        .sign(SECRET);

        assert!(
            token.verify_attenuation_chain(nonce, 0),
            "root token must verify against its own nonce at level 0"
        );
    }

    #[test]
    fn verify_attenuation_chain_rejects_wrong_root() {
        let nonce = "test-root-nonce";
        let token = DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .context_nonce(nonce.to_string())
        .sign(SECRET);

        assert!(
            !token.verify_attenuation_chain("wrong-root", 0),
            "wrong root nonce must fail"
        );
    }

    #[test]
    fn verify_attenuation_chain_rejects_exceeded_max_attenuation() {
        let nonce = "test-root-nonce";
        let token = DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .attenuation(0, 255) // exceeds SYSTEM_MAX_ATTENUATION
        .context_nonce(nonce.to_string())
        .sign(SECRET);

        assert!(
            !token.verify_attenuation_chain(nonce, 0),
            "max_attenuation exceeding system limit must fail"
        );
    }

    // ── validate_context_nonce ───────────────────────────────────────────

    #[test]
    fn validate_context_nonce_accepts_exact_match() {
        let nonce = "execution-context-42";
        let token = DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .context_nonce(nonce.to_string())
        .sign(SECRET);

        assert!(
            token.validate_context_nonce(nonce),
            "exact context nonce must validate"
        );
    }

    #[test]
    fn validate_context_nonce_accepts_prefix_match() {
        let nonce = "execution-context-42";
        let token = DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .context_nonce(format!("{}-attenuated-uuid123", nonce))
        .sign(SECRET);

        assert!(
            token.validate_context_nonce(nonce),
            "attenuated nonce must validate against root prefix"
        );
    }

    #[test]
    fn validate_context_nonce_rejects_mismatch() {
        let token = DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .context_nonce("real-context".to_string())
        .sign(SECRET);

        assert!(
            !token.validate_context_nonce("wrong-context"),
            "mismatched nonce must fail"
        );
    }

    // ── root_context_nonce ───────────────────────────────────────────────

    #[test]
    fn root_context_nonce_returns_root_before_attenuation() {
        let nonce = "root-nonce";
        let token = DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .context_nonce(nonce.to_string())
        .sign(SECRET);

        assert_eq!(
            token.root_context_nonce(),
            nonce,
            "unattenuated token must return its own nonce as root"
        );
    }

    #[test]
    fn root_context_nonce_extracts_root_from_attenuated() {
        let root = "root-nonce";
        let token = DelegationTokenBuilder::new(
            DelegationResource::Tool,
            "inference".to_string(),
            DelegationAction::Execute,
            alice(),
            bob(),
        )
        .context_nonce(format!("{}-attenuated-uuid123", root))
        .sign(SECRET);

        assert_eq!(
            token.root_context_nonce(),
            root,
            "attenuated nonce must extract root before '-attenuated-'"
        );
    }

    // ── Serde roundtrip for DelegationResource / DelegationAction ────────

    #[test]
    fn serde_roundtrip_delegation_resource() {
        for resource in [
            DelegationResource::Tool,
            DelegationResource::Template,
            DelegationResource::Registry,
        ] {
            let json = serde_json::to_string(&resource).expect("serialize must succeed");
            let decoded: DelegationResource =
                serde_json::from_str(&json).expect("deserialize must succeed");
            assert_eq!(decoded, resource, "serde roundtrip must preserve resource");
        }
    }

    #[test]
    fn serde_roundtrip_delegation_action() {
        for action in [
            DelegationAction::Read,
            DelegationAction::Write,
            DelegationAction::Execute,
        ] {
            let json = serde_json::to_string(&action).expect("serialize must succeed");
            let decoded: DelegationAction =
                serde_json::from_str(&json).expect("deserialize must succeed");
            assert_eq!(decoded, action, "serde roundtrip must preserve action");
        }
    }
}
