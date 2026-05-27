//! Replicant registration and authentication commands
//!
//! This module handles replicant identity registration and login.
//! A replicant is the in-system persona that a human uses to access hKask.
//!
//! ## Architecture
//!
//! Functions are split into two layers:
//! - **Application functions** (pure, no I/O): `register_replicant_with_passphrase`,
//!   `login_with_passphrase`, `get_replicant`, `get_replicants`, `get_sessions`,
//!   `revoke_session`
//! - **CLI adapters** (interactive I/O): `register_replicant`, `login_replicant`,
//!   `show_replicant`, `list_replicants`, `list_sessions`, `logout`

use crate::errors::UserError;
use hkask_storage::user_store::UserStore;
use hkask_types::RegistrationRequest;
use std::sync::{Arc, Mutex};
use zeroize::Zeroizing;

// =============================================================================
// Application functions — pure, no I/O, testable
// =============================================================================

/// Register a new replicant identity (non-interactive)
///
/// Validates the passphrase, constructs a registration request, and persists
/// the new human user + replicant identity via the store.
pub fn register_replicant_with_passphrase(
    store: &Arc<Mutex<UserStore>>,
    replicant_name: &str,
    first_name: &str,
    last_name: &str,
    email: &str,
    phone: Option<&str>,
    passphrase: Zeroizing<String>,
) -> Result<hkask_types::ReplicantIdentity, UserError> {
    RegistrationRequest::validate_passphrase(&passphrase)
        .map_err(|e| UserError::InvalidPassphrase(e.to_string()))?;

    let request = RegistrationRequest {
        replicant_name: replicant_name.to_string(),
        first_name: first_name.to_string(),
        last_name: last_name.to_string(),
        email: email.to_string(),
        phone: phone.map(|s| s.to_string()),
        passphrase: (*passphrase).clone(),
    };

    let store = store
        .lock()
        .map_err(|e| UserError::DatabaseError(format!("Lock poisoned: {}", e)))?;
    store
        .register_replicant(request)
        .map_err(|e| UserError::RegistrationFailed(e.to_string()))
}

/// Login as a replicant identity (non-interactive)
///
/// Verifies the passphrase against the stored hash and creates a session.
pub fn login_with_passphrase(
    store: &Arc<Mutex<UserStore>>,
    replicant_name: &str,
    passphrase: Zeroizing<String>,
) -> Result<hkask_types::UserSession, UserError> {
    let store = store
        .lock()
        .map_err(|e| UserError::DatabaseError(format!("Lock poisoned: {}", e)))?;
    store
        .login(replicant_name, &passphrase)
        .map_err(|_| UserError::LoginFailed("Invalid credentials".to_string()))
}

/// Get a replicant identity by name
pub fn get_replicant(
    store: &Arc<Mutex<UserStore>>,
    replicant_name: &str,
) -> Result<hkask_types::ReplicantIdentity, UserError> {
    let store = store
        .lock()
        .map_err(|e| UserError::DatabaseError(format!("Lock poisoned: {}", e)))?;
    store
        .get_replicant(replicant_name)
        .map_err(|e| UserError::DatabaseError(e.to_string()))?
        .ok_or_else(|| UserError::NotFound(format!("Replicant '{}'", replicant_name)))
}

/// List replicant identities for a human user
pub fn get_replicants(
    store: &Arc<Mutex<UserStore>>,
    user_id: &hkask_types::UserID,
) -> Result<Vec<hkask_types::ReplicantIdentity>, UserError> {
    let store = store
        .lock()
        .map_err(|e| UserError::DatabaseError(format!("Lock poisoned: {}", e)))?;
    store
        .list_replicants(user_id)
        .map_err(|e| UserError::DatabaseError(e.to_string()))
}

/// List active sessions for a replicant
pub fn get_sessions(
    store: &Arc<Mutex<UserStore>>,
    replicant_name: &str,
) -> Result<Vec<hkask_types::UserSession>, UserError> {
    let store = store
        .lock()
        .map_err(|e| UserError::DatabaseError(format!("Lock poisoned: {}", e)))?;
    store
        .list_sessions(replicant_name)
        .map_err(|e| UserError::DatabaseError(e.to_string()))
}

/// Revoke a session by ID
pub fn revoke_session(
    store: &Arc<Mutex<UserStore>>,
    session_id: &str,
) -> Result<hkask_types::UserSession, UserError> {
    let store = store
        .lock()
        .map_err(|e| UserError::DatabaseError(format!("Lock poisoned: {}", e)))?;
    let session = store
        .get_session(session_id)
        .map_err(|e| UserError::DatabaseError(e.to_string()))?
        .ok_or_else(|| UserError::SessionNotFound(session_id.to_string()))?;
    store
        .logout(session_id)
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;
    Ok(session)
}

