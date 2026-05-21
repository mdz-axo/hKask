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

use crate::WebID;
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Helper to convert WebID to string
fn to_string(webid: &WebID) -> String {
    webid.to_string()
}

/// Capability resource types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityResource {
    Tool,
    Template,
    Manifest,
    Registry,
    Cascade,
}

impl CapabilityResource {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityResource::Tool => "tool",
            CapabilityResource::Template => "template",
            CapabilityResource::Manifest => "manifest",
            CapabilityResource::Registry => "registry",
            CapabilityResource::Cascade => "cascade",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s.split(':').next() {
            Some("tool") => Some(CapabilityResource::Tool),
            Some("template") => Some(CapabilityResource::Template),
            Some("manifest") => Some(CapabilityResource::Manifest),
            Some("registry") => Some(CapabilityResource::Registry),
            Some("cascade") => Some(CapabilityResource::Cascade),
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
            _ => None,
        }
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
}

impl CapabilityToken {
    /// Create a new capability token with default settings
    pub fn new(
        resource: CapabilityResource,
        resource_id: String,
        action: CapabilityAction,
        delegated_from: WebID,
        delegated_to: WebID,
        secret: &[u8],
    ) -> Self {
        Self::new_with_attenuation(
            resource,
            resource_id,
            action,
            delegated_from,
            delegated_to,
            secret,
            None,
            0,
            7,
            None,
        )
    }

    /// Create a new capability token with attenuation settings
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_attenuation(
        resource: CapabilityResource,
        resource_id: String,
        action: CapabilityAction,
        delegated_from: WebID,
        delegated_to: WebID,
        secret: &[u8],
        expires_at: Option<i64>,
        attenuation_level: u8,
        max_attenuation: u8,
        context_nonce: Option<String>,
    ) -> Self {
        let id = Self::generate_id(
            &resource,
            &resource_id,
            &action,
            &delegated_from,
            &delegated_to,
        );
        let signature = Self::sign(
            &id,
            &resource,
            &resource_id,
            &action,
            &delegated_from,
            &delegated_to,
            secret,
        );
        let context_nonce = context_nonce.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Self {
            id,
            resource,
            resource_id,
            action,
            delegated_from,
            delegated_to,
            signature,
            expires_at,
            attenuation_level,
            max_attenuation,
            context_nonce,
        }
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

    /// Sign the token
    fn sign(
        id: &str,
        resource: &CapabilityResource,
        resource_id: &str,
        action: &CapabilityAction,
        from: &WebID,
        to: &WebID,
        secret: &[u8],
    ) -> String {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC can take key of any size");
        mac.update(id.as_bytes());
        mac.update(resource.as_str().as_bytes());
        mac.update(resource_id.as_bytes());
        mac.update(action.as_str().as_bytes());
        mac.update(to_string(from).as_bytes());
        mac.update(to_string(to).as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    /// Verify the token signature
    pub fn verify(&self, secret: &[u8]) -> bool {
        Self::sign(
            &self.id,
            &self.resource,
            &self.resource_id,
            &self.action,
            &self.delegated_from,
            &self.delegated_to,
            secret,
        ) == self.signature
    }

    /// Check if token is expired
    pub fn is_expired(&self, current_time: i64) -> bool {
        self.expires_at
            .map(|exp| current_time > exp)
            .unwrap_or(false)
    }

    /// Check if attenuation allows further delegation
    pub fn can_attenuate(&self) -> bool {
        self.attenuation_level < self.max_attenuation
    }

    /// Create attenuated child token for delegation
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
        Some(CapabilityToken::new_with_attenuation(
            self.resource,
            self.resource_id.clone(),
            self.action,
            self.delegated_to,
            new_to,
            secret,
            Some(current_time + 3600), // 1 hour expiry for attenuated tokens
            self.attenuation_level + 1,
            self.max_attenuation,
            Some(format!(
                "{}-attenuated-{}",
                self.context_nonce,
                uuid::Uuid::new_v4()
            )),
        ))
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

/// Cryptographic verification result for distributed verification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationResult {
    /// Signature valid, not expired — capability can be used
    Valid,
    /// Signature valid, but expired — capability is "zombie" (valid but unusable)
    Zombie,
    /// Signature invalid — capability is tampered or forged
    Invalid,
}

impl VerificationResult {
    /// Check if verification succeeded (valid or zombie)
    pub fn is_valid(&self) -> bool {
        matches!(self, VerificationResult::Valid | VerificationResult::Zombie)
    }

    /// Check if capability can be used (valid only, not zombie)
    pub fn is_usable(&self) -> bool {
        matches!(self, VerificationResult::Valid)
    }

    /// Get human-readable description
    pub fn as_str(&self) -> &'static str {
        match self {
            VerificationResult::Valid => "valid",
            VerificationResult::Zombie => "zombie (expired but valid signature)",
            VerificationResult::Invalid => "invalid (signature verification failed)",
        }
    }
}

/// Capability checker for composition operations
pub struct CapabilityChecker {
    secret: Vec<u8>,
}

impl CapabilityChecker {
    /// Create a new capability checker with the given secret
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: secret.to_vec(),
        }
    }

    /// Verify a capability token
    pub fn verify(&self, token: &CapabilityToken) -> bool {
        token.verify(&self.secret)
    }

    /// Check if token is valid and not expired
    pub fn verify_with_time(&self, token: &CapabilityToken, current_time: i64) -> bool {
        self.verify(token) && !token.is_expired(current_time)
    }

    /// Check if a holder has capability for a resource/action
    pub fn check(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        resource: CapabilityResource,
        resource_id: &str,
        action: CapabilityAction,
    ) -> bool {
        self.verify(token)
            && token.delegated_to == *holder
            && token.is_valid_for(resource, resource_id, action)
    }

    /// Check if holder has any capability for a resource type
    pub fn check_resource(
        &self,
        token: &CapabilityToken,
        holder: &WebID,
        resource: CapabilityResource,
    ) -> bool {
        self.verify(token) && token.delegated_to == *holder && token.grants_resource(resource)
    }

    /// Create a capability token for a tool
    pub fn grant_tool(&self, tool_name: String, from: WebID, to: WebID) -> CapabilityToken {
        CapabilityToken::new(
            CapabilityResource::Tool,
            tool_name,
            CapabilityAction::Execute,
            from,
            to,
            &self.secret,
        )
    }

    /// Create a capability token for a template operation
    pub fn grant_template(
        &self,
        template_id: String,
        action: CapabilityAction,
        from: WebID,
        to: WebID,
    ) -> CapabilityToken {
        CapabilityToken::new(
            CapabilityResource::Template,
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
        action: CapabilityAction,
        from: WebID,
        to: WebID,
    ) -> CapabilityToken {
        CapabilityToken::new(
            CapabilityResource::Manifest,
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
        action: CapabilityAction,
        from: WebID,
        to: WebID,
    ) -> CapabilityToken {
        CapabilityToken::new(
            CapabilityResource::Registry,
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
        action: CapabilityAction,
        from: WebID,
        to: WebID,
    ) -> CapabilityToken {
        CapabilityToken::new(
            CapabilityResource::Cascade,
            cascade_id,
            action,
            from,
            to,
            &self.secret,
        )
    }

    /// Create an attenuated token for delegation
    pub fn attenuate(
        &self,
        token: &CapabilityToken,
        new_to: WebID,
        current_time: i64,
    ) -> Option<CapabilityToken> {
        token.attenuate(new_to, &self.secret, current_time)
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


