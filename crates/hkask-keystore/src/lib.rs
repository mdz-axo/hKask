//! hKask Keystore — OS keychain and encryption

pub mod encryption;
pub mod keychain;

pub use encryption::derive_key;
pub use keychain::{KeyRing, Keychain, get_or_create_ocap_secret, resolve};
