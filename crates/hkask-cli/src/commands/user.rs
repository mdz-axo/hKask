//! Replicant registration and authentication commands
//!
//! This module handles replicant identity registration and login.
//! A replicant is the in-system persona that a human uses to access hKask.
//!
//! ## Architecture
//!
//! - **Application functions** delegate to `UserService` in the service layer.
//! - **CLI adapters** (interactive I/O): `register_replicant`, `login_replicant`,
//!   `show_replicant`, `list_replicants`, `list_sessions`, `logout`

use crate::cli::ReplicantAction;
use crate::errors::UserError;
use hkask_services::{ServiceContext, UserService};
use zeroize::Zeroizing;

// Application functions — delegate to UserService

/// Register a new replicant identity (non-interactive)
pub fn register_replicant_with_passphrase(
    ctx: &ServiceContext,
    replicant_name: &str,
    first_name: &str,
    last_name: &str,
    email: &str,
    phone: Option<&str>,
    passphrase: Zeroizing<String>,
) -> Result<hkask_types::ReplicantIdentity, UserError> {
    UserService::register(
        ctx,
        replicant_name,
        first_name,
        last_name,
        email,
        phone,
        passphrase,
    )
    .map_err(Into::into)
}

/// Login as a replicant identity (non-interactive)
pub fn login_with_passphrase(
    ctx: &ServiceContext,
    replicant_name: &str,
    passphrase: Zeroizing<String>,
) -> Result<hkask_types::UserSession, UserError> {
    UserService::login(ctx, replicant_name, passphrase).map_err(Into::into)
}

/// Get a replicant identity by name
pub fn get_replicant(
    ctx: &ServiceContext,
    replicant_name: &str,
) -> Result<hkask_types::ReplicantIdentity, UserError> {
    UserService::get_replicant(ctx, replicant_name).map_err(Into::into)
}

/// List replicant identities for a human user
pub fn get_replicants(
    ctx: &ServiceContext,
    user_id: &hkask_types::UserID,
) -> Result<Vec<hkask_types::ReplicantIdentity>, UserError> {
    UserService::list_replicants(ctx, user_id).map_err(Into::into)
}

/// List active sessions for a replicant
pub fn get_sessions(
    ctx: &ServiceContext,
    replicant_name: &str,
) -> Result<Vec<hkask_types::UserSession>, UserError> {
    UserService::list_sessions(ctx, replicant_name).map_err(Into::into)
}

/// Revoke a session by ID
pub fn revoke_session(
    ctx: &ServiceContext,
    session_id: &str,
) -> Result<hkask_types::UserSession, UserError> {
    UserService::revoke_session(ctx, session_id).map_err(Into::into)
}

// CLI adapters — interactive I/O wrappers

/// Register a new replicant identity (interactive)
///
/// Prompts for passphrase with confirmation, then delegates to
/// `register_replicant_with_passphrase`.
pub fn register_replicant(
    ctx: &ServiceContext,
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
        io::stdout().flush().expect("stdout flush failed");
        let mut passphrase = String::new();
        io::stdin()
            .read_line(&mut passphrase)
            .expect("stdin read failed");
        let passphrase = passphrase.trim().to_string();

        if let Err(e) = UserService::validate_passphrase(&passphrase) {
            eprintln!("  ✗ {}", e);
            continue;
        }

        print!("Confirm passphrase: ");
        io::stdout().flush().expect("stdout flush failed");
        let mut confirm = String::new();
        io::stdin()
            .read_line(&mut confirm)
            .expect("stdin read failed");
        let confirm = confirm.trim().to_string();

        if passphrase != confirm {
            eprintln!("  ✗ Passphrases do not match");
            continue;
        }

        match register_replicant_with_passphrase(
            ctx,
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
    ctx: &ServiceContext,
    replicant_name: &str,
) -> Result<hkask_types::UserSession, UserError> {
    use std::io::{self, Write};

    print!("Enter passphrase for replicant '{}': ", replicant_name);
    io::stdout().flush().expect("stdout flush failed");
    let mut passphrase = String::new();
    io::stdin()
        .read_line(&mut passphrase)
        .expect("stdin read failed");
    let passphrase = Zeroizing::new(passphrase.trim().to_string());

    let session = login_with_passphrase(ctx, replicant_name, passphrase)?;
    println!("\n✅ Login successful!");
    println!("  Welcome, {}!", session.replicant_name);
    println!("  Session ID: {}", session.session_id);
    Ok(session)
}

/// Show replicant identity info (interactive display)
pub fn show_replicant(ctx: &ServiceContext, replicant_name: &str) -> Result<(), UserError> {
    let identity = get_replicant(ctx, replicant_name)?;

    println!("\n👤 Replicant Info:");
    println!("  Replicant name: {}", identity.replicant_name);
    println!("  WebID: {}", identity.replicant_webid.redacted_display());
    println!(
        "  User ID: {}",
        identity.user_id.as_uuid().to_string()[..8].to_string() + "..."
    );
    println!(
        "  Primary: {}",
        if identity.is_primary { "yes" } else { "no" }
    );
    println!(
        "  Created: {}",
        chrono::DateTime::from_timestamp(identity.created_at, 0)
            .expect("valid unix timestamp from creation")
            .format("%Y-%m-%d")
    );

    if let Some(last) = identity.last_login {
        let dt = chrono::DateTime::from_timestamp(last, 0).expect("valid unix timestamp");
        println!("  Last login: {}", dt.format("%Y-%m-%d %H:%M"));
    }

    Ok(())
}

/// List replicant identities for a human user (interactive display)
pub fn list_replicants(
    ctx: &ServiceContext,
    user_id: &hkask_types::UserID,
) -> Result<Vec<hkask_types::ReplicantIdentity>, UserError> {
    let identities = get_replicants(ctx, user_id)?;

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
            let dt = chrono::DateTime::from_timestamp(last, 0).expect("valid unix timestamp");
            println!("     Last login: {}", dt.format("%Y-%m-%d %H:%M"));
        }
    }

    Ok(identities)
}

