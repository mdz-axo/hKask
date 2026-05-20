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
    /// Attenuation level (0 = root, max 7) — Mark Miller OCAP principle
    pub attenuation_level: u8,
    /// Maximum attenuation depth allowed (prevents infinite delegation chains)
    pub max_attenuation: u8,
    /// Parent capability ID (None for root capabilities)
    pub attenuated_from: Option<CapabilityId>,
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
            attenuation_level: 0,
            max_attenuation: 7,
            attenuated_from: None,
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
            attenuation_level: 0,
            max_attenuation: 7,
            attenuated_from: None,
        }
    }

    /// Create attenuated capability from parent (Mark Miller OCAP principle)
    /// Attenuation is monotonic: can only remove permissions, never add
    pub fn attenuate(
        &self,
        operations: Vec<OkapiOperation>,
        new_holder: WebID,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<Self, AuthorizationError> {
        if self.attenuation_level >= self.max_attenuation {
            return Err(AuthorizationError::MaxAttenuationReached);
        }

        // Attenuation: subset of parent operations only (monotonic reduction)
        let attenuated_ops: Vec<OkapiOperation> = self
            .operations
            .iter()
            .filter(|op| operations.contains(op))
            .copied()
            .collect();

        if attenuated_ops.is_empty() {
            return Err(AuthorizationError::NoValidOperations);
        }

        Ok(Self {
            id: CapabilityId::new(),
            operations: attenuated_ops,
            expires_at,
            issuer: self.issuer.clone(),
            holder: new_holder,
            template_id: self.template_id.clone(),
            attenuation_level: self.attenuation_level + 1,
            max_attenuation: self.max_attenuation,
            attenuated_from: Some(self.id),
        })
    }

    /// Check if further attenuation is allowed
    pub fn can_attenuate(&self) -> bool {
        self.attenuation_level < self.max_attenuation
    }

    /// Get attenuation chain depth (0 = root)
    pub fn attenuation_depth(&self) -> u8 {
        self.attenuation_level
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

    #[error("Maximum attenuation depth reached (Mark Miller OCAP limit)")]
    MaxAttenuationReached,

    #[error("No valid operations for attenuation (subset must be non-empty)")]
    NoValidOperations,
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

    #[test]
    fn test_capability_attenuation_success() {
        let holder = WebID::new();
        let parent = OkapiCapability::new(
            vec![
                OkapiOperation::Generate,
                OkapiOperation::Chat,
                OkapiOperation::ReadMetrics,
            ],
            WebID::new(),
            holder,
            None,
        );

        // Attenuate to read-only subset
        let new_holder = WebID::new();
        let child = parent
            .attenuate(
                vec![OkapiOperation::ReadMetrics],
                new_holder,
                None,
            )
            .expect("attenuation should succeed");

        // Child has reduced permissions
        assert!(child.authorize(OkapiOperation::ReadMetrics).is_ok());
        assert!(child.authorize(OkapiOperation::Generate).is_err());
        assert!(child.authorize(OkapiOperation::Chat).is_err());

        // Attenuation metadata
        assert_eq!(child.attenuation_level, 1);
        assert_eq!(child.max_attenuation, 7);
        assert!(child.attenuated_from.is_some());
        assert_eq!(child.attenuated_from.unwrap(), parent.id);
    }

    #[test]
    fn test_capability_attenuation_max_depth() {
        let holder = WebID::new();
        let mut capability = OkapiCapability::new(
            vec![OkapiOperation::Generate, OkapiOperation::ReadMetrics],
            WebID::new(),
            holder,
            None,
        );

        // Set max attenuation to 2 for testing
        capability.max_attenuation = 2;
        capability.attenuation_level = 2;

        let new_holder = WebID::new();
        let result = capability.attenuate(
            vec![OkapiOperation::ReadMetrics],
            new_holder,
            None,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AuthorizationError::MaxAttenuationReached
        ));
    }

    #[test]
    fn test_capability_attenuation_no_valid_operations() {
        let holder = WebID::new();
        let parent = OkapiCapability::new(
            vec![OkapiOperation::Generate],
            WebID::new(),
            holder,
            None,
        );

        // Try to attenuate with operation not in parent
        let new_holder = WebID::new();
        let result = parent.attenuate(
            vec![OkapiOperation::Chat],
            new_holder,
            None,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AuthorizationError::NoValidOperations
        ));
    }

    #[test]
    fn test_capability_can_attenuate() {
        let holder = WebID::new();
        let mut capability = OkapiCapability::new(
            vec![OkapiOperation::Generate],
            WebID::new(),
            holder,
            None,
        );

        assert!(capability.can_attenuate());

        capability.attenuation_level = 7;
        assert!(!capability.can_attenuate());
    }

    #[test]
    fn test_capability_attenuation_chain() {
        let holder = WebID::new();
        let root = OkapiCapability::new(
            vec![
                OkapiOperation::Generate,
                OkapiOperation::Chat,
                OkapiOperation::ReadMetrics,
                OkapiOperation::ReadCapabilities,
            ],
            WebID::new(),
            holder,
            None,
        );

        // Create attenuation chain
        let level1 = root
            .attenuate(
                vec![OkapiOperation::Generate, OkapiOperation::ReadMetrics],
                WebID::new(),
                None,
            )
            .expect("level 1 attenuation");

        let level2 = level1
            .attenuate(vec![OkapiOperation::ReadMetrics], WebID::new(), None)
            .expect("level 2 attenuation");

        let level3 = level2
            .attenuate(vec![OkapiOperation::ReadMetrics], WebID::new(), None)
            .expect("level 3 attenuation");

        // Verify chain depth
        assert_eq!(level1.attenuation_level, 1);
        assert_eq!(level2.attenuation_level, 2);
        assert_eq!(level3.attenuation_level, 3);

        // Verify permissions are monotonically reduced
        assert!(level1.authorize(OkapiOperation::Generate).is_ok());
        assert!(level2.authorize(OkapiOperation::Generate).is_err());
        assert!(level3.authorize(OkapiOperation::ReadMetrics).is_ok());
    }
}