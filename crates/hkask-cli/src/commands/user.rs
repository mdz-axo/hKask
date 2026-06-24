//! Replicant registration and authentication — delegates to AgentService.

use std::sync::{Arc, Mutex};

use crate::cli::ReplicantAction;
use hkask_services::ServiceError;
use hkask_services_core::{RegistrationRequest, ReplicantIdentity, UserSession};
use hkask_storage::user_store::UserStore;
use hkask_types::UserID;
use zeroize::Zeroizing;

type Store = Arc<Mutex<UserStore>>;

fn build_store() -> Store {
    crate::commands::helpers::build_service_context()
        .user_store()
        .clone()
}

/// Unwrap an I/O result or print the error and exit.
/// Used for interactive stdin/stdout operations where failure is terminal.
fn io_or_die<T>(result: std::io::Result<T>, context: &str) -> T {
    result.unwrap_or_else(|e| {
        eprintln!("I/O error ({context}): {e}");
        std::process::exit(1);
    })
}

fn validate_passphrase(passphrase: &str) -> Result<(), ServiceError> {
    if passphrase.len() < 8 || !passphrase.chars().all(|c| c.is_alphanumeric()) {
        return Err(ServiceError::InvalidPassphrase {
            source: None,
            message: "Passphrase does not meet requirements: 8+ alphanumeric chars, mixed case"
                .into(),
        });
    }
    let has_upper = passphrase.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = passphrase.chars().any(|c| c.is_ascii_lowercase());
    if !has_upper || !has_lower {
        return Err(ServiceError::InvalidPassphrase {
            source: None,
            message: "Passphrase does not meet requirements: 8+ alphanumeric chars, mixed case"
                .into(),
        });
    }
    Ok(())
}

