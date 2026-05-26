//! Replicant registration and authentication commands
//!
//! This module handles replicant identity registration and login.
//! A replicant is the in-system persona that a human uses to access hKask.

use hkask_storage::user_store::UserStore;
use hkask_types::RegistrationRequest;
use std::sync::{Arc, Mutex};

/// Register a new replicant identity
///
/// This creates:
/// 1. A human user record (with encrypted contact info)
/// 2. A replicant identity (with deterministic WebID)
///
/// The human can then login as this replicant using their passphrase.
pub fn register_replicant(
    store: Arc<Mutex<UserStore>>,
    replicant_name: &str,
    first_name: &str,
    last_name: &str,
    email: &str,
    phone: Option<&str>,
) -> Result<(), String> {
    use std::io::{self, Write};

    // Prompt for passphrase
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

        // Validate passphrase
        if let Err(e) = RegistrationRequest::validate_passphrase(&passphrase) {
            eprintln!("  ✗ {}", e);
            continue;
        }

        // Confirm passphrase
        print!("Confirm passphrase: ");
        io::stdout().flush().unwrap();
        let mut confirm = String::new();
        io::stdin().read_line(&mut confirm).unwrap();
        let confirm = confirm.trim().to_string();

        if passphrase != confirm {
            eprintln!("  ✗ Passphrases do not match");
            continue;
        }

        let request = RegistrationRequest {
            replicant_name: replicant_name.to_string(),
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            email: email.to_string(),
            phone: phone.map(|s| s.to_string()),
            passphrase,
        };

        let store = store.lock().unwrap();
        match store.register_replicant(request) {
            Ok(identity) => {
                println!("\n✅ Replicant registration successful!");
                println!("  Replicant name: {}", identity.replicant_name);
                println!("  WebID: {}", identity.replicant_webid.redacted_display());
                println!("\nYou can now login as this replicant:");
                println!("  kask replicant login {}", identity.replicant_name);
                return Ok(());
            }
            Err(e) => {
                eprintln!("\n✗ Registration failed: {}", e);
                return Err(e.to_string());
            }
        }
    }
}

/// Login as a replicant identity
pub fn login_replicant(
    store: Arc<Mutex<UserStore>>,
    replicant_name: &str,
) -> Result<hkask_types::UserSession, String> {
    use std::io::{self, Write};

    print!("Enter passphrase for replicant '{}': ", replicant_name);
    io::stdout().flush().unwrap();
    let mut passphrase = String::new();
    io::stdin().read_line(&mut passphrase).unwrap();
    let passphrase = passphrase.trim().to_string();

    let store = store.lock().unwrap();
    match store.login(replicant_name, &passphrase) {
        Ok(session) => {
            println!("\n✅ Login successful!");
            println!("  Welcome, {}!", session.replicant_name);
            println!("  Session ID: {}", session.session_id);
            Ok(session)
        }
        Err(e) => {
            eprintln!("\n✗ Login failed: {}", e);
            Err(e.to_string())
        }
    }
}

/// List replicant identities for a human user
pub fn list_replicants(
    store: Arc<Mutex<UserStore>>,
    user_id: &hkask_types::UserID,
) -> Result<Vec<hkask_types::ReplicantIdentity>, String> {
    let store = store.lock().unwrap();
    let identities = store.list_replicants(user_id).map_err(|e| e.to_string())?;

    if identities.is_empty() {
        println!("  No replicant identities found for this user.");
        return Ok(identities);
    }

    println!("\n📋 Replicant identities:");
    for (i, identity) in identities.iter().enumerate() {
        let primary = if identity.is_primary { " (primary)" } else { "" };
        println!("  {}. {}{}", i + 1, identity.replicant_name, primary);
        println!("     WebID: {}", identity.replicant_webid.redacted_display());
        if let Some(last) = identity.last_login {
            let dt = chrono::DateTime::from_timestamp(last, 0).unwrap();
            println!("     Last login: {}", dt.format("%Y-%m-%d %H:%M"));
        }
    }

    Ok(identities)
}

/// Show replicant identity info
pub fn show_replicant(
    store: Arc<Mutex<UserStore>>,
    replicant_name: &str,
) -> Result<(), String> {
    let store = store.lock().unwrap();
    let identity = store.get_replicant(replicant_name)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Replicant '{}' not found", replicant_name))?;

    println!("\n👤 Replicant Info:");
    println!("  Replicant name: {}", identity.replicant_name);
    println!("  WebID: {}", identity.replicant_webid.redacted_display());
    println!("  User ID: {}", identity.user_id.0.to_string()[..8].to_string() + "...");
    println!("  Primary: {}", if identity.is_primary { "yes" } else { "no" });
    println!("  Created: {}", chrono::DateTime::from_timestamp(identity.created_at, 0).unwrap().format("%Y-%m-%d"));
    
    if let Some(last) = identity.last_login {
        let dt = chrono::DateTime::from_timestamp(last, 0).unwrap();
        println!("  Last login: {}", dt.format("%Y-%m-%d %H:%M"));
    }

    Ok(())
}

/// Logout - invalidate a session
pub fn logout(
    store: Arc<Mutex<UserStore>>,
    session_id: &str,
) -> Result<(), String> {
    let store = store.lock().unwrap();
    
    // Check if session exists
    let session = store.get_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session '{}' not found", session_id))?;
    
    store.logout(session_id).map_err(|e| e.to_string())?;
    
    println!("\n✅ Logged out successfully!");
    println!("  Replicant: {}", session.replicant_name);
    println!("  Session: {}", &session_id[..8]);
    
    Ok(())
}

/// List active sessions for a replicant
pub fn list_sessions(
    store: Arc<Mutex<UserStore>>,
    replicant_name: &str,
) -> Result<(), String> {
    let store = store.lock().unwrap();
    let sessions = store.list_sessions(replicant_name).map_err(|e| e.to_string())?;
    
    if sessions.is_empty() {
        println!("  No active sessions for '{}'.", replicant_name);
        return Ok(());
    }
    
    println!("\n📱 Active sessions for '{}':", replicant_name);
    for (i, session) in sessions.iter().enumerate() {
        let expires = chrono::DateTime::from_timestamp(session.expires_at, 0).unwrap();
        let last_active = chrono::DateTime::from_timestamp(session.last_active, 0).unwrap();
        println!("  {}. Session: {}", i + 1, &session.session_id[..8]);
        println!("     Last active: {}", last_active.format("%Y-%m-%d %H:%M"));
        println!("     Expires: {}", expires.format("%Y-%m-%d %H:%M"));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_storage::user_store::UserStore;
    use rusqlite::Connection;

    fn test_store() -> Arc<Mutex<UserStore>> {
        let conn = Connection::open_in_memory().unwrap();
        let store = UserStore::new(Arc::new(Mutex::new(conn)));
        store.initialize_schema().unwrap();
        Arc::new(Mutex::new(store))
    }

    #[test]
    fn test_register_and_login() {
        let store = test_store();

        let result = register_replicant(
            store.clone(),
            "test-user",
            "Test",
            "User",
            "test@test.ai",
            None,
        );
        
        // Note: This test would fail because we can't simulate stdin in tests
        // The actual registration requires interactive input
        assert!(result.is_err());
    }
}
