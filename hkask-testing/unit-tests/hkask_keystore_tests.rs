// Auto-extracted inline tests for hkask-keystore
// Extracted: Thu May 21 00:22:37 PDT 2026

// === From encryption.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let salt = EncryptionService::generate_salt();
        let service = EncryptionService::new("test-passphrase", &salt).unwrap();

        let plaintext = b"Hello, World!";
        let ciphertext = service.encrypt(plaintext).unwrap();
        let decrypted = service.decrypt(&ciphertext).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_different_passphrases() {
        let salt = EncryptionService::generate_salt();
        let service1 = EncryptionService::new("passphrase1", &salt).unwrap();
        let service2 = EncryptionService::new("passphrase2", &salt).unwrap();

        let plaintext = b"Secret data";
        let ciphertext = service1.encrypt(plaintext).unwrap();

        // Different passphrase should fail to decrypt
        let result = service2.decrypt(&ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_passphrase() {
        let salt = EncryptionService::generate_salt();
        let result = EncryptionService::new("", &salt);
        assert!(matches!(result, Err(EncryptionError::InvalidPassphrase)));
    }

    #[test]
    fn test_salt_generation() {
        let salt1 = EncryptionService::generate_salt();
        let salt2 = EncryptionService::generate_salt();
        assert_ne!(salt1, salt2);
    }

    #[test]
    fn test_decrypt_invalid_ciphertext() {
        let salt = EncryptionService::generate_salt();
        let service = EncryptionService::new("passphrase", &salt).unwrap();
        let result = service.decrypt(&[0u8; 5]); // Too short
        assert!(matches!(result, Err(EncryptionError::Decryption(_))));
    }

    #[test]
    fn test_decrypt_empty_ciphertext() {
        let salt = EncryptionService::generate_salt();
        let service = EncryptionService::new("passphrase", &salt).unwrap();
        let result = service.decrypt(&[]);
        assert!(matches!(result, Err(EncryptionError::Decryption(_))));
    }
}

// === From keychain.rs ===
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keychain_new() {
        let keychain = Keychain::new("test-service");
        assert_eq!(keychain.service_name, "test-service");
    }

    #[test]
    fn test_keychain_default() {
        let keychain = Keychain::default();
        assert_eq!(keychain.service_name, "hkask");
    }
}

