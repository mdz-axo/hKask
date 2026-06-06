use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum KeystoreError {
    #[error("Platform keychain error: {0}")]
    Platform(String),

    #[error("Secret not found: {0}")]
    NotFound(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),
}

impl From<crate::keychain::KeychainError> for KeystoreError {
    fn from(err: crate::keychain::KeychainError) -> Self {
        match err {
            crate::keychain::KeychainError::Platform(msg) => KeystoreError::Platform(msg),
            crate::keychain::KeychainError::NotFound(msg) => KeystoreError::NotFound(msg),
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
            crate::encryption::EncryptionError::Decryption(msg) => KeystoreError::Encryption(msg),
            crate::encryption::EncryptionError::InvalidPassphrase => {
                KeystoreError::Encryption("Invalid passphrase".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::EncryptionError;
    use crate::keychain::KeychainError;

    // ── From<KeychainError> ─────────────────────────────────────────────

    // P8 invariant: KeychainError::Platform maps to KeystoreError::Platform,
    // preserving the original message.
    #[test]
    fn keystore_error_from_keychain_platform_preserves_message() {
        let msg = "platform failure";
        let err: KeystoreError = KeychainError::Platform(msg.to_string()).into();
        assert!(
            matches!(err, KeystoreError::Platform(ref m) if m == msg),
            "KeychainError::Platform must map to KeystoreError::Platform preserving message"
        );
    }

    // P8 invariant: KeychainError::NotFound maps to KeystoreError::NotFound,
    // preserving the original message.
    #[test]
    fn keystore_error_from_keychain_not_found_preserves_message() {
        let msg = "secret missing";
        let err: KeystoreError = KeychainError::NotFound(msg.to_string()).into();
        assert!(
            matches!(err, KeystoreError::NotFound(ref m) if m == msg),
            "KeychainError::NotFound must map to KeystoreError::NotFound preserving message"
        );
    }

    // ── From<EncryptionError> ───────────────────────────────────────────

    // P8 invariant: EncryptionError::KeyDerivation maps to KeystoreError::KeyDerivation.
    #[test]
    fn keystore_error_from_encryption_key_derivation_maps_correctly() {
        let msg = "argon2 failed";
        let err: KeystoreError = EncryptionError::KeyDerivation(msg.to_string()).into();
        assert!(
            matches!(err, KeystoreError::KeyDerivation(ref m) if m == msg),
            "EncryptionError::KeyDerivation must map to KeystoreError::KeyDerivation"
        );
    }

    // P8 invariant: EncryptionError::Encryption maps to KeystoreError::Encryption.
    #[test]
    fn keystore_error_from_encryption_failure_maps_correctly() {
        let msg = "aes error";
        let err: KeystoreError = EncryptionError::Encryption(msg.to_string()).into();
        assert!(
            matches!(err, KeystoreError::Encryption(ref m) if m == msg),
            "EncryptionError::Encryption must map to KeystoreError::Encryption"
        );
    }

    // P8 invariant: EncryptionError::Decryption maps to KeystoreError::Encryption
    // (not to a Decryption variant — KeystoreError has none).
    #[test]
    fn keystore_error_from_decryption_failure_maps_to_encryption_variant() {
        let msg = "decrypt failure";
        let err: KeystoreError = EncryptionError::Decryption(msg.to_string()).into();
        assert!(
            matches!(err, KeystoreError::Encryption(ref m) if m == msg),
            "EncryptionError::Decryption must map to KeystoreError::Encryption (not Decryption)"
        );
    }

    // P8 invariant: EncryptionError::InvalidPassphrase maps to
    // KeystoreError::Encryption with message "Invalid passphrase".
    #[test]
    fn keystore_error_from_invalid_passphrase_maps_to_encryption_variant() {
        let err: KeystoreError = EncryptionError::InvalidPassphrase.into();
        assert!(
            matches!(err, KeystoreError::Encryption(ref m) if m == "Invalid passphrase"),
            "EncryptionError::InvalidPassphrase must map to KeystoreError::Encryption(\"Invalid passphrase\")"
        );
    }

    // ── Display ───────────────────────────────────────────────────────

    // P8 invariant: each KeystoreError variant produces its expected Display format.
    #[test]
    fn keystore_error_display_formats_correctly() {
        assert_eq!(
            KeystoreError::Platform("p".to_string()).to_string(),
            "Platform keychain error: p",
            "Platform variant Display must match #[error] format"
        );
        assert_eq!(
            KeystoreError::NotFound("n".to_string()).to_string(),
            "Secret not found: n",
            "NotFound variant Display must match #[error] format"
        );
        assert_eq!(
            KeystoreError::Encryption("e".to_string()).to_string(),
            "Encryption error: e",
            "Encryption variant Display must match #[error] format"
        );
        assert_eq!(
            KeystoreError::KeyDerivation("k".to_string()).to_string(),
            "Key derivation failed: k",
            "KeyDerivation variant Display must match #[error] format"
        );
    }
}
