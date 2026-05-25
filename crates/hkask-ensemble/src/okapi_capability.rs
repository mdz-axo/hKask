//! Okapi Capability Operations
//!
//! Domain-specific operations for Okapi inference capabilities.
//! Uses CapabilityToken from hkask-types as the underlying primitive.

use hkask_types::{CapabilityAction, CapabilityResource, CapabilityToken, Caveat, WebID};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

/// Okapi capability error
#[derive(Debug, Error)]
pub enum OkapiCapabilityError {
    #[error("Capability expired")]
    Expired,

    #[error("Unauthorized operation: requested {requested}, granted {granted:?}")]
    Unauthorized {
        requested: String,
        granted: Vec<String>,
    },

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Caveat verification failed: {0}")]
    CaveatVerificationFailed(String),
}

/// Create an Okapi capability token with specified operations
pub fn create_okapi_capability(
    operations: Vec<OkapiOperation>,
    issuer: WebID,
    holder: WebID,
    expires_in: chrono::Duration,
    secret: &[u8],
) -> CapabilityToken {
    let expires_at = chrono::Utc::now() + expires_in;
    let expiry_timestamp = expires_at.timestamp();

    let mut token = CapabilityToken::new(
        CapabilityResource::Tool,
        "okapi".to_string(),
        CapabilityAction::Execute,
        issuer,
        holder,
        secret,
    );

    // Add expiration caveat
    token = token.add_caveat(Caveat::expiration(expiry_timestamp), secret);

    // Add visibility caveat (default to private)
    token = token.add_caveat(Caveat::visibility("private"), secret);

    // Add operation caveats
    for op in operations {
        token = token.add_caveat(Caveat::operation(op.to_string()), secret);
    }

    token
}

/// Create an Okapi capability scoped to a specific template
pub fn create_okapi_capability_for_template(
    operations: Vec<OkapiOperation>,
    issuer: WebID,
    holder: WebID,
    template_id: &str,
    expires_in: chrono::Duration,
    secret: &[u8],
) -> CapabilityToken {
    let expires_at = chrono::Utc::now() + expires_in;
    let expiry_timestamp = expires_at.timestamp();

    let mut token = CapabilityToken::new(
        CapabilityResource::Tool,
        "okapi".to_string(),
        CapabilityAction::Execute,
        issuer,
        holder,
        secret,
    );

    // Add expiration caveat
    token = token.add_caveat(Caveat::expiration(expiry_timestamp), secret);

    // Add visibility caveat (default to private)
    token = token.add_caveat(Caveat::visibility("private"), secret);

    // Add template caveat
    token = token.add_caveat(Caveat::template(template_id), secret);

    // Add operation caveats
    for op in operations {
        token = token.add_caveat(Caveat::operation(op.to_string()), secret);
    }

    token
}

/// Verify an Okapi capability token
pub fn verify_okapi_capability(
    token: &CapabilityToken,
    secret: &[u8],
    operations: &[OkapiOperation],
) -> Result<(), OkapiCapabilityError> {
    // Verify signature
    if !token.verify(secret) {
        return Err(OkapiCapabilityError::InvalidSignature);
    }

    // Build caveat context
    let requested_ops: Vec<String> = operations.iter().map(|o| o.to_string()).collect();
    let ctx = hkask_types::CaveatContext::new()
        .with_operations(requested_ops.clone())
        .with_visibility("private".to_string());

    // Verify caveats
    token.verify_caveats(&ctx).map_err(|e| {
        let granted_ops: Vec<String> = token
            .caveats
            .iter()
            .filter(|c| c.caveat_id == "operation")
            .map(|c| c.data.clone())
            .collect();

        if e.contains("expired") {
            OkapiCapabilityError::Expired
        } else if e.contains("operation") {
            OkapiCapabilityError::Unauthorized {
                requested: requested_ops.join(", "),
                granted: granted_ops,
            }
        } else {
            OkapiCapabilityError::CaveatVerificationFailed(e)
        }
    })?;

    Ok(())
}

/// Check if a capability has a specific operation
pub fn has_operation(token: &CapabilityToken, operation: OkapiOperation) -> bool {
    token
        .caveats
        .iter()
        .any(|c| c.caveat_id == "operation" && c.data == operation.to_string())
}

/// Get all granted operations from a capability
pub fn granted_operations(token: &CapabilityToken) -> Vec<String> {
    token
        .caveats
        .iter()
        .filter(|c| c.caveat_id == "operation")
        .map(|c| c.data.clone())
        .collect()
}

/// Check if capability is expired
pub fn is_expired(token: &CapabilityToken) -> bool {
    if let Some(expiry_str) = token.get_caveat_data("expiration")
        && let Ok(expiry) = expiry_str.parse::<i64>()
    {
        return chrono::Utc::now().timestamp() > expiry;
    }
    false
}

/// Attenuate capability for template-scoped access
pub fn attenuate_for_template(
    token: &CapabilityToken,
    template_id: &str,
    secret: &[u8],
) -> CapabilityToken {
    token.add_caveat(Caveat::template(template_id), secret)
}

/// Create capability for system default access
pub fn default_system_capability(holder: WebID, secret: &[u8]) -> CapabilityToken {
    create_okapi_capability(
        vec![
            OkapiOperation::Generate,
            OkapiOperation::Chat,
            OkapiOperation::ReadMetrics,
            OkapiOperation::ReadCapabilities,
        ],
        WebID::new(), // System issuer
        holder,
        chrono::Duration::days(30),
        secret,
    )
}

/// Create read-only capability
pub fn read_only_capability(holder: WebID, secret: &[u8]) -> CapabilityToken {
    create_okapi_capability(
        vec![
            OkapiOperation::ReadMetrics,
            OkapiOperation::ReadCapabilities,
        ],
        WebID::new(),
        holder,
        chrono::Duration::days(30),
        secret,
    )
}