// =============================================================================
// CLI adapters — interactive I/O wrappers
// =============================================================================

/// Register a new replicant identity (interactive)
///
/// Prompts for passphrase with confirmation, then delegates to
/// `register_replicant_with_passphrase`.
pub fn register_replicant(
    store: &Arc<Mutex<UserStore>>,
    replicant_name: &str,
    first_name: &str,
    last_name: &str,
    email: &str,
    phone: Option<&str>,
) -> Result<(), UserError> {
    use std::io::{self, Write};

    println!("\nPassphrase requirements:");
    println!("  - At least 8 characters");
    println!("  - Only alphanumeric (a-z, A-Z, 0-9)");
    println!("  - Must contain both uppercase and lowercase\n");

    loop {
        print!("Enter passphrase: ");
        io::stdout().flush().unwrap();
        let mut passphrase = String::new();
        io::stdin().read_line(&mut passphrase).unwrap();
        let passphrase = passphrase.trim().to_string();

        if let Err(e) = RegistrationRequest::validate_passphrase(&passphrase) {
            eprintln!("  ✗ {}", e);
            continue;
        }

        print!("Confirm passphrase: ");
        io::stdout().flush().unwrap();
        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm).unwrap();
        let confirm = confirm.trim().to_string();

        if passphrase != confirm {
            eprintln!("  ✗ Passphrases do not match");
            continue;
        }

        match register_replicant_with_passphrase(
            store,
            replicant_name,
            first_name,
            last_name,
            email,
            phone,
            Zeroizing::new(passphrase),
        ) {
            Ok(identity) => {
                println!("\n✅ Replicant registration successful!");
                println!("  Replicant name: {}", identity.replicant_name);
                println!("  WebID: {}", identity.replicant_webid.redacted_display());
                println!("\nYou can now login as this replicant:");
                println!("  kask replicant login {}", identity.replicant_name);
                return Ok(());
            }
            Err(e) => {
                eprintln!("\n✗ {}", e);
                return Err(e);
            }
        }
    }
}

/// Login as a replicant identity (interactive)
///
/// Prompts for passphrase, then delegates to `login_with_passphrase`.
pub fn login_replicant(
    store: &Arc<Mutex<UserStore>>,
    replicant_name: &str,
) -> Result<hkask_types::UserSession, UserError> {
    use std::io::{self, Write};

    print!("Enter passphrase for replicant '{}': ", replicant_name);
    io::stdout().flush().unwrap();
    let mut passphrase = String::new();
    io::stdin().read_line(&mut passphrase).unwrap();
    let passphrase = Zeroizing::new(passphrase.trim().to_string());

    let session = login_with_passphrase(store, replicant_name, passphrase)?;
    println!("\n✅ Login successful!");
    println!("  Welcome, {}!", session.replicant_name);
    println!("  Session ID: {}", session.session_id);
    Ok(session)
}

/// Show replicant identity info (interactive display)
pub fn show_replicant(
    store: &Arc<Mutex<UserStore>>,
    replicant_name: &str,
) -> Result<(), UserError> {
    let identity = get_replicant(store, replicant_name)?;

    println!("\n👤 Replicant Info:");
    println!("  Replicant name: {}", identity.replicant_name);
    println!("  WebID: {}", identity.replicant_webid.redacted_display());
    println!(
        "  User ID: {}",
        identity.user_id.0.to_string()[..8].to_string() + "..."
    );
    println!(
        "  Primary: {}",
        if identity.is_primary { "yes" } else { "no" }
    );
    println!(
        "  Created: {}",
        chrono::DateTime::from_timestamp(identity.created_at, 0)
            .unwrap()
            .format("%Y-%m-%d")
    );

    if let Some(last) = identity.last_login {
        let dt = chrono::DateTime::from_timestamp(last, 0).unwrap();
        println!("  Last login: {}", dt.format("%Y-%m-%d %H:%M"));
    }

    Ok(())
}

