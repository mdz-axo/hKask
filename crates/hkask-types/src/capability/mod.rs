//! Delegation tokens (OCAP) — inter-agent capability delegation
//
//! Two token kinds: **Loop authority** (ZST tokens in `tokens.rs`) prove loop-authorized operations;
//! **Delegation** (`DelegationToken`) are HMAC-signed tokens for inter-agent delegation with cryptographic attenuation.

/// Shared structural bound: capability attenuation, cascade depth, subgoal nesting.
pub const SYSTEM_MAX_RECURSION: u8 = 7;

/// Capability-domain alias for SYSTEM_MAX_RECURSION.
pub const SYSTEM_MAX_ATTENUATION: u8 = SYSTEM_MAX_RECURSION;

/// Verified authentication context — the caller's identity and capability token.
///
/// Carries the verified `DelegationToken` and `WebID` of the caller. When
/// provided to service operations, the service uses this identity to derive
/// operation-specific capability tokens (via `CapabilityChecker::grant_*`)
/// instead of minting ad-hoc system-level tokens from config secrets.
///
/// This type lives in the domain crate because it represents a verified
/// identity boundary — not a surface-specific concern. Both API (via
/// middleware verification) and CLI (via keystore secret resolution) produce
/// `AuthContext` through different mechanisms but arrive at the same type.
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// The verified capability token.
    pub token: super::DelegationToken,
    /// The WebID of the token holder.
    pub webid: super::WebID,
}

/// F-SYN-010: typed attenuation level (newtype wrapper around `u8`).
///
/// The inner `u8` is a *system constant* — the absolute maximum is
/// `SYSTEM_MAX_RECURSION = 7` (see FUT-001 for the open question of
/// whether this is a hard constant or a configurable cap). The
/// `new()` constructor enforces the cap; `get()` and `as_u8()` are
/// the only ways to read the inner value.
///
/// New code should use this type instead of raw `u8` for attenuation
/// levels. Existing fields (`DelegationToken.attenuation_level`,
/// `DelegationToken.max_attenuation`) still use `u8` for
/// serde-stability and cross-crate compatibility; migrating them
/// is a separate PR (one finding per PR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AttenuationLevel(u8);

impl AttenuationLevel {
    /// Construct an `AttenuationLevel` from a raw `u8`, enforcing the
    /// system cap. Returns `Err` for any value `> SYSTEM_MAX_RECURSION`.
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

    /// Construct without checking the cap. **For internal use only**
    /// (e.g. deserialisation paths that trust the wire format).
    /// Prefer `new()` for new code.
    pub fn unchecked(level: u8) -> Self {
        Self(level)
    }

    /// The inner value, as a `u8`.
    pub fn as_u8(&self) -> u8 {
        self.0
    }

    /// The system-wide maximum attenuation level.
    pub const fn max() -> u8 {
        SYSTEM_MAX_RECURSION
    }
}

impl std::fmt::Display for AttenuationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Errors from [`AttenuationLevel::new`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AttenuationError {
    /// The supplied level exceeds the system cap (`SYSTEM_MAX_RECURSION = 7`).
    #[error("attenuation level {level} exceeds system maximum {max}")]
    ExceedsSystemMax { level: u8, max: u8 },
}

pub(crate) mod hmac_ops;
pub mod verification;

pub mod tokens;
pub use tokens::{ConsolidationToken, IssuerVerification};

pub use verification::{
    CapabilityChecker, TOKEN_ERR_EXPIRED, TOKEN_ERR_INVALID_SIGNATURE, TOKEN_ERR_NO_CHECKER,
    VerificationOutcome, require_read_access, require_write_access, token_err_insufficient_access,
    token_err_tool_access_denied, verify_delegation_token, verify_delegation_token_now,
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
/// - `tool:cns:execute` covers `tool:cns:write` (same domain, execute ≥ write)
/// - `tool:cns:execute` covers `tool:cns:read` (execute ≥ read)
/// - `tool:cns:read` does **not** cover `tool:cns:execute` (read ≱ execute)
/// - `tool:cns:write` covers `tool:cns:read` (write ≥ read) but not `tool:cns:execute`
/// - `tool:cns:execute` does **not** cover `tool:semantic:execute` (different domain)
///
/// Note: Unknown action strings in capability specs default to `Execute`.
/// For example, `"tool:cns:emit"` parses as `{resource: Tool, resource_id: "cns", action: Execute}`.
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

/// Type alias for spec-code alignment.
///
/// DDMVSS and trust-security-observability.md reference `CapabilityToken`.
/// This alias preserves the spec vocabulary while the code uses `DelegationToken`
/// as the canonical name (changed during implementation for semantic clarity).
///
/// FocusingAssumption FA-T2: This alias exists solely for spec-code alignment.
/// All new code should use `DelegationToken` directly.
pub type CapabilityToken = DelegationToken;
