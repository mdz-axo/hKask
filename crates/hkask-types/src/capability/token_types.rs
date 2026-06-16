//! Delegation token types — Ed25519-signed OCAP tokens with cryptographic attenuation.

use crate::WebID;
use crate::wallet::Ed25519PublicKey;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::resources::{DelegationAction, DelegationResource};

/// Shared structural bound: capability attenuation, cascade depth, subgoal nesting.
pub const SYSTEM_MAX_RECURSION: u8 = 7;

/// Capability-domain alias for SYSTEM_MAX_RECURSION.
pub const SYSTEM_MAX_ATTENUATION: u8 = SYSTEM_MAX_RECURSION;

/// [NORMATIVE] Typed attenuation level (0..SYSTEM_MAX_RECURSION). New code should use this over raw `u8`. (P5 — Essentialism).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct AttenuationLevel(u8);

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
pub(crate) enum AttenuationError {
    #[error("attenuation level {level} exceeds system maximum {max}")]
    ExceedsSystemMax { level: u8, max: u8 },
}

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

/// Additive restrictions on a capability token.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Caveat {
    pub caveat_id: String,
    pub data: String,
}

/// Ed25519 signature for delegation token authentication.
///
/// [NORMATIVE] Wraps a 64-byte Ed25519 signature. Verification uses the
/// token's `public_key` field — no shared secret required (P4 — Clear Boundaries).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct TokenSignature(#[serde(with = "hex::serde")] pub [u8; 64]);

/// Ed25519-signed OCAP token for inter-agent capability delegation.
///
/// [NORMATIVE] Signatures are asymmetric (Ed25519) — the issuer signs with
/// a private key, verifiers use the public key. Token forgery requires the
/// private key (P4 — Clear Boundaries).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationToken {
    pub id: String,
    pub resource: DelegationResource,
    pub resource_id: String,
    pub action: DelegationAction,
    pub delegated_from: WebID,
    pub delegated_to: WebID,
    /// Ed25519 signature over the token payload.
    pub signature: TokenSignature,
    /// Ed25519 public key for signature verification.
    pub public_key: Ed25519PublicKey,
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
    public_key: Ed25519PublicKey,
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
    signing_key: SigningKey,
}

impl DelegationTokenBuilder {
    pub fn new(
        resource: DelegationResource,
        resource_id: String,
        action: DelegationAction,
        delegated_from: WebID,
        delegated_to: WebID,
        signing_key: &SigningKey,
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
            signing_key: signing_key.clone(),
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
    pub fn sign(self) -> DelegationToken {
        let id = DelegationToken::generate_id(
            &self.resource,
            &self.resource_id,
            &self.action,
            &self.delegated_from,
            &self.delegated_to,
        );
        let public_key = Ed25519PublicKey(self.signing_key.verifying_key().to_bytes());
        let payload = SigningPayload {
            id,
            resource: self.resource,
            resource_id: self.resource_id,
            action: self.action,
            from: self.delegated_from,
            to: self.delegated_to,
            public_key,
            caveats: self.caveats,
        };
        let signature = DelegationToken::sign_payload(&payload, &self.signing_key);
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
            public_key,
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
        signing_key: &SigningKey,
    ) -> Self {
        DelegationTokenBuilder::new(
            resource,
            resource_id,
            action,
            delegated_from,
            delegated_to,
            signing_key,
        )
        .sign()
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

    fn sign_payload(payload: &SigningPayload, signing_key: &SigningKey) -> TokenSignature {
        // Build canonical byte representation for signing
        let mut buf = Vec::new();
        buf.extend_from_slice(payload.id.as_bytes());
        buf.extend_from_slice(payload.resource.as_str().as_bytes());
        buf.extend_from_slice(payload.resource_id.as_bytes());
        buf.extend_from_slice(payload.action.as_str().as_bytes());
        buf.extend_from_slice(wid(&payload.from).as_bytes());
        buf.extend_from_slice(wid(&payload.to).as_bytes());
        buf.extend_from_slice(&payload.public_key.0);
        for caveat in &payload.caveats {
            buf.extend_from_slice(caveat.caveat_id.as_bytes());
            buf.extend_from_slice(caveat.data.as_bytes());
        }
        let signature = signing_key.sign(&buf);
        TokenSignature(signature.to_bytes())
    }

    /// Ed25519 signature verification using the token's public key.
    pub fn verify(&self) -> bool {
        let verifying_key = match VerifyingKey::from_bytes(&self.public_key.0) {
            Ok(vk) => vk,
            Err(_) => return false,
        };
        let mut buf = Vec::new();
        buf.extend_from_slice(self.id.as_bytes());
        buf.extend_from_slice(self.resource.as_str().as_bytes());
        buf.extend_from_slice(self.resource_id.as_bytes());
        buf.extend_from_slice(self.action.as_str().as_bytes());
        buf.extend_from_slice(wid(&self.delegated_from).as_bytes());
        buf.extend_from_slice(wid(&self.delegated_to).as_bytes());
        buf.extend_from_slice(&self.public_key.0);
        for caveat in &self.caveats {
            buf.extend_from_slice(caveat.caveat_id.as_bytes());
            buf.extend_from_slice(caveat.data.as_bytes());
        }
        let signature = ed25519_dalek::Signature::from_bytes(&self.signature.0);
        verifying_key.verify(&buf, &signature).is_ok()
    }

    /// Raw Ed25519 signature bytes (64 bytes).
    pub fn signature_bytes(&self) -> &[u8; 64] {
        &self.signature.0
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
    /// Requires the issuer's signing key to produce a valid signature.
    pub fn attenuate(
        &self,
        new_to: WebID,
        signing_key: &SigningKey,
        current_time: i64,
    ) -> Option<DelegationToken> {
        self.attenuate_with_expiry(new_to, signing_key, Some(current_time + 3600))
    }

    /// Create attenuated child token with custom expiry.
    pub fn attenuate_with_expiry(
        &self,
        new_to: WebID,
        signing_key: &SigningKey,
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
            signing_key,
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

        for caveat in &self.caveats {
            builder = builder.caveat(caveat.clone());
        }

        Some(builder.sign())
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
    pub fn verify_cryptographic(&self) -> bool {
        self.verify()
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
