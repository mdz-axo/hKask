//! Replicant registration and authentication — delegates to AgentService.

use std::sync::{Arc, Mutex};

use crate::cli::ReplicantAction;
use hkask_services::ServiceError;
use hkask_storage::user_store::UserStore;
use hkask_types::UserID;
use hkask_types::identity::{RegistrationRequest, ReplicantIdentity, UserSession};
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

/// REQ: CLI-051
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
        .unwrap()
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

/// REQ: CLI-052
/// pre:  store is a valid UserStore; replicant_name is non-empty; passphrase is the correct credential
/// post: returns a UserSession on successful authentication or ServiceError::LoginFailed on invalid credentials
pub fn login_with_passphrase(
    store: &Store,
    replicant_name: &str,
    passphrase: Zeroizing<String>,
) -> Result<UserSession, ServiceError> {
    store
        .lock()
        .unwrap()
        .login(replicant_name, &passphrase)
        .map_err(|_| ServiceError::LoginFailed {
            source: None,
            message: "Invalid credentials".into(),
        })
}

/// REQ: CLI-053
/// pre:  store is a valid UserStore; replicant_name is non-empty
/// post: returns the ReplicantIdentity if found, or ServiceError::UserNotFound if the replicant does not exist
pub fn get_replicant(
    store: &Store,
    replicant_name: &str,
) -> Result<ReplicantIdentity, ServiceError> {
    store
        .lock()
        .unwrap()
        .get_replicant(replicant_name)?
        .ok_or_else(|| ServiceError::UserNotFound {
            source: None,
            message: format!("Replicant '{}'", replicant_name),
        })
}

/// REQ: CLI-054
/// pre:  store is a valid UserStore; user_id is a valid UserID
/// post: returns all replicant identities belonging to the given user; empty vec if none
pub fn get_replicants(
    store: &Store,
    user_id: &UserID,
) -> Result<Vec<ReplicantIdentity>, ServiceError> {
    store
        .lock()
        .unwrap()
        .list_replicants(user_id)
        .map_err(Into::into)
}

/// REQ: CLI-055
/// pre:  store is a valid UserStore; replicant_name is non-empty
/// post: returns all active sessions for the replicant; empty vec if none
pub fn get_sessions(store: &Store, replicant_name: &str) -> Result<Vec<UserSession>, ServiceError> {
    store
        .lock()
        .unwrap()
        .list_sessions(replicant_name)
        .map_err(Into::into)
}

/// REQ: CLI-056
/// pre:  store is a valid UserStore; session_id is a non-empty session identifier
/// post: revokes the session (logs out) and returns the revoked UserSession; ServiceError if session not found
pub fn revoke_session(store: &Store, session_id: &str) -> Result<UserSession, ServiceError> {
    let session = store
        .lock()
        .unwrap()
        .get_session(session_id)?
        .ok_or_else(|| ServiceError::UserNotFound {
            source: None,
            message: format!("Session '{}'", session_id),
        })?;
    store.lock().unwrap().logout(session_id)?;
    Ok(session)
}

/// REQ: CLI-057
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

/// REQ: CLI-058
/// pre:  stdin is available; replicant must exist in store
/// post: prompts for name and passphrase; prints session info on success or error message on failure
pub fn login_replicant() {
    use std::io::{self, Write};
    let mut name = String::new();
    print!("Replicant name: ");
    io_or_die(io::stdout().flush(), "flush stdout");
    io_or_die(io::stdin().read_line(&mut name), "read name");
    let store = build_store();
    if let Ok(Some(identity)) = store.lock().unwrap().get_replicant(name.trim()) {
        print!("Enter passphrase: ");
        io_or_die(io::stdout().flush(), "flush stdout");
        let mut passphrase = String::new();
        io_or_die(io::stdin().read_line(&mut passphrase), "read passphrase");
        match store.lock().unwrap().login(name.trim(), passphrase.trim()) {
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

/// REQ: CLI-059
/// pre:  store is a valid UserStore; replicant_name is non-empty and exists
/// post: prints replicant details (name, user_id, created_at, primary status) to stdout; ServiceError if not found
pub fn show_replicant(store: &Store, replicant_name: &str) -> Result<(), ServiceError> {
    let identity = store
        .lock()
        .unwrap()
        .get_replicant(replicant_name)?
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

/// REQ: CLI-060
/// pre:  store is a valid UserStore
/// post: prints all replicants with name, primary status, user_id, and created_at; prints "No replicants registered." if empty
pub fn list_replicants(store: &Store) -> Result<(), ServiceError> {
    let user_id = hkask_types::UserID::new();
    let replicants = store
        .lock()
        .unwrap()
        .list_replicants(&user_id)
        .map_err(ServiceError::from)?;
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

/// REQ: CLI-061
/// pre:  store is a valid UserStore; session_id is a non-empty active session identifier
/// post: revokes the session and prints confirmation; ServiceError if session not found
pub fn logout(store: &Store, session_id: &str) -> Result<(), ServiceError> {
    let session = store
        .lock()
        .unwrap()
        .get_session(session_id)?
        .ok_or_else(|| ServiceError::UserNotFound {
            source: None,
            message: format!("Session '{}'", session_id),
        })?;
    store.lock().unwrap().logout(session_id)?;
    println!("Session revoked: {}", session.session_id);
    Ok(())
}

/// REQ: CLI-062
/// pre:  store is a valid UserStore; replicant_name is non-empty
/// post: prints all active sessions with session_id and last_active timestamp; prints "No active sessions." if none
pub fn list_sessions(store: &Store, replicant_name: &str) -> Result<(), ServiceError> {
    let sessions = store.lock().unwrap().list_sessions(replicant_name)?;
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

/// REQ: CLI-063
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
    }
}

/// REQ: CLI-064
/// pre:  replicant_name exists in store; stdin is available for interactive input
/// post: prompts for old and new passphrase; validates match; updates passphrase and invalidates existing sessions on success
/// Interactive passphrase change for a replicant.
pub fn change_passphrase(replicant_name: &str) {
    use std::io::{self, Write};
    let store = build_store();

    // Verify identity exists
    if store
        .lock()
        .unwrap()
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

    match store.lock().unwrap().change_passphrase(
        replicant_name,
        old_passphrase.trim(),
        new_passphrase.trim(),
    ) {
        Ok(()) => {
            println!("  ✓ Passphrase changed for {}", replicant_name);
            println!("  All existing sessions invalidated — login again.");
        }
        Err(e) => eprintln!("  ✗ {}", e),
    }
}
