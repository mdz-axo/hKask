//! hKask Keystore — OS keychain, encryption, and master key derivation

pub mod encryption;
pub mod error;
pub mod keychain;
pub mod master_key;
pub mod spec_signer;
pub mod version_file;

pub use encryption::derive_key;
pub use error::KeystoreError;
pub use keychain::{
    Keychain, KeychainError, get_or_create_ocap_secret, resolve, resolve_acp_secret,
    resolve_capability_key, resolve_db_passphrase, resolve_mcp_secret, resolve_mcp_security_key,
    resolve_secret_chain, resolve_treasury_key, resolve_wallet_seed, sign_api_key_capability,
};
pub use master_key::{
    DEFAULT_KEY_VERSION, derive_all_internal_secrets, derive_all_internal_secrets_with_version,
    derive_sub_key, derive_sub_key_with_version,
};
pub use spec_signer::{Ed25519SpecSigner, SpecSignatureError};
