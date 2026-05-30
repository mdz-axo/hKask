//! hKask Keystore — OS keychain, encryption, and master key derivation

pub mod admin;
pub mod encryption;
pub mod error;
pub mod keychain;
pub mod master_key;

pub use encryption::derive_key;
pub use error::KeystoreError;
pub use keychain::{KeyRing, Keychain, KeychainError, get_or_create_ocap_secret, resolve};
pub use master_key::{
    InternalSecrets, derive_all_internal_secrets, derive_data_category_key, derive_sub_key,
    resolve_derived,
};
pub use zeroize::Zeroizing;