fn validate_registration(request: &RegistrationRequest) -> Result<(), ServiceError> {
    if request.replicant_name.is_empty()
        || request.replicant_name.len() > 64
        || !request
            .replicant_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ServiceError::ValidationError {
            source: None,
            message: "Invalid replicant name".into(),
        });
    }
    if request.first_name.is_empty() || request.last_name.is_empty() {
        return Err(ServiceError::ValidationError {
            source: None,
            message: "Required name field is empty".into(),
        });
    }
    if request.email.is_empty() || !request.email.contains('@') {
        return Err(ServiceError::ValidationError {
            source: None,
            message: "Invalid contact information format".into(),
        });
    }
    if let Some(phone) = &request.phone
        && !phone.starts_with('+')
    {
        return Err(ServiceError::ValidationError {
            source: None,
            message: "Invalid contact information format".into(),
        });
    }
    validate_passphrase(&request.passphrase)?;
    Ok(())
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  store is a valid UserStore; replicant_name, first_name, last_name, email are non-empty; passphrase meets validation (8+ alphanumeric, mixed case)
/// post: registers a new replicant identity in the store; returns ReplicantIdentity on success or ServiceError on validation/store failure
pub fn register_replicant_with_passphrase(
    store: &Store,
    replicant_name: &str,
    first_name: &str,
    last_name: &str,
    email: &str,
    phone: Option<&str>,
    passphrase: Zeroizing<String>,
) -> Result<ReplicantIdentity, ServiceError> {
    validate_passphrase(&passphrase)?;
    let request = RegistrationRequest {
        replicant_name: replicant_name.to_string(),
        first_name: first_name.to_string(),
        last_name: last_name.to_string(),
        email: email.to_string(),
        phone: phone.map(|s| s.to_string()),
        passphrase: (*passphrase).clone(),
    };
    validate_registration(&request)?;
    store
        .lock()
        .expect("CLI operation")
        .register_replicant(
            request.replicant_name,
            request.email,
            request.phone,
            request.first_name,
            request.last_name,
            request.passphrase,
        )
        .map_err(|e| ServiceError::UserStore {
            message: e.to_string(),
        })
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  store is a valid UserStore; replicant_name is non-empty; passphrase is the correct credential
/// post: returns a UserSession on successful authentication or ServiceError::LoginFailed on invalid credentials
pub fn login_with_passphrase(
    store: &Store,
    replicant_name: &str,
    passphrase: Zeroizing<String>,
) -> Result<UserSession, ServiceError> {
    store
        .lock()
        .expect("CLI operation")
        .login(replicant_name, &passphrase)
        .map_err(|_| ServiceError::LoginFailed {
            source: None,
            message: "Invalid credentials".into(),
        })
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  store is a valid UserStore; replicant_name is non-empty
/// post: returns the ReplicantIdentity if found, or ServiceError::UserNotFound if the replicant does not exist
pub fn get_replicant(
    store: &Store,
    replicant_name: &str,
) -> Result<ReplicantIdentity, ServiceError> {
    store
        .lock()
        .expect("CLI operation")
        .get_replicant(replicant_name)
        .map_err(|e| ServiceError::UserStore {
            message: e.to_string(),
        })?
        .ok_or_else(|| ServiceError::UserNotFound {
            source: None,
            message: format!("Replicant '{}'", replicant_name),
        })
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  store is a valid UserStore; user_id is a valid UserID
/// post: returns all replicant identities belonging to the given user; empty vec if none
pub fn get_replicants(
    store: &Store,
    user_id: &UserID,
) -> Result<Vec<ReplicantIdentity>, ServiceError> {
    store
        .lock()
        .expect("CLI operation")
        .list_replicants(user_id)
        .map_err(|e| ServiceError::UserStore {
            message: e.to_string(),
        })
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  store is a valid UserStore; replicant_name is non-empty
/// post: returns all active sessions for the replicant; empty vec if none
pub fn get_sessions(store: &Store, replicant_name: &str) -> Result<Vec<UserSession>, ServiceError> {
    store
        .lock()
        .expect("CLI operation")
        .list_sessions(replicant_name)
        .map_err(|e| ServiceError::UserStore {
            message: e.to_string(),
        })
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  store is a valid UserStore; session_id is a non-empty session identifier
/// post: revokes the session (logs out) and returns the revoked UserSession; ServiceError if session not found
pub fn revoke_session(store: &Store, session_id: &str) -> Result<UserSession, ServiceError> {
    let session = store
        .lock()
        .expect("CLI operation")
        .get_session(session_id)
        .map_err(|e| ServiceError::UserStore {
            message: e.to_string(),
        })?
        .ok_or_else(|| ServiceError::UserNotFound {
            source: None,
            message: format!("Session '{}'", session_id),
        })?;
    store
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .logout(session_id)
        .map_err(|e| ServiceError::UserStore {
            message: e.to_string(),
        })?;
    Ok(session)
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  stdin is available for interactive input
/// post: prompts user for replicant details and passphrase; registers on success, prints ✓; exits on validation failure
/// Register a new replicant identity (interactive)
pub fn register_replicant() {
    use std::io::{self, Write};
    let mut name = String::new();
    let mut first = String::new();
    let mut last = String::new();
    let mut email = String::new();
    let mut phone = String::new();

    print!("Replicant name: ");
    io_or_die(io::stdout().flush(), "flush stdout");
    io_or_die(io::stdin().read_line(&mut name), "read name");
    print!("First name: ");
    io_or_die(io::stdout().flush(), "flush stdout");
    io_or_die(io::stdin().read_line(&mut first), "read first name");
    print!("Last name: ");
    io_or_die(io::stdout().flush(), "flush stdout");
    io_or_die(io::stdin().read_line(&mut last), "read last name");
    print!("Email: ");
    io_or_die(io::stdout().flush(), "flush stdout");
    io_or_die(io::stdin().read_line(&mut email), "read email");
    print!("Phone (optional): ");
    io_or_die(io::stdout().flush(), "flush stdout");
    io_or_die(io::stdin().read_line(&mut phone), "read phone");

    loop {
        print!("Enter passphrase: ");
        io_or_die(io::stdout().flush(), "flush stdout");
        let mut passphrase = String::new();
        io_or_die(io::stdin().read_line(&mut passphrase), "read passphrase");
        let passphrase = passphrase.trim().to_string();
        if let Err(e) = validate_passphrase(&passphrase) {
            eprintln!("  ✗ {}", e);
            continue;
        }
        let store = build_store();
        match register_replicant_with_passphrase(
            &store,
            name.trim(),
            first.trim(),
            last.trim(),
            email.trim(),
            if phone.trim().is_empty() {
                None
            } else {
                Some(phone.trim())
            },
            Zeroizing::new(passphrase),
        ) {
            Ok(identity) => {
                println!("  ✓ Replicant registered: {}", identity.replicant_name);
                return;
            }
            Err(e) => {
                eprintln!("  ✗ {}", e);
                break;
            }
        }
    }
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  stdin is available; replicant must exist in store
/// post: prompts for name and passphrase; prints session info on success or error message on failure
pub fn login_replicant() {
    use std::io::{self, Write};
    let mut name = String::new();
    print!("Replicant name: ");
    io_or_die(io::stdout().flush(), "flush stdout");
    io_or_die(io::stdin().read_line(&mut name), "read name");
    let store = build_store();
    if let Ok(Some(identity)) = store
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_replicant(name.trim())
    {
        print!("Enter passphrase: ");
        io_or_die(io::stdout().flush(), "flush stdout");
        let mut passphrase = String::new();
        io_or_die(io::stdin().read_line(&mut passphrase), "read passphrase");
        match store
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .login(name.trim(), passphrase.trim())
        {
            Ok(session) => {
                println!("  ✓ Logged in as {}", identity.replicant_name);
                println!("  Session: {}", session.session_id);
            }
            Err(_) => eprintln!("  ✗ Login failed"),
        }
    } else {
        eprintln!("  ✗ Replicant not found: {}", name.trim());
    }
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  store is a valid UserStore; replicant_name is non-empty and exists
/// post: prints replicant details (name, user_id, created_at, primary status) to stdout; ServiceError if not found
pub fn show_replicant(store: &Store, replicant_name: &str) -> Result<(), ServiceError> {
    let identity = store
        .lock()
        .expect("CLI operation")
        .get_replicant(replicant_name)
        .map_err(|e| ServiceError::UserStore {
            message: e.to_string(),
        })?
        .ok_or_else(|| ServiceError::UserNotFound {
            source: None,
            message: format!("Replicant '{}'", replicant_name),
        })?;
    println!("Replicant: {}", identity.replicant_name);
    println!("  User ID: {}", identity.user_id);
    println!("  Created: {}", identity.created_at);
    if identity.is_primary {
        println!("  Primary: yes");
    }
    Ok(())
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  store is a valid UserStore
/// post: prints all replicants with name, primary status, user_id, and created_at; prints "No replicants registered." if empty
pub fn list_replicants(store: &Store) -> Result<(), ServiceError> {
    let user_id = hkask_types::UserID::new();
    let replicants = store
        .lock()
        .expect("CLI operation")
        .list_replicants(&user_id)
        .map_err(|e| ServiceError::UserStore {
            message: e.to_string(),
        })?;
    if replicants.is_empty() {
        println!("No replicants registered.");
        return Ok(());
    }
    println!("Replicants ({}):", replicants.len());
    for r in replicants {
        println!(
            "  {} ({})",
            r.replicant_name,
            if r.is_primary { "primary" } else { "secondary" }
        );
        println!("    User ID: {}", r.user_id);
        println!("    Created: {}", r.created_at);
    }
    Ok(())
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  store is a valid UserStore; session_id is a non-empty active session identifier
/// post: revokes the session and prints confirmation; ServiceError if session not found
pub fn logout(store: &Store, session_id: &str) -> Result<(), ServiceError> {
    let session = store
        .lock()
        .expect("CLI operation")
        .get_session(session_id)
        .map_err(|e| ServiceError::UserStore {
            message: e.to_string(),
        })?
        .ok_or_else(|| ServiceError::UserNotFound {
            source: None,
            message: format!("Session '{}'", session_id),
        })?;
    store
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .logout(session_id)
        .map_err(|e| ServiceError::UserStore {
            message: e.to_string(),
        })?;
    println!("Session revoked: {}", session.session_id);
    Ok(())
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  store is a valid UserStore; replicant_name is non-empty
/// post: prints all active sessions with session_id and last_active timestamp; prints "No active sessions." if none
pub fn list_sessions(store: &Store, replicant_name: &str) -> Result<(), ServiceError> {
    let sessions = store
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .list_sessions(replicant_name)
        .map_err(|e| ServiceError::UserStore {
            message: e.to_string(),
        })?;
    if sessions.is_empty() {
        println!("No active sessions.");
        return Ok(());
    }
    println!("Active sessions for {}: {}", replicant_name, sessions.len());
    for s in sessions {
        let last_active = chrono::DateTime::from_timestamp(s.last_active, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();
        println!("  Session: {}", s.session_id);
        println!("     Last active: {}", last_active);
    }
    Ok(())
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  action is a valid ReplicantAction variant
/// post: dispatches to the appropriate handler (register, login, show, list, sessions, logout, passphrase); prints results or errors
pub fn run_replicant(action: crate::cli::ReplicantAction) {
    match action {
        ReplicantAction::Register { .. } => register_replicant(),
        ReplicantAction::Login { .. } => login_replicant(),
        ReplicantAction::Show { replicant_name } => {
            let store = build_store();
            super::helpers::or_exit(show_replicant(&store, &replicant_name), "Show failed");
        }
        ReplicantAction::List { .. } => {
            let store = build_store();
            super::helpers::or_exit(list_replicants(&store), "List failed");
        }
        ReplicantAction::Sessions { replicant_name } => {
            let store = build_store();
            super::helpers::or_exit(list_sessions(&store, &replicant_name), "Sessions failed");
        }
        ReplicantAction::Logout { session_id } => {
            let store = build_store();
            super::helpers::or_exit(logout(&store, &session_id), "Logout failed");
        }
        ReplicantAction::Passphrase { replicant_name } => {
            change_passphrase(&replicant_name);
        }
        ReplicantAction::Rename { from, to } => {
            replicant_rename(&from, &to);
        }
        ReplicantAction::Merge { from, into } => {
            replicant_merge(&from, &into);
        }
        ReplicantAction::Delete { name } => {
            replicant_delete(&name);
        }
    }
}

/// Rename a replicant via the API.
fn replicant_rename(from: &str, to: &str) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        let client = reqwest::Client::new();
        let base_url =
            std::env::var("HKASK_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let resp = client
            .post(format!("{base_url}/api/v1/replicants/rename"))
            .json(&serde_json::json!({"from": from, "to": to}))
            .send()
            .await;
        match resp {
            Ok(r) if r.status().is_success() => {
                println!("Replicant renamed: {from} -> {to}");
            }
            Ok(r) => {
                let body = r.text().await.unwrap_or_default();
                eprintln!("Rename failed: {body}");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Request failed: {e}");
                std::process::exit(1);
            }
        }
    });
}

/// Merge two replicants via the API.
fn replicant_merge(from: &str, into: &str) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        let client = reqwest::Client::new();
        let base_url =
            std::env::var("HKASK_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let resp = client
            .post(format!("{base_url}/api/v1/replicants/merge"))
            .json(&serde_json::json!({"from": from, "into": into}))
            .send()
            .await;
        match resp {
            Ok(r) if r.status().is_success() => {
                let receipt: serde_json::Value = r.json().await.unwrap_or_default();
                println!(
                    "Merged: {} triples from {from} into {into}",
                    receipt["triple_count"].as_u64().unwrap_or(0)
                );
            }
            Ok(r) => {
                let body = r.text().await.unwrap_or_default();
                eprintln!("Merge failed: {body}");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Request failed: {e}");
                std::process::exit(1);
            }
        }
    });
}

/// Delete a replicant via the API.
fn replicant_delete(name: &str) {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async {
        let client = reqwest::Client::new();
        let base_url =
            std::env::var("HKASK_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let resp = client
            .delete(format!("{base_url}/api/v1/replicants/{name}"))
            .send()
            .await;
        match resp {
            Ok(r) if r.status().is_success() => {
                println!("Replicant deleted: {name}");
            }
            Ok(r) => {
                let body = r.text().await.unwrap_or_default();
                eprintln!("Delete failed: {body}");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Request failed: {e}");
                std::process::exit(1);
            }
        }
    });
}

/// expect: "I can access all hKask functionality through the kask CLI"
/// pre:  replicant_name exists in store; stdin is available for interactive input
/// post: prompts for old and new passphrase; validates match; updates passphrase and invalidates existing sessions on success
/// Interactive passphrase change for a replicant.
pub fn change_passphrase(replicant_name: &str) {
    use std::io::{self, Write};
    let store = build_store();

    // Verify identity exists
    if store
        .lock()
        .expect("CLI operation")
        .get_replicant(replicant_name)
        .unwrap_or(None)
        .is_none()
    {
        eprintln!("  ✗ Replicant not found: {}", replicant_name);
        return;
    }

    print!("Old passphrase: ");
    io_or_die(io::stdout().flush(), "flush stdout");
    let mut old_passphrase = String::new();
    io_or_die(
        io::stdin().read_line(&mut old_passphrase),
        "read old passphrase",
    );

    print!("New passphrase: ");
    io_or_die(io::stdout().flush(), "flush stdout");
    let mut new_passphrase = String::new();
    io_or_die(
        io::stdin().read_line(&mut new_passphrase),
        "read new passphrase",
    );

    print!("Confirm new passphrase: ");
    io_or_die(io::stdout().flush(), "flush stdout");
    let mut confirm = String::new();
    io_or_die(io::stdin().read_line(&mut confirm), "read confirm");

    if new_passphrase.trim() != confirm.trim() {
        eprintln!("  ✗ Passphrases do not match");
        return;
    }

    match store
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .change_passphrase(replicant_name, old_passphrase.trim(), new_passphrase.trim())
    {
        Ok(()) => {
            println!("  ✓ Passphrase changed for {}", replicant_name);
            println!("  All existing sessions invalidated — login again.");
        }
        Err(e) => eprintln!("  ✗ {}", e),
    }
}
