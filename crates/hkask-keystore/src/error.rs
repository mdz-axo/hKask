use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeystoreError {
    #[error("Platform keychain error: {0}")]
    Platform(String),

    #[error("Secret not found: {0}")]
    NotFound(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),

    #[error("Operation not supported: {0}")]
    NotSupported(String),

    #[error("IO error: {0}")]
    Io(String),
}

impl From<crate::keychain::KeychainError> for KeystoreError {
    fn from(err: crate::keychain::KeychainError) -> Self {
        match err {
            crate::keychain::KeychainError::Platform(msg) => KeystoreError::Platform(msg),
            crate::keychain::KeychainError::NotFound(msg) => KeystoreError::NotFound(msg),
            crate::keychain::KeychainError::Encryption(msg) => KeystoreError::Encryption(msg),
        }
    }
}

impl From<crate::encryption::EncryptionError> for KeystoreError {
    fn from(err: crate::encryption::EncryptionError) -> Self {
        match err {
            crate::encryption::EncryptionError::KeyDerivation(msg) => {
                KeystoreError::KeyDerivation(msg)
            }
            crate::encryption::EncryptionError::Encryption(msg) => KeystoreError::Encryption(msg),
            crate::encryption::EncryptionError::Decryption(msg) => {
                KeystoreError::Encryption(msg)
            }
            crate::encryption::EncryptionError::InvalidPassphrase => {
                KeystoreError::Encryption("Invalid passphrase".to_string())
            }
        }
    }
}

impl From<std::io::Error> for KeystoreError {
    fn from(err: std::io::Error) -> Self {
        KeystoreError::Io(err.to_string())
    }
}
