//! Capability-Based Security for Okapi Access
//!
//! Implements Macaroon-backed capability tokens for unforgeable authorization.
//! Follows principle of least authority (Mark Miller / Bruce Schneier).

use chrono::{DateTime, Utc};
use hkask_types::{TemplateID, Visibility, WebID};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

use crate::macaroon::{Macaroon, MacaroonBuilder, MacaroonError};

/// Capability ID — unforgeable authorization token
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub uuid::Uuid);

impl CapabilityId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
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

impl std::fmt::Display for OkapiOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OkapiOperation::Generate => write!(f, "generate"),
            OkapiOperation::Chat => write!(f, "chat"),
            OkapiOperation::Embed => write!(f, "embed"),
            OkapiOperation::ReadMetrics => write!(f, "read_metrics"),
            OkapiOperation::ReadCapabilities => write!(f, "read_capabilities"),
            OkapiOperation::SwapAdapter => write!(f, "swap_adapter"),
        }
    }
}

/// Capability token — Macaroon-backed unforgeable authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkapiCapability {
    /// Unique capability identifier
    pub id: CapabilityId,
    /// Issuer of this capability
    pub issuer: WebID,
    /// Holder of this capability
    pub holder: WebID,
    /// The macaroon token that proves authorization
    pub macaroon: Macaroon,
    /// Template scope (if any)
    pub template_id: Option<TemplateID>,
    /// Expiration time (for convenience, also in macaroon caveat)
    pub expires_at: Option<DateTime<Utc>>,
    /// Visibility level required to use this capability
    pub visibility: Visibility,
}

/// Authorization error
#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("Capability not found")]
    CapabilityNotFound,

    #[error("Capability expired")]
    CapabilityExpired,

    #[error("Unauthorized operation: {0}")]
    Unauthorized(String),

    #[error("Macaroon invalid: {0}")]
    MacaroonInvalid(String),

    #[error("Registry error: {0}")]
    Registry(String),
}

impl From<MacaroonError> for AuthorizationError {
    fn from(err: MacaroonError) -> Self {
        match err {
            MacaroonError::InvalidSignature => AuthorizationError::MacaroonInvalid(
                "Signature invalid - may have been tampered with".to_string(),
            ),
            MacaroonError::Expired => AuthorizationError::CapabilityExpired,
            MacaroonError::Unauthorized => {
                AuthorizationError::Unauthorized("Caveat prohibits this operation".to_string())
            }
            MacaroonError::UnknownCaveat => {
                AuthorizationError::MacaroonInvalid("Unknown caveat type".to_string())
            }
            MacaroonError::InvalidCaveat => {
                AuthorizationError::MacaroonInvalid("Invalid caveat data".to_string())
            }
        }
    }
}

impl OkapiCapability {
    /// Create new capability with specified operations
    ///
    /// # Arguments
    /// * `operations` - Operations this capability permits
    /// * `issuer` - WebID that issued this capability
    /// * `holder` - WebID that holds this capability
    /// * `expires_in` - Duration until expiration (e.g., Duration::days(30))
    /// * `key` - 32-byte HMAC key for macaroon signing
    pub fn new(
        operations: Vec<OkapiOperation>,
        issuer: WebID,
        holder: WebID,
        expires_in: chrono::Duration,
        key: &[u8; 32],
    ) -> Self {
        let id = CapabilityId::new();
        let identifier = format!("{}->{}", issuer, holder);
        let expires_at = Some(Utc::now() + expires_in);
        let expiry_timestamp = (Utc::now() + expires_in).timestamp();

        // Build macaroon with caveats
        let mut builder = MacaroonBuilder::new("hkask-ensemble", &identifier);
        builder = builder.expires_at(expiry_timestamp);

        for op in &operations {
            builder = builder.allows_operation(&op.to_string());
        }

        let macaroon = builder.build(key);

        Self {
            id,
            issuer,
            holder,
            macaroon,
            template_id: None,
            visibility: Visibility::Private,
            expires_at,
        }
    }

    /// Create capability scoped to a specific template
    pub fn for_template(
        operations: Vec<OkapiOperation>,
        issuer: WebID,
        holder: WebID,
        template_id: TemplateID,
        expires_in: chrono::Duration,
        key: &[u8; 32],
    ) -> Self {
        let id = CapabilityId::new();
        let identifier = format!("{}->{}->{}", issuer, holder, template_id);
        let expires_at = Some(Utc::now() + expires_in);
        let expiry_timestamp = (Utc::now() + expires_in).timestamp();

        // Build macaroon with caveats
        let mut builder = MacaroonBuilder::new("hkask-ensemble", &identifier);
        builder = builder.expires_at(expiry_timestamp);
        builder = builder.for_template(&template_id.to_string());

        for op in &operations {
            builder = builder.allows_operation(&op.to_string());
        }

        let macaroon = builder.build(key);

        Self {
            id,
            issuer,
            holder,
            macaroon,
            template_id: Some(template_id),
            expires_at,
            visibility: Visibility::Private,
        }
    }

