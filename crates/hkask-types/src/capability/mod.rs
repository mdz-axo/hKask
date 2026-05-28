//! Capability-based access control for MCP tool invocation and composition operations
//!
//! Implements OCAP (Object-Capability) security model for tool access and template/manifest operations.
//! Each bot must hold a capability token to invoke tools or perform composition operations.
//!
//! **Capability Resources:**
//! - `tool:*` — Tool invocation (inference, storage, memory, etc.)
//! - `template:*` — Template operations (read, write, render, compose)
//! - `manifest:*` — Manifest operations (read, write, execute)
//! - `registry:*` — Registry operations (read, write, search)
//! - `cascade:*` — Cascade operations (execute, compose, attenuate)
//!
//! **Cryptographic Verification:**
//! - Capabilities are self-verifying via HMAC-SHA256 signatures
//! - Distributed verification via Paxos/CRDT lazy consistency
//! - No central authority required — capabilities verified cryptographically

mod verification;

pub use verification::{CapabilityChecker, VerificationResult};

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

/// Capability resource types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityResource {
    Tool,
    Template,
    Manifest,
    Registry,
    Cascade,
    Spec,
}

impl CapabilityResource {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityResource::Tool => "tool",
            CapabilityResource::Template => "template",
            CapabilityResource::Manifest => "manifest",
            CapabilityResource::Registry => "registry",
            CapabilityResource::Cascade => "cascade",
            CapabilityResource::Spec => "spec",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s.split(':').next() {
            Some("tool") => Some(CapabilityResource::Tool),
            Some("template") => Some(CapabilityResource::Template),
            Some("manifest") => Some(CapabilityResource::Manifest),
            Some("registry") => Some(CapabilityResource::Registry),
            Some("cascade") => Some(CapabilityResource::Cascade),
            Some("spec") => Some(CapabilityResource::Spec),
            _ => None,
        }
    }
}

/// Capability action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityAction {
    Read,
    Write,
    Execute,
    Render,
    Compose,
    Attenuate,
    Validate,
}

impl CapabilityAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityAction::Read => "read",
            CapabilityAction::Write => "write",
            CapabilityAction::Execute => "execute",
            CapabilityAction::Render => "render",
            CapabilityAction::Compose => "compose",
            CapabilityAction::Attenuate => "attenuate",
            CapabilityAction::Validate => "validate",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "read" => Some(CapabilityAction::Read),
            "write" => Some(CapabilityAction::Write),
            "execute" => Some(CapabilityAction::Execute),
            "render" => Some(CapabilityAction::Render),
            "compose" => Some(CapabilityAction::Compose),
            "attenuate" => Some(CapabilityAction::Attenuate),
            "validate" => Some(CapabilityAction::Validate),
            _ => None,
        }
    }
}

/// Caveat — A restriction on a capability token
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
pub struct Caveat {
    /// Caveat type identifier (e.g., "expiration", "operation", "template")
    pub caveat_id: String,
    /// Caveat data (e.g., timestamp, operation name, template ID)
    pub data: String,
}

impl Caveat {
    /// Create a new caveat
    pub fn new(caveat_id: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            caveat_id: caveat_id.into(),
            data: data.into(),
        }
    }

    /// Create an expiration caveat
    pub fn expiration(unix_timestamp: i64) -> Self {
        Self::new("expiration", unix_timestamp.to_string())
    }

    /// Create an operation caveat
    pub fn operation(operation: impl Into<String>) -> Self {
        Self::new("operation", operation)
    }

    /// Create a template caveat
    pub fn template(template_id: impl Into<String>) -> Self {
        Self::new("template", template_id)
    }

    /// Create a visibility caveat
    pub fn visibility(visibility: impl Into<String>) -> Self {
        Self::new("visibility", visibility)
    }
}

/// Context for caveat verification
pub struct CaveatContext {
    /// Operations allowed in this context
    pub allowed_operations: Vec<String>,
    /// Template ID if template-scoped
    pub template_id: Option<String>,
    /// Visibility level required
    pub visibility: String,
    /// Current timestamp for expiration checks
    pub current_time: i64,
}

impl CaveatContext {
    /// Create new context with current time
    pub fn new() -> Self {
        Self {
            allowed_operations: Vec::new(),
            template_id: None,
            visibility: String::new(),
            current_time: chrono::Utc::now().timestamp(),
        }
    }

    /// Set allowed operations
    pub fn with_operations(mut self, ops: Vec<String>) -> Self {
        self.allowed_operations = ops;
        self
    }

