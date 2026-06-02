//! UserStore — Human user identity and authentication storage
//!
//! This module provides:
//! - User registration with encrypted PII
//! - Replicant identity management
//! - Passphrase-based authentication using Argon2id
//! - Session management

use argon2::{PasswordHasher, PasswordVerifier, password_hash::PasswordHash};
use base64::Engine;
use hkask_types::{
    HumanUser, InfrastructureError, RegistrationRequest, ReplicantIdentity, UserID, UserSession,
};
use rand::RngCore;
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use zeroize::Zeroizing;

#[derive(Error, Debug)]
pub enum UserStoreError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),

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
    #[error("Password hash error: {0}")]
    PasswordHash(String),
}

impl From<rusqlite::Error> for UserStoreError {
    fn from(e: rusqlite::Error) -> Self {
        UserStoreError::Infra(InfrastructureError::Database(e.to_string()))
    }
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

    pub fn initialize_schema(&self) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        conn.execute_batch(include_str!("sql/users.sql"))?;
        Ok(())
    }

    pub fn register_replicant(&self, request: RegistrationRequest) -> Result<ReplicantIdentity> {
        request.validate()?;

        if self.get_replicant(&request.replicant_name)?.is_some() {
            return Err(UserStoreError::ReplicantNameTaken(request.replicant_name));
        }

        let user_id = UserID::new();
        let salt = Self::generate_salt();
        let master_salt = Self::generate_salt();
        let passphrase_hash = Self::hash_passphrase(&request.passphrase, &salt)?;
        let pii_key = Self::derive_pii_key(&request.passphrase, &master_salt)?;

        let email_enc = Self::encrypt_pii(request.email.as_bytes(), &pii_key)?;
        let phone_enc = request
            .phone
            .as_ref()
            .map(|p| Self::encrypt_pii(p.as_bytes(), &pii_key))
            .transpose()?;
        let first_name_enc = Self::encrypt_pii(request.first_name.as_bytes(), &pii_key)?;
        let last_name_enc = Self::encrypt_pii(request.last_name.as_bytes(), &pii_key)?;

        let mut conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let tx = conn.transaction()?;

        tx.execute(
            "INSERT INTO human_users (user_id, email_enc, phone_enc, passphrase_hash, salt, master_salt, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                user_id.0.to_string(),
                email_enc,
                phone_enc,
                passphrase_hash,
                salt,
                master_salt,
                chrono::Utc::now().timestamp()
            ],
        )?;

        let identity = ReplicantIdentity::new(
            request.replicant_name.clone(),
            user_id,
            first_name_enc,
            last_name_enc,
            true,
        );

        tx.execute(
            "INSERT INTO replicant_identities
             (replicant_name, user_id, replicant_webid, first_name_enc, last_name_enc, is_primary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                identity.replicant_name,
                identity.user_id.0.to_string(),
                identity.replicant_webid.to_string(),
                identity.first_name_enc,
                identity.last_name_enc,
                1,
                chrono::Utc::now().timestamp()
            ],
        )?;

        tx.commit()?;
        Ok(identity)
    }

    pub fn login(&self, replicant_name: &str, passphrase: &str) -> Result<UserSession> {
        let identity = self
            .get_replicant(replicant_name)?
            .ok_or(UserStoreError::NotFound(replicant_name.into()))?;

        let human = self.get_user(&identity.user_id)?;
        let verified = Self::verify_passphrase(passphrase, &human.passphrase_hash)?;
        if !verified {
            return Err(UserStoreError::InvalidCredentials);
        }

        let session = self.create_session(&identity)?;
        self.update_last_login(&identity.replicant_name)?;
        Ok(session)
    }

    pub fn logout(&self, session_id: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        conn.execute(
            "DELETE FROM user_sessions WHERE session_id = ?1",
            params![session_id],
        )?;
        Ok(())
    }

    pub fn get_session(&self, session_id: &str) -> Result<Option<UserSession>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT session_id, replicant_name, replicant_webid, user_id, session_key_salt, expires_at, last_active
             FROM user_sessions WHERE session_id = ?1",
        )?;

        match stmt.query_row(params![session_id], |row| {
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
        }) {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(UserStoreError::from(e)),
        }
    }

    pub fn list_sessions(&self, replicant_name: &str) -> Result<Vec<UserSession>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
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

    pub fn get_replicant(&self, replicant_name: &str) -> Result<Option<ReplicantIdentity>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT replicant_name, user_id, replicant_webid, first_name_enc, last_name_enc,
                    persona_yaml, is_primary, created_at, last_login
             FROM replicant_identities WHERE replicant_name = ?1",
        )?;

        match stmt.query_row(params![replicant_name], |row| {
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
                persona_yaml: row.get(5)?,
                is_primary: row.get::<_, i64>(6)? != 0,
                created_at: row.get(7)?,
                last_login: row.get(8)?,
            })
        }) {
            Ok(identity) => Ok(Some(identity)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(UserStoreError::from(e)),
        }
    }

    pub fn get_user(&self, user_id: &UserID) -> Result<HumanUser> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
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

    pub fn list_replicants(&self, user_id: &UserID) -> Result<Vec<ReplicantIdentity>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT replicant_name, user_id, replicant_webid, first_name_enc, last_name_enc,
                    persona_yaml, is_primary, created_at, last_login
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
                    persona_yaml: row.get(5)?,
                    is_primary: row.get::<_, i64>(6)? != 0,
                    created_at: row.get(7)?,
                    last_login: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(replicants)
    }

    fn create_session(&self, identity: &ReplicantIdentity) -> Result<UserSession> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let session_key_salt = Self::generate_salt();
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + 86400 * 7;

        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
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
                expires_at,
                now
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

    fn update_last_login(&self, replicant_name: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        conn.execute(
            "UPDATE replicant_identities SET last_login = ?1 WHERE replicant_name = ?2",
            params![chrono::Utc::now().timestamp(), replicant_name],
        )?;
        Ok(())
    }

    fn generate_salt() -> String {
        let mut salt = [0u8; 16];
        rand::rng().fill_bytes(&mut salt);
        hex::encode(salt)
    }

    fn hash_passphrase(passphrase: &str, salt: &str) -> Result<String> {
        use argon2::password_hash::SaltString;
        use argon2::{Algorithm, Argon2, Params, Version};

        let salt_bytes = hex::decode(salt)
            .map_err(|e| UserStoreError::KeyDerivation(format!("Invalid salt hex: {}", e)))?;

        let salt_string = SaltString::from_b64(
            &base64::engine::general_purpose::STANDARD_NO_PAD.encode(&salt_bytes),
        )
        .map_err(|e| UserStoreError::KeyDerivation(format!("Salt error: {}", e)))?;

        let params = Params::new(19456, 2, 1, None)
            .map_err(|e| UserStoreError::KeyDerivation(e.to_string()))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        let password_hash = argon2
            .hash_password(passphrase.as_bytes(), &salt_string)
            .map_err(|e| UserStoreError::PasswordHash(e.to_string()))?;

        Ok(password_hash.to_string())
    }

    fn verify_passphrase(passphrase: &str, hash: &str) -> Result<bool> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| UserStoreError::PasswordHash(e.to_string()))?;

        match argon2::Argon2::default().verify_password(passphrase.as_bytes(), &parsed_hash) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub(crate) fn derive_pii_key(
        passphrase: &str,
        master_salt: &str,
    ) -> Result<Zeroizing<[u8; 32]>> {
        use hkask_keystore::encryption::derive_key;
        derive_key(
            passphrase,
            &hex::decode(master_salt).map_err(|e| UserStoreError::KeyDerivation(e.to_string()))?,
        )
        .map_err(|e| UserStoreError::KeyDerivation(e.to_string()))
    }

    pub(crate) fn encrypt_pii(plaintext: &[u8], key: &Zeroizing<[u8; 32]>) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};

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
}
