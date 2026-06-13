//! Human identity and authentication types — Loop 6 (Cybernetics): Access Guard
//!
//! Cybernetics subloop 6.1 (Access Guard) governs who can access what.
//! Human users, replicant identities, and sessions are verified at this boundary.

use crate::WebID;
use crate::id::UserID;
use serde::{Deserialize, Serialize};

/// Loop: Cybernetics
/// Human user — owns contact info (email, phone for recovery only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanUser {
    pub user_id: UserID,
    pub email_enc: Vec<u8>,
    pub phone_enc: Option<Vec<u8>>,
    pub passphrase_hash: String,
    pub salt: String,
    pub master_salt: String,
    pub created_at: i64,
    pub last_active: Option<i64>,
    pub passphrase_set_at: Option<i64>,
}

impl HumanUser {
    pub fn new(
        email_enc: Vec<u8>,
        phone_enc: Option<Vec<u8>>,
        passphrase_hash: String,
        salt: String,
        master_salt: String,
    ) -> Self {
        Self {
            user_id: UserID::new(),
            email_enc,
            phone_enc,
            passphrase_hash,
            salt,
            master_salt,
            created_at: chrono::Utc::now().timestamp(),
            last_active: None,
            passphrase_set_at: None,
        }
    }
}

/// Loop: Cybernetics
/// Replicant identity — the in-system persona users log in AS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicantIdentity {
    pub replicant_name: String,
    pub user_id: UserID,
    pub replicant_webid: WebID,
    pub first_name_enc: Vec<u8>,
    pub last_name_enc: Vec<u8>,
    pub persona_yaml: Option<String>,
    pub is_primary: bool,
    pub created_at: i64,
    pub last_login: Option<i64>,
}

impl ReplicantIdentity {
    pub fn derive_webid(replicant_name: &str) -> WebID {
        WebID::from_persona_with_namespace(replicant_name.as_bytes(), "replicant")
    }

    pub fn new(
        replicant_name: String,
        user_id: UserID,
        first_name_enc: Vec<u8>,
        last_name_enc: Vec<u8>,
        is_primary: bool,
    ) -> Self {
        Self {
            replicant_webid: Self::derive_webid(&replicant_name),
            replicant_name,
            user_id,
            first_name_enc,
            last_name_enc,
            persona_yaml: None,
            is_primary,
            created_at: chrono::Utc::now().timestamp(),
            last_login: None,
        }
    }
}

/// Loop: Cybernetics
/// Active user session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    pub session_id: String,
    pub replicant_name: String,
    pub replicant_webid: WebID,
    pub user_id: UserID,
    pub session_key_salt: String,
    pub expires_at: i64,
    pub last_active: i64,
}

impl UserSession {
    pub fn is_expired(&self, now: i64) -> bool {
        now > self.expires_at
    }
}

/// Loop: Cybernetics
/// Registration request for new replicant identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationRequest {
    pub replicant_name: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub passphrase: String,
}

#[derive(Debug, Clone, thiserror::Error)]
/// Loop: Cybernetics
///
/// Variants are grouped by shared recovery path:
/// - `InvalidReplicantName` — name must be 1–64 alphanumeric/hyphen/underscore chars
/// - `EmptyName` — required name field is missing
/// - `InvalidContact` — email or phone format is wrong
/// - `InvalidPassphrase` — passphrase doesn't meet requirements
pub enum RegistrationError {
    #[error("Invalid replicant name: must be 1-64 chars, alphanumeric with hyphens/underscores")]
    InvalidReplicantName,
    #[error("Required name field is empty")]
    EmptyName,
    #[error("Invalid contact information format")]
    InvalidContact,
    #[error("Passphrase does not meet requirements: 8+ alphanumeric chars, mixed case")]
    InvalidPassphrase,
}