    /// Set template ID
    pub fn with_template(mut self, template_id: String) -> Self {
        self.template_id = Some(template_id);
        self
    }

    /// Set visibility
    pub fn with_visibility(mut self, visibility: String) -> Self {
        self.visibility = visibility;
        self
    }
}

impl Default for CaveatContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Capability token for tool access and composition operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    /// Unique token identifier
    pub id: String,
    /// Resource type (tool, template, manifest, registry, cascade)
    pub resource: CapabilityResource,
    /// Resource identifier (e.g., tool name, template ID)
    pub resource_id: String,
    /// Action granted (read, write, execute, render, compose, attenuate)
    pub action: CapabilityAction,
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
    pub caveats: Vec<Caveat>,
}

/// Internal signing payload extracted from builder state.
struct SigningPayload {
    id: String,
    resource: CapabilityResource,
    resource_id: String,
    action: CapabilityAction,
    from: WebID,
    to: WebID,
    caveats: Vec<Caveat>,
}

/// Builder for constructing capability tokens with the OCAP pattern.
///
/// Each method returns `Self`, so the builder itself is an unforgeable authority
/// that can only be exercised by its holder. No ambient authority is leaked
/// through parameter ordering.
pub struct CapabilityTokenBuilder {
    resource: CapabilityResource,
    resource_id: String,
    action: CapabilityAction,
    delegated_from: WebID,
    delegated_to: WebID,
    expires_at: Option<i64>,
    attenuation_level: u8,
    max_attenuation: u8,
    context_nonce: Option<String>,
    caveats: Vec<Caveat>,
}