/// List replicant identities for a human user (interactive display)
pub fn list_replicants(
    store: &Arc<Mutex<UserStore>>,
    user_id: &hkask_types::UserID,
) -> Result<Vec<hkask_types::ReplicantIdentity>, UserError> {
    let identities = get_replicants(store, user_id)?;

    if identities.is_empty() {
        println!("  No replicant identities found for this user.");
        return Ok(identities);
    }

    println!("\n📋 Replicant identities:");
    for (i, identity) in identities.iter().enumerate() {
        let primary = if identity.is_primary {
            " (primary)"
        } else {
            ""
        };
        println!("  {}. {}{}", i + 1, identity.replicant_name, primary);
        println!(
            "     WebID: {}",
            identity.replicant_webid.redacted_display()
        );
        if let Some(last) = identity.last_login {
            let dt = chrono::DateTime::from_timestamp(last, 0).unwrap();
            println!("     Last login: {}", dt.format("%Y-%m-%d %H:%M"));
        }
    }

    Ok(identities)
}

/// Logout — invalidate a session (interactive display)
pub fn logout(store: &Arc<Mutex<UserStore>>, session_id: &str) -> Result<(), UserError> {
    let session = revoke_session(store, session_id)?;
    println!("\n✅ Logged out successfully!");
    println!("  Replicant: {}", session.replicant_name);
    println!("  Session: {}", &session_id[..8]);
    Ok(())
}

