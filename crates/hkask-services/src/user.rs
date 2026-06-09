//! User registration and authentication — replicant identity management.

use hkask_types::{RegistrationRequest, ReplicantIdentity, UserID, UserSession};
use zeroize::Zeroizing;

use crate::ServiceContext;
use crate::error::ServiceError;

pub struct UserService;

impl UserService {
    /// Validate a passphrase meets requirements.
    ///
    /// Requirements: 8+ characters, alphanumeric only, mixed case.
    ///
    /// # REQ: svc-user-001 — validate_passphrase rejects weak passphrases
    pub fn validate_passphrase(passphrase: &str) -> Result<(), ServiceError> {
        if passphrase.len() < 8 {
            return Err(ServiceError::InvalidPassphrase(
                "Passphrase does not meet requirements: 8+ alphanumeric chars, mixed case".into(),
            ));
        }
        if !passphrase.chars().all(|c| c.is_alphanumeric()) {
            return Err(ServiceError::InvalidPassphrase(
                "Passphrase does not meet requirements: 8+ alphanumeric chars, mixed case".into(),
            ));
        }
        let has_upper = passphrase.chars().any(|c| c.is_ascii_uppercase());
        let has_lower = passphrase.chars().any(|c| c.is_ascii_lowercase());
        if !has_upper || !has_lower {
            return Err(ServiceError::InvalidPassphrase(
                "Passphrase does not meet requirements: 8+ alphanumeric chars, mixed case".into(),
            ));
        }
        Ok(())
    }

    /// Validate all fields of a registration request.
    ///
    /// # REQ: svc-user-002 — validate_registration rejects invalid fields
    pub fn validate_registration(request: &RegistrationRequest) -> Result<(), ServiceError> {
        if request.replicant_name.is_empty()
            || request.replicant_name.len() > 64
            || !request
                .replicant_name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ServiceError::ValidationError(
                "Invalid replicant name: must be 1-64 chars, alphanumeric with hyphens/underscores"
                    .into(),
            ));
        }
        if request.first_name.is_empty() || request.last_name.is_empty() {
            return Err(ServiceError::ValidationError(
                "Required name field is empty".into(),
            ));
        }
        if request.email.is_empty() || !request.email.contains('@') {
            return Err(ServiceError::ValidationError(
                "Invalid contact information format".into(),
            ));
        }
        if let Some(phone) = &request.phone
            && !phone.starts_with('+')
        {
            return Err(ServiceError::ValidationError(
                "Invalid contact information format".into(),
            ));
        }
        Self::validate_passphrase(&request.passphrase)?;
        Ok(())
    }

    /// Register a new replicant identity.
    ///
    /// Validates passphrase and registration fields, then persists
    /// the new human user + replicant identity via the store.
    ///
    /// # REQ: svc-user-003 — register validates and persists replicant
    pub fn register(
        ctx: &ServiceContext,
        replicant_name: &str,
        first_name: &str,
        last_name: &str,
        email: &str,
        phone: Option<&str>,
        passphrase: Zeroizing<String>,
    ) -> Result<ReplicantIdentity, ServiceError> {
        Self::validate_passphrase(&passphrase)?;

        let request = RegistrationRequest {
            replicant_name: replicant_name.to_string(),
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            phone: phone.map(|s| s.to_string()),
            passphrase: (*passphrase).clone(),
        };

        Self::validate_registration(&request)?;

        let store = ctx.user_store.lock()?;
        store
            .register_replicant(
                request.replicant_name,
                request.email,
                request.phone,
                request.first_name,
                request.last_name,
                request.passphrase,
            )
            .map_err(Into::into)
    }

    /// Login as a replicant identity.
    ///
    /// Deliberately opaque: returns `ServiceError::LoginFailed` regardless
    /// of the underlying cause to prevent information leakage.
    ///
    /// # REQ: svc-user-004 — login returns opaque error on failure
    pub fn login(
        ctx: &ServiceContext,
        replicant_name: &str,
        passphrase: Zeroizing<String>,
    ) -> Result<UserSession, ServiceError> {
        let store = ctx.user_store.lock()?;
        store
            .login(replicant_name, &passphrase)
            .map_err(|_| ServiceError::LoginFailed("Invalid credentials".to_string()))
    }

    /// Get a replicant identity by name.
    ///
    /// # REQ: svc-user-005 — get_replicant returns not-found for unknown name
    pub fn get_replicant(
        ctx: &ServiceContext,
        replicant_name: &str,
    ) -> Result<ReplicantIdentity, ServiceError> {
        let store = ctx.user_store.lock()?;
        store
            .get_replicant(replicant_name)?
            .ok_or_else(|| ServiceError::UserNotFound(format!("Replicant '{}'", replicant_name)))
    }

    /// List replicant identities for a human user.
    ///
    /// # REQ: svc-user-006 — list_replicants delegates to store
    pub fn list_replicants(
        ctx: &ServiceContext,
        user_id: &UserID,
    ) -> Result<Vec<ReplicantIdentity>, ServiceError> {
        let store = ctx.user_store.lock()?;
        store.list_replicants(user_id).map_err(Into::into)
    }

    /// List active sessions for a replicant.
    ///
    /// # REQ: svc-user-007 — list_sessions delegates to store
    pub fn list_sessions(
        ctx: &ServiceContext,
        replicant_name: &str,
    ) -> Result<Vec<UserSession>, ServiceError> {
        let store = ctx.user_store.lock()?;
        store.list_sessions(replicant_name).map_err(Into::into)
    }

    /// Revoke a session by ID.
    ///
    /// Composite operation: retrieves the session, then invalidates it.
    ///
    /// # REQ: svc-user-008 — revoke_session returns not-found for unknown session
    pub fn revoke_session(
        ctx: &ServiceContext,
        session_id: &str,
    ) -> Result<UserSession, ServiceError> {
        let store = ctx.user_store.lock()?;
        let session = store
            .get_session(session_id)?
            .ok_or_else(|| ServiceError::UserNotFound(format!("Session '{}'", session_id)))?;
        store.logout(session_id)?;
        Ok(session)
    }
}
