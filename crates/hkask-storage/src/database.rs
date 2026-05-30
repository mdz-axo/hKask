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

/// Default embedding dimension (configurable via HKASK_EMBEDDING_DIM)
pub const DEFAULT_EMBEDDING_DIM: usize = 384;

/// Get embedding dimension from environment or default
pub fn embedding_dim() -> usize {
    std::env::var("HKASK_EMBEDDING_DIM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_EMBEDDING_DIM)
}

fn load_sqlite_vec() -> Result<(), DatabaseError> {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        // SAFETY: sqlite3_vec_init is the canonical entry point for the sqlite-vec
        // extension. sqlite3_auto_extension expects a sqlite3_ext_init_fn which is
        // equivalent to extern "C" fn(*mut sqlite3, *mut *const c_char, *const sqlite3_api_routines) -> c_int.
        // sqlite3_vec_init has signature fn() -> (), so we transmute the function pointer
        // to the expected entry point type. This is the standard pattern used by
        // sqlite-vec and rusqlite projects — the actual registration is handled
        // internally by sqlite3_vec_init when invoked.
        type Sqlite3ExtInitFn = unsafe extern "C" fn(
            *mut rusqlite::ffi::sqlite3,
            *mut *mut std::os::raw::c_char,
            *const rusqlite::ffi::sqlite3_api_routines,
        ) -> std::os::raw::c_int;

        // sqlite3_vec_init is a void fn() that internally handles the sqlite3
        // extension registration. We cast its address to the expected entry point
        // type for sqlite3_auto_extension, which is the standard FFI registration pattern.
        let init_fn: Sqlite3ExtInitFn =
            std::mem::transmute::<_, Sqlite3ExtInitFn>(sqlite_vec::sqlite3_vec_init as *const ());
        rusqlite::ffi::sqlite3_auto_extension(Some(init_fn));
    });
    Ok(())
}

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
/// The `Arc<Mutex<>>` allows multiple stores (TripleStore, EmbeddingStore, etc.) to share
/// the same connection in a thread-safe manner.
///
/// **Thread Safety:** This type is `Send` and `Sync` for multi-threaded access.
pub struct Database {
    pub(crate) conn: Arc<Mutex<Connection>>,
    salt: [u8; SQLCIPHER_SALT_SIZE],
}

impl Database {
    /// Open database with passphrase for encryption
    pub fn open(path: &str, passphrase: &str) -> Result<Self, DatabaseError> {
        load_sqlite_vec()?;
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
        load_sqlite_vec()?;
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
        let dim = embedding_dim();
        conn.execute_batch(
            &format!(
            "CREATE TABLE IF NOT EXISTS triples (id TEXT PRIMARY KEY, entity TEXT NOT NULL, attribute TEXT NOT NULL, value TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT, transaction_at TEXT DEFAULT (datetime('now')), confidence REAL NOT NULL DEFAULT 1.0, perspective TEXT, visibility TEXT NOT NULL DEFAULT 'private', owner_webid TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS embeddings (id TEXT PRIMARY KEY, entity_ref TEXT REFERENCES triples(id), vector BLOB NOT NULL, dimensions INTEGER NOT NULL, model TEXT NOT NULL, created_at TEXT DEFAULT (datetime('now')));
            CREATE VIRTUAL TABLE IF NOT EXISTS vec_embeddings USING vec0(id TEXT PRIMARY KEY, embedding float[{dim}]);
            CREATE TABLE IF NOT EXISTS nu_events (id TEXT PRIMARY KEY, timestamp TEXT NOT NULL, observer_webid TEXT NOT NULL, span_category TEXT NOT NULL, span_path TEXT NOT NULL, phase TEXT NOT NULL, observation TEXT NOT NULL, regulation TEXT, outcome TEXT, recursion_depth INTEGER NOT NULL, parent_event TEXT, visibility TEXT NOT NULL DEFAULT 'private');
            CREATE TABLE IF NOT EXISTS blobs (id TEXT PRIMARY KEY, content_type TEXT NOT NULL, size INTEGER NOT NULL, blake3_hash TEXT NOT NULL, data BLOB NOT NULL, created_at TEXT DEFAULT (datetime('now')), visibility TEXT NOT NULL DEFAULT 'private', owner_webid TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS audit_log (id TEXT PRIMARY KEY, timestamp TEXT NOT NULL, actor_webid TEXT NOT NULL, action TEXT NOT NULL, resource TEXT NOT NULL, outcome TEXT NOT NULL, details TEXT, ip_address TEXT, created_at TEXT DEFAULT (datetime('now')));
            CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp ON audit_log(timestamp);
            CREATE INDEX IF NOT EXISTS idx_audit_log_actor ON audit_log(actor_webid);
            CREATE TABLE IF NOT EXISTS cns_variety_checkpoint (domain TEXT PRIMARY KEY, variety_count INTEGER NOT NULL, last_updated TEXT NOT NULL, threshold INTEGER NOT NULL DEFAULT 10);
            CREATE TABLE IF NOT EXISTS cns_alerts (id TEXT PRIMARY KEY, timestamp TEXT NOT NULL, alert_type TEXT NOT NULL, severity TEXT NOT NULL, domain TEXT, message TEXT NOT NULL, resolved INTEGER NOT NULL DEFAULT 0, resolved_at TEXT);
            CREATE TABLE IF NOT EXISTS agent_registry (name TEXT PRIMARY KEY, agent_kind TEXT NOT NULL, definition_json TEXT NOT NULL, token_hash TEXT NOT NULL, registered_at TEXT NOT NULL, source_yaml TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS idx_agent_registry_kind ON agent_registry(agent_kind);
            CREATE TABLE IF NOT EXISTS goals (id TEXT PRIMARY KEY, webid TEXT NOT NULL, text TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'pending', visibility TEXT NOT NULL DEFAULT 'private', created_at TEXT DEFAULT (datetime('now')), completed_at TEXT, parent_goal_id TEXT, depth INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS goal_criteria (id TEXT PRIMARY KEY, goal_id TEXT REFERENCES goals(id), type TEXT NOT NULL, description TEXT NOT NULL, satisfied INTEGER NOT NULL DEFAULT 0);
            CREATE TABLE IF NOT EXISTS goal_artifacts (id TEXT PRIMARY KEY, goal_id TEXT REFERENCES goals(id), artifact_ref TEXT NOT NULL, artifact_type TEXT NOT NULL, created_at TEXT DEFAULT (datetime('now')));
            CREATE TABLE IF NOT EXISTS goal_semantic_memory (id TEXT PRIMARY KEY, webid TEXT NOT NULL, goal_id TEXT NOT NULL, memory_json TEXT NOT NULL, created_at TEXT DEFAULT (datetime('now')));
            CREATE INDEX IF NOT EXISTS idx_goal_semantic_webid ON goal_semantic_memory(webid);
            CREATE INDEX IF NOT EXISTS idx_goal_semantic_goal_id ON goal_semantic_memory(goal_id);
            CREATE TABLE IF NOT EXISTS goal_episodic_memory (id TEXT PRIMARY KEY, webid TEXT NOT NULL, goal_id TEXT NOT NULL, memory_json TEXT NOT NULL, created_at TEXT DEFAULT (datetime('now')));
            CREATE INDEX IF NOT EXISTS idx_goal_episodic_webid ON goal_episodic_memory(webid);
            CREATE INDEX IF NOT EXISTS idx_goal_episodic_goal_id ON goal_episodic_memory(goal_id);")
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