/// Logout — invalidate a session (interactive display)
pub fn logout(ctx: &ServiceContext, session_id: &str) -> Result<(), UserError> {
    let session = revoke_session(ctx, session_id)?;
    println!("\n✅ Logged out successfully!");
    println!("  Replicant: {}", session.replicant_name);
    println!("  Session: {}", &session_id[..8]);
    Ok(())
}

/// List active sessions for a replicant (interactive display)
pub fn list_sessions(
    ctx: &ServiceContext,
    replicant_name: &str,
) -> Result<Vec<hkask_types::UserSession>, UserError> {
    let sessions = get_sessions(ctx, replicant_name)?;

    if sessions.is_empty() {
        println!("  No active sessions for '{}'.", replicant_name);
        return Ok(sessions);
    }

    println!("\n📱 Active sessions for '{}':", replicant_name);
    for (i, session) in sessions.iter().enumerate() {
        let expires =
            chrono::DateTime::from_timestamp(session.expires_at, 0).expect("valid unix timestamp");
        let last_active =
            chrono::DateTime::from_timestamp(session.last_active, 0).expect("valid unix timestamp");
        println!("  {}. Session: {}", i + 1, &session.session_id[..8]);
        println!("     Last active: {}", last_active.format("%Y-%m-%d %H:%M"));
        println!("     Expires: {}", expires.format("%Y-%m-%d %H:%M"));
    }

    Ok(sessions)
}

/// CLI handler for `kask replicant` subcommand
pub fn run_replicant(action: crate::cli::ReplicantAction) {
    use hkask_types::UserID;

    let config = super::helpers::or_exit(
        hkask_services::ServiceConfig::from_env(),
        "Failed to resolve config",
    );
    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
        eprintln!("Runtime error: {e}");
        std::process::exit(1)
    });
    let ctx = super::helpers::or_exit(
        rt.block_on(hkask_services::ServiceContext::build(config)),
        "Failed to build ServiceContext",
    );

    match action {
        ReplicantAction::Register {
            replicant_name,
            first_name,
            last_name,
            email,
            phone,
        } => {
            super::helpers::or_exit(
                register_replicant(
                    &ctx,
                    &replicant_name,
                    &first_name,
                    &last_name,
                    &email,
                    phone.as_deref(),
                ),
                "Registration failed",
            );
        }
        ReplicantAction::Login { replicant_name } => {
            let session =
                super::helpers::or_exit(login_replicant(&ctx, &replicant_name), "Login failed");
            println!("Session ID: {}", session.session_id);
            println!(
                "\nTo logout: kask replicant logout {}",
                &session.session_id[..8]
            );
        }
        ReplicantAction::Logout { session_id } => {
            super::helpers::or_exit(logout(&ctx, &session_id), "Logout failed");
        }
        ReplicantAction::Sessions { replicant_name } => {
            super::helpers::or_exit(
                list_sessions(&ctx, &replicant_name),
                "Failed to list sessions",
            );
        }
        ReplicantAction::List { user_id } => {
            if let Some(uid) = user_id {
                let user_id = uid.parse::<UserID>().unwrap_or_else(|e| {
                    eprintln!("Invalid user ID: {e}");
                    std::process::exit(1)
                });
                super::helpers::or_exit(
                    list_replicants(&ctx, &user_id),
                    "Failed to list identities",
                );
            } else {
                eprintln!("--user-id is required");
                std::process::exit(1);
            }
        }
        ReplicantAction::Show { replicant_name } => {
            super::helpers::or_exit(
                show_replicant(&ctx, &replicant_name),
                "Failed to show replicant",
            );
        }
    }
}
