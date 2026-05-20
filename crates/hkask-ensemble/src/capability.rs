//! Capability-Based Security for Okapi Access
//!
//! Implements Macaroon-backed capability tokens for unforgeable authorization.
//! Follows principle of least authority (Mark Miller / Bruce Schneier).

use chrono::{DateTime, Utc};
use hkask_keystore::KeyRing;
use hkask_types::{TemplateID, Visibility, WebID};
use serde::{Deserialize, Serialize};
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
///
/// Fields are private to enforce encapsulation. Use getter methods for access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkapiCapability {
    /// Unique capability identifier
    id: CapabilityId,
    /// Issuer of this capability
    issuer: WebID,
    /// Holder of this capability
    holder: WebID,
    /// The macaroon token that proves authorization
    macaroon: Macaroon,
    /// Template scope (if any)
    template_id: Option<TemplateID>,
    /// Expiration time (for convenience, also in macaroon caveat)
    expires_at: Option<DateTime<Utc>>,
    /// Visibility level required to use this capability
    visibility: Visibility,
}

/// Capability configuration with key ring for rotation
#[derive(Debug, Clone)]
pub struct CapabilityConfig {
    key_ring: KeyRing,
}

impl CapabilityConfig {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            key_ring: KeyRing::new(key),
        }
    }

    pub fn with_key_ring(key_ring: KeyRing) -> Self {
        Self { key_ring }
    }

    pub fn key_ring(&self) -> &KeyRing {
        &self.key_ring
    }

    pub fn rotate_key(&mut self, new_key: [u8; 32]) {
        self.key_ring.rotate(new_key);
    }
}

/// Authorization error
#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("Capability not found")]
    CapabilityNotFound,

    #[error("Capability expired")]
    CapabilityExpired,

    #[error("Unauthorized operation: requested {requested:?}, granted {granted:?}")]
    Unauthorized {
        requested: String,
        granted: Vec<String>,
    },

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
            MacaroonError::Unauthorized => AuthorizationError::Unauthorized {
                requested: "unknown".to_string(),
                granted: vec![],
            },
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
        let visibility = Visibility::Private;

        // Build macaroon with caveats
        let mut builder = MacaroonBuilder::new("hkask-ensemble", &identifier);
        builder = builder.expires_at(expiry_timestamp);
        builder = builder.with_visibility(visibility.as_str());

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
            visibility,
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
        let visibility = Visibility::Private;

        // Build macaroon with caveats
        let mut builder = MacaroonBuilder::new("hkask-ensemble", &identifier);
        builder = builder.expires_at(expiry_timestamp);
        builder = builder.for_template(&template_id.to_string());
        builder = builder.with_visibility(visibility.as_str());

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
            visibility,
        }
    }

    /// Verify capability macaroon signature and caveats
    ///
    /// # Arguments
    /// * `key` - 32-byte HMAC key for verification
    /// * `operations` - Operations to check against operation caveats
    ///
    /// # Invariants
    /// - Signature must be valid
    /// - All caveats must be satisfied (expiration, template, visibility)
    /// - At least one requested operation must match a granted operation caveat
    pub fn verify(
        &self,
        key: &[u8; 32],
        operations: &[OkapiOperation],
    ) -> Result<(), AuthorizationError> {
        // Verify macaroon signature
        self.macaroon.verify(key).map_err(|e| {
            AuthorizationError::MacaroonInvalid(format!("Signature verification failed: {:?}", e))
        })?;

        // Build caveat context with requested operations and visibility
        let requested_ops: Vec<String> = operations.iter().map(|o| o.to_string()).collect();
        let mut ctx = crate::macaroon::CaveatContext::new()
            .with_operations(requested_ops.clone())
            .with_visibility(self.visibility.as_str().to_string());

        // Set template_id if this capability is template-scoped
        if let Some(tid) = self.template_id {
            ctx = ctx.with_template(tid.to_string());
        }

        // Verify all caveats (including operations and visibility)
        self.macaroon.verify_caveats(&ctx).map_err(|e| {
            // Extract granted operations for error message
            let granted_ops: Vec<String> = self
                .macaroon
                .caveats
                .iter()
                .filter(|c| c.caveat_id == "operation")
                .map(|c| c.data.clone())
                .collect();

            match e {
                crate::macaroon::MacaroonError::Unauthorized => AuthorizationError::Unauthorized {
                    requested: requested_ops.join(", "),
                    granted: granted_ops,
                },
                crate::macaroon::MacaroonError::Expired => AuthorizationError::CapabilityExpired,
                _ => AuthorizationError::MacaroonInvalid(format!(
                    "Caveat verification failed: {:?}",
                    e
                )),
            }
        })?;

        Ok(())
    }

    /// Check if capability has a specific operation
    pub fn has_operation(&self, operation: OkapiOperation) -> bool {
        self.macaroon
            .caveats
            .iter()
            .any(|c| c.caveat_id == "operation" && c.data == operation.to_string())
    }

    /// Get all granted operations from this capability
    pub fn granted_operations(&self) -> Vec<String> {
        self.macaroon
            .caveats
            .iter()
            .filter(|c| c.caveat_id == "operation")
            .map(|c| c.data.clone())
            .collect()
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
            issuer: self.issuer(),
            holder: self.holder(),
            macaroon: attenuated_macaroon,
            template_id: Some(template_id),
            expires_at: self.expires_at(),
            visibility: self.visibility(),
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

    /// Get capability ID
    pub fn id(&self) -> CapabilityId {
        self.id
    }

    /// Get macaroon reference for serialization
    pub fn macaroon(&self) -> &Macaroon {
        &self.macaroon
    }

    /// Get issuer WebID
    pub fn issuer(&self) -> WebID {
        self.issuer
    }

    /// Get holder/subject WebID
    pub fn holder(&self) -> WebID {
        self.holder
    }

    /// Get holder/subject WebID (alias for holder())
    pub fn subject(&self) -> WebID {
        self.holder
    }

    /// Get template ID reference
    pub fn template_id(&self) -> Option<TemplateID> {
        self.template_id
    }

    /// Get expiration time reference
    pub fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires_at
    }

    /// Get visibility level
    pub fn visibility(&self) -> Visibility {
        self.visibility
    }

    /// Create capability with custom visibility (for testing)
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

