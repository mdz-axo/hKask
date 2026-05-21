//! Database connection with SQLCipher encryption
//!
//! Uses SQLCipher with AES-256-CBC encryption.
//! Passphrases are derived using Argon2id to produce 256-bit encryption keys.
//!
//! **Spec Reference:** architecture v0.21.0 §2.3

use hkask_keystore::derive_key;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Salt size for SQLCipher key derivation
pub const SQLCIPHER_SALT_SIZE: usize = 16;

/// SQLCipher key size (256 bits for AES-256)
pub const SQLCIPHER_KEY_SIZE: usize = 32;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("SQLCipher error: {0}")]
    SqlCipher(String),
    #[error("Key derivation error: {0}")]
    KeyDerivation(String),
}

/// Database wrapper with SQLCipher support
///
/// Uses `Arc<Mutex<>>` for connection sharing across threads.
/// The `Arc<Mutex<>>` allows multiple stores (TripleStore, EmbeddingStore, BlobStore) to share
/// the same connection in a thread-safe manner.
///
/// **Thread Safety:** This type is `Send` and `Sync` for multi-threaded access.
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    salt: [u8; SQLCIPHER_SALT_SIZE],
}

impl Database {
    /// Open database with passphrase for encryption
    pub fn open(path: &str, passphrase: &str) -> Result<Self, DatabaseError> {
        if passphrase.is_empty() {
            return Err(DatabaseError::KeyDerivation(
                "Passphrase cannot be empty".to_string(),
            ));
        }
        if passphrase.len() < 8 {
            return Err(DatabaseError::KeyDerivation(
                "Passphrase must be at least 8 characters".to_string(),
            ));
        }

        let salt_path = format!("{}.salt", path);
        let salt = if let Ok(salt_bytes) = std::fs::read(&salt_path) {
            if salt_bytes.len() != SQLCIPHER_SALT_SIZE {
                return Err(DatabaseError::SqlCipher(
                    "Invalid salt file size".to_string(),
                ));
            }
            let mut salt = [0u8; SQLCIPHER_SALT_SIZE];
            salt.copy_from_slice(&salt_bytes);
            salt
        } else {
            let salt = generate_salt();
            std::fs::write(&salt_path, salt)
                .map_err(|e| DatabaseError::SqlCipher(format!("Failed to write salt: {}", e)))?;
            salt
        };

        let conn = Connection::open(path)?;
        Self::configure_encryption(&conn, passphrase, &salt)?;
        Self::initialize_schema(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            salt,
        })
    }

    /// Open in-memory database (unencrypted, for testing)
    pub fn in_memory() -> Result<Self, DatabaseError> {
        let conn = Connection::open_in_memory()?;
        Self::initialize_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            salt: [0u8; SQLCIPHER_SALT_SIZE],
        })
    }

    fn configure_encryption(
        conn: &Connection,
        passphrase: &str,
        salt: &[u8],
    ) -> Result<(), DatabaseError> {
        let key = derive_key(passphrase, salt)
            .map_err(|e| DatabaseError::KeyDerivation(e.to_string()))?;
        let key_hex = hex::encode(*key);
        conn.execute_batch(&format!("PRAGMA key = 'x\"{}\"';", key_hex))?;
        Ok(())
    }

    fn initialize_schema(conn: &Connection) -> Result<(), DatabaseError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS triples (id TEXT PRIMARY KEY, entity TEXT NOT NULL, attribute TEXT NOT NULL, value TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT, transaction_at TEXT DEFAULT (datetime('now')), confidence REAL NOT NULL DEFAULT 1.0, perspective TEXT, visibility TEXT NOT NULL DEFAULT 'private', owner_webid TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS embeddings (id TEXT PRIMARY KEY, entity_ref TEXT REFERENCES triples(id), vector BLOB NOT NULL, dimensions INTEGER NOT NULL, model TEXT NOT NULL, created_at TEXT DEFAULT (datetime('now')));
            CREATE TABLE IF NOT EXISTS nu_events (id TEXT PRIMARY KEY, timestamp TEXT NOT NULL, observer_webid TEXT NOT NULL, span_category TEXT NOT NULL, span_path TEXT NOT NULL, phase TEXT NOT NULL, observation TEXT NOT NULL, regulation TEXT, outcome TEXT, recursion_depth INTEGER NOT NULL, parent_event TEXT, visibility TEXT NOT NULL DEFAULT 'private');
            CREATE TABLE IF NOT EXISTS blobs (id TEXT PRIMARY KEY, content_type TEXT NOT NULL, size INTEGER NOT NULL, blake3_hash TEXT NOT NULL, data BLOB NOT NULL, created_at TEXT DEFAULT (datetime('now')), visibility TEXT NOT NULL DEFAULT 'private', owner_webid TEXT NOT NULL);"
        )?;
        Ok(())
    }

    /// Get database connection for shared access
    pub fn conn_arc(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }

    /// Get the salt used for key derivation
    pub fn salt(&self) -> &[u8; SQLCIPHER_SALT_SIZE] {
        &self.salt
    }
}

fn generate_salt() -> [u8; SQLCIPHER_SALT_SIZE] {
    use rand::Rng;
    rand::rng().random()
}
