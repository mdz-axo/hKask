//! Visibility types — OCAP-enforced access control
//!
//! Implements capability-based access control with cryptographic WebID verification.
//! Per architecture v0.21.0 §3.1 §3.2, visibility is enforced through OCAP capabilities
//! with cryptographic delegation chains.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

/// Visibility level for artifacts
///
/// **Note:** This enum is retained for backward compatibility.
/// Primary access control is through OCAP capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    #[default]
    Private,
    Public,
    Shared,
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Private => "private",
            Visibility::Public => "public",
            Visibility::Shared => "shared",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "private" | "Private" => Some(Visibility::Private),
            "public" | "Public" => Some(Visibility::Public),
            "shared" | "Shared" => Some(Visibility::Shared),
            _ => None,
        }
    }

    pub fn is_private(&self) -> bool {
        matches!(self, Visibility::Private)
    }

    pub fn is_public(&self) -> bool {
        matches!(self, Visibility::Public)
    }

    pub fn is_shared(&self) -> bool {
        matches!(self, Visibility::Shared)
    }
}

/// Signature algorithm for capability verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignatureAlgorithm {
    Ed25519,
    HmacSha256,
}

impl SignatureAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            SignatureAlgorithm::Ed25519 => "ed25519",
            SignatureAlgorithm::HmacSha256 => "sha256-hmac",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "ed25519" | "Ed25519" => Some(SignatureAlgorithm::Ed25519),
            "sha256-hmac" | "HmacSha256" => Some(SignatureAlgorithm::HmacSha256),
            _ => None,
        }
    }
}

/// Cryptographic signature for capability verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySignature {
    pub signature: Vec<u8>,
    pub algorithm: SignatureAlgorithm,
    pub signed_by: String,
}

impl CapabilitySignature {
    pub fn new(signature: Vec<u8>, algorithm: SignatureAlgorithm, signed_by: &str) -> Self {
        Self {
            signature,
            algorithm,
            signed_by: signed_by.to_string(),
        }
    }

    pub fn new_ed25519(signature: [u8; 64], signed_by: &str) -> Self {
        Self {
            signature: signature.to_vec(),
            algorithm: SignatureAlgorithm::Ed25519,
            signed_by: signed_by.to_string(),
        }
    }

    pub fn verify(&self, data: &[u8], public_key: &[u8]) -> bool {
        match self.algorithm {
            SignatureAlgorithm::Ed25519 => self.verify_ed25519(data, public_key),
            SignatureAlgorithm::HmacSha256 => self.verify_hmac(data, public_key),
        }
    }

    fn verify_ed25519(&self, data: &[u8], public_key: &[u8]) -> bool {
        if self.signature.len() != 64 || public_key.len() != 32 {
            return false;
        }

        let sig_bytes: [u8; 64] = match self.signature[..64].try_into() {
            Ok(sig) => sig,
            Err(_) => return false,
        };
        let signature = Signature::from_bytes(&sig_bytes);

        let verifying_key = match VerifyingKey::try_from(public_key) {
            Ok(key) => key,
            Err(_) => return false,
        };

        verifying_key.verify(data, &signature).is_ok()
    }

    fn verify_hmac(&self, data: &[u8], key: &[u8]) -> bool {
        use hmac::{Hmac, Mac};
        type HmacSha256 = Hmac<Sha256>;

        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        mac.verify_slice(&self.signature).is_ok()
    }
}

/// OCAP capability for delegation
///
/// **Security:** Signature is required (not optional) to prevent forgery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub resource: String,
    pub action: String,
    pub granted_by: String,
    pub granted_to: String,
    pub signature: CapabilitySignature,
    pub expires_at: Option<i64>,
}

impl Capability {
    pub fn new(resource: &str, action: &str, granted_by: &str, granted_to: &str) -> Self {
        Self {
            resource: resource.to_string(),
            action: action.to_string(),
            granted_by: granted_by.to_string(),
            granted_to: granted_to.to_string(),
            signature: CapabilitySignature::new(vec![], SignatureAlgorithm::Ed25519, granted_by),
            expires_at: None,
        }
    }

    pub fn with_signature(mut self, signature: CapabilitySignature) -> Self {
        self.signature = signature;
        self
    }

    pub fn with_expiry(mut self, timestamp: i64) -> Self {
        self.expires_at = Some(timestamp);
        self
    }

