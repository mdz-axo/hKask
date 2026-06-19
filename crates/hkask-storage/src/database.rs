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
pub(crate) const DEFAULT_EMBEDDING_DIM: usize = 1024;
/// Get embedding dimension from environment or default
pub(crate) fn embedding_dim() -> usize {
    std::env::var("HKASK_EMBEDDING_DIM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_EMBEDDING_DIM)
}
fn load_sqlite_vec() -> Result<(), DatabaseError> {
    use std::sync::Once;
    static INIT: Once = Once::new();
    // SAFETY: sqlite3_vec_init is the canonical entry point for the sqlite-vec
    // extension. sqlite3_auto_extension expects a sqlite3_ext_init_fn which is
    // equivalent to extern "C" fn(...) -> c_int. sqlite3_vec_init has signature
    // fn() -> (), so we transmute the function pointer to the expected entry
    // point type. This is the standard pattern used by sqlite-vec and rusqlite.
    // SAFETY: per above — FFI transmute for sqlite3 extension registration.
    INIT.call_once(|| unsafe {
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
pub(crate) const SQLCIPHER_SALT_SIZE: usize = 16;
#[derive(Error, Debug)]
#[non_exhaustive]
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
}
impl Database {
    /// Open database with passphrase for encryption, optional schema extensions.
    fn open_impl(
        path: &str,
        passphrase: &str,
        extensions: Option<&str>,
    ) -> Result<Self, DatabaseError> {
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
        if let Some(ext) = extensions {
            conn.execute_batch(ext)?;
        }
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
    /// Open database with passphrase for encryption
    /// Open an encrypted database at the given path.
    ///
    /// \[P4\] Motivating: Clear Boundaries — open encrypted SQLite database
    /// \[P1\] Constraining: User Sovereignty — passphrase protects local data
    pub fn open(path: &str, passphrase: &str) -> Result<Self, DatabaseError> {
        Self::open_impl(path, passphrase, None)
    }
    /// Open database with passphrase and custom schema extensions
    ///
    /// After initializing the core schema, executes the provided DDL string.
    /// This allows MCP servers and other consumers to add custom tables
    /// (e.g., FTS5 virtual tables, domain-specific indexes) while
    /// inheriting the core encrypted storage infrastructure.
    ///
    /// # Arguments
    /// * `path` — Path to the SQLite database file
    /// * `passphrase` — Passphrase for SQLCipher encryption
    /// * `extensions` — Additional DDL to execute after core schema init
    ///
    /// Open database with additional DDL extensions.
    ///
    /// \[P4\] Motivating: Clear Boundaries — open encrypted DB with DDL extensions
    pub fn open_with_extensions(
        path: &str,
        passphrase: &str,
        extensions: &str,
    ) -> Result<Self, DatabaseError> {
        Self::open_impl(path, passphrase, Some(extensions))
    }
    /// Open in-memory database (unencrypted, for testing)
    fn in_memory_impl(extensions: Option<&str>) -> Result<Self, DatabaseError> {
        load_sqlite_vec()?;
        let conn = Connection::open_in_memory()?;
        Self::initialize_schema(&conn)?;
        if let Some(ext) = extensions {
            conn.execute_batch(ext)?;
        }
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
    /// Open in-memory database (unencrypted, for testing)
    /// Open an in-memory database (unencrypted, for testing).
    ///
    /// \[P4\] Motivating: Clear Boundaries — open in-memory DB for tests
    pub fn in_memory() -> Result<Self, DatabaseError> {
        Self::in_memory_impl(None)
    }
    /// Open in-memory database with custom schema extensions (unencrypted, for testing)
    ///
    /// After initializing the core schema, executes the provided DDL string.
    /// This allows MCP servers and other consumers to add custom tables
    /// (e.g., FTS5 virtual tables, domain-specific indexes) while
    /// inheriting the core storage infrastructure.
    ///
    /// # Arguments
    /// * `extensions` — Additional DDL to execute after core schema init
    ///
    /// Open in-memory database with extensions.
    ///
    /// \[P4\] Motivating: Clear Boundaries — open in-memory DB with extensions
    pub fn in_memory_with_extensions(extensions: &str) -> Result<Self, DatabaseError> {
        Self::in_memory_impl(Some(extensions))
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
        let schema = include_str!("sql/schema.sql");
        let dim = embedding_dim();
        conn.execute_batch(&schema.replace("$DIM", &dim.to_string()))?;
        Ok(())
    }
    /// Get database connection for shared access
    /// Get a clone of the shared connection Arc.
    ///
    /// \[P4\] Motivating: Clear Boundaries — share connection Arc with stores
    pub fn conn_arc(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }
}
/// Open a database from a path and passphrase, with in-memory fallback.
///
/// If `path` is `":memory:"`, opens an in-memory database (unencrypted).
/// Otherwise, opens an encrypted database at the given path with the passphrase.
///
/// This is the canonical way to open a database from CLI/API code that
/// resolves the path and passphrase from environment variables or keychain.
/// Open a database from path or :memory:.
///
/// \[P4\] Motivating: Clear Boundaries — infallible encrypted DB open
pub fn open_database(path: &str, passphrase: &str) -> Result<Database, DatabaseError> {
    if path == ":memory:" {
        Database::in_memory()
    } else {
        Database::open(path, passphrase)
    }
}
/// Open an in-memory database, panicking on failure.
///
/// Use this in test fixtures and CLI startup where an in-memory DB
/// failure is always a bug, never a recoverable condition. Replaces the
/// repeated `Database::in_memory().expect("in-memory db")` pattern.
///
/// For recoverable contexts (API, services), use `Database::in_memory()`
/// and propagate the error with `?`.
/// Create an in-memory database for testing.
///
/// \[P4\] Motivating: Clear Boundaries — infallible in-memory DB open
pub fn in_memory_db() -> Database {
    Database::in_memory().expect("in-memory database initialization should never fail")
}
fn generate_salt() -> [u8; SQLCIPHER_SALT_SIZE] {
    use rand::Rng;
    rand::rng().random()
}
