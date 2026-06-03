//! Registration validation — CLI-layer input validation
//!
//! Validation logic for `RegistrationRequest` lives here because it
//! operates on plaintext passphrases and other raw user input.
//! The storage layer should never see validation logic — it trusts
//! the caller (CLI or API) to have validated before persisting.

use hkask_types::{RegistrationError, RegistrationRequest};

/// Validate all fields of a `RegistrationRequest`.
///
/// Checks replicant name, names, contact info, and passphrase.
/// Returns `Err(RegistrationError)` on the first violation.
pub fn validate_registration(request: &RegistrationRequest) -> Result<(), RegistrationError> {
    if request.replicant_name.is_empty()
        || request.replicant_name.len() > 64
        || !request
            .replicant_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(RegistrationError::InvalidReplicantName);
    }
    if request.first_name.is_empty() || request.last_name.is_empty() {
        return Err(RegistrationError::EmptyName);
    }
    if request.email.is_empty() || !request.email.contains('@') {
        return Err(RegistrationError::InvalidContact);
    }
    if let Some(phone) = &request.phone
        && !phone.starts_with('+')
    {
        return Err(RegistrationError::InvalidContact);
    }
    validate_passphrase(&request.passphrase)?;
    Ok(())
}

/// Validate passphrase: alphanumeric only (upper + lowercase), min 8 chars
pub fn validate_passphrase(passphrase: &str) -> Result<(), RegistrationError> {
    if passphrase.len() < 8 {
        return Err(RegistrationError::InvalidPassphrase);
    }
    if !passphrase.chars().all(|c| c.is_alphanumeric()) {
        return Err(RegistrationError::InvalidPassphrase);
    }
    let has_upper = passphrase.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = passphrase.chars().any(|c| c.is_ascii_lowercase());
    if !has_upper || !has_lower {
        return Err(RegistrationError::InvalidPassphrase);
    }
    Ok(())
}
