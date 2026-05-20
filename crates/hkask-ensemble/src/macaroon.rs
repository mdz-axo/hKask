//! Macaroon-based Capabilities for hKask
//!
//! Implements lightweight cryptographic capabilities using HMAC-SHA256.
//! Macaroons provide tamper-evident authorization without UCAN chain complexity.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

/// A Macaroon capability for hKask
///
/// Macaroons are lightweight cryptographic tokens that prove authorization.
/// They are signed with HMAC-SHA256, making them tamper-evident and fast to verify.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macaroon {
    /// Where this macaroon was created (e.g., "hkask-ensemble")
    pub location: String,

    /// Unique identifier (e.g., WebID or capability ID)
    pub identifier: String,

    /// Caveats (limitations on this capability)
    pub caveats: Vec<Caveat>,

    /// HMAC-SHA256 signature over location + identifier + caveats
    pub signature: [u8; 32],
}

/// A caveat limits the capability's scope
///
/// Caveats are additive restrictions on what the capability holder can do.
/// Common caveat types: expiration, operation, template, visibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Caveat {
    /// Caveat type identifier (e.g., "expiration", "operation", "template")
    pub caveat_id: String,

    /// Caveat data (e.g., timestamp, operation name, template ID)
    pub data: String,
}

/// Context for caveat verification
pub struct CaveatContext {
    /// Operations allowed in this context
    pub allowed_operations: Vec<String>,
    /// Template ID if template-scoped
    pub template_id: Option<String>,
    /// Visibility level required
    pub visibility: String,
    /// Current timestamp for expiration checks
    pub current_time: i64,
}

impl CaveatContext {
    /// Create new context with current time
    pub fn new() -> Self {
        Self {
            allowed_operations: Vec::new(),
            template_id: None,
            visibility: String::new(),
            current_time: chrono::Utc::now().timestamp(),
        }
    }

    /// Set allowed operations
    pub fn with_operations(mut self, ops: Vec<String>) -> Self {
        self.allowed_operations = ops;
        self
    }

    /// Set template ID
    pub fn with_template(mut self, template_id: String) -> Self {
        self.template_id = Some(template_id);
        self
    }

    /// Set visibility
    pub fn with_visibility(mut self, visibility: String) -> Self {
        self.visibility = visibility;
        self
    }
}

impl Default for CaveatContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Macaroon errors
#[derive(Debug, PartialEq, Eq, Error)]
pub enum MacaroonError {
    #[error("Invalid signature - macaroon may have been tampered with")]
    InvalidSignature,

    #[error("Macaroon expired - validity period has ended")]
    Expired,

    #[error("Unauthorized by caveat - capability does not permit this action")]
    Unauthorized,

    #[error("Unknown caveat type - caveat ID not recognized")]
    UnknownCaveat,

    #[error("Invalid caveat data - caveat data format is incorrect")]
    InvalidCaveat,
}

impl Macaroon {
    /// Create a new macaroon with HMAC-SHA256 signature
    ///
    /// # Arguments
    /// * `location` - Where this macaroon originates (e.g., "hkask-ensemble")
    /// * `identifier` - Unique identifier for this capability
    /// * `key` - 32-byte HMAC key for signing
    pub fn new(location: &str, identifier: &str, key: &[u8; 32]) -> Self {
        let mut mac = HmacSha256::new_from_slice(key).unwrap();
        mac.update(location.as_bytes());
        mac.update(identifier.as_bytes());

        let mut signature = [0u8; 32];
        signature.copy_from_slice(&mac.finalize().into_bytes());

        Self {
            location: location.to_string(),
            identifier: identifier.to_string(),
            caveats: Vec::new(),
            signature,
        }
    }

    /// Add a caveat to create an attenuated macaroon
    ///
    /// Each caveat adds a restriction. The macaroon is re-signed to include
    /// the new caveat, making the attenuation tamper-evident.
    ///
    /// # Arguments
    /// * `caveat` - The caveat to add
    /// * `key` - 32-byte HMAC key for re-signing
    pub fn add_caveat(&self, caveat: Caveat, key: &[u8; 32]) -> Self {
        let mut new_mac = self.clone();
        new_mac.caveats.push(caveat.clone());

        // Re-sign with caveat included
        let mut mac = HmacSha256::new_from_slice(key).unwrap();
        mac.update(new_mac.location.as_bytes());
        mac.update(new_mac.identifier.as_bytes());
        for c in &new_mac.caveats {
            mac.update(c.caveat_id.as_bytes());
            mac.update(c.data.as_bytes());
        }

        new_mac
            .signature
            .copy_from_slice(&mac.finalize().into_bytes());
        new_mac
    }