    pub fn matches(&self, resource: &str, action: &str) -> bool {
        self.resource == resource && self.action == action
    }

    pub fn signing_data(&self) -> Vec<u8> {
        format!(
            "{}|{}|{}|{}",
            self.resource, self.action, self.granted_by, self.granted_to
        )
        .into_bytes()
    }

    pub fn verify_signature(&self, public_key: &[u8]) -> bool {
        self.signature.verify(&self.signing_data(), public_key)
    }

    pub fn is_expired(&self, current_time: i64) -> bool {
        self.expires_at
            .map(|exp| current_time > exp)
            .unwrap_or(false)
    }
}

/// OCAP delegation record with cryptographic verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    pub id: String,
    pub capability: Capability,
    pub delegator: String,
    pub delegate: String,
    pub expires_at: Option<i64>,
    pub signature: CapabilitySignature,
    pub parent_delegation: Option<String>,
}

impl Delegation {
    pub fn new(id: &str, capability: Capability, delegator: &str, delegate: &str) -> Self {
        Self {
            id: id.to_string(),
            capability,
            delegator: delegator.to_string(),
            delegate: delegate.to_string(),
            expires_at: None,
            signature: CapabilitySignature::new(vec![], SignatureAlgorithm::Ed25519, delegator),
            parent_delegation: None,
        }
    }

    pub fn with_expiry(mut self, timestamp: i64) -> Self {
        self.expires_at = Some(timestamp);
        self
    }

    pub fn with_signature(mut self, signature: CapabilitySignature) -> Self {
        self.signature = signature;
        self
    }

    pub fn with_parent(mut self, parent_id: &str) -> Self {
        self.parent_delegation = Some(parent_id.to_string());
        self
    }

    pub fn is_expired(&self, current_time: i64) -> bool {
        self.expires_at
            .map(|exp| current_time > exp)
            .unwrap_or(false)
    }

    pub fn is_valid(&self, current_time: i64) -> bool {
        !self.is_expired(current_time)
    }

    pub fn signing_data(&self) -> Vec<u8> {
        format!(
            "{}|{}|{}|{}|{}",
            self.id,
            String::from_utf8_lossy(&self.capability.signing_data()),
            self.delegator,
            self.delegate,
            self.expires_at.unwrap_or(0)
        )
        .into_bytes()
    }

    pub fn verify_signature(&self, public_key: &[u8]) -> bool {
        self.signature.verify(&self.signing_data(), public_key)
    }

    pub fn verify_chain(
        &self,
        public_keys: &std::collections::HashMap<String, Vec<u8>>,
        current_time: i64,
        delegation_store: &DelegationStore,
    ) -> bool {
        if !self.is_valid(current_time) {
            return false;
        }

        if !self.verify_signature(
            public_keys
                .get(&self.delegator)
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
        ) {
            return false;
        }

        if let Some(parent_id) = &self.parent_delegation {
            let parent = delegation_store.get(parent_id);
            if parent.is_none() {
                return false;
            }
            let parent = parent.unwrap();
            if !parent.verify_chain(public_keys, current_time, delegation_store) {
                return false;
            }
            if self.delegator != parent.delegate {
                return false;
            }
        }

        true
    }
}

/// Delegation store for chain verification
#[derive(Debug, Clone, Default)]
pub struct DelegationStore {
    delegations: std::collections::HashMap<String, Delegation>,
}

impl DelegationStore {
    pub fn new() -> Self {
        Self {
            delegations: std::collections::HashMap::new(),
        }
    }

    pub fn add(&mut self, delegation: Delegation) {
        self.delegations.insert(delegation.id.clone(), delegation);
    }

    pub fn get(&self, id: &str) -> Option<&Delegation> {
        self.delegations.get(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<Delegation> {
        self.delegations.remove(id)
    }
}

/// Revocation list for capability/delegation revocation
#[derive(Debug, Clone, Default)]
pub struct RevocationList {
    revoked_ids: std::collections::HashSet<String>,
}

impl RevocationList {
    pub fn new() -> Self {
        Self {
            revoked_ids: std::collections::HashSet::new(),
        }
    }

    pub fn revoke(&mut self, id: &str) {
        self.revoked_ids.insert(id.to_string());
    }

    pub fn is_revoked(&self, id: &str) -> bool {
        self.revoked_ids.contains(id)
    }

    pub fn unrevoke(&mut self, id: &str) {
        self.revoked_ids.remove(id);
    }
}

/// Access control decision with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessDecision {
    pub allowed: bool,
    pub reason: Option<String>,
    pub missing_capabilities: Vec<String>,
}

impl AccessDecision {
    pub fn allow() -> Self {
        Self {
            allowed: true,
            reason: Some("Access granted".to_string()),
            missing_capabilities: vec![],
        }
    }

