//! UserStore — Human user identity, Argon2id auth, encrypted PII, session management.
use crate::Store;
use crate::archive::MergeReceipt;
use argon2::{PasswordHasher, PasswordVerifier, password_hash::PasswordHash};
use base64::Engine;
use hkask_services_core::{
    HumanUser, Invite, InviteStatus, OAuthProvider, ReplicantIdentity, Role, UserSession,
};
use hkask_types::wallet::WalletId;
use hkask_types::{InfrastructureError, UserID};
use rand::RngCore;
use rusqlite::OptionalExtension;
use rusqlite::params;
use std::str::FromStr;
use thiserror::Error;
use zeroize::Zeroizing;
const REPLICANT_COLUMNS: &str = "replicant_name, user_id, replicant_webid, wallet_id, first_name_enc, last_name_enc, persona_yaml, is_primary, created_at, last_login";
const SESSION_COLUMNS: &str = "session_id, replicant_name, replicant_webid, user_id, session_key_salt, expires_at, last_active";
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
    #[error("Key derivation error: {0}")]
    KeyDerivation(String),
    #[error("Password hash error: {0}")]
    PasswordHash(String),
    #[error("Passphrase expired {0} days ago — must change")]
    PassphraseExpired(i64),
}
impl_from_rusqlite!(UserStoreError, Infra);
pub type UserResult<T> = std::result::Result<T, UserStoreError>;
define_store!(UserStore);
fn replicant_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ReplicantIdentity> {
    Ok(ReplicantIdentity {
        replicant_name: row.get(0)?,
        user_id: row.get(1)?,
        replicant_webid: row.get(2)?,
        wallet_id: row.get::<_, Option<String>>(3)?.and_then(|s| {
            use std::str::FromStr;
            WalletId::from_str(&s).ok()
        }),
        first_name_enc: row.get(4)?,
        last_name_enc: row.get(5)?,
        persona_yaml: row.get(6)?,
        is_primary: row.get::<_, i64>(7)? != 0,
        created_at: row.get(8)?,
        last_login: row.get(9)?,
    })
}
fn session_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserSession> {
    Ok(UserSession {
        session_id: row.get(0)?,
        replicant_name: row.get(1)?,
        replicant_webid: row.get(2)?,
        user_id: row.get(3)?,
        session_key_salt: row.get(4)?,
        expires_at: row.get(5)?,
        last_active: row.get(6)?,
    })
}
impl UserStore {
    /// Initialize the user store schema.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — schema for users, replicants, sessions
    /// post: users, replicants, sessions tables created if not exists
    pub fn initialize_schema(&self) -> UserResult<()> {
        let conn = self.lock_conn()?;
        conn.execute_batch(include_str!("sql/users.sql"))?;
        // Migration: add passphrase_set_at if not present
        conn.execute_batch("ALTER TABLE human_users ADD COLUMN passphrase_set_at INTEGER;")
            .ok(); // ignore error if column already exists
        // Migration: add wallet_id if not present (multi-wallet support)
        conn.execute_batch("ALTER TABLE replicant_identities ADD COLUMN wallet_id TEXT;")
            .ok(); // ignore error if column already exists
        // Migration: add OAuth provider columns (DEP-001)
        conn.execute_batch("ALTER TABLE human_users ADD COLUMN oauth_provider TEXT;")
            .ok();
        conn.execute_batch("ALTER TABLE human_users ADD COLUMN oauth_provider_user_id TEXT;")
            .ok();
        conn.execute_batch("ALTER TABLE human_users ADD COLUMN oauth_display_name TEXT;")
            .ok();
        // Migration: add role column (multi-user, P1)
        conn.execute_batch(
            "ALTER TABLE human_users ADD COLUMN role TEXT NOT NULL DEFAULT 'member';",
        )
        .ok();
        Ok(())
    }
    /// Register a new replicant.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — register a replicant
    /// \[P2\] Constraining: Affirmative Consent — passphrase requirements enforced
    /// pre:  replicant_name is non-empty, passphrase meets requirements
    /// post: replicant and user records created
    pub fn register_replicant(
        &self,
        replicant_name: String,
        email: String,
        phone: Option<String>,
        first_name: String,
        last_name: String,
        passphrase: String,
    ) -> UserResult<ReplicantIdentity> {
        if self.get_replicant(&replicant_name)?.is_some() {
            return Err(UserStoreError::ReplicantNameTaken(replicant_name));
        }
        let user_id = UserID::new();
        let salt = Self::generate_salt();
        let master_salt = Self::generate_salt();
        let passphrase_hash = Self::hash_passphrase(&passphrase, &salt)?;
        let pii_key = Self::derive_pii_key(&passphrase, &master_salt)?;
        let email_enc = Self::encrypt_pii(email.as_bytes(), &pii_key)?;
        let phone_enc = phone
            .as_ref()
            .map(|p| Self::encrypt_pii(p.as_bytes(), &pii_key))
            .transpose()?;
        let first_name_enc = Self::encrypt_pii(first_name.as_bytes(), &pii_key)?;
        let last_name_enc = Self::encrypt_pii(last_name.as_bytes(), &pii_key)?;
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO human_users (user_id, email_enc, phone_enc, passphrase_hash, salt, master_salt, created_at, passphrase_set_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                user_id,
                email_enc,
                phone_enc,
                passphrase_hash,
                salt,
                master_salt,
                chrono::Utc::now().timestamp(),
                chrono::Utc::now().timestamp(),
            ],
        )?;
        let identity =
            ReplicantIdentity::new(replicant_name, user_id, first_name_enc, last_name_enc, true);
        tx.execute(
            "INSERT INTO replicant_identities
             (replicant_name, user_id, replicant_webid, first_name_enc, last_name_enc, is_primary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                identity.replicant_name,
                identity.user_id,
                identity.replicant_webid,
                identity.first_name_enc,
                identity.last_name_enc,
                1,
                chrono::Utc::now().timestamp()
            ],
        )?;
        tx.commit()?;
        Ok(identity)
    }
    /// Find or create a human user via OAuth sign-in.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// pre:  provider is a valid OAuthProvider; provider_user_id is the external ID from the provider
    /// post: if user exists with matching provider + provider_user_id → returns existing (user, replicant)
    /// post: if user does not exist → creates new HumanUser + primary ReplicantIdentity + returns both
    pub fn find_or_create_oauth_user(
        &self,
        provider: &hkask_types::identity::OAuthProvider,
        provider_user_id: &str,
        email: &str,
        display_name: &str,
    ) -> UserResult<(HumanUser, ReplicantIdentity)> {
        // Try to find existing user by OAuth identity
        if let Some((user, replicant)) = self.find_user_by_oauth(provider, provider_user_id)? {
            // Update last_active and display_name
            let now = chrono::Utc::now().timestamp();
            let conn = self.lock_conn()?;
            conn.execute(
                "UPDATE human_users SET last_active = ?1, oauth_display_name = ?2 WHERE user_id = ?3",
                params![now, display_name, user.user_id],
            )?;
            conn.execute(
                "UPDATE replicant_identities SET last_login = ?1 WHERE replicant_name = ?2",
                params![now, replicant.replicant_name],
            )?;
            return Ok((user, replicant));
        }
        // Create new user — OAuth users get a generated passphrase (never used directly)
        let generated_passphrase = uuid::Uuid::new_v4().to_string();
        let user_id = UserID::new();
        let salt = Self::generate_salt();
        let master_salt = Self::generate_salt();
        let passphrase_hash = Self::hash_passphrase(&generated_passphrase, &salt)?;
        let pii_key = Self::derive_pii_key(&generated_passphrase, &master_salt)?;
        let email_enc = Self::encrypt_pii(email.as_bytes(), &pii_key)?;
        // Derive replicant name from display name, with dedup on collision
        let mut replicant_name = sanitize_replicant_name(display_name);
        let mut suffix: u32 = 1;
        while self.get_replicant(&replicant_name)?.is_some() {
            suffix += 1;
            replicant_name = format!("{}_{}", sanitize_replicant_name(display_name), suffix);
            if suffix > 100 {
                // Fallback: use UUID suffix to guarantee uniqueness
                replicant_name = format!(
                    "{}_{}",
                    sanitize_replicant_name(display_name),
                    &uuid::Uuid::new_v4().to_string()[..8]
                );
                break;
            }
        }
        let first_name_enc = Self::encrypt_pii(display_name.as_bytes(), &pii_key)?;
        let last_name_enc = Self::encrypt_pii(b"", &pii_key)?;
        let provider_str = provider.to_string();
        let now = chrono::Utc::now().timestamp();
        let mut conn = self.lock_conn()?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO human_users (user_id, email_enc, phone_enc, passphrase_hash, salt, master_salt, created_at, passphrase_set_at, oauth_provider, oauth_provider_user_id, oauth_display_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                user_id,
                email_enc,
                Option::<Vec<u8>>::None, // no phone for OAuth users
                passphrase_hash,
                salt,
                master_salt,
                now,
                now,
                provider_str,
                provider_user_id,
                display_name,
            ],
        )?;
        let identity = ReplicantIdentity::new(
            replicant_name.clone(),
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
                identity.user_id,
                identity.replicant_webid,
                identity.first_name_enc,
                identity.last_name_enc,
                1,
                now
            ],
        )?;
        tx.commit()?;
        let user = self.get_user(&user_id)?;
        Ok((user, identity))
    }
    /// Find a human user by OAuth provider identity.
    ///
    /// expect: "The system provides durable storage for archival data"
    /// pre:  provider is a valid OAuthProvider; provider_user_id is non-empty
    /// post: returns Some((user, primary_replicant)) if found; None if not found
    fn find_user_by_oauth(
        &self,
        provider: &hkask_types::identity::OAuthProvider,
        provider_user_id: &str,
    ) -> UserResult<Option<(HumanUser, ReplicantIdentity)>> {
        let provider_str = provider.to_string();
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT user_id FROM human_users WHERE oauth_provider = ?1 AND oauth_provider_user_id = ?2",
        )?;
        let user_id: Option<String> = stmt
            .query_row(params![provider_str, provider_user_id], |row| row.get(0))
            .optional()
            .map_err(UserStoreError::from)?;
        match user_id {
            Some(uid_str) => {
                let uid = UserID::from_str(&uid_str).map_err(|e| {
                    UserStoreError::Infra(hkask_types::InfrastructureError::Database(format!(
                        "Invalid user_id: {e}"
                    )))
                })?;
                let user = self.get_user(&uid)?;
                let replicants = self.list_replicants(&uid)?;
                let primary = replicants
                    .into_iter()
                    .find(|r| r.is_primary)
                    .ok_or_else(|| UserStoreError::NotFound("primary replicant".into()))?;
                Ok(Some((user, primary)))
            }
            None => Ok(None),
        }
    }
    /// Create a session and return it (used by OAuth flow and login).
    ///
    /// expect: "The system provides durable storage for archival data"
    /// pre:  identity is a valid ReplicantIdentity
    /// post: returns a new UserSession with 7-day expiry
    pub fn create_oauth_session(&self, identity: &ReplicantIdentity) -> UserResult<UserSession> {
        let session = self.create_session(identity)?;
        self.update_last_login(&identity.replicant_name)?;
        Ok(session)
    }
    /// List all replicant names across all users (for collision detection during migration).
    ///
    /// expect: "The system provides durable storage for archival data"
    /// pre:  none
    /// post: returns Vec of all replicant_name values
    pub fn list_all_replicant_names(&self) -> UserResult<Vec<String>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT replicant_name FROM replicant_identities")?;
        let names: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .map_err(UserStoreError::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(UserStoreError::from)?;
        Ok(names)
    }
    /// Rename a replicant (used after migration when auto-rename occurred).
    ///
    /// expect: "The system provides durable storage for archival data"
    /// pre:  from_name exists; to_name does not exist
    /// post: replicant_identities.replicant_name updated
    pub fn rename_replicant(&self, from_name: &str, to_name: &str) -> UserResult<()> {
        let conn = self.lock_conn()?;
        let rows = conn.execute(
            "UPDATE replicant_identities SET replicant_name = ?1 WHERE replicant_name = ?2",
            rusqlite::params![to_name, from_name],
        )?;
        if rows == 0 {
            return Err(UserStoreError::NotFound(from_name.into()));
        }
        Ok(())
    }
    /// Delete a replicant and all its associated data.
    ///
    /// expect: "The system provides durable storage for archival data"
    /// pre:  replicant_name exists
    /// post: replicant_identities row deleted; sessions deleted
    pub fn delete_replicant(&self, replicant_name: &str) -> UserResult<()> {
        let conn = self.lock_conn()?;
        let rows = conn.execute(
            "DELETE FROM replicant_identities WHERE replicant_name = ?1",
            rusqlite::params![replicant_name],
        )?;
        if rows == 0 {
            return Err(UserStoreError::NotFound(replicant_name.into()));
        }
        conn.execute(
            "DELETE FROM user_sessions WHERE replicant_name = ?1",
            rusqlite::params![replicant_name],
        )?;
        Ok(())
    }
    /// Merge triples from a source replicant into a target replicant.
    /// Updates entity field where it matches the source replicant name.
    ///
    /// expect: "The system provides durable storage for migration data"
    /// pre:  source_name and target_name are valid replicant names
    /// post: all triples with entity = source_name updated to entity = target_name
    /// post: returns MergeReceipt with triple_count
    pub fn merge_replicant_triples(
        &self,
        source_name: &str,
        target_name: &str,
    ) -> UserResult<MergeReceipt> {
        let conn = self.lock_conn()?;
        let count = conn.execute(
            "UPDATE triples SET entity = ?1 WHERE entity = ?2",
            rusqlite::params![target_name, source_name],
        )?;
        Ok(MergeReceipt {
            triple_count: count as u64,
            source: source_name.to_string(),
            target: target_name.to_string(),
        })
    }
    /// Find a replicant by WebID.
    ///
    /// expect: "The system provides durable storage for archival data"
    /// pre:  webid is a valid WebID
    /// post: returns Some(ReplicantIdentity) if found, None otherwise
    pub fn get_replicant_by_webid(
        &self,
        webid: &hkask_types::WebID,
    ) -> UserResult<Option<ReplicantIdentity>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {REPLICANT_COLUMNS} FROM replicant_identities WHERE replicant_webid = ?1"
        ))?;
        match stmt.query_row(rusqlite::params![webid.to_string()], replicant_from_row) {
            Ok(i) => Ok(Some(i)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(UserStoreError::from(e)),
        }
    }
    /// Login a replicant with passphrase.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — authenticate replicant session
    /// pre:  replicant_name is registered, passphrase is correct
    /// post: returns UserSession on success
    /// post: returns Err if credentials invalid
    pub fn login(&self, replicant_name: &str, passphrase: &str) -> UserResult<UserSession> {
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
        // Check passphrase expiry (60 days)
        if let Some(days_old) = self
            .check_passphrase_expiry(replicant_name, 60)
            .unwrap_or(None)
        {
            tracing::warn!(
                replicant = %replicant_name,
                days_old,
                "Passphrase expired — user must change"
            );
            // Return the session but flag the expiry
            return Err(UserStoreError::PassphraseExpired(days_old));
        }
        Ok(session)
    }
    /// Logout a session.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — invalidate session
    /// pre:  session_id is valid
    /// post: session invalidated
    pub fn logout(&self, session_id: &str) -> UserResult<()> {
        let conn = self.lock_conn()?;
        conn.execute(
            "DELETE FROM user_sessions WHERE session_id = ?1",
            params![session_id],
        )?;
        Ok(())
    }
    /// Change a replicant's passphrase. Requires the old passphrase for verification.
    /// Change a replicant's passphrase.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — change replicant passphrase
    /// pre:  replicant_name is registered, old_passphrase is correct
    /// post: passphrase updated
    pub fn change_passphrase(
        &self,
        replicant_name: &str,
        old_passphrase: &str,
        new_passphrase: &str,
    ) -> UserResult<()> {
        let identity = self
            .get_replicant(replicant_name)?
            .ok_or(UserStoreError::NotFound(replicant_name.into()))?;
        let human = self.get_user(&identity.user_id)?;
        let verified = Self::verify_passphrase(old_passphrase, &human.passphrase_hash)?;
        if !verified {
            return Err(UserStoreError::InvalidCredentials);
        }
        // Hash new passphrase with existing salt and master_salt
        let new_hash = Self::hash_passphrase(new_passphrase, &human.salt)?;
        let now = chrono::Utc::now().timestamp();
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE human_users SET passphrase_hash = ?1, passphrase_set_at = ?2 WHERE user_id = ?3",
            params![new_hash, now, identity.user_id],
        )?;
        // Invalidate all existing sessions for this replicant
        conn.execute(
            "DELETE FROM user_sessions WHERE replicant_name = ?1",
            params![replicant_name],
        )?;
        Ok(())
    }
    /// Check if a replicant's passphrase is older than `max_age_days`.
    /// Returns `Some(days_old)` if expired, `None` if still valid or no timestamp.
    /// Check if a passphrase has expired.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P9\] Motivating: Homeostatic Self-Regulation — detect passphrase rotation need
    /// pre:  replicant_name is registered
    /// post: returns true if passphrase needs rotation
    pub fn check_passphrase_expiry(
        &self,
        replicant_name: &str,
        max_age_days: i64,
    ) -> UserResult<Option<i64>> {
        let identity = self
            .get_replicant(replicant_name)?
            .ok_or(UserStoreError::NotFound(replicant_name.into()))?;
        let human = self.get_user(&identity.user_id)?;
        let set_at = match human.passphrase_set_at {
            Some(ts) => ts,
            None => return Ok(None), // no timestamp set, can't check
        };
        let now = chrono::Utc::now().timestamp();
        let age_seconds = now - set_at;
        let age_days = age_seconds / 86400;
        if age_days > max_age_days {
            Ok(Some(age_days))
        } else {
            Ok(None)
        }
    }
    /// Get a session by ID.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — get session by ID
    /// pre:  session_id is non-empty
    /// post: returns Some(session) if valid, None otherwise
    pub fn get_session(&self, session_id: &str) -> UserResult<Option<UserSession>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {SESSION_COLUMNS} FROM user_sessions WHERE session_id = ?1"
        ))?;
        match stmt.query_row(params![session_id], session_from_row) {
            Ok(s) => Ok(Some(s)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(UserStoreError::from(e)),
        }
    }
    /// List sessions for a replicant.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — list active sessions
    /// pre:  replicant_name is non-empty
    /// post: returns Vec of active sessions
    pub fn list_sessions(&self, replicant_name: &str) -> UserResult<Vec<UserSession>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {SESSION_COLUMNS} FROM user_sessions WHERE replicant_name = ?1 ORDER BY last_active DESC"
        ))?;
        Ok(collect_rows!(
            stmt,
            params![replicant_name],
            session_from_row
        ))
    }
    /// Get a replicant by name.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — get replicant by name
    /// pre:  replicant_name is non-empty
    /// post: returns Some(identity) if found, None otherwise
    pub fn get_replicant(&self, replicant_name: &str) -> UserResult<Option<ReplicantIdentity>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {REPLICANT_COLUMNS} FROM replicant_identities WHERE replicant_name = ?1"
        ))?;
        match stmt.query_row(params![replicant_name], replicant_from_row) {
            Ok(i) => Ok(Some(i)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(UserStoreError::from(e)),
        }
    }
    /// Get a human user by ID.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — get human user by ID
    /// pre:  user_id is valid
    /// post: returns HumanUser
    pub fn get_user(&self, user_id: &UserID) -> UserResult<HumanUser> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT user_id, email_enc, phone_enc, passphrase_hash, salt, master_salt, created_at, last_active, passphrase_set_at,
                    oauth_provider, oauth_provider_user_id, oauth_display_name, role
             FROM human_users WHERE user_id = ?1",
        )?;
        stmt.query_row(params![user_id], |row| {
            Ok(HumanUser {
                user_id: *user_id,
                email_enc: row.get(1)?,
                phone_enc: row.get(2)?,
                passphrase_hash: row.get(3)?,
                salt: row.get(4)?,
                master_salt: row.get(5)?,
                created_at: row.get(6)?,
                last_active: row.get(7)?,
                passphrase_set_at: row.get(8)?,
                oauth_provider: row
                    .get::<_, Option<String>>(9)?
                    .and_then(|s| s.parse().ok()),
                oauth_provider_user_id: row.get(10)?,
                oauth_display_name: row.get(11)?,
                role: row
                    .get::<_, String>(12)
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(Role::Member),
            })
        })
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                UserStoreError::NotFound(user_id.as_uuid().to_string())
            }
            other => UserStoreError::from(other),
        })
    }
    /// Create an invite code for a new member.
    ///
    /// expect: "I can send an invite to bring a new user onto my server"
    /// \[P2\] Goal: Affirmative Consent — admin explicitly invites each member
    ///Constraining: User Sovereignty — invite expires and is revocable
    /// pre:  created_by is a valid UserID with Admin role
    /// post: invite row created with status Pending, 7-day expiry
    pub fn create_invite(&self, created_by: &UserID) -> UserResult<Invite> {
        let conn = self.lock_conn()?;
        let invite_id = uuid::Uuid::new_v4().to_string();
        let code = uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string();
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + 7 * 24 * 3600;
        conn.execute(
            "INSERT INTO invites (invite_id, created_by, code, status, created_at, expires_at)
             VALUES (?1, ?2, ?3, 'pending', ?4, ?5)",
            params![invite_id, created_by, code, now, expires_at],
        )?;
        Ok(Invite {
            invite_id,
            created_by: *created_by,
            code,
            status: InviteStatus::Pending,
            created_at: now,
            expires_at,
            accepted_at: None,
            accepted_user_id: None,
        })
    }
    /// Look up an invite by code.
    ///
    /// expect: "I can look up an invite code to see if it's still valid"
    /// pre:  code is a valid invite code
    /// post: returns Some(Invite) if found and not expired, None otherwise
    pub fn lookup_invite(&self, code: &str) -> UserResult<Option<Invite>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT invite_id, created_by, code, status, created_at, expires_at, accepted_at, accepted_user_id
             FROM invites WHERE code = ?1",
        )?;
        let result = stmt
            .query_row(params![code], |row| {
                Ok(Invite {
                    invite_id: row.get(0)?,
                    created_by: row.get(1)?,
                    code: row.get(2)?,
                    status: row
                        .get::<_, String>(3)?
                        .parse()
                        .unwrap_or(InviteStatus::Pending),
                    created_at: row.get(4)?,
                    expires_at: row.get(5)?,
                    accepted_at: row.get(6)?,
                    accepted_user_id: row.get(7)?,
                })
            })
            .optional()?;
        Ok(result)
    }
    /// Accept an invite, linking the accepting user to the invite.
    ///
    /// expect: "I can accept an invite to join a server"
    /// pre:  code is valid, invite is Pending and not expired
    /// post: invite status updated to Accepted, accepted_user_id set
    pub fn accept_invite(&self, code: &str, accepted_user_id: &UserID) -> UserResult<Invite> {
        let conn = self.lock_conn()?;
        let now = chrono::Utc::now().timestamp();
        let rows = conn.execute(
            "UPDATE invites SET status = 'accepted', accepted_at = ?1, accepted_user_id = ?2
             WHERE code = ?3 AND status = 'pending' AND expires_at > ?4",
            params![now, accepted_user_id, code, now],
        )?;
        if rows == 0 {
            return Err(UserStoreError::NotFound(
                "Invite not found or expired".into(),
            ));
        }
        self.lookup_invite(code)?
            .ok_or_else(|| UserStoreError::NotFound("Invite not found after accept".into()))
    }
    /// List all invites created by a user.
    ///
    /// expect: "I can see all the invites I've sent and their status"
    /// pre:  created_by is a valid UserID
    /// post: returns list of Invite records ordered by creation time (newest first)
    pub fn list_invites(&self, created_by: &UserID) -> UserResult<Vec<Invite>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT invite_id, created_by, code, status, created_at, expires_at, accepted_at, accepted_user_id
             FROM invites WHERE created_by = ?1 ORDER BY created_at DESC",
        )?;
        let invites: Vec<Invite> = stmt
            .query_map(params![created_by], |row| {
                Ok(Invite {
                    invite_id: row.get(0)?,
                    created_by: row.get(1)?,
                    code: row.get(2)?,
                    status: row
                        .get::<_, String>(3)?
                        .parse()
                        .unwrap_or(InviteStatus::Pending),
                    created_at: row.get(4)?,
                    expires_at: row.get(5)?,
                    accepted_at: row.get(6)?,
                    accepted_user_id: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(invites)
    }
    /// List all active sessions across all users (admin-only).
    ///
    /// expect: "As an admin I can see all active sessions on my server"
    /// pre:  caller must have Admin role (checked at middleware layer)
    /// post: returns list of UserSession records with active (non-expired) sessions
    pub fn list_all_sessions(&self) -> UserResult<Vec<UserSession>> {
        let conn = self.lock_conn()?;
        let now = chrono::Utc::now().timestamp();
        let mut stmt = conn.prepare(
            "SELECT session_id, replicant_name, replicant_webid, user_id, session_key_salt, expires_at, last_active
             FROM user_sessions WHERE expires_at > ?1 ORDER BY last_active DESC",
        )?;
        let sessions: Vec<UserSession> = stmt
            .query_map(params![now], |row| {
                Ok(UserSession {
                    session_id: row.get(0)?,
                    replicant_name: row.get(1)?,
                    replicant_webid: row.get(2)?,
                    user_id: row.get(3)?,
                    session_key_salt: row.get(4)?,
                    expires_at: row.get(5)?,
                    last_active: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(sessions)
    }
    /// Set the role of a user (admin-only).
    ///
    /// expect: "As an admin I can promote a user to admin or demote to member"
    /// pre:  caller must have Admin role (checked at middleware layer)
    /// post: user's role updated in database
    pub fn set_user_role(
        &self,
        user_id: &UserID,
        role: hkask_types::identity::Role,
    ) -> UserResult<()> {
        let conn = self.lock_conn()?;
        let rows = conn.execute(
            "UPDATE human_users SET role = ?1 WHERE user_id = ?2",
            params![role.to_string(), user_id],
        )?;
        if rows == 0 {
            return Err(UserStoreError::NotFound(user_id.as_uuid().to_string()));
        }
        Ok(())
    }
    /// List replicants for a user.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — list replicants owned by user
    /// pre:  user_id is valid
    /// post: returns Vec of replicants owned by user
    pub fn list_replicants(&self, user_id: &UserID) -> UserResult<Vec<ReplicantIdentity>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&format!(
            "SELECT {REPLICANT_COLUMNS} FROM replicant_identities WHERE user_id = ?1 ORDER BY is_primary DESC, created_at ASC"
        ))?;
        Ok(collect_rows!(stmt, params![user_id], replicant_from_row))
    }
    /// Get the wallet ID for a replicant.
    /// Get wallet ID for a replicant.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — get wallet ID for replicant
    /// pre:  replicant_name is non-empty
    /// post: returns Some(WalletId) if set, None otherwise
    pub fn get_wallet_id(&self, replicant_name: &str) -> UserResult<Option<WalletId>> {
        let identity = self
            .get_replicant(replicant_name)?
            .ok_or(UserStoreError::NotFound(replicant_name.into()))?;
        Ok(identity.wallet_id)
    }
    /// Set the wallet ID for a replicant (called during onboarding after wallet creation).
    /// Set wallet ID for a replicant.
    ///
    /// expect: "My user data and sovereignty boundaries are stored under my control"
    /// \[P1\] Motivating: User Sovereignty — set wallet ID for replicant
    /// pre:  replicant_name is registered, wallet_id is valid
    /// post: wallet_id stored for replicant
    pub fn set_wallet_id(&self, replicant_name: &str, wallet_id: WalletId) -> UserResult<()> {
        let conn = self.lock_conn()?;
        let rows = conn.execute(
            "UPDATE replicant_identities SET wallet_id = ?1 WHERE replicant_name = ?2",
            params![wallet_id.to_string(), replicant_name],
        )?;
        if rows == 0 {
            return Err(UserStoreError::NotFound(replicant_name.into()));
        }
        Ok(())
    }
    fn create_session(&self, identity: &ReplicantIdentity) -> UserResult<UserSession> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let session_key_salt = Self::generate_salt();
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + 86400 * 7;
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO user_sessions
             (session_id, replicant_name, replicant_webid, user_id, session_key_salt, expires_at, last_active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                session_id,
                identity.replicant_name,
                identity.replicant_webid,
                identity.user_id,
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
    fn update_last_login(&self, replicant_name: &str) -> UserResult<()> {
        let conn = self.lock_conn()?;
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
    fn hash_passphrase(passphrase: &str, salt: &str) -> UserResult<String> {
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
    fn verify_passphrase(passphrase: &str, hash: &str) -> UserResult<bool> {
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
    ) -> UserResult<Zeroizing<[u8; 32]>> {
        use hkask_keystore::encryption::derive_key;
        derive_key(
            passphrase,
            &hex::decode(master_salt).map_err(|e| UserStoreError::KeyDerivation(e.to_string()))?,
        )
        .map_err(|e| UserStoreError::KeyDerivation(e.to_string()))
    }
    pub(crate) fn encrypt_pii(plaintext: &[u8], key: &Zeroizing<[u8; 32]>) -> UserResult<Vec<u8>> {
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
/// Sanitize a display name into a valid replicant name.
///
/// Replicant names must be 1-64 alphanumeric characters with hyphens/underscores.
/// This converts spaces to underscores and strips invalid characters.
/// expect: "The system provides durable storage for archival data"
fn sanitize_replicant_name(display_name: &str) -> String {
    let sanitized: String = display_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '_'
            } else {
                '-' // Replace other special chars with hyphen
            }
        })
        .collect();
    // Trim leading/trailing hyphens and underscores
    let trimmed = sanitized.trim_matches(|c: char| c == '-' || c == '_');
    if trimmed.is_empty() {
        format!("user-{}", &uuid::Uuid::new_v4().to_string()[..8])
    } else {
        // Truncate to 64 chars
        trimmed.chars().take(64).collect()
    }
}