/// Builder for constructing OkapiCapability with explicit parameters
///
/// Enforces explicit visibility and expiration, following least authority principle.
pub struct OkapiCapabilityBuilder {
    operations: Vec<OkapiOperation>,
    issuer: WebID,
    holder: WebID,
    expires_in: chrono::Duration,
    visibility: Option<Visibility>,
    template_id: Option<TemplateID>,
}

impl OkapiCapabilityBuilder {
    /// Create new builder with required parameters
    pub fn new(operations: Vec<OkapiOperation>, issuer: WebID, holder: WebID) -> Self {
        Self {
            operations,
            issuer,
            holder,
            expires_in: chrono::Duration::days(30), // Default, but can be overridden
            visibility: None,
            template_id: None,
        }
    }

    /// Set expiration duration (required - no silent defaults)
    pub fn expires_in(mut self, duration: chrono::Duration) -> Self {
        self.expires_in = duration;
        self
    }

    /// Set visibility level (required - no silent defaults)
    pub fn visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = Some(visibility);
        self
    }

    /// Set template scope (optional)
    pub fn for_template(mut self, template_id: TemplateID) -> Self {
        self.template_id = Some(template_id);
        self
    }

    /// Build capability with key
    pub fn build(self, key: &[u8; 32]) -> OkapiCapability {
        let visibility = self.visibility.unwrap_or(Visibility::Private);
        let expires_in = self.expires_in;
        let expires_at = Some(Utc::now() + expires_in);
        let expiry_timestamp = (Utc::now() + expires_in).timestamp();

        let identifier = if let Some(tid) = self.template_id {
            format!("{}->{}->{}", self.issuer, self.holder, tid)
        } else {
            format!("{}->{}", self.issuer, self.holder)
        };

        let mut builder = MacaroonBuilder::new("hkask-ensemble", &identifier);
        builder = builder.expires_at(expiry_timestamp);
        builder = builder.with_visibility(visibility.as_str());

        if let Some(tid) = self.template_id {
            builder = builder.for_template(&tid.to_string());
        }

        for op in &self.operations {
            builder = builder.allows_operation(&op.to_string());
        }

        let macaroon = builder.build(key);

        OkapiCapability {
            id: CapabilityId::new(),
            issuer: self.issuer,
            holder: self.holder,
            macaroon,
            template_id: self.template_id,
            expires_at,
            visibility,
        }
    }
}

impl OkapiCapability {
    /// Create builder for explicit capability construction
    pub fn builder(
        operations: Vec<OkapiOperation>,
        issuer: WebID,
        holder: WebID,
    ) -> OkapiCapabilityBuilder {
        OkapiCapabilityBuilder::new(operations, issuer, holder)
    }
}

