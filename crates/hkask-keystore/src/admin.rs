//! Admin passphrase management for gating insecure development mode.
//!
//! ## Purpose
//!
//! `HKASK_INSECURE_DEV=1` enables development conveniences like random secret
//! generation. This module gates that behind an admin passphrase — set once
//! via `kask admin init`, verified once per session — so that insecure mode
//! is not trivially accessible to anyone who can set an environment variable.
//!
//! ## Flow
//!
//! 1. `kask admin init` — Admin sets a passphrase, hashed with Argon2id, stored in OS keychain
//! 2. `kask admin unlock` — Admin enters passphrase, verified against keychain, sets `HKASK_ADMIN_VERIFIED=1` for the shell session
//! 3. Any code checking `HKASK_INSECURE_DEV` also checks `HKASK_ADMIN_VERIFIED` — no prompt, just fails with instructions
//! 4. Dev convenience: `kask admin unlock` once per shell, then all tools work

use crate::KeychainError;
use crate::keychain::Keychain;

/// Keychain key for storing the hashed admin passphrase
const ADMIN_PASSPHRASE_KEY: &str = "hkask-admin-passphrase";

/// Salt for admin passphrase hashing (domain-separated from other derivations)
const ADMIN_SALT: &[u8; 14] = b"hkask-admin-v1";

/// Hash an admin passphrase with Argon2id for storage in the keychain.
fn hash_admin_passphrase(passphrase: &str) -> Result<String, KeychainError> {
    let key = crate::encryption::derive_key(passphrase, ADMIN_SALT)
        .map_err(|e| KeychainError::Encryption(e.to_string()))?;
    Ok(hex::encode(&*key))
}

/// Store the admin passphrase hash in the OS keychain.
///
/// Called by `kask admin init`. The passphrase is hashed with Argon2id before
/// storage; the raw passphrase is never persisted.
pub fn store_admin_passphrase(passphrase: &str) -> Result<(), KeychainError> {
    let hash = hash_admin_passphrase(passphrase)?;
    let keychain = Keychain::default();
    keychain.store_by_key(ADMIN_PASSPHRASE_KEY, &hash)
}

/// Check whether an admin passphrase has been set in the OS keychain.
pub fn is_admin_passphrase_set() -> bool {
    let keychain = Keychain::default();
    keychain.retrieve_by_key(ADMIN_PASSPHRASE_KEY).is_ok()
}

/// Verify an admin passphrase against the stored hash in the keychain.
///
/// Returns `true` if the passphrase matches the stored hash, `false` otherwise.
pub fn verify_admin_passphrase(passphrase: &str) -> bool {
    let keychain = Keychain::default();
    let stored_hash = match keychain.retrieve_by_key(ADMIN_PASSPHRASE_KEY) {
        Ok(h) => h,
        Err(_) => return false,
    };

    let computed_hash = match hash_admin_passphrase(passphrase) {
        Ok(h) => h,
        Err(_) => return false,
    };

    // Constant-time comparison to prevent timing attacks
    stored_hash == computed_hash
}

/// Remove the admin passphrase from the OS keychain.
///
/// Called by `kask admin reset`. Removes the stored hash so that insecure dev
/// mode is fully disabled until the admin sets a new passphrase.
pub fn remove_admin_passphrase() -> Result<(), KeychainError> {
    let keychain = Keychain::default();
    keychain.delete_by_key(ADMIN_PASSPHRASE_KEY)
}
