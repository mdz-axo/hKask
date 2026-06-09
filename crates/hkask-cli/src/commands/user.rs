//! Replicant registration and authentication — call user store directly.

use std::sync::{Arc, Mutex};

use crate::cli::ReplicantAction;
use crate::errors::UserError;
use hkask_storage::user_store::UserStore;
use hkask_types::{RegistrationRequest, ReplicantIdentity, UserID, UserSession};
use zeroize::Zeroizing;

type Store = Arc<Mutex<UserStore>>;

fn validate_passphrase(passphrase: &str) -> Result<(), UserError> {
    if passphrase.len() < 8 || !passphrase.chars().all(|c| c.is_alphanumeric()) {
        return Err(UserError::from(
            hkask_services::ServiceError::InvalidPassphrase(
                "Passphrase does not meet requirements: 8+ alphanumeric chars, mixed case".into(),
            ),
        ));
    }
    let has_upper = passphrase.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = passphrase.chars().any(|c| c.is_ascii_lowercase());
    if !has_upper || !has_lower {
        return Err(UserError::from(
            hkask_services::ServiceError::InvalidPassphrase(
                "Passphrase does not meet requirements: 8+ alphanumeric chars, mixed case".into(),
            ),
        ));
    }
    Ok(())
}

fn validate_registration(request: &RegistrationRequest) -> Result<(), UserError> {
    if request.replicant_name.is_empty()
        || request.replicant_name.len() > 64
        || !request
            .replicant_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(UserError::from(
            hkask_services::ServiceError::ValidationError("Invalid replicant name".into()),
        ));
    }
    if request.first_name.is_empty() || request.last_name.is_empty() {
        return Err(UserError::from(
            hkask_services::ServiceError::ValidationError("Required name field is empty".into()),
        ));
    }
    if request.email.is_empty() || !request.email.contains('@') {
        return Err(UserError::from(
            hkask_services::ServiceError::ValidationError(
                "Invalid contact information format".into(),
            ),
        ));
    }
    if let Some(phone) = &request.phone
        && !phone.starts_with('+')
    {
        return Err(UserError::from(
            hkask_services::ServiceError::ValidationError(
                "Invalid contact information format".into(),
            ),
        ));
    }
    validate_passphrase(&request.passphrase)?;
    Ok(())
}

pub fn register_replicant_with_passphrase(
    store: &Store,
    replicant_name: &str,
    first_name: &str,
    last_name: &str,
    email: &str,
    phone: Option<&str>,
    passphrase: Zeroizing<String>,
) -> Result<ReplicantIdentity, UserError> {
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

pub fn login_with_passphrase(
    store: &Store,
    replicant_name: &str,
    passphrase: Zeroizing<String>,
) -> Result<UserSession, UserError> {
    store
        .lock()
        .unwrap()
        .login(replicant_name, &passphrase)
        .map_err(|_| {
            UserError::from(hkask_services::ServiceError::LoginFailed(
                "Invalid credentials".into(),
            ))
        })
}

pub fn get_replicant(store: &Store, replicant_name: &str) -> Result<ReplicantIdentity, UserError> {
    store
        .lock()
        .unwrap()
        .get_replicant(replicant_name)?
        .ok_or_else(|| {
            UserError::from(hkask_services::ServiceError::UserNotFound(format!(
                "Replicant '{}'",
                replicant_name
            )))
        })
}

pub fn get_replicants(
    store: &Store,
    user_id: &UserID,
) -> Result<Vec<ReplicantIdentity>, UserError> {
    store
        .lock()
        .unwrap()
        .list_replicants(user_id)
        .map_err(Into::into)
}

pub fn get_sessions(store: &Store, replicant_name: &str) -> Result<Vec<UserSession>, UserError> {
    store
        .lock()
        .unwrap()
        .list_sessions(replicant_name)
        .map_err(Into::into)
}

pub fn revoke_session(store: &Store, session_id: &str) -> Result<UserSession, UserError> {
    let session = store
        .lock()
        .unwrap()
        .get_session(session_id)?
        .ok_or_else(|| {
            UserError::from(hkask_services::ServiceError::UserNotFound(format!(
                "Session '{}'",
                session_id
            )))
        })?;
    store.lock().unwrap().logout(session_id)?;
    Ok(session)
}

fn build_store() -> Store {
    let config = hkask_services::ServiceConfig::from_env().expect("Failed to resolve config");
    let db = hkask_storage::Database::open(&config.db_path, &config.db_passphrase)
        .expect("Failed to open DB");
    Arc::new(Mutex::new(UserStore::new(db.conn_arc())))
}

/// Register a new replicant identity (interactive)
pub fn register_replicant() {
    use std::io::{self, Write};
    let mut name = String::new();
    let mut first = String::new();
    let mut last = String::new();
    let mut email = String::new();
    let mut phone = String::new();

    print!("Replicant name: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut name).unwrap();
    print!("First name: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut first).unwrap();
    print!("Last name: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut last).unwrap();
    print!("Email: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut email).unwrap();
    print!("Phone (optional): ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut phone).unwrap();

    loop {
        print!("Enter passphrase: ");
        io::stdout().flush().unwrap();
        let mut passphrase = String::new();
        io::stdin().read_line(&mut passphrase).unwrap();
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

pub fn login_replicant() {
    use std::io::{self, Write};
    let mut name = String::new();
    print!("Replicant name: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut name).unwrap();
    let store = build_store();
    if let Ok(Some(identity)) = store.lock().unwrap().get_replicant(name.trim()) {
        print!("Enter passphrase: ");
        io::stdout().flush().unwrap();
        let mut passphrase = String::new();
        io::stdin().read_line(&mut passphrase).unwrap();
        match store.lock().unwrap().login(name.trim(), &passphrase.trim()) {
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

pub fn show_replicant(store: &Store, replicant_name: &str) -> Result<(), UserError> {
    let identity = store
        .lock()
        .unwrap()
        .get_replicant(replicant_name)?
        .ok_or_else(|| {
            UserError::from(hkask_services::ServiceError::UserNotFound(format!(
                "Replicant '{}'",
                replicant_name
            )))
        })?;
    println!("Replicant: {}", identity.replicant_name);
    println!("  User ID: {}", identity.user_id);
    println!("  Created: {}", identity.created_at);
    if identity.is_primary {
        println!("  Primary: yes");
    }
    Ok(())
}

pub fn list_replicants(store: &Store) -> Result<(), UserError> {
    let user_id = hkask_types::UserID::new();
    let replicants = store
        .lock()
        .unwrap()
        .list_replicants(&user_id)
        .map_err(|e| UserError::from(hkask_services::ServiceError::from(e)))?;
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

pub fn logout(store: &Store, session_id: &str) -> Result<(), UserError> {
    let session = store
        .lock()
        .unwrap()
        .get_session(session_id)?
        .ok_or_else(|| {
            UserError::from(hkask_services::ServiceError::UserNotFound(format!(
                "Session '{}'",
                session_id
            )))
        })?;
    store.lock().unwrap().logout(session_id)?;
    println!("Session revoked: {}", session.session_id);
    Ok(())
}

pub fn list_sessions(store: &Store, replicant_name: &str) -> Result<(), UserError> {
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
    }
}