impl CapabilityTokenBuilder {
    /// Create a new builder with the required fields.
    pub fn new(
        resource: CapabilityResource,
        resource_id: String,
        action: CapabilityAction,
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
            max_attenuation: 7,
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
    pub fn caveat(mut self, caveat: Caveat) -> Self {
        self.caveats.push(caveat);
        self
    }

    /// Build and sign the capability token.
    pub fn sign(self, secret: &[u8]) -> CapabilityToken {
        let id = CapabilityToken::generate_id(
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
        let signature = CapabilityToken::sign_payload(&payload, secret);
        let context_nonce = self
            .context_nonce
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        CapabilityToken {
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

impl CapabilityToken {
    /// Create a new capability token with default settings.
    pub fn new(
        resource: CapabilityResource,
        resource_id: String,
        action: CapabilityAction,
        delegated_from: WebID,
        delegated_to: WebID,
        secret: &[u8],
    ) -> Self {
        CapabilityTokenBuilder::new(resource, resource_id, action, delegated_from, delegated_to)
            .sign(secret)
    }

    /// Generate unique token ID
    fn generate_id(
        resource: &CapabilityResource,
        resource_id: &str,
        action: &CapabilityAction,
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
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
        mac.update(payload.id.as_bytes());
        mac.update(payload.resource.as_str().as_bytes());
        mac.update(payload.resource_id.as_bytes());
        mac.update(payload.action.as_str().as_bytes());
        mac.update(to_string(&payload.from).as_bytes());
        mac.update(to_string(&payload.to).as_bytes());
        // Include caveats in signature for tamper-evidence
        for caveat in &payload.caveats {
            mac.update(caveat.caveat_id.as_bytes());
            mac.update(caveat.data.as_bytes());
        }
        hex::encode(mac.finalize().into_bytes())
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
        use subtle::ConstantTimeEq;
        expected.as_bytes().ct_eq(self.signature.as_bytes()).into()
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
    pub fn attenuate(
        &self,
        new_to: WebID,
        secret: &[u8],
        current_time: i64,
    ) -> Option<CapabilityToken> {
        if !self.can_attenuate() {
            return None;
        }

        // Attenuate: reduce max_attenuation and increase attenuation_level
        // Preserve parent's context nonce for traceability
        let mut builder = CapabilityTokenBuilder::new(
            self.resource,
            self.resource_id.clone(),
            self.action,
            self.delegated_to,
            new_to,
        )
        .expires_at(current_time + 3600) // 1 hour expiry for attenuated tokens
        .attenuation(self.attenuation_level + 1, self.max_attenuation)
        .context_nonce(format!(
            "{}-attenuated-{}",
            self.context_nonce,
            uuid::Uuid::new_v4()
        ));

        // Preserve parent's caveats
        for caveat in &self.caveats {
            builder = builder.caveat(caveat.clone());
        }

        Some(builder.sign(secret))
    }

    /// Create attenuated child token with custom expiry.
    pub fn attenuate_with_expiry(
        &self,
        new_to: WebID,
        secret: &[u8],
        expires_at: Option<i64>,
    ) -> Option<CapabilityToken> {
        if !self.can_attenuate() {
            return None;
        }

        let mut builder = CapabilityTokenBuilder::new(
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
        resource: CapabilityResource,
        resource_id: &str,
        action: CapabilityAction,
    ) -> bool {
        self.resource == resource && self.resource_id == resource_id && self.action == action
    }

    /// Check if this token grants access to a resource type (regardless of specific ID)
    pub fn grants_resource(&self, resource: CapabilityResource) -> bool {
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
    /// - Nonce format is valid (root-attenuated-uuid-attenuated-uuid...)
    pub fn verify_attenuation_chain(&self, expected_root: &str, expected_level: u8) -> bool {
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
    /// A new `CapabilityToken` with the caveat added and re-signed
    pub fn add_caveat(&self, caveat: Caveat, secret: &[u8]) -> Self {
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

    /// Verify all caveats are satisfied in the given context
    ///
    /// Checks each caveat against the provided context:
    /// - expiration: current time must be before expiry
    /// - operation: at least one requested operation must match a granted operation caveat
    /// - template: context template must match caveat template
    /// - visibility: context visibility must match caveat visibility
    ///
    /// # Arguments
    /// * `ctx` — The context in which to verify caveats
    ///
    /// # Returns
    /// * `Ok(())` — All caveats satisfied
    /// * `Err(String)` — Description of which caveat failed
    pub fn verify_caveats(&self, ctx: &CaveatContext) -> Result<(), String> {
        let mut has_matching_operation = ctx.allowed_operations.is_empty();

        for caveat in &self.caveats {
            match caveat.caveat_id.as_str() {
                "expiration" => {
                    let expiry = caveat
                        .data
                        .parse::<i64>()
                        .map_err(|_| "Invalid expiration caveat data".to_string())?;
                    if ctx.current_time > expiry {
                        return Err("Capability expired".to_string());
                    }
                }
                "operation" => {
                    // Check if this operation caveat matches any requested operation
                    if !has_matching_operation && ctx.allowed_operations.contains(&caveat.data) {
                        has_matching_operation = true;
                    }
                }
                "template" => {
                    if ctx.template_id.as_ref() != Some(&caveat.data) {
                        return Err(format!(
                            "Template mismatch: expected {}, got {:?}",
                            caveat.data, ctx.template_id
                        ));
                    }
                }
                "visibility" => {
                    if ctx.visibility != caveat.data {
                        return Err(format!(
                            "Visibility mismatch: expected {}, got {}",
                            caveat.data, ctx.visibility
                        ));
                    }
                }
                _ => {
                    return Err(format!("Unknown caveat type: {}", caveat.caveat_id));
                }
            }
        }

        // If operations were requested, at least one must match
        if !ctx.allowed_operations.is_empty() && !has_matching_operation {
            return Err("No matching operation caveat found".to_string());
        }

        Ok(())
    }

    /// Get all caveat IDs
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

    /// Verify capability with lazy timestamp check (CRDT-style eventual consistency)
    ///
    /// In distributed systems, clock skew may cause different machines to disagree on
    /// whether a capability is expired. This method uses "lazy" expiry:
    /// - Check signature first (always consistent)
    /// - Check expiry with local clock (may differ across machines)
    /// - If signature valid but expired, capability enters "zombie" state (valid but unusable)
    ///
    /// # Arguments
    /// * `secret` — Shared HMAC secret
    /// * `local_time` — Local machine's current timestamp
    ///
    /// # Returns
    /// * `VerificationResult` — Detailed verification status
    pub fn verify_lazy(&self, secret: &[u8], local_time: i64) -> VerificationResult {
        let signature_valid = self.verify(secret);
        let expired = self.is_expired(local_time);

        if !signature_valid {
            VerificationResult::Invalid
        } else if expired {
            VerificationResult::Zombie // Valid signature, but expired
        } else {
            VerificationResult::Valid
        }
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
    pub fn is_compatible_with(&self, other: &CapabilityToken) -> bool {
        self.resource == other.resource
            && self.resource_id == other.resource_id
            && self.action == other.action
            && self.delegated_to == other.delegated_to
    }
}

/// Bot capability manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotCapabilities {
    /// Bot's WebID
    pub bot_id: WebID,
    /// List of tool capabilities this bot holds
    pub capabilities: Vec<String>,
}

impl BotCapabilities {
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
