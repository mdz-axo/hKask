//! Visibility types — OCAP-enforced access control
//!
//! **DEPRECATION NOTICE:** Per architecture v0.21.0 §3.1 §3.2, visibility should be
//! enforced through OCAP capabilities only not a typed enum. This enum is retained
//! temporarily for Phase 1 compatibility but will be removed in Phase 2.
//!
//! **Migration path:** Replace `Visibility` checks with capability delegation verification.

use serde::{Deserialize, Serialize};

/// Visibility level for artifacts
///
/// **Note:** This enum is a temporary placeholder. Per spec visibility should be
/// capability-based only not an enum. See architecture v0.21.0 §3.1.
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

/// OCAP capability for delegation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub resource: String,
    pub action: String,
    pub granted_by: String,
    pub granted_to: String,
}

impl Capability {
    pub fn new(resource: &str, action: &str, granted_by: &str, granted_to: &str) -> Self {
        Self {
            resource: resource.to_string(),
            action: action.to_string(),
            granted_by: granted_by.to_string(),
            granted_to: granted_to.to_string(),
        }
    }

    pub fn matches(&self, resource: &str, action: &str) -> bool {
        self.resource == resource && self.action == action
    }
}

/// OCAP delegation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delegation {
    pub id: String,
    pub capability: Capability,
    pub delegator: String,
    pub delegate: String,
    pub expires_at: Option<i64>,
}

impl Delegation {
    pub fn new(id: &str, capability: Capability, delegator: &str, delegate: &str) -> Self {
        Self {
            id: id.to_string(),
            capability,
            delegator: delegator.to_string(),
            delegate: delegate.to_string(),
            expires_at: None,
        }
    }

    pub fn with_expiry(mut self, timestamp: i64) -> Self {
        self.expires_at = Some(timestamp);
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
}

/// Access control decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessDecision {
    Allow,
    Deny,
}

/// Evaluate access based on visibility and capabilities
///
/// **Security Note:** This function uses string comparison for owner/requester.
/// For production use, replace with WebID-based comparison and cryptographic
/// capability signature verification. See architecture v0.21.0 §1.2.
pub fn evaluate_access(
    visibility: Visibility,
    owner: &str,
    requester: &str,
    capabilities: &[Capability],
    resource: &str,
    action: &str,
) -> AccessDecision {
    // Owner always has access
    if owner == requester {
        return AccessDecision::Allow;
    }

    match visibility {
        Visibility::Public => AccessDecision::Allow,
        Visibility::Private => AccessDecision::Deny,
        Visibility::Shared => {
            // Check if requester has capability
            // **TODO:** Verify capability signatures and delegation chains
            if capabilities
                .iter()
                .any(|cap| cap.matches(resource, action) && cap.granted_to == requester)
            {
                AccessDecision::Allow
            } else {
                AccessDecision::Deny
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_visibility_from_str() {
        assert_eq!(Visibility::parse_str("private"), Some(Visibility::Private));
        assert_eq!(Visibility::parse_str("Public"), Some(Visibility::Public));
        assert_eq!(Visibility::parse_str("invalid"), None);
    }

    #[test]
    fn test_visibility_predicates() {
        assert!(Visibility::Private.is_private());
        assert!(Visibility::Public.is_public());
        assert!(Visibility::Shared.is_shared());
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
    fn test_delegation_expiry() {
        let cap = Capability::new("memory", "read", "alice", "bob");
        let delegation = Delegation::new("del-1", cap, "alice", "bob").with_expiry(1000);

        assert!(!delegation.is_expired(500));
        assert!(delegation.is_expired(1500));
        assert!(delegation.is_valid(500));
        assert!(!delegation.is_valid(1500));
    }

    #[test]
    fn test_evaluate_access_owner() {
        let caps = vec![];
        let result = evaluate_access(
            Visibility::Private,
            "alice",
            "alice",
            &caps,
            "memory",
            "read",
        );
        assert_eq!(result, AccessDecision::Allow);
    }

    #[test]
    fn test_evaluate_access_public() {
        let caps = vec![];
        let result = evaluate_access(Visibility::Public, "alice", "bob", &caps, "memory", "read");
        assert_eq!(result, AccessDecision::Allow);
    }

    #[test]
    fn test_evaluate_access_private() {
        let caps = vec![];
        let result = evaluate_access(Visibility::Private, "alice", "bob", &caps, "memory", "read");
        assert_eq!(result, AccessDecision::Deny);
    }

    #[test]
    fn test_evaluate_access_shared_with_capability() {
        let cap = Capability::new("memory", "read", "alice", "bob");
        let caps = vec![cap];
        let result = evaluate_access(Visibility::Shared, "alice", "bob", &caps, "memory", "read");
        assert_eq!(result, AccessDecision::Allow);
    }

    #[test]
    fn test_evaluate_access_shared_without_capability() {
        let caps = vec![];
        let result = evaluate_access(Visibility::Shared, "alice", "bob", &caps, "memory", "read");
        assert_eq!(result, AccessDecision::Deny);
    }
}
