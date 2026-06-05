//! Admin passphrase commands for `kask admin`.
//!
//! Manages the admin passphrase used to gate `HKASK_INSECURE_DEV=1` mode.
//! Once set via `kask admin init`, every use of `HKASK_INSECURE_DEV=1` will
//! prompt for the admin passphrase — no persistent unlock, no backdoor.
//!
//! The admin passphrase is hashed with Argon2id and stored in the OS keychain
//! (service: hkask, key: hkask-admin-passphrase). Raw passphrase is never
//! persisted.

use hkask_keystore;

use crate::cli::AdminAction;

/// Verify that insecure dev mode is authorized via admin passphrase.
///
/// Called by insecure-dev gating code in `config.rs` and `bootstrap.rs`.
/// Always prompts for the admin passphrase — no caching, no session unlock.
/// Every use of `HKASK_INSECURE_DEV=1` requires re-authentication.
///
/// If no admin passphrase has been set via `kask admin init`, prints
/// instructions and returns `false`.
pub fn verify_admin_for_dev_mode() -> bool {
    // Check if admin passphrase is stored in keychain
    if !hkask_keystore::admin::is_admin_passphrase_set() {
        eprintln!("HKASK_INSECURE_DEV=1 is set but no admin passphrase is configured.");
        eprintln!("Run `kask admin init` to set an admin passphrase for development mode.");
        return false;
    }

    // Prompt for admin passphrase — every time
    eprintln!("HKASK_INSECURE_DEV=1 requires admin authentication.");
    let passphrase = match rpassword::prompt_password("Admin passphrase: ") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Failed to read admin passphrase.");
            return false;
        }
    };

    if !hkask_keystore::admin::verify_admin_passphrase(&passphrase) {
        eprintln!("Invalid admin passphrase.");
        return false;
    }

    true
}

/// Set the admin passphrase (hash + store in keychain).
///
/// Prompts for passphrase with confirmation. Hashes with Argon2id before storage.
/// Called by `kask admin init`.
pub fn admin_init() {
    eprintln!("Setting up admin passphrase for hKask insecure development mode.");
    eprintln!("This passphrase will be required every time HKASK_INSECURE_DEV=1 is used.\n");

    let passphrase = match prompt_admin_passphrase_with_confirm() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error reading passphrase: {}", e);
            std::process::exit(1);
        }
    };

    match hkask_keystore::admin::store_admin_passphrase(&passphrase) {
        Ok(()) => {
            eprintln!(
                "\nAdmin passphrase set. HKASK_INSECURE_DEV=1 will now require this passphrase."
            );
        }
        Err(e) => {
            eprintln!("Failed to store admin passphrase: {}", e);
            std::process::exit(1);
        }
    }
}

/// Remove the admin passphrase (disables insecure dev mode entirely).
/// Called by `kask admin reset`.
pub fn admin_reset() {
    match hkask_keystore::admin::remove_admin_passphrase() {
        Ok(()) => {
            eprintln!("Admin passphrase removed. Insecure dev mode is now disabled.");
        }
        Err(e) => {
            eprintln!("Failed to remove admin passphrase: {}", e);
            std::process::exit(1);
        }
    }
}

/// CLI handler for `kask admin` subcommand
pub fn run_admin(action: crate::cli::AdminAction) {
    match action {
        AdminAction::Init => {
            admin_init();
        }
        AdminAction::Reset => {
            admin_reset();
        }
    }
}

// ── Prompting helpers ───────────────────────────────────────────────────

fn prompt_admin_passphrase() -> Result<String, std::io::Error> {
    rpassword::prompt_password("Admin passphrase: ")
}

fn prompt_admin_passphrase_with_confirm() -> Result<String, std::io::Error> {
    loop {
        let passphrase = prompt_admin_passphrase()?;
        if passphrase.is_empty() {
            eprintln!("Passphrase cannot be empty. Please try again.\n");
            continue;
        }
        if passphrase.len() < 8 {
            eprintln!("Passphrase must be at least 8 characters. Please try again.\n");
            continue;
        }
        let confirm = rpassword::prompt_password("Confirm admin passphrase: ")?;
        if passphrase == confirm {
            return Ok(passphrase);
        }
        eprintln!("Passphrases don't match. Please try again.\n");
    }
}
