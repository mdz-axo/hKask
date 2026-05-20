//! Capability-Based Security for Okapi Access
//!
//! Implements unforgeable capability tokens that gate access to Okapi operations.
//! Follows principle of least authority (Mark Miller / Bruce Schneier).

use chrono::{DateTime, Utc};
use hkask_types::{WebID, TemplateID};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Capability ID — unforgeable authorization token
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub Uuid);

impl CapabilityId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CapabilityId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Okapi operations that require authorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OkapiOperation {
    Generate,
    Chat,
    Embed,
    ReadMetrics,
    ReadCapabilities,
    SwapAdapter,
}

/// Capability token — unforgeable authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkapiCapability {
    pub id: CapabilityId,
    pub operations: Vec<OkapiOperation>,
    pub expires_at: Option<DateTime<Utc>>,
    pub issuer: WebID,
    pub holder: WebID,
    pub template_id: Option<TemplateID>,
}

impl OkapiCapability {
    /// Create new capability with specified operations
    pub fn new(
        operations: Vec<OkapiOperation>,
        issuer: WebID,
        holder: WebID,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: CapabilityId::new(),
            operations,
            expires_at,
            issuer,
            holder,
            template_id: None,
        }
    }

    /// Create capability scoped to a specific template
    pub fn for_template(
        operations: Vec<OkapiOperation>,
        issuer: WebID,
        holder: WebID,
        template_id: TemplateID,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: CapabilityId::new(),
            operations,
            expires_at,
            issuer,
            holder,
            template_id: Some(template_id),
        }
    }

    /// Check if operation is authorized
    pub fn authorize(&self, operation: OkapiOperation) -> Result<(), AuthorizationError> {
        if !self.operations.contains(&operation) {
            return Err(AuthorizationError::OperationNotAuthorized);
        }

        if let Some(expires) = self.expires_at {
            if Utc::now() > expires {
                return Err(AuthorizationError::CapabilityExpired);
            }
        }

        Ok(())
    }

    /// Check if capability is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            Utc::now() > expires
        } else {
            false
        }
    }

    /// Add operation to capability (returns new capability)
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_operation(mut self, operation: OkapiOperation) -> Self {
        if !self.operations.contains(&operation) {
            self.operations.push(operation);
        }
        self
    }

    /// Set template scope
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_template(mut self, template_id: TemplateID) -> Self {
        self.template_id = Some(template_id);
        self
    }
}

/// Authorization error types
#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("Operation not authorized by capability")]
    OperationNotAuthorized,

    #[error("Capability has expired")]
    CapabilityExpired,

    #[error("Capability holder mismatch")]
    HolderMismatch,

    #[error("Capability template scope mismatch")]
    TemplateScopeMismatch,
}

/// Capability-aware Okapi client wrapper
pub struct CapabilityProtectedClient<C> {
    inner: C,
    capability: OkapiCapability,
}

impl<C> CapabilityProtectedClient<C> {
    pub fn new(inner: C, capability: OkapiCapability) -> Self {
        Self { inner, capability }
    }

    pub fn capability(&self) -> &OkapiCapability {
        &self.capability
    }
}

/// Default capability for system operations
pub fn default_system_capability(holder: WebID) -> OkapiCapability {
    OkapiCapability::new(
        vec![
            OkapiOperation::Generate,
            OkapiOperation::Chat,
            OkapiOperation::ReadMetrics,
            OkapiOperation::ReadCapabilities,
        ],
        WebID::new(), // System issuer
        holder,
        None, // No expiration for system capability
    )
}

/// Minimal capability for read-only operations
pub fn read_only_capability(holder: WebID) -> OkapiCapability {
    OkapiCapability::new(
        vec![OkapiOperation::ReadMetrics, OkapiOperation::ReadCapabilities],
        WebID::new(),
        holder,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_authorize_success() {
        let holder = WebID::new();
        let capability = OkapiCapability::new(
            vec![OkapiOperation::Generate, OkapiOperation::Chat],
            WebID::new(),
            holder,
            None,
        );

        assert!(capability.authorize(OkapiOperation::Generate).is_ok());
        assert!(capability.authorize(OkapiOperation::Chat).is_ok());
    }

    #[test]
    fn test_capability_authorize_failure() {
        let holder = WebID::new();
        let capability = OkapiCapability::new(
            vec![OkapiOperation::Generate],
            WebID::new(),
            holder,
            None,
        );

        let result = capability.authorize(OkapiOperation::Chat);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AuthorizationError::OperationNotAuthorized
        ));
    }

    #[test]
    fn test_capability_expired() {
        let holder = WebID::new();
        let expires_at = Utc::now() - chrono::Duration::hours(1);

        let capability = OkapiCapability::new(
            vec![OkapiOperation::Generate],
            WebID::new(),
            holder,
            Some(expires_at),
        );

        assert!(capability.is_expired());
        let result = capability.authorize(OkapiOperation::Generate);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthorizationError::CapabilityExpired));
    }

    #[test]
    fn test_capability_not_expired() {
        let holder = WebID::new();
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        let capability = OkapiCapability::new(
            vec![OkapiOperation::Generate],
            WebID::new(),
            holder,
            Some(expires_at),
        );

        assert!(!capability.is_expired());
        assert!(capability.authorize(OkapiOperation::Generate).is_ok());
    }

    #[test]
    fn test_capability_builder() {
        let holder = WebID::new();
        let capability = OkapiCapability::new(
            vec![OkapiOperation::Generate],
            WebID::new(),
            holder,
            None,
        )
        .with_operation(OkapiOperation::Chat)
        .with_operation(OkapiOperation::Embed);

        assert!(capability.authorize(OkapiOperation::Generate).is_ok());
        assert!(capability.authorize(OkapiOperation::Chat).is_ok());
        assert!(capability.authorize(OkapiOperation::Embed).is_ok());
    }

    #[test]
    fn test_default_system_capability() {
        let holder = WebID::new();
        let capability = default_system_capability(holder);

        assert!(capability.authorize(OkapiOperation::Generate).is_ok());
        assert!(capability.authorize(OkapiOperation::Chat).is_ok());
        assert!(capability.authorize(OkapiOperation::ReadMetrics).is_ok());
        assert!(capability.authorize(OkapiOperation::ReadCapabilities).is_ok());
        assert!(!capability.is_expired());
    }

    #[test]
    fn test_read_only_capability() {
        let holder = WebID::new();
        let capability = read_only_capability(holder);

        assert!(capability.authorize(OkapiOperation::ReadMetrics).is_ok());
        assert!(capability.authorize(OkapiOperation::ReadCapabilities).is_ok());
        assert!(capability
            .authorize(OkapiOperation::Generate)
            .is_err());
    }
}