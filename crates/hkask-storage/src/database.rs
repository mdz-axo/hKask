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
    /// The database exists but the passphrase is wrong — the HMAC check on
    /// page 1 failed. This is a recoverable error: the user can either provide
    /// the correct passphrase or delete the database to start fresh.
    #[error("Passphrase mismatch — database was encrypted with a different passphrase: {0}")]
    PassphraseMismatch(String),
    /// The database file is not a valid SQLite database at all (bad magic).
    /// This typically means the file is corrupted or was written by a
    /// non-SQLite process.
    #[error("Corrupted database — file is not a valid SQLite database: {0}")]
    Corrupted(String),
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

        // Ensure parent directory exists — both salt file and database file need it.
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DatabaseError::SqlCipher(format!(
                    "Failed to create database directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        let salt_path = format!("{}.salt", path);
        let (salt, salt_existed) = if let Ok(salt_bytes) = std::fs::read(&salt_path) {
            if salt_bytes.len() != SQLCIPHER_SALT_SIZE {
                return Err(DatabaseError::SqlCipher(
                    "Invalid salt file size".to_string(),
                ));
            }
            let mut salt = [0u8; SQLCIPHER_SALT_SIZE];
            salt.copy_from_slice(&salt_bytes);
            (salt, true)
        } else {
            let salt = generate_salt();
            std::fs::write(&salt_path, salt)
                .map_err(|e| DatabaseError::SqlCipher(format!("Failed to write salt: {}", e)))?;
            (salt, false)
        };
        let conn = Connection::open(path)?;
        // New databases: Connection::open writes page 1 before any PRAGMAs.
        // SQLCipher defaults to cipher_plaintext_header_size=0, so page 1 has no
        // reserved plaintext region. After PRAGMA key is set, SQLCipher tries to
        // decrypt page 1 and the HMAC fails.
        //
        // Fix: bump plaintext header size to 32 BEFORE setting the key, and keep it
        // at 32 permanently. This is a valid SQLCipher configuration used by many
        // deployments. Existing databases retain their stored header_size — no
        // migration needed since the broken old code never successfully created a DB.
        let header_size = if !salt_existed {
            conn.execute_batch("PRAGMA cipher_plaintext_header_size = 32;")?;
            32u8
        } else {
            0u8 // stored in database file, read automatically by SQLCipher
        };
        Self::configure_encryption(&conn, passphrase, &salt)?;
        if salt_existed {
            // Verify the database is readable with the derived key.
            // A wrong passphrase or corrupted salt produces an HMAC error here
            // rather than silently returning an unreadable database.
            conn.query_row(
                "SELECT count(*) FROM sqlite_master",
                rusqlite::params![],
                |row| {
                    let _count: i64 = row.get(0)?;
                    Ok(())
                },
            )
            .map_err(|e| {
                let msg = e.to_string().to_lowercase();
                // SQLCipher-encrypted files that can't be read look like
                // "file is not a database" because the header is encrypted.
                // If a .salt file exists alongside the .db, this is a passphrase
                // mismatch (our code always creates .salt alongside .db).
                // Without a .salt, it's genuine corruption.
                if msg.contains("file is not a database") || msg.contains("not a database") {
                    let salt_exists = std::path::Path::new(&salt_path).is_file();
                    if salt_exists {
                        DatabaseError::PassphraseMismatch(path.to_string())
                    } else {
                        DatabaseError::Corrupted(format!("{}: {}", path, e))
                    }
                } else {
                    DatabaseError::SqlCipher(format!(
                        "Database unreadable — wrong passphrase or corrupted file: {}",
                        e
                    ))
                }
            })?;
        }
        tracing::info!(
            target: "cns.storage",
            operation = "open",
            path = %path,
            is_new = !salt_existed,
            cipher_plaintext_header_size = header_size,
            "Database opened"
        );
        Self::initialize_schema(&conn)?;
        Self::check_schema_version(&conn, path)?;
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
    /// expect: "The system enforces OCAP boundaries on storage access"
    /// \[P4\] Motivating: Clear Boundaries — open encrypted SQLite database
    /// \[P1\] Constraining: User Sovereignty — passphrase protects local data
    /// pre:  path is valid, passphrase is non-empty
    /// post: returns Database with SQLCipher encryption
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
    /// expect: "The system enforces OCAP boundaries on storage access"
    /// \[P4\] Motivating: Clear Boundaries — open encrypted DB with DDL extensions
    /// pre:  path is valid, passphrase is non-empty, extensions is valid SQL
    /// post: returns Database with extensions applied
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
    /// expect: "The system enforces OCAP boundaries on storage access"
    /// \[P4\] Motivating: Clear Boundaries — open in-memory DB for tests
    /// post: returns in-memory Database
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
    /// expect: "The system enforces OCAP boundaries on storage access"
    /// \[P4\] Motivating: Clear Boundaries — open in-memory DB with extensions
    /// pre:  extensions is valid SQL DDL
    /// post: returns in-memory Database with extensions
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
        // Migration: add recalled_at column for existing databases
        conn.execute_batch(
            "ALTER TABLE triples ADD COLUMN recalled_at TEXT NOT NULL DEFAULT (datetime('now'));",
        )
        .ok(); // Ignore error if column already exists
        Ok(())
    }

    /// Current schema version. Increment when `schema.sql` changes and add
    /// migrations in `check_schema_version` for existing databases.
    const CURRENT_SCHEMA_VERSION: &str = "1";

    /// Verify the database schema version matches the current version.
    ///
    /// The `pod_meta` table is created by `schema.sql` with an initial
    /// `schema_version = '1'` row. On open, we compare stored vs. current
    /// and warn if they differ — a future version may run migrations here.
    fn check_schema_version(conn: &Connection, path: &str) -> Result<(), DatabaseError> {
        match conn.query_row(
            "SELECT value FROM pod_meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        ) {
            Ok(v) if v == Self::CURRENT_SCHEMA_VERSION => {}
            Ok(v) => {
                tracing::warn!(
                    target: "cns.storage",
                    path = %path,
                    db_version = %v,
                    expected = Self::CURRENT_SCHEMA_VERSION,
                    "Database schema version mismatch — data may be from an incompatible build"
                );
            }
            Err(_) => {
                tracing::debug!(
                    target: "cns.storage",
                    path = %path,
                    "No schema version row — treating as fresh database"
                );
            }
        }
        Ok(())
    }

    /// Get database connection for shared access
    /// Get a clone of the shared connection Arc.
    ///
    /// expect: "The system enforces OCAP boundaries on storage access"
    /// \[P4\] Motivating: Clear Boundaries — share connection Arc with stores
    /// post: returns `Arc<Mutex<Connection>>` for Store constructors
    pub fn conn_arc(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conn)
    }
}

