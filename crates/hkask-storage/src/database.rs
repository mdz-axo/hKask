//! Database connection with SQLCipher encryption
//!
//! Uses SQLCipher with AES-256-CBC encryption.
//! Passphrases are derived using Argon2id to produce 256-bit encryption keys.
//!
//! **Spec Reference:** architecture v0.21.0 §2.3

use hkask_keystore::derive_key;
use rusqlite::Connection;
use std::rc::Rc;
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
/// Uses `Rc` for connection sharing because all database operations occur on a single thread.
/// The `Rc` allows multiple stores (TripleStore, EmbeddingStore, BlobStore) to share
/// the same connection without violating Rust's borrowing rules.
///
/// **Thread Safety:** This type is not `Send` or `Sync`. For multi-threaded access,
/// use separate `Database` instances per thread or wrap in `Arc<Mutex<Database>>`.
pub struct Database {
    conn: Rc<Connection>,
    salt: [u8; SQLCIPHER_SALT_SIZE],
}

impl Database {
    /// Open database with passphrase for encryption
    ///
    /// The passphrase is derived using Argon2id to produce a 256-bit key for SQLCipher.
    /// A random salt is generated and stored with the database file.
    ///
    /// **Security:** For production use, store the salt securely (e.g., in a separate file
    /// or database header) to prevent rainbow table attacks.
    ///
    /// **Passphrase Requirements:**
    /// - Minimum 8 characters
    /// - Cannot be empty
    pub fn open(path: &str, passphrase: &str) -> Result<Self, DatabaseError> {
        // Validate passphrase
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

        // Try to read existing salt from file, or generate new one
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
            // Generate new salt
            let salt = generate_salt();
            std::fs::write(&salt_path, salt)
                .map_err(|e| DatabaseError::SqlCipher(format!("Failed to write salt: {}", e)))?;
            salt
        };

        let conn = Connection::open(path)?;

        // Configure SQLCipher encryption with derived key
        Self::configure_encryption(&conn, passphrase, &salt)?;

        // Initialize schema
        Self::initialize_schema(&conn)?;

        Ok(Self {
            conn: Rc::new(conn),
            salt,
        })
    }

    /// Open in-memory database (unencrypted, for testing)
    pub fn in_memory() -> Result<Self, DatabaseError> {
        let conn = Connection::open_in_memory()?;

        // Initialize schema
        Self::initialize_schema(&conn)?;

        Ok(Self {
            conn: Rc::new(conn),
            salt: [0u8; SQLCIPHER_SALT_SIZE],
        })
    }

    /// Open database with explicit salt (for when salt is stored separately)
    pub fn with_salt(
        path: &str,
        passphrase: &str,
        salt: &[u8; SQLCIPHER_SALT_SIZE],
    ) -> Result<Self, DatabaseError> {
        let conn = Connection::open(path)?;

        // Configure SQLCipher encryption with derived key
        Self::configure_encryption(&conn, passphrase, salt)?;

        // Initialize schema
        Self::initialize_schema(&conn)?;

        Ok(Self {
            conn: Rc::new(conn),
            salt: *salt,
        })
    }

    /// Configure SQLCipher encryption on the connection
    fn configure_encryption(
        conn: &Connection,
        passphrase: &str,
        salt: &[u8],
    ) -> Result<(), DatabaseError> {
        // Derive 256-bit key from passphrase using Argon2id
        let key = derive_key(passphrase, salt)
            .map_err(|e| DatabaseError::KeyDerivation(e.to_string()))?;

        // Convert key to hex for SQLCipher PRAGMA
        let key_hex = hex::encode(*key);

        // Set the encryption key (hex-encoded for SQLCipher)
        conn.execute_batch(&format!("PRAGMA key = 'x\"{}\"';", key_hex))?;

        // Verify SQLCipher is active
        let cipher_version: String = conn
            .query_row("PRAGMA cipher_version;", [], |row| row.get(0))
            .map_err(|e| DatabaseError::SqlCipher(format!("SQLCipher not available: {}", e)))?;

        // Verify encryption is working by checking cipher_provider
        let cipher_provider: String = conn
            .query_row("PRAGMA cipher_provider;", [], |row| row.get(0))
            .map_err(|e| {
                DatabaseError::SqlCipher(format!("SQLCipher provider check failed: {}", e))
            })?;

        // Verify cipher algorithm is AES-256
        let cipher: String = conn
            .query_row("PRAGMA cipher;", [], |row| row.get(0))
            .map_err(|e| DatabaseError::SqlCipher(format!("Failed to get cipher: {}", e)))?;

        if !cipher.to_uppercase().contains("AES-256") {
            return Err(DatabaseError::SqlCipher(format!(
                "Expected AES-256 cipher, got: {}",
                cipher
            )));
        }

        tracing::info!(
            target: "hkask::storage",
            cipher_version = %cipher_version,
            cipher_provider = %cipher_provider,
            cipher = %cipher,
            "SQLCipher encryption enabled"
        );

        Ok(())
    }

    /// Initialize database schema
    fn initialize_schema(conn: &Connection) -> Result<(), DatabaseError> {
        // Bitemporal triples table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS triples (
                id              TEXT PRIMARY KEY,
                entity          TEXT NOT NULL,
                attribute       TEXT NOT NULL,
                value           TEXT NOT NULL,
                valid_from      TEXT NOT NULL,
                valid_to        TEXT,
                transaction_at  TEXT NOT NULL DEFAULT (datetime('now')),
                confidence      REAL NOT NULL DEFAULT 1.0,
                perspective     TEXT,
                visibility      TEXT NOT NULL DEFAULT 'private',
                owner_webid     TEXT NOT NULL
            )",
            [],
        )?;

        // Embeddings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS embeddings (
                id              TEXT PRIMARY KEY,
                entity_ref      TEXT REFERENCES triples(id),
                vector          BLOB NOT NULL,
                dimensions      INTEGER NOT NULL,
                model           TEXT NOT NULL,
                created_at      TEXT DEFAULT (datetime('now'))
            )",
            [],
        )?;

        // ν-events table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS nu_events (
                id              TEXT PRIMARY KEY,
                timestamp       TEXT NOT NULL,
                observer_webid  TEXT NOT NULL,
                span_category   TEXT NOT NULL,
                span_path       TEXT NOT NULL,
                phase           TEXT NOT NULL,
                observation     TEXT NOT NULL,
                regulation      TEXT,
                outcome         TEXT,
                recursion_depth INTEGER NOT NULL,
                parent_event    TEXT,
                visibility      TEXT NOT NULL DEFAULT 'private'
            )",
            [],
        )?;

        // Blobs table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blobs (
                id              TEXT PRIMARY KEY,
                content_type    TEXT NOT NULL,
                size            INTEGER NOT NULL,
                blake3_hash     TEXT NOT NULL,
                data            BLOB NOT NULL,
                created_at      TEXT DEFAULT (datetime('now')),
                visibility      TEXT NOT NULL DEFAULT 'private',
                owner_webid     TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    /// Get database connection
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Get database connection as Rc for shared ownership
    pub fn conn_rc(&self) -> Rc<Connection> {
        Rc::clone(&self.conn)
    }

    /// Get the salt used for key derivation
    pub fn salt(&self) -> &[u8; SQLCIPHER_SALT_SIZE] {
        &self.salt
    }
}

/// Generate a random salt for key derivation
fn generate_salt() -> [u8; SQLCIPHER_SALT_SIZE] {
    use rand::RngCore;
    let mut salt = [0u8; SQLCIPHER_SALT_SIZE];
    rand::rng().fill_bytes(&mut salt);
    salt
}