    /// Verify capability macaroon signature and caveats
    ///
    /// # Arguments
    /// * `key` - 32-byte HMAC key for verification
    /// * `operations` - Operations to check against operation caveats
    pub fn verify(
        &self,
        key: &[u8; 32],
        operations: &[OkapiOperation],
    ) -> Result<(), AuthorizationError> {
        // Verify macaroon signature
        self.macaroon.verify(key)?;

        // Build caveat context with current time
        let ctx = crate::macaroon::CaveatContext::new();

        // Verify caveats
        self.macaroon.verify_caveats(&ctx)?;

        // Check if at least one of the requested operations is granted by this capability
        let granted_ops: Vec<String> = self
            .macaroon
            .caveats
            .iter()
            .filter(|c| c.caveat_id == "operation")
            .map(|c| c.data.clone())
            .collect();

        let requested_ops: Vec<String> = operations.iter().map(|o| o.to_string()).collect();

        // At least one requested operation must be granted
        let has_matching_op = requested_ops.iter().any(|op| granted_ops.contains(op));

        if !has_matching_op {
            return Err(AuthorizationError::Unauthorized(
                "Capability does not grant any of the requested operations".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if capability has a specific operation
    pub fn has_operation(&self, operation: OkapiOperation) -> bool {
        self.macaroon
            .caveats
            .iter()
            .any(|c| c.caveat_id == "operation" && c.data == operation.to_string())
    }

    /// Check if capability is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|exp| Utc::now() > exp).unwrap_or(false)
    }

    /// Attenuate capability for template-scoped access
    ///
    /// Creates a new capability with additional template caveat.
    /// The attenuated capability is more restricted than the original.
    pub fn attenuate_for_template(&self, template_id: TemplateID, key: &[u8; 32]) -> Self {
        let attenuated_macaroon = self.macaroon.clone().add_caveat(
            crate::macaroon::Caveat {
                caveat_id: "template".to_string(),
                data: template_id.to_string(),
            },
            key,
        );

        Self {
            id: CapabilityId::new(), // New ID for attenuated capability
            issuer: self.issuer,
            holder: self.holder,
            macaroon: attenuated_macaroon,
            template_id: Some(template_id),
            expires_at: self.expires_at,
            visibility: self.visibility,
        }
    }

    /// Create capability from existing macaroon (for deserialization)
    pub fn from_macaroon(
        macaroon: Macaroon,
        _operations: Vec<OkapiOperation>,
        issuer: WebID,
        holder: WebID,
        expires_at_timestamp: Option<i64>,
        template_id: Option<TemplateID>,
        _visibility_str: &str,
    ) -> Self {
        let visibility = Visibility::parse_str(_visibility_str).unwrap_or(Visibility::Private);
        let expires_at = expires_at_timestamp.and_then(|ts| DateTime::from_timestamp(ts, 0));

        Self {
            id: CapabilityId::new(),
            issuer,
            holder,
            macaroon,
            template_id,
            expires_at,
            visibility,
        }
    }

    /// Get macaroon for serialization
    pub fn macaroon(&self) -> &Macaroon {
        &self.macaroon
    }

    /// Get issuer WebID
    pub fn issuer(&self) -> WebID {
        self.issuer
    }

    /// Get holder/subject WebID
    pub fn subject(&self) -> WebID {
        self.holder
    }

    /// Get template ID
    pub fn template_id(&self) -> Option<TemplateID> {
        self.template_id
    }

    /// Get expiration time
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }

    /// Get visibility
    pub fn visibility(&self) -> Visibility {
        self.visibility
    }

    /// Get operations from macaroon caveats
    pub fn operations(&self) -> Vec<OkapiOperation> {
        self.macaroon
            .caveats
            .iter()
            .filter(|c| c.caveat_id == "operation")
            .filter_map(|c| OkapiOperation::from_str(c.data.as_str()).ok())
            .collect()
    }
}

impl std::str::FromStr for OkapiOperation {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "generate" => Ok(Self::Generate),
            "chat" => Ok(Self::Chat),
            "embed" => Ok(Self::Embed),
            "read_metrics" => Ok(Self::ReadMetrics),
            "read_capabilities" => Ok(Self::ReadCapabilities),
            "swap_adapter" => Ok(Self::SwapAdapter),
            _ => Err(()),
        }
    }
}

