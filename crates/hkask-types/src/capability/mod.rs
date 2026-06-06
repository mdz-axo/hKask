//! Delegation tokens (OCAP) — inter-agent capability delegation
//
//! Two token kinds: **Loop authority** (ZST tokens in `tokens.rs`) prove loop-authorized operations;
//! **Delegation** (`DelegationToken`) are HMAC-signed tokens for inter-agent delegation with cryptographic attenuation.
//! Backward-compatible aliases (`CapabilityToken`, etc.) are provided for migration.

/// System-wide maximum recursion depth.
///
/// Grounds all structurally-bounded recursion across the system:
/// capability attenuation levels, template cascade depth, subgoal nesting.
/// Each domain may further restrict (≤ this value) but none may exceed.
///
/// The name `RECURSION` is the structural bound; `SYSTEM_MAX_ATTENUATION`
/// is the capability-specific alias — same number, different Miller designation
/// (naming is a security boundary for least-authority reasoning).
pub const SYSTEM_MAX_RECURSION: u8 = 7;

/// System-wide maximum attenuation depth.
/// Tokens with max_attenuation exceeding this value are rejected at verification time.
///
/// This is a capability-domain alias for [`SYSTEM_MAX_RECURSION`] — sharing one
/// literal prevents accidental drift between attenuation, cascade, and subgoal bounds.
pub const SYSTEM_MAX_ATTENUATION: u8 = SYSTEM_MAX_RECURSION;

pub(crate) mod hmac_ops;
mod verification;

pub mod tokens;
pub use tokens::{ConsolidationToken, IssuerVerification};

pub use verification::CapabilityChecker;

/// Backward-compatible type aliases for the renamed delegation types.
/// New code should use `DelegationResource`, `DelegationAction`, `DelegationToken`,
/// `DelegationTokenBuilder`, and `AgentDelegation` directly.
pub type CapabilityResource = DelegationResource;
pub type CapabilityAction = DelegationAction;
pub type CapabilityToken = DelegationToken;

use crate::WebID;
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Helper to convert WebID to string
fn to_string(webid: &WebID) -> String {
    webid.to_string()
}

/// Base64 encode bytes to string
fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// Base64 decode string to bytes
fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| e.to_string())
}

/// Parsed capability specification — the canonical result of parsing a
/// colon-separated capability string like `"tool:inference:call"` or
/// `"registry:episodic_memory:read"`.
///
/// This is the **single source of truth** for how capability strings map to
/// typed OCAP fields. All code that parses capability strings must use
/// [`CapabilitySpec::parse`] instead of rolling its own parser.
///
/// # Format
///
/// - 2-part: `"resource:action"` → `resource_id = full string`
/// - 3-part: `"resource:domain:action"` → `resource_id = domain part`
///
/// # Example
///
/// ```
/// use hkask_types::CapabilitySpec;
/// let spec = CapabilitySpec::parse("registry:episodic_memory:read").unwrap();
/// assert_eq!(spec.resource, hkask_types::DelegationResource::Registry);
/// assert_eq!(spec.resource_id, "episodic_memory");
/// assert_eq!(spec.action, hkask_types::DelegationAction::Read);
/// ```
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

/// Error type for capability string parsing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapabilityParseError {
    #[error(
        "Invalid capability format: expected 'resource:action' or 'resource:domain:action', got '{0}'"
    )]
    InvalidFormat(String),
    #[error("Unknown resource type: '{0}'. Valid types: tool, template, registry, memory")]
    UnknownResource(String),
}

/// Delegation resource types — what an agent can act on
/// Loop: Cybernetics (Access Guard subloop 6.1)
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

/// Delegation action types — what an agent can do to a resource
/// Loop: Cybernetics (Access Guard subloop 6.1)
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
}

/// Caveat — A restriction on a capability token
/// Loop: Cybernetics
///
/// Caveats are additive restrictions that limit the scope of a capability.
/// Each caveat has a type identifier and associated data.
///
/// # Common Caveat Types
/// - `expiration`: Unix timestamp after which the capability is invalid
/// - `operation`: Specific operation allowed (e.g., "generate", "chat")
/// - `template`: Template ID scope restriction
/// - `visibility`: Visibility level requirement (e.g., "private", "shared")
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Caveat {
    /// Caveat type identifier (e.g., "expiration", "operation", "template")
    pub caveat_id: String,
    /// Caveat data (e.g., timestamp, operation name, template ID)
    pub data: String,
}

// OCAP infrastructure: caveat methods are part of the capability security model
// but not yet consumed by runtime enforcement. Retain for OCAP completeness.
#[allow(dead_code)]
impl Caveat {
    /// Create a new caveat
    pub(crate) fn new(caveat_id: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            caveat_id: caveat_id.into(),
            data: data.into(),
        }
    }

    /// Create an expiration caveat
    pub(crate) fn expiration(unix_timestamp: i64) -> Self {
        Self::new("expiration", unix_timestamp.to_string())
    }

    /// Create an operation caveat
    pub(crate) fn operation(operation: impl Into<String>) -> Self {
        Self::new("operation", operation)
    }

    /// Create a template caveat
    pub(crate) fn template(template_id: impl Into<String>) -> Self {
        Self::new("template", template_id)
    }

    /// Create a visibility caveat
    pub(crate) fn visibility(visibility: impl Into<String>) -> Self {
        Self::new("visibility", visibility)
    }
}

