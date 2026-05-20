//! hKask Keystore — OS keychain and encryption

pub mod encryption;
pub mod keychain;

pub use encryption::derive_key;
