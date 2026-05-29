//! Admin passphrase commands for `kask admin`.
//!
//! Manages the admin passphrase used to gate `HKASK_INSECURE_DEV=1` mode.
//! Once set, developers can unlock insecure dev mode for a shell session
//! with `kask admin unlock`.
//!
//! The admin passphrase is hashed with Argon2id and stored in the OS keychain
//! (service: hkask, key: hkask-admin-passphrase). Raw passphrase is never
//! persisted.

use hkask_keystore;

/// Verify that insecure dev mode is authorized via admin passphrase.
///
/// Called by insecure-dev gating code in `config.rs` and `bootstrap.rs`.
/// Returns `true` if:
/// - `HKASK_ADMIN_VERIFIED=1` is already set in this session, OR
/// - The admin passphrase is verified interactively and cached
///
/// If no admin passphrase has been set via `kask admin init`, prints
/// instructions and returns `false`.
pub fn verify_admin_for_dev_mode() -> bool {
    // Already verified this session
    if std::env::var("HKASK_ADMIN_VERIFIED").as_deref() == Ok("1") {
        return true;
    }

    // Check if admin passphrase is stored in keychain
    if !hkask_keystore::admin::is_admin_passphrase_set() {
        eprintln!("HKASK_INSECURE_DEV=1 is set but no admin passphrase is configured.");
        eprintln!("Run `kask admin init` to set an admin passphrase for development mode.");
        return false;
    }

    // Prompt for admin passphrase
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

    // Cache for this process
    unsafe {
        std::env::set_var("HKASK_ADMIN_VERIFIED", "1");
    }

    true
}

/// Set the admin passphrase (hash + store in keychain).
///
/// Prompts for passphrase with confirmation. Hashes with Argon2id before storage.
/// Called by `kask admin init`.
pub fn admin_init() {
    eprintln!("Setting up admin passphrase for hKask insecure development mode.");
    eprintln!(
        "This passphrase gates HKASK_INSECURE_DEV=1 — use `kask admin unlock` to activate.\n"
    );

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
                "\nAdmin passphrase set successfully. Use `kask admin unlock` to enable HKASK_INSECURE_DEV=1."
            );
        }
        Err(e) => {
            eprintln!("Failed to store admin passphrase: {}", e);
            std::process::exit(1);
        }
    }
}

/// Verify admin passphrase and unlock insecure dev mode for this shell session.
///
/// Sets `HKASK_ADMIN_VERIFIED=1` in the current process.
/// Called by `kask admin unlock`.
pub fn admin_unlock() {
    // Check if already unlocked
    if std::env::var("HKASK_ADMIN_VERIFIED").as_deref() == Ok("1") {
        eprintln!("Insecure dev mode is already unlocked for this session.");
        return;
    }

    // Check if admin passphrase is set
    if !hkask_keystore::admin::is_admin_passphrase_set() {
        eprintln!("No admin passphrase configured. Run `kask admin init` first.");
        std::process::exit(1);
    }

    let passphrase = match rpassword::prompt_password("Admin passphrase: ") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to read passphrase: {}", e);
            std::process::exit(1);
        }
    };

    if !hkask_keystore::admin::verify_admin_passphrase(&passphrase) {
        eprintln!("Invalid admin passphrase.");
        std::process::exit(1);
    }

    unsafe {
        std::env::set_var("HKASK_ADMIN_VERIFIED", "1");
    }

    eprintln!(
        "Insecure dev mode unlocked for this session. Set HKASK_INSECURE_DEV=1 and run hKask tools."
    );
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