/// Capability token for tool access and composition operations
/// Loop: Cybernetics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationToken {
    /// Unique token identifier
    pub id: String,
    /// Resource type (tool, template, manifest, registry, cascade)
    pub resource: DelegationResource,
    /// Resource identifier (e.g., tool name, template ID)
    pub resource_id: String,
    /// Action granted (read, write, execute, render, compose, attenuate)
    pub action: DelegationAction,
    /// WebID that delegated this capability
    pub delegated_from: WebID,
    /// WebID that received this capability
    pub delegated_to: WebID,
    /// Token signature (HMAC over fields)
    pub signature: String,
    /// Expiration timestamp (Unix epoch seconds)
    pub expires_at: Option<i64>,
    /// Attenuation level (0 = full authority, increases with each delegation)
    pub attenuation_level: u8,
    /// Maximum attenuation level allowed (prevents infinite delegation)
    pub max_attenuation: u8,
    /// Context nonce for binding token to specific execution context
    pub context_nonce: String,
    /// Caveats (restrictions on this capability)
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

/// Builder for constructing delegation tokens with the OCAP pattern.
///
/// Each method returns `Self`, so the builder itself is an unforgeable authority
/// that can only be exercised by its holder. No ambient authority is leaked
/// through parameter ordering.
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
    /// Create a new builder with the required fields.
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

    /// Set expiration timestamp.
    pub fn expires_at(mut self, ts: i64) -> Self {
        self.expires_at = Some(ts);
        self
    }

    /// Set attenuation level and max.
    pub fn attenuation(mut self, level: u8, max: u8) -> Self {
        self.attenuation_level = level;
        self.max_attenuation = max;
        self
    }

    /// Set context nonce.
    pub fn context_nonce(mut self, nonce: String) -> Self {
        self.context_nonce = Some(nonce);
        self
    }

    /// Add a caveat.
    pub(crate) fn caveat(mut self, caveat: Caveat) -> Self {
        self.caveats.push(caveat);
        self
    }

    /// Build and sign the delegation token.
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
    /// Create a new delegation token with default settings.
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

    /// Generate unique token ID
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

    /// Sign the token payload using HMAC-SHA256.
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

    /// Verify the token signature using constant-time comparison.
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

    /// Check if token is expired
    pub fn is_expired(&self, current_time: i64) -> bool {
        self.expires_at
            .map(|exp| current_time > exp)
            .unwrap_or(false)
    }

    /// Get the holder (recipient) of this capability token
    pub fn holder(&self) -> WebID {
        self.delegated_to
    }

    /// Get the issuer (delegator) of this capability token
    pub fn issuer(&self) -> WebID {
        self.delegated_from
    }

    /// Serialize token to base64-encoded JSON
    pub fn to_base64(&self) -> Result<String, serde_json::Error> {
        let json = serde_json::to_string(self)?;
        Ok(base64_encode(json.as_bytes()))
    }

    /// Deserialize token from base64-encoded JSON
    pub fn from_base64(encoded: &str) -> Result<Self, String> {
        let json = base64_decode(encoded)?;
        serde_json::from_slice(&json).map_err(|e| e.to_string())
    }

    /// Check if attenuation allows further delegation
    pub fn can_attenuate(&self) -> bool {
        self.attenuation_level < self.max_attenuation
    }

    /// Create attenuated child token for delegation.
    /// Uses a 1-hour expiry from `current_time`.
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
    // OCAP infrastructure: caveat attenuation awaits runtime enforcement
    #[allow(dead_code)]
    pub(crate) fn add_caveat(&self, caveat: Caveat, secret: &[u8]) -> Self {
        let mut new_token = self.clone();
        new_token.caveats.push(caveat);

        // Re-sign with the new caveat included
        let payload = SigningPayload {
            id: new_token.id.clone(),
            resource: new_token.resource,
            resource_id: new_token.resource_id.clone(),
            action: new_token.action,
            from: new_token.delegated_from,
            to: new_token.delegated_to,
            caveats: new_token.caveats.clone(),
        };
        new_token.signature = Self::sign_payload(&payload, secret);

        new_token
    }

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

    /// Access all caveats on this token
    // OCAP infrastructure: caveat access awaits runtime enforcement
    #[allow(dead_code)]
    pub(crate) fn caveats(&self) -> &[Caveat] {
        &self.caveats
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

/// Agent delegation manifest
/// Loop: Cybernetics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDelegation {
    /// Agent's WebID
    pub bot_id: WebID,
    /// List of tool capabilities this agent holds
    pub capabilities: Vec<String>,
}

impl AgentDelegation {
    pub fn new(bot_id: WebID) -> Self {
        Self {
            bot_id,
            capabilities: vec![],
        }
    }

    pub fn with_capabilities(mut self, caps: Vec<&str>) -> Self {
        self.capabilities = caps.into_iter().map(String::from).collect();
        self
    }

    pub fn has_capability(&self, tool_name: &str) -> bool {
        self.capabilities.iter().any(|cap| cap == tool_name)
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
}