    pub fn deny(reason: &str, missing: Vec<String>) -> Self {
        Self {
            allowed: false,
            reason: Some(reason.to_string()),
            missing_capabilities: missing,
        }
    }
}

/// Access evaluator with separated responsibilities
pub struct AccessEvaluator {
    public_keys: std::collections::HashMap<String, Vec<u8>>,
    current_time: i64,
    delegation_store: DelegationStore,
    revocation_list: RevocationList,
}

impl AccessEvaluator {
    pub fn new(public_keys: std::collections::HashMap<String, Vec<u8>>, current_time: i64) -> Self {
        Self {
            public_keys,
            current_time,
            delegation_store: DelegationStore::new(),
            revocation_list: RevocationList::new(),
        }
    }

    pub fn with_delegation_store(mut self, store: DelegationStore) -> Self {
        self.delegation_store = store;
        self
    }

    pub fn with_revocation_list(mut self, list: RevocationList) -> Self {
        self.revocation_list = list;
        self
    }

    pub fn evaluate(
        &self,
        visibility: Visibility,
        owner: &str,
        requester: &str,
        capabilities: &[Capability],
        resource: &str,
        action: &str,
    ) -> AccessDecision {
        if owner == requester {
            return AccessDecision::allow();
        }

        match visibility {
            Visibility::Public => AccessDecision::allow(),
            Visibility::Private => AccessDecision::deny("Private resource - owner only", vec![]),
            Visibility::Shared => {
                let mut missing = vec![];
                for cap in capabilities.iter() {
                    if cap.matches(resource, action) && cap.granted_to == requester {
                        if cap.is_expired(self.current_time) {
                            missing.push(format!("capability_expired:{}", cap.granted_by));
                            continue;
                        }
                        if self.revocation_list.is_revoked(&cap.granted_by) {
                            missing.push(format!("capability_revoked:{}", cap.granted_by));
                            continue;
                        }
                        if self.verify_capability_signature(cap) {
                            return AccessDecision::allow();
                        } else {
                            missing.push(format!("invalid_signature:{}", cap.granted_by));
                        }
                    }
                }
                AccessDecision::deny("No valid capability", missing)
            }
        }
    }

    fn verify_capability_signature(&self, cap: &Capability) -> bool {
        self.public_keys
            .get(&cap.granted_by)
            .map(|key| cap.verify_signature(key))
            .unwrap_or(false)
    }

