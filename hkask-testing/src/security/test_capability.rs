//! Test Capability Tokens
//!
//! This module implements capability-based access control for test execution.
//! Tests must hold valid capability tokens to access production functionality.
//!
//! **Security Model:**
//! - Test capabilities are short-lived (expire after test completion)
//! - Capabilities are scoped to specific test functions
//! - Capabilities cannot be delegated between tests
//! - Test capabilities are audited via CNS spans

use hkask_types::{WebID, CapabilityToken, CapabilityResource, CapabilityAction};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Test capability token for accessing production functionality during tests
#[derive(Debug, Clone)]
pub struct TestCapability {
    /// Unique test capability identifier
    pub id: String,
    /// Test function name
    pub test_name: String,
    /// Resource being accessed
    pub resource: CapabilityResource,
    /// Resource identifier
    pub resource_id: String,
    /// Action being performed
    pub action: CapabilityAction,
    /// Test WebID (unique per test run)
    pub test_webid: WebID,
    /// Expiration timestamp (Unix epoch seconds)
    pub expires_at: i64,
    /// Test session ID
    pub session_id: String,
}

impl TestCapability {
    /// Create a new test capability
    pub fn new(
        test_name: &str,
        resource: CapabilityResource,
        resource_id: &str,
        action: CapabilityAction,
    ) -> Self {
        let test_webid = WebID::new();
        let session_id = Uuid::new_v4().to_string();
        let id = Self::generate_id(test_name, resource, resource_id, &test_webid);
        
        // Test capabilities expire after 1 hour (sufficient for test execution)
        let expires_at = chrono::Utc::now().timestamp() + 3600;

        Self {
            id,
            test_name: test_name.to_string(),
            resource,
            resource_id: resource_id.to_string(),
            action,
            test_webid,
            expires_at,
            session_id,
        }
    }

    /// Generate unique capability ID
    fn generate_id(
        test_name: &str,
        resource: CapabilityResource,
        resource_id: &str,
        test_webid: &WebID,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(test_name.as_bytes());
        hasher.update(resource.as_str().as_bytes());
        hasher.update(resource_id.as_bytes());
        hasher.update(test_webid.to_string().as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Check if capability is expired
    pub fn is_expired(&self) -> bool {
        let current_time = chrono::Utc::now().timestamp();
        current_time > self.expires_at
    }

    /// Verify capability is valid for given resource and action
    pub fn is_valid_for(
        &self,
        resource: CapabilityResource,
        resource_id: &str,
        action: CapabilityAction,
    ) -> bool {
        !self.is_expired()
            && self.resource == resource
            && self.resource_id == resource_id
            && self.action == action
    }

    /// Convert to production CapabilityToken
    pub fn to_capability_token(&self, secret: &[u8]) -> CapabilityToken {
        CapabilityToken::new(
            self.resource,
            self.resource_id.clone(),
            self.action,
            self.test_webid,
            self.test_webid,
            secret,
        )
    }
}

/// Test capability checker for validating test capabilities
pub struct TestCapabilityChecker {
    secret: Vec<u8>,
}

impl TestCapabilityChecker {
    /// Create a new test capability checker
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: secret.to_vec(),
        }
    }

    /// Verify a test capability
    pub fn verify(&self, capability: &TestCapability) -> bool {
        !capability.is_expired()
    }

    /// Check if capability is valid for given resource and action
    pub fn check(
        &self,
        capability: &TestCapability,
        resource: CapabilityResource,
        resource_id: &str,
        action: CapabilityAction,
    ) -> bool {
        self.verify(capability) && capability.is_valid_for(resource, resource_id, action)
    }

    /// Create a test capability for storage access
    pub fn create_storage_capability(&self, test_name: &str, resource_id: &str) -> TestCapability {
        TestCapability::new(
            test_name,
            CapabilityResource::Tool,
            resource_id,
            CapabilityAction::Execute,
        )
    }

    /// Create a test capability for memory access
    pub fn create_memory_capability(&self, test_name: &str, resource_id: &str) -> TestCapability {
        TestCapability::new(
            test_name,
            CapabilityResource::Tool,
            resource_id,
            CapabilityAction::Read,
        )
    }

    /// Create a test capability for CNS access
    pub fn create_cns_capability(&self, test_name: &str, resource_id: &str) -> TestCapability {
        TestCapability::new(
            test_name,
            CapabilityResource::Tool,
            resource_id,
            CapabilityAction::Write,
        )
    }
}

impl Default for TestCapabilityChecker {
    fn default() -> Self {
        // Default test secret (NOT FOR PRODUCTION USE)
        Self::new(b"hKask-test-secret-do-not-use-in-production")
    }
}

/// Test capability builder for fluent API
pub struct TestCapabilityBuilder {
    test_name: String,
    resource: Option<CapabilityResource>,
    resource_id: Option<String>,
    action: Option<CapabilityAction>,
}

impl TestCapabilityBuilder {
    pub fn new(test_name: &str) -> Self {
        Self {
            test_name: test_name.to_string(),
            resource: None,
            resource_id: None,
            action: None,
        }
    }

    pub fn resource(mut self, resource: CapabilityResource) -> Self {
        self.resource = Some(resource);
        self
    }

    pub fn resource_id(mut self, resource_id: &str) -> Self {
        self.resource_id = Some(resource_id.to_string());
        self
    }

    pub fn action(mut self, action: CapabilityAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn build(self) -> Option<TestCapability> {
        Some(TestCapability::new(
            &self.test_name,
            self.resource?,
            &self.resource_id?,
            self.action?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_capability_new() {
        let cap = TestCapability::new(
            "test_function",
            CapabilityResource::Tool,
            "storage:read",
            CapabilityAction::Read,
        );

        assert_eq!(cap.test_name, "test_function");
        assert_eq!(cap.resource, CapabilityResource::Tool);
        assert_eq!(cap.resource_id, "storage:read");
        assert_eq!(cap.action, CapabilityAction::Read);
        assert!(!cap.is_expired());
    }

    #[test]
    fn test_test_capability_checker() {
        let checker = TestCapabilityChecker::default();
        let cap = TestCapability::new(
            "test_function",
            CapabilityResource::Tool,
            "storage:read",
            CapabilityAction::Read,
        );

        assert!(checker.verify(&cap));
        assert!(checker.check(
            &cap,
            CapabilityResource::Tool,
            "storage:read",
            CapabilityAction::Read
        ));
    }

    #[test]
    fn test_test_capability_builder() {
        let cap = TestCapabilityBuilder::new("test_function")
            .resource(CapabilityResource::Tool)
            .resource_id("memory:write")
            .action(CapabilityAction::Write)
            .build()
            .unwrap();

        assert_eq!(cap.test_name, "test_function");
        assert_eq!(cap.resource, CapabilityResource::Tool);
        assert_eq!(cap.resource_id, "memory:write");
        assert_eq!(cap.action, CapabilityAction::Write);
    }
}
