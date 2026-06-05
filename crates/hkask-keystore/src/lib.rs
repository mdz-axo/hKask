//! hKask Keystore — OS keychain, encryption, and master key derivation

pub mod encryption;
pub mod error;
pub mod keychain;
pub mod master_key;

pub use encryption::derive_key;
pub use error::KeystoreError;
pub use keychain::{
    Keychain, KeychainError, get_or_create_ocap_secret, resolve, resolve_acp_secret,
    resolve_capability_key, resolve_db_passphrase, resolve_mcp_secret, resolve_mcp_security_key,
    resolve_secret_chain,
};
pub use master_key::{derive_all_internal_secrets, derive_sub_key};