/// Create capability for system default access
pub fn default_system_capability(holder: WebID, key: &[u8; 32]) -> OkapiCapability {
    OkapiCapability::new(
        vec![
            OkapiOperation::Generate,
            OkapiOperation::Chat,
            OkapiOperation::ReadMetrics,
            OkapiOperation::ReadCapabilities,
        ],
        WebID::new(), // System issuer
        holder,
        chrono::Duration::days(30),
        key,
    )
}

/// Create read-only capability
pub fn read_only_capability(holder: WebID, key: &[u8; 32]) -> OkapiCapability {
    OkapiCapability::new(
        vec![
            OkapiOperation::ReadMetrics,
            OkapiOperation::ReadCapabilities,
        ],
        WebID::new(),
        holder,
        chrono::Duration::days(30),
        key,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [0x42; 32]
    }

    #[test]
    fn test_capability_creation() {
        let key = test_key();
        let holder = WebID::new();
        let issuer = WebID::new();

        let cap = OkapiCapability::new(
            vec![OkapiOperation::Generate, OkapiOperation::Chat],
            issuer,
            holder,
            chrono::Duration::days(30),
            &key,
        );

        assert_eq!(cap.issuer, issuer);
        assert_eq!(cap.holder, holder);
        assert!(cap.expires_at.is_some());
        assert!(!cap.is_expired());
    }

    #[test]
    fn test_capability_verification() {
        let key = test_key();
        let holder = WebID::new();

        let cap = OkapiCapability::new(
            vec![OkapiOperation::Generate],
            WebID::new(),
            holder,
            chrono::Duration::days(30),
            &key,
        );

        // Should verify successfully
        assert!(cap.verify(&key, &[OkapiOperation::Generate]).is_ok());

        // Should fail for unauthorized operation
        assert!(cap.verify(&key, &[OkapiOperation::Embed]).is_err());
    }

    #[test]
    fn test_template_scoped_capability() {
        let key = test_key();
        let holder = WebID::new();
        let template_id = TemplateID::new();

        let cap = OkapiCapability::for_template(
            vec![OkapiOperation::Generate],
            WebID::new(),
            holder,
            template_id,
            chrono::Duration::days(30),
            &key,
        );

        assert_eq!(cap.template_id, Some(template_id));
        assert!(cap.macaroon.has_caveat_type("template"));
        assert_eq!(
            cap.macaroon.get_caveat_data("template"),
            Some(template_id.to_string().as_str())
        );
    }

    #[test]
    fn test_capability_attenuation() {
        let key = test_key();
        let holder = WebID::new();
        let template_id = TemplateID::new();

        let cap = OkapiCapability::new(
            vec![OkapiOperation::Generate],
            WebID::new(),
            holder,
            chrono::Duration::days(30),
            &key,
        );

        let attenuated = cap.attenuate_for_template(template_id, &key);

        assert_eq!(attenuated.issuer, cap.issuer);
        assert_eq!(attenuated.holder, cap.holder);
        assert_eq!(attenuated.template_id, Some(template_id));
        assert!(attenuated.macaroon.has_caveat_type("template"));
    }

    #[test]
    fn test_capability_expiration() {
        let key = test_key();
        let holder = WebID::new();

        // Create capability that expires in 1 second
        let cap = OkapiCapability::new(
            vec![OkapiOperation::Generate],
            WebID::new(),
            holder,
            chrono::Duration::seconds(1),
            &key,
        );

        // Should not be expired initially
        assert!(!cap.is_expired());

        // Wait for expiration
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Should now be expired
        assert!(cap.is_expired());
    }

    #[test]
    fn test_default_system_capability() {
        let key = test_key();
        let holder = WebID::new();

        let cap = default_system_capability(holder, &key);

        assert!(cap.verify(&key, &[OkapiOperation::Generate]).is_ok());
        assert!(cap.verify(&key, &[OkapiOperation::Chat]).is_ok());
        assert!(cap.verify(&key, &[OkapiOperation::ReadMetrics]).is_ok());
        assert!(cap.verify(&key, &[OkapiOperation::Embed]).is_err());
    }

    #[test]
    fn test_read_only_capability() {
        let key = test_key();
        let holder = WebID::new();

        let cap = read_only_capability(holder, &key);

        assert!(cap.verify(&key, &[OkapiOperation::ReadMetrics]).is_ok());
        assert!(
            cap.verify(&key, &[OkapiOperation::ReadCapabilities])
                .is_ok()
        );
        assert!(cap.verify(&key, &[OkapiOperation::Generate]).is_err());
    }
}