    pub fn verify_delegation_chain(&self, delegation: &Delegation) -> bool {
        delegation.verify_chain(&self.public_keys, self.current_time, &self.delegation_store)
    }
}

/// Evaluate access based on visibility and capabilities with cryptographic verification
///
/// Uses WebID-based comparison and verifies capability signatures.
/// Per architecture v0.21.0 §3.1 §3.2.
///
/// # Arguments
/// * `visibility` - The visibility level of the resource
/// * `owner` - WebID of the resource owner
/// * `requester` - WebID of the requesting party
/// * `capabilities` - List of capabilities to check
/// * `resource` - Resource identifier being accessed
/// * `action` - Action being requested
/// * `public_keys` - Map of WebID to public key for signature verification
/// * `current_time` - Current timestamp for expiry checking (Unix epoch seconds)
///
/// # Returns
/// AccessDecision with allowed status and metadata
#[allow(clippy::too_many_arguments)]
pub fn evaluate_access(
    visibility: Visibility,
    owner: &str,
    requester: &str,
    capabilities: &[Capability],
    resource: &str,
    action: &str,
    public_keys: &std::collections::HashMap<String, Vec<u8>>,
    current_time: i64,
) -> AccessDecision {
    let evaluator = AccessEvaluator::new(public_keys.clone(), current_time);
    evaluator.evaluate(visibility, owner, requester, capabilities, resource, action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use std::collections::HashMap;

    #[test]
    fn test_visibility_default() {
        assert_eq!(Visibility::default(), Visibility::Private);
    }

    #[test]
    fn test_visibility_as_str() {
        assert_eq!(Visibility::Private.as_str(), "private");
        assert_eq!(Visibility::Public.as_str(), "public");
        assert_eq!(Visibility::Shared.as_str(), "shared");
    }

    #[test]
    fn test_signature_algorithm() {
        assert_eq!(SignatureAlgorithm::Ed25519.as_str(), "ed25519");
        assert_eq!(SignatureAlgorithm::HmacSha256.as_str(), "sha256-hmac");
        assert_eq!(
            SignatureAlgorithm::parse_str("ed25519"),
            Some(SignatureAlgorithm::Ed25519)
        );
        assert_eq!(SignatureAlgorithm::parse_str("invalid"), None);
    }

    #[test]
    fn test_capability_new() {
        let cap = Capability::new("memory", "read", "alice", "bob");
        assert_eq!(cap.resource, "memory");
        assert_eq!(cap.action, "read");
        assert_eq!(cap.granted_by, "alice");
        assert_eq!(cap.granted_to, "bob");
    }

    #[test]
    fn test_capability_matches() {
        let cap = Capability::new("memory", "read", "alice", "bob");
        assert!(cap.matches("memory", "read"));
        assert!(!cap.matches("memory", "write"));
        assert!(!cap.matches("storage", "read"));
    }

    #[test]
    fn test_capability_signing_data() {
        let cap = Capability::new("memory", "read", "alice", "bob");
        let data = cap.signing_data();
        assert!(!data.is_empty());
    }

    #[test]
    fn test_capability_with_expiry() {
        let cap = Capability::new("memory", "read", "alice", "bob").with_expiry(1000);
        assert!(!cap.is_expired(500));
        assert!(cap.is_expired(1500));
    }

    #[test]
    fn test_delegation_expiry() {
        let cap = Capability::new("memory", "read", "alice", "bob");
        let delegation = Delegation::new("del-1", cap, "alice", "bob").with_expiry(1000);

        assert!(!delegation.is_expired(500));
        assert!(delegation.is_expired(1500));
        assert!(delegation.is_valid(500));
        assert!(!delegation.is_valid(1500));
    }

    #[test]
    fn test_delegation_chain() {
        let cap = Capability::new("memory", "read", "alice", "bob");
        let delegation = Delegation::new("del-1", cap, "alice", "bob")
            .with_expiry(1000)
            .with_parent("del-parent");

        assert_eq!(delegation.parent_delegation, Some("del-parent".to_string()));
    }

    #[test]
    fn test_delegation_store() {
        let mut store = DelegationStore::new();
        let cap = Capability::new("memory", "read", "alice", "bob");
        let delegation = Delegation::new("del-1", cap, "alice", "bob");

        store.add(delegation.clone());
        assert!(store.get("del-1").is_some());
        assert!(store.get("del-2").is_none());

        store.remove("del-1");
        assert!(store.get("del-1").is_none());
    }

    #[test]
    fn test_revocation_list() {
        let mut list = RevocationList::new();
        assert!(!list.is_revoked("cap-1"));

        list.revoke("cap-1");
        assert!(list.is_revoked("cap-1"));

        list.unrevoke("cap-1");
        assert!(!list.is_revoked("cap-1"));
    }

    #[test]
    fn test_access_decision() {
        let allow = AccessDecision::allow();
        assert!(allow.allowed);

        let deny = AccessDecision::deny("test", vec!["missing".to_string()]);
        assert!(!deny.allowed);
        assert_eq!(deny.reason, Some("test".to_string()));
        assert_eq!(deny.missing_capabilities, vec!["missing".to_string()]);
    }

    #[test]
    fn test_access_evaluator_owner() {
        let caps = vec![];
        let public_keys = HashMap::new();
        let evaluator = AccessEvaluator::new(public_keys, 0);
        let result = evaluator.evaluate(
            Visibility::Private,
            "alice",
            "alice",
            &caps,
            "memory",
            "read",
        );
        assert!(result.allowed);
    }

    #[test]
    fn test_access_evaluator_public() {
        let caps = vec![];
        let public_keys = HashMap::new();
        let evaluator = AccessEvaluator::new(public_keys, 0);
        let result =
            evaluator.evaluate(Visibility::Public, "alice", "bob", &caps, "memory", "read");
        assert!(result.allowed);
    }

    #[test]
    fn test_access_evaluator_private() {
        let caps = vec![];
        let public_keys = HashMap::new();
        let evaluator = AccessEvaluator::new(public_keys, 0);
        let result =
            evaluator.evaluate(Visibility::Private, "alice", "bob", &caps, "memory", "read");
        assert!(!result.allowed);
    }

    #[test]
    fn test_access_evaluator_shared_without_capability() {
        let caps = vec![];
        let public_keys = HashMap::new();
        let evaluator = AccessEvaluator::new(public_keys, 0);
        let result =
            evaluator.evaluate(Visibility::Shared, "alice", "bob", &caps, "memory", "read");
        assert!(!result.allowed);
    }

    #[test]
    fn test_access_evaluator_shared_with_ed25519_signature() {
        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key().to_bytes();

        let mut cap = Capability::new("memory", "read", "alice", "bob");
        let signing_data = cap.signing_data();
        let signature = signing_key.sign(&signing_data).to_bytes();

        cap.signature = CapabilitySignature::new_ed25519(signature, "alice");

        let caps = vec![cap];
        let mut public_keys = HashMap::new();
        public_keys.insert("alice".to_string(), verifying_key.to_vec());

        let evaluator = AccessEvaluator::new(public_keys, 0);
        let result =
            evaluator.evaluate(Visibility::Shared, "alice", "bob", &caps, "memory", "read");
        assert!(result.allowed);
    }

    #[test]
    fn test_access_evaluator_shared_with_expired_capability() {
        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key().to_bytes();

        let mut cap = Capability::new("memory", "read", "alice", "bob").with_expiry(1000);
        let signing_data = cap.signing_data();
        let signature = signing_key.sign(&signing_data).to_bytes();

        cap.signature = CapabilitySignature::new_ed25519(signature, "alice");

        let caps = vec![cap];
        let mut public_keys = HashMap::new();
        public_keys.insert("alice".to_string(), verifying_key.to_vec());

        let evaluator = AccessEvaluator::new(public_keys, 2000); // Time after expiry
        let result =
            evaluator.evaluate(Visibility::Shared, "alice", "bob", &caps, "memory", "read");
        assert!(!result.allowed);
        assert!(
            result
                .missing_capabilities
                .iter()
                .any(|s| s.contains("expired"))
        );
    }

    #[test]
    fn test_access_evaluator_shared_with_revoked_capability() {
        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key().to_bytes();

        let mut cap = Capability::new("memory", "read", "alice", "bob");
        let signing_data = cap.signing_data();
        let signature = signing_key.sign(&signing_data).to_bytes();

        cap.signature = CapabilitySignature::new_ed25519(signature, "alice");

        let caps = vec![cap];
        let mut public_keys = HashMap::new();
        public_keys.insert("alice".to_string(), verifying_key.to_vec());

        let mut revocation_list = RevocationList::new();
        revocation_list.revoke("alice");

        let evaluator = AccessEvaluator::new(public_keys, 0).with_revocation_list(revocation_list);
        let result =
            evaluator.evaluate(Visibility::Shared, "alice", "bob", &caps, "memory", "read");
        assert!(!result.allowed);
        assert!(
            result
                .missing_capabilities
                .iter()
                .any(|s| s.contains("revoked"))
        );
    }

    #[test]
    fn test_ed25519_signature_verification() {
        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key().to_bytes();

        let data = b"test data";
        let signature = signing_key.sign(data);

        let cap_sig = CapabilitySignature::new_ed25519(signature.to_bytes(), "test");
        assert!(cap_sig.verify(data, &verifying_key.to_vec()));

        let bad_data = b"bad data";
        assert!(!cap_sig.verify(bad_data, &verifying_key.to_vec()));
    }

    #[test]
    fn test_hmac_signature_verification() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let key = b"test_secret_key";
        let data = b"test data";

        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        let signature = mac.finalize().into_bytes().to_vec();

        let cap_sig =
            CapabilitySignature::new(signature.clone(), SignatureAlgorithm::HmacSha256, "test");
        assert!(cap_sig.verify(data, key));

        let bad_data = b"bad data";
        assert!(!cap_sig.verify(bad_data, key));
    }
}