    /// Verify the macaroon signature
    ///
    /// Returns Ok if the signature is valid (macaroon hasn't been tampered with).
    /// Returns Err if signature is invalid.
    ///
    /// # Arguments
    /// * `key` - 32-byte HMAC key for verification
    pub fn verify(&self, key: &[u8; 32]) -> Result<(), MacaroonError> {
        let mut mac = HmacSha256::new_from_slice(key).unwrap();
        mac.update(self.location.as_bytes());
        mac.update(self.identifier.as_bytes());
        for c in &self.caveats {
            mac.update(c.caveat_id.as_bytes());
            mac.update(c.data.as_bytes());
        }

        mac.verify_slice(&self.signature)
            .map_err(|_| MacaroonError::InvalidSignature)
    }

    /// Verify all caveats are satisfied in the given context
    ///
    /// Checks each caveat against the provided context:
    /// - expiration: current time must be before expiry
    /// - operation: at least one requested operation must match a granted operation caveat
    /// - template: context template must match caveat template
    /// - visibility: context visibility must match caveat visibility
    ///
    /// # Arguments
    /// * `ctx` - The context in which to verify caveats
    ///
    /// # Invariants
    /// - All non-operation caveats must be satisfied
    /// - At least one operation caveat must match a requested operation (if operations provided)
    /// - Empty `allowed_operations` means operation caveats are skipped (for backward compatibility)
    pub fn verify_caveats(&self, ctx: &CaveatContext) -> Result<(), MacaroonError> {
        let mut has_matching_operation = ctx.allowed_operations.is_empty();

        for caveat in &self.caveats {
            match caveat.caveat_id.as_str() {
                "expiration" => {
                    let expiry = caveat
                        .data
                        .parse::<i64>()
                        .map_err(|_| MacaroonError::InvalidCaveat)?;
                    if ctx.current_time > expiry {
                        return Err(MacaroonError::Expired);
                    }
                }
                "operation" => {
                    // Check if this operation caveat matches any requested operation
                    if !has_matching_operation && ctx.allowed_operations.contains(&caveat.data) {
                        has_matching_operation = true;
                    }
                }
                "template" => {
                    if ctx.template_id.as_ref() != Some(&caveat.data) {
                        return Err(MacaroonError::Unauthorized);
                    }
                }
                "visibility" => {
                    if ctx.visibility != caveat.data {
                        return Err(MacaroonError::Unauthorized);
                    }
                }
                _ => return Err(MacaroonError::UnknownCaveat),
            }
        }

        // If operations were requested, at least one must match
        if !ctx.allowed_operations.is_empty() && !has_matching_operation {
            return Err(MacaroonError::Unauthorized);
        }

        Ok(())
    }

    /// Get all caveat IDs
    pub fn caveat_ids(&self) -> Vec<&str> {
        self.caveats.iter().map(|c| c.caveat_id.as_str()).collect()
    }

    /// Check if macaroon has a specific caveat type
    pub fn has_caveat_type(&self, caveat_type: &str) -> bool {
        self.caveats.iter().any(|c| c.caveat_id == caveat_type)
    }

    /// Get caveat data for a specific caveat type
    pub fn get_caveat_data(&self, caveat_type: &str) -> Option<&str> {
        self.caveats
            .iter()
            .find(|c| c.caveat_id == caveat_type)
            .map(|c| c.data.as_str())
    }
}

/// Builder for creating macaroons with multiple caveats
pub struct MacaroonBuilder {
    location: String,
    identifier: String,
    caveats: Vec<Caveat>,
}

impl MacaroonBuilder {
    /// Create new builder
    pub fn new(location: &str, identifier: &str) -> Self {
        Self {
            location: location.to_string(),
            identifier: identifier.to_string(),
            caveats: Vec::new(),
        }
    }

    /// Add caveat
    pub fn add_caveat(mut self, caveat_id: &str, data: &str) -> Self {
        self.caveats.push(Caveat {
            caveat_id: caveat_id.to_string(),
            data: data.to_string(),
        });
        self
    }

    /// Add expiration caveat
    pub fn expires_at(mut self, timestamp: i64) -> Self {
        self.caveats.push(Caveat {
            caveat_id: "expiration".to_string(),
            data: timestamp.to_string(),
        });
        self
    }

    /// Add operation caveat
    pub fn allows_operation(mut self, operation: &str) -> Self {
        self.caveats.push(Caveat {
            caveat_id: "operation".to_string(),
            data: operation.to_string(),
        });
        self
    }

    /// Add template caveat
    pub fn for_template(mut self, template_id: &str) -> Self {
        self.caveats.push(Caveat {
            caveat_id: "template".to_string(),
            data: template_id.to_string(),
        });
        self
    }

    /// Add visibility caveat
    pub fn with_visibility(mut self, visibility: &str) -> Self {
        self.caveats.push(Caveat {
            caveat_id: "visibility".to_string(),
            data: visibility.to_string(),
        });
        self
    }

    /// Build and sign macaroon
    pub fn build(self, key: &[u8; 32]) -> Macaroon {
        let mut mac = Macaroon::new(&self.location, &self.identifier, key);
        for caveat in self.caveats {
            mac = mac.add_caveat(caveat, key);
        }
        mac
    }
}
