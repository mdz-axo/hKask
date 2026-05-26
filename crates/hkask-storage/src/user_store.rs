//! UserStore — Human user identity and authentication storage
//!
//! This module provides:
//! - User registration with encrypted PII
//! - Replicant identity management
//! - Passphrase-based authentication
//! - Session management

use hkask_types::{HumanUser, RegistrationRequest, ReplicantIdentity, UserSession, UserID};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use zeroize::Zeroizing;
use rand::RngCore;

#[derive(Error, Debug)]
pub enum UserStoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("User not found: {0}")]
    NotFound(String),
    #[error("Replicant name already registered: {0}")]
    ReplicantNameTaken(String),
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Decryption error: {0}")]
    Decryption(String),
    #[error("Registration error: {0}")]
    Registration(#[from] hkask_types::RegistrationError),
    #[error("Key derivation error: {0}")]
    KeyDerivation(String),
}

pub type Result<T> = std::result::Result<T, UserStoreError>;

#[derive(Clone)]
pub struct UserStore {
    conn: Arc<Mutex<Connection>>,
}

impl UserStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Initialize database schema
    pub fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(include_str!("sql/users.sql"))?;
        Ok(())
    }

    /// Register a new replicant identity (creates human user if first)
    pub fn register_replicant(&self, request: RegistrationRequest) -> Result<ReplicantIdentity> {
        request.validate()?;

        // Check if replicant name exists
        if self.get_replicant(&request.replicant_name)?.is_some() {
            return Err(UserStoreError::ReplicantNameTaken(request.replicant_name));
        }

        // New user - create human_users + replicant_identities
        let user_id = UserID::new();
        let salt = Self::generate_salt();
        let master_salt = Self::generate_salt();
        let passphrase_hash = Self::hash_passphrase(&request.passphrase, &salt)?;
        let pii_key = Self::derive_pii_key(&request.passphrase, &master_salt)?;

        let conn = self.conn.lock().unwrap();

        // Encrypt PII
        let email_enc = Self::encrypt_pii(request.email.as_bytes(), &pii_key)?;
        let phone_enc = request
            .phone
            .as_ref()
            .map(|p| Self::encrypt_pii(p.as_bytes(), &pii_key))
            .transpose()?;
        let first_name_enc = Self::encrypt_pii(request.first_name.as_bytes(), &pii_key)?;
        let last_name_enc = Self::encrypt_pii(request.last_name.as_bytes(), &pii_key)?;

        // Insert human user
        conn.execute(
            "INSERT INTO human_users (user_id, email_enc, phone_enc, passphrase_hash, salt, master_salt, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                user_id.0.to_string(),
                email_enc,
                phone_enc,
                passphrase_hash,
                salt,
                master_salt,
                chrono::Utc::now().to_rfc3339()
            ],
        )?;

        // Insert replicant identity (email/phone stored here for quick access)
        let identity = ReplicantIdentity::new(
            request.replicant_name.clone(),
            user_id,
            first_name_enc,
            last_name_enc,
            email_enc,
            phone_enc.clone(),
        );

        conn.execute(
            "INSERT INTO replicant_identities 
             (replicant_name, user_id, replicant_webid, first_name_enc, last_name_enc, email_enc, phone_enc, is_primary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                identity.replicant_name,
                identity.user_id.0.to_string(),
                identity.replicant_webid.to_string(),
                identity.first_name_enc,
                identity.last_name_enc,
                identity.email_enc,
                identity.phone_enc,
                1, // First replicant is primary
                chrono::Utc::now().to_rfc3339()
            ],
        )?;

        Ok(identity)
    }

    /// Login as a replicant identity with passphrase
    pub fn login(&self, replicant_name: &str, passphrase: &str) -> Result<UserSession> {
        let identity = self
            .get_replicant(replicant_name)?
            .ok_or(UserStoreError::NotFound(replicant_name.into()))?;

        // Get human user and verify passphrase
        let human = self.get_user(&identity.user_id)?;
        let verified = Self::verify_passphrase(passphrase, &human.passphrase_hash, &human.salt)?;
        if !verified {
            return Err(UserStoreError::InvalidCredentials);
        }

        // Create session
        let session = self.create_session(&identity)?;

        // Update last login
        self.update_last_login(&identity.replicant_name)?;

        Ok(session)
    }

    /// Get replicant by name
    pub fn get_replicant(&self, replicant_name: &str) -> Result<Option<ReplicantIdentity>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT replicant_name, user_id, replicant_webid, first_name_enc, last_name_enc, 
                    email_enc, phone_enc, persona_yaml, is_primary, created_at, last_login
             FROM replicant_identities WHERE replicant_name = ?1",
        )?;

        let result: Option<ReplicantIdentity> = stmt
            .query_row(params![replicant_name], |row| {
                Ok(ReplicantIdentity {
                    replicant_name: row.get(0)?,
                    user_id: UserID(row.get::<_, String>(1)?.parse().map_err(|_| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Invalid UUID",
                            )),
                        )
                    })?),
                    replicant_webid: hkask_types::WebID::from_string(&row.get::<_, String>(2)?),
                    first_name_enc: row.get(3)?,
                    last_name_enc: row.get(4)?,
                    email_enc: row.get(5)?,
                    phone_enc: row.get(6)?,
                    persona_yaml: row.get(7)?,
                    is_primary: row.get::<_, i64>(8)? != 0,
                    created_at: row.get(9)?,
                    last_login: row.get(10)?,
                })
            })
            .ok();

        Ok(result)
    }

    /// Get human user by ID
    pub fn get_user(&self, user_id: &UserID) -> Result<HumanUser> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT user_id, email_enc, phone_enc, passphrase_hash, salt, master_salt, created_at, last_active
             FROM human_users WHERE user_id = ?1",
        )?;

        stmt.query_row(params![user_id.0.to_string()], |row| {
            Ok(HumanUser {
                user_id: *user_id,
                email_enc: row.get(1)?,
                phone_enc: row.get(2)?,
                passphrase_hash: row.get(3)?,
                salt: row.get(4)?,
                master_salt: row.get(5)?,
                created_at: row.get(6)?,
                last_active: row.get(7)?,
            })
        })
        .map_err(|_| UserStoreError::NotFound(user_id.0.to_string()))
    }

    /// List all replicants for a user
    pub fn list_replicants(&self, user_id: &UserID) -> Result<Vec<ReplicantIdentity>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT replicant_name, user_id, replicant_webid, first_name_enc, last_name_enc, 
                    email_enc, phone_enc, persona_yaml, is_primary, created_at, last_login
             FROM replicant_identities WHERE user_id = ?1 ORDER BY is_primary DESC, created_at ASC",
        )?;

        let replicants = stmt
            .query_map(params![user_id.0.to_string()], |row| {
                Ok(ReplicantIdentity {
                    replicant_name: row.get(0)?,
                    user_id: UserID(row.get::<_, String>(1)?.parse().map_err(|_| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Invalid UUID",
                            )),
                        )
                    })?),
                    replicant_webid: hkask_types::WebID::from_string(&row.get::<_, String>(2)?),
                    first_name_enc: row.get(3)?,
                    last_name_enc: row.get(4)?,
                    email_enc: row.get(5)?,
                    phone_enc: row.get(6)?,
                    persona_yaml: row.get(7)?,
                    is_primary: row.get::<_, i64>(8)? != 0,
                    created_at: row.get(9)?,
                    last_login: row.get(10)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(replicants)
    }

    /// Create session
    fn create_session(&self, identity: &ReplicantIdentity) -> Result<UserSession> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let session_key_salt = Self::generate_salt();
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + 86400 * 7; // 7 days

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO user_sessions 
             (session_id, replicant_name, replicant_webid, user_id, session_key_salt, expires_at, last_active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                session_id,
                identity.replicant_name,
                identity.replicant_webid.to_string(),
                identity.user_id.0.to_string(),
                session_key_salt,
                chrono::DateTime::from_timestamp(expires_at, 0)
                    .unwrap()
                    .to_rfc3339(),
                chrono::Utc::now().to_rfc3339()
            ],
        )?;

        Ok(UserSession {
            session_id,
            replicant_name: identity.replicant_name.clone(),
            replicant_webid: identity.replicant_webid,
            user_id: identity.user_id,
            session_key_salt,
            expires_at,
            last_active: now,
        })
    }

    /// Update last login timestamp
    fn update_last_login(&self, replicant_name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE replicant_identities SET last_login = ?1 WHERE replicant_name = ?2",
            params![chrono::Utc::now().to_rfc3339(), replicant_name],
        )?;
        Ok(())
    }

    /// Logout - invalidate session
    pub fn logout(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM user_sessions WHERE session_id = ?1",
            params![session_id],
        )?;
        Ok(())
    }

    /// Get active session
    pub fn get_session(&self, session_id: &str) -> Result<Option<UserSession>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT session_id, replicant_name, replicant_webid, user_id, session_key_salt, expires_at, last_active
             FROM user_sessions WHERE session_id = ?1",
        )?;

        let result: Option<UserSession> = stmt
            .query_row(params![session_id], |row| {
                Ok(UserSession {
                    session_id: row.get(0)?,
                    replicant_name: row.get(1)?,
                    replicant_webid: hkask_types::WebID::from_string(&row.get::<_, String>(2)?),
                    user_id: UserID(row.get::<_, String>(3)?.parse().map_err(|_| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Invalid UUID",
                            )),
                        )
                    })?),
                    session_key_salt: row.get(4)?,
                    expires_at: row.get(5)?,
                    last_active: row.get(6)?,
                })
            })
            .ok();

        Ok(result)
    }

    /// List active sessions for a replicant
    pub fn list_sessions(&self, replicant_name: &str) -> Result<Vec<UserSession>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT session_id, replicant_name, replicant_webid, user_id, session_key_salt, expires_at, last_active
             FROM user_sessions WHERE replicant_name = ?1 ORDER BY last_active DESC",
        )?;

        let sessions = stmt
            .query_map(params![replicant_name], |row| {
                Ok(UserSession {
                    session_id: row.get(0)?,
                    replicant_name: row.get(1)?,
                    replicant_webid: hkask_types::WebID::from_string(&row.get::<_, String>(2)?),
                    user_id: UserID(row.get::<_, String>(3)?.parse().map_err(|_| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "Invalid UUID",
                            )),
                        )
                    })?),
                    session_key_salt: row.get(4)?,
                    expires_at: row.get(5)?,
                    last_active: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sessions)
    }

    /// Cleanup expired sessions
    pub fn cleanup_expired_sessions(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let deleted = conn.execute(
            "DELETE FROM user_sessions WHERE expires_at < ?1",
            params![now],
        )?;
        Ok(deleted)
    }

    // === Cryptographic Helpers ===

    fn generate_salt() -> String {
        let mut salt = [0u8; 16];
        rand::rng().fill_bytes(&mut salt);
        hex::encode(salt)
    }

    fn hash_passphrase(passphrase: &str, salt: &str) -> Result<String> {
        use hkask_keystore::encryption::derive_key;
        let key = derive_key(passphrase, salt.as_bytes())
            .map_err(|e| UserStoreError::KeyDerivation(e.to_string()))?;
        Ok(hex::encode(&key[..]))
    }

    fn verify_passphrase(passphrase: &str, hash: &str, salt: &str) -> Result<bool> {
        use hkask_keystore::encryption::derive_key;
        use subtle::ConstantTimeEq;
        
        let computed = derive_key(passphrase, salt.as_bytes())
            .map_err(|e| UserStoreError::KeyDerivation(e.to_string()))?;
        
        let stored = hex::decode(hash).map_err(|e| {
            UserStoreError::KeyDerivation(format!("Invalid hash hex: {}", e))
        })?;
        
        Ok(stored.ct_eq(computed.as_slice()).into())
    }

    fn derive_pii_key(passphrase: &str, master_salt: &str) -> Result<Zeroizing<[u8; 32]>> {
        use hkask_keystore::encryption::derive_key;
        derive_key(passphrase, &hex::decode(master_salt).map_err(|e| {
            UserStoreError::KeyDerivation(e.to_string())
        })?)
        .map_err(|e| UserStoreError::KeyDerivation(e.to_string()))
    }

    fn encrypt_pii(plaintext: &[u8], key: &Zeroizing<[u8; 32]>) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
        use rand::RngCore;

        let cipher = Aes256Gcm::new_from_slice(&**key)
            .map_err(|e| UserStoreError::Encryption(e.to_string()))?;

        let mut nonce_bytes = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| UserStoreError::Encryption(e.to_string()))?;

        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    fn decrypt_pii(ciphertext: &[u8], key: &Zeroizing<[u8; 32]>) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};

        if ciphertext.len() < 12 {
            return Err(UserStoreError::Decryption("Ciphertext too short".into()));
        }

        let cipher = Aes256Gcm::new_from_slice(&**key)
            .map_err(|e| UserStoreError::Decryption(e.to_string()))?;

        let nonce = Nonce::from_slice(&ciphertext[..12]);
        let data = &ciphertext[12..];

        cipher
            .decrypt(nonce, data)
            .map_err(|e| UserStoreError::Decryption(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::RegistrationRequest;

    fn test_store() -> UserStore {
        let conn = Connection::open_in_memory().unwrap();
        let store = UserStore::new(Arc::new(Mutex::new(conn)));
        store.initialize_schema().unwrap();
        store
    }

    #[test]
    fn test_register_replicant() {
        let store = test_store();
        let request = RegistrationRequest {
            replicant_name: "alice2".to_string(),
            first_name: "Alice".to_string(),
            last_name: "AI".to_string(),
            email: "alice@alice.ai".to_string(),
            phone: Some("+15551234567".to_string()),
            passphrase: "AlicePass123".to_string(),
        };

        let identity = store.register_replicant(request).unwrap();
        assert_eq!(identity.replicant_name, "alice2");
        assert!(identity.is_primary);
    }

    #[test]
    fn test_login_passphrase() {
        let store = test_store();
        let request = RegistrationRequest {
            replicant_name: "bob1".to_string(),
            first_name: "Bob".to_string(),
            last_name: "Bot".to_string(),
            email: "bob@bob.ai".to_string(),
            phone: None,
            passphrase: "BobPass456".to_string(),
        };

        store.register_replicant(request).unwrap();

        let session = store.login("bob1", "BobPass456").unwrap();
        assert_eq!(session.replicant_name, "bob1");
    }

    #[test]
    fn test_login_wrong_passphrase() {
        let store = test_store();
        let request = RegistrationRequest {
            replicant_name: "charlie".to_string(),
            first_name: "Charlie".to_string(),
            last_name: "Test".to_string(),
            email: "charlie@test.ai".to_string(),
            phone: None,
            passphrase: "CharliePass789".to_string(),
        };

        store.register_replicant(request).unwrap();

        let result = store.login("charlie", "WrongPass");
        assert!(result.is_err());
    }

    #[test]
    fn test_passphrase_validation() {
        // Valid passphrase
        assert!(RegistrationRequest::validate_passphrase("AlicePass123").is_ok());
        
        // Too short
        assert!(RegistrationRequest::validate_passphrase("Ab1").is_err());
        
        // No uppercase
        assert!(RegistrationRequest::validate_passphrase("alicepass123").is_err());
        
        // No lowercase
        assert!(RegistrationRequest::validate_passphrase("ALICEPASS123").is_err());
        
        // Special characters not allowed
        assert!(RegistrationRequest::validate_passphrase("Alice@Pass123").is_err());
    }
}
