//! Human user identity and authentication types
//!
//! This module provides:
//! - **HumanUser**: The human behind the system (contact info for recovery)
//! - **ReplicantIdentity**: In-system persona that users log in AS
//! - **UserSession**: Active authenticated sessions

use crate::WebID;
use serde::{Deserialize, Serialize};

crate::id::define_id_type!(UserID, from_string);

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
        }
    }
}

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
        WebID::from_persona_with_namespace(replicant_name.as_bytes(), "hkask-replicant")
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

impl RegistrationRequest {
    pub fn validate(&self) -> Result<(), RegistrationError> {
        if self.replicant_name.is_empty() {
            return Err(RegistrationError::EmptyReplicantName);
        }
        if self.replicant_name.len() > 64 {
            return Err(RegistrationError::ReplicantNameTooLong);
        }
        if !self
            .replicant_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(RegistrationError::InvalidReplicantName);
        }
        if self.first_name.is_empty() {
            return Err(RegistrationError::EmptyFirstName);
        }
        if self.last_name.is_empty() {
            return Err(RegistrationError::EmptyLastName);
        }
        if self.email.is_empty() || !self.email.contains('@') {
            return Err(RegistrationError::InvalidEmail);
        }
        if let Some(phone) = &self.phone
            && !phone.starts_with('+')
        {
            return Err(RegistrationError::InvalidPhone);
        }
        Self::validate_passphrase(&self.passphrase)?;
        Ok(())
    }

    /// Validate passphrase: alphanumeric only (upper + lowercase), min 8 chars
    pub fn validate_passphrase(passphrase: &str) -> Result<(), RegistrationError> {
        if passphrase.len() < 8 {
            return Err(RegistrationError::PassphraseTooShort);
        }
        if !passphrase.chars().all(|c| c.is_alphanumeric()) {
            return Err(RegistrationError::PassphraseInvalidChars);
        }
        let has_upper = passphrase.chars().any(|c| c.is_ascii_uppercase());
        let has_lower = passphrase.chars().any(|c| c.is_ascii_lowercase());
        if !has_upper || !has_lower {
            return Err(RegistrationError::PassphraseCaseRequired);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum RegistrationError {
    #[error("Replicant name cannot be empty")]
    EmptyReplicantName,
    #[error("Replicant name too long (max 64 chars)")]
    ReplicantNameTooLong,
    #[error("Replicant name must be alphanumeric (a-z, 0-9, -, _)")]
    InvalidReplicantName,
    #[error("First name cannot be empty")]
    EmptyFirstName,
    #[error("Last name cannot be empty")]
    EmptyLastName,
    #[error("Invalid email address")]
    InvalidEmail,
    #[error("Phone number must be in E.164 format (e.g., +15551234567)")]
    InvalidPhone,
    #[error("Passphrase must be at least 8 characters")]
    PassphraseTooShort,
    #[error("Passphrase must contain only alphanumeric characters (a-z, A-Z, 0-9)")]
    PassphraseInvalidChars,
    #[error("Passphrase must contain both uppercase and lowercase letters")]
    PassphraseCaseRequired,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passphrase_validation() {
        // Valid passphrase
        assert!(RegistrationRequest::validate_passphrase("AlicePass123").is_ok());

        // Too short
        assert!(RegistrationRequest::validate_passphrase("Ab1").is_err());

        // No uppercase
        assert!(RegistrationRequest::validate_passphrase("alicepass123").is_err());

        // No lowercase
        assert!(RegistrationRequest::validate_passphrase("ALICEPASS123").is_err());

        // Special characters not allowed
        assert!(RegistrationRequest::validate_passphrase("Alice@Pass123").is_err());
    }
}