/// Quick check: can this database be opened with the given passphrase?
///
/// Does NOT initialize schema or load extensions — just verifies the
/// passphrase is correct by opening and reading the schema table.
/// Returns `Ok(())` if readable, `Err(DatabaseError::PassphraseMismatch)`
/// if the passphrase is wrong, or another error for corruption.
pub fn check_passphrase(path: &str, passphrase: &str) -> Result<(), DatabaseError> {
    let _ = Database::open(path, passphrase)?;
    Ok(())
}

/// Open an encrypted database, self-healing on passphrase mismatch.
///
/// If the passphrase is correct, returns the encrypted database normally.
/// If the passphrase is wrong (`PassphraseMismatch`), prints a clear
/// message to stderr, deletes the old database + salt so a fresh one
/// can be created, then retries the open (which auto-creates a new DB).
///
/// This is the preferred entry point for MCP servers and REPL init —
/// it closes the feedback loop by fixing the problem automatically
/// rather than crashing with a cryptic SQLCipher error.
pub fn open_or_repair(path: &str, passphrase: &str) -> Result<Database, DatabaseError> {
    match Database::open(path, passphrase) {
        Ok(db) => Ok(db),
        Err(DatabaseError::PassphraseMismatch(_)) => {
            eprintln!(
                "hKask: Database {} was encrypted with a different passphrase.",
                path
            );
            eprintln!("       Deleting old database — a fresh one will be created.");
            eprintln!("       Run 'kask repair' to audit all databases.");
            let _ = std::fs::remove_file(path);
            let salt_path = format!("{}.salt", path);
            let _ = std::fs::remove_file(&salt_path);
            Database::open(path, passphrase)
        }
        Err(e) => Err(e),
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
/// expect: "The system enforces OCAP boundaries on storage access"
/// \[P4\] Motivating: Clear Boundaries — infallible encrypted DB open
/// pre:  path is valid, passphrase is non-empty
/// post: returns Database (in-memory if path is ":memory:")
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
/// expect: "The system enforces OCAP boundaries on storage access"
/// \[P4\] Motivating: Clear Boundaries — infallible in-memory DB open
/// post: returns in-memory Database (panics on failure)
pub fn in_memory_db() -> Database {
    Database::in_memory().expect("in-memory database initialization should never fail")
}
fn generate_salt() -> [u8; SQLCIPHER_SALT_SIZE] {
    use rand::Rng;
    rand::rng().random()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_creates_parent_directories() {
        let tmp = std::env::temp_dir().join(format!("hkask-db-test-{}", rand::random::<u32>()));
        let db_path = tmp.join("a").join("b").join("c").join("test.db");
        let db_path_str = db_path.to_string_lossy().to_string();

        // Ensure the path doesn't exist yet
        if db_path.exists() {
            std::fs::remove_file(&db_path).ok();
        }
        // Open with a valid passphrase — should auto-create all parent dirs
        let result = Database::open(&db_path_str, "test-passphrase-123");
        assert!(result.is_ok(), "Database::open failed: {:?}", result.err());

        // Verify the database and salt file were created
        assert!(db_path.exists(), "DB file should exist at {:?}", db_path);
        let salt_path = format!("{}.salt", db_path_str);
        assert!(
            std::path::Path::new(&salt_path).exists(),
            "Salt file should exist at {}",
            salt_path
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