/// List active sessions for a replicant (interactive display)
pub fn list_sessions(
    store: &Arc<Mutex<UserStore>>,
    replicant_name: &str,
) -> Result<Vec<hkask_types::UserSession>, UserError> {
    let sessions = get_sessions(store, replicant_name)?;

    if sessions.is_empty() {
        println!("  No active sessions for '{}'.", replicant_name);
        return Ok(sessions);
    }

    println!("\n📱 Active sessions for '{}':", replicant_name);
    for (i, session) in sessions.iter().enumerate() {
        let expires = chrono::DateTime::from_timestamp(session.expires_at, 0).unwrap();
        let last_active = chrono::DateTime::from_timestamp(session.last_active, 0).unwrap();
        println!("  {}. Session: {}", i + 1, &session.session_id[..8]);
        println!("     Last active: {}", last_active.format("%Y-%m-%d %H:%M"));
        println!("     Expires: {}", expires.format("%Y-%m-%d %H:%M"));
    }

    Ok(sessions)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::user_store::UserStore;
    use rusqlite::Connection;

    struct TestFixture {
        store: Arc<Mutex<UserStore>>,
    }

    impl TestFixture {
        fn new() -> Self {
            let conn = Connection::open_in_memory().unwrap();
            let store = UserStore::new(Arc::new(Mutex::new(conn)));
            store.initialize_schema().unwrap();
            Self {
                store: Arc::new(Mutex::new(store)),
            }
        }

        fn register_default(&self) -> hkask_types::ReplicantIdentity {
            register_replicant_with_passphrase(
                &self.store,
                "alice",
                "Alice",
                "Smith",
                "alice@example.com",
                None,
                Zeroizing::new("ValidPass123".to_string()),
            )
            .unwrap()
        }

        fn register_user(
            &self,
            name: &str,
            email: &str,
            passphrase: &str,
        ) -> Result<hkask_types::ReplicantIdentity, UserError> {
            register_replicant_with_passphrase(
                &self.store,
                name,
                "First",
                "Last",
                email,
                None,
                Zeroizing::new(passphrase.to_string()),
            )
        }
    }

    // --- Registration ---

    #[test]
    fn test_register_valid() {
        let f = TestFixture::new();
        let identity = f.register_default();
        assert_eq!(identity.replicant_name, "alice");
        assert!(identity.is_primary);
    }

    #[test]
    fn test_register_passphrase_too_short() {
        let f = TestFixture::new();
        let err = f.register_user("bob", "bob@test.ai", "Ab1").unwrap_err();
        assert!(matches!(err, UserError::InvalidPassphrase(_)));
    }

    #[test]
    fn test_register_passphrase_no_uppercase() {
        let f = TestFixture::new();
        let err = f
            .register_user("bob", "bob@test.ai", "lowercase123")
            .unwrap_err();
        assert!(matches!(err, UserError::InvalidPassphrase(_)));
    }

    #[test]
    fn test_register_passphrase_no_lowercase() {
        let f = TestFixture::new();
        let err = f
            .register_user("bob", "bob@test.ai", "UPPERCASE123")
            .unwrap_err();
        assert!(matches!(err, UserError::InvalidPassphrase(_)));
    }

    #[test]
    fn test_register_passphrase_special_chars() {
        let f = TestFixture::new();
        let err = f
            .register_user("bob", "bob@test.ai", "Special@123")
            .unwrap_err();
        assert!(matches!(err, UserError::InvalidPassphrase(_)));
    }

    #[test]
    fn test_register_duplicate_name() {
        let f = TestFixture::new();
        f.register_default();
        let err = f
            .register_user("alice", "other@test.ai", "OtherPass123")
            .unwrap_err();
        assert!(matches!(err, UserError::RegistrationFailed(_)));
    }

    #[test]
    fn test_register_empty_name() {
        let f = TestFixture::new();
        let err = f.register_user("", "a@b.ai", "ValidPass123").unwrap_err();
        assert!(matches!(err, UserError::RegistrationFailed(_)));
    }

    #[test]
    fn test_register_invalid_email() {
        let f = TestFixture::new();
        let err = f
            .register_user("bob", "not-an-email", "ValidPass123")
            .unwrap_err();
        assert!(matches!(err, UserError::RegistrationFailed(_)));
    }

    #[test]
    fn test_register_with_phone() {
        let f = TestFixture::new();
        let identity = register_replicant_with_passphrase(
            &f.store,
            "carol",
            "Carol",
            "Danvers",
            "carol@test.ai",
            Some("+15551234567"),
            Zeroizing::new("CarolPass123".to_string()),
        )
        .unwrap();
        assert_eq!(identity.replicant_name, "carol");
    }

    #[test]
    fn test_register_invalid_phone() {
        let f = TestFixture::new();
        let err = register_replicant_with_passphrase(
            &f.store,
            "dave",
            "Dave",
            "Test",
            "dave@test.ai",
            Some("555-1234"),
            Zeroizing::new("DavePass123".to_string()),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::RegistrationFailed(_)));
    }

    // --- Login ---

    #[test]
    fn test_login_success() {
        let f = TestFixture::new();
        f.register_default();
        let session = login_with_passphrase(
            &f.store,
            "alice",
            Zeroizing::new("ValidPass123".to_string()),
        )
        .unwrap();
        assert_eq!(session.replicant_name, "alice");
        assert!(!session.session_id.is_empty());
    }

    #[test]
    fn test_login_wrong_passphrase() {
        let f = TestFixture::new();
        f.register_default();
        let err = login_with_passphrase(
            &f.store,
            "alice",
            Zeroizing::new("WrongPass123".to_string()),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::LoginFailed(_)));
    }

    #[test]
    fn test_login_nonexistent_user() {
        let f = TestFixture::new();
        let err = login_with_passphrase(
            &f.store,
            "ghost",
            Zeroizing::new("GhostPass123".to_string()),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::LoginFailed(_)));
    }

    // --- Session management ---

    #[test]
    fn test_revoke_session() {
        let f = TestFixture::new();
        f.register_default();
        let session = login_with_passphrase(
            &f.store,
            "alice",
            Zeroizing::new("ValidPass123".to_string()),
        )
        .unwrap();
        let revoked = revoke_session(&f.store, &session.session_id).unwrap();
        assert_eq!(revoked.replicant_name, "alice");
    }

    #[test]
    fn test_revoke_nonexistent_session() {
        let f = TestFixture::new();
        let err = revoke_session(&f.store, "no-such-session").unwrap_err();
        assert!(matches!(err, UserError::SessionNotFound(_)));
    }

    #[test]
    fn test_get_sessions_empty() {
        let f = TestFixture::new();
        f.register_default();
        let sessions = get_sessions(&f.store, "alice").unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_get_sessions_after_login() {
        let f = TestFixture::new();
        f.register_default();
        login_with_passphrase(
            &f.store,
            "alice",
            Zeroizing::new("ValidPass123".to_string()),
        )
        .unwrap();
        let sessions = get_sessions(&f.store, "alice").unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].replicant_name, "alice");
    }

    // --- Queries ---

    #[test]
    fn test_get_replicant() {
        let f = TestFixture::new();
        f.register_default();
        let identity = get_replicant(&f.store, "alice").unwrap();
        assert_eq!(identity.replicant_name, "alice");
        assert!(identity.is_primary);
    }

    #[test]
    fn test_get_replicant_not_found() {
        let f = TestFixture::new();
        let err = get_replicant(&f.store, "ghost").unwrap_err();
        assert!(matches!(err, UserError::NotFound(_)));
    }

    #[test]
    fn test_get_replicants_empty() {
        let f = TestFixture::new();
        let user_id = hkask_types::UserID::new();
        let result = get_replicants(&f.store, &user_id).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_replicants_after_registration() {
        let f = TestFixture::new();
        let identity = f.register_default();
        let result = get_replicants(&f.store, &identity.user_id).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].replicant_name, "alice");
    }
}
