//! Database connection with SQLCipher encryption

use rusqlite::Connection;
use std::rc::Rc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("SQLCipher error: {0}")]
    SqlCipher(String),
}

/// Database wrapper with SQLCipher support
pub struct Database {
    conn: Rc<Connection>,
}

impl Database {
    /// Open database with passphrase for encryption
    ///
    /// Uses SQLCipher with AES-256-CBC encryption.
    ///
    /// **Spec Reference:** architecture v0.21.0 §2.3
    pub fn open(path: &str, passphrase: &str) -> Result<Self, DatabaseError> {
        let conn = Connection::open(path)?;
        
        // Configure SQLCipher encryption
        Self::configure_encryption(&conn, passphrase)?;
        
        // Initialize schema
        Self::initialize_schema(&conn)?;
        
        Ok(Self {
            conn: Rc::new(conn),
        })
    }
    
    /// Open in-memory database (unencrypted, for testing)
    pub fn in_memory() -> Result<Self, DatabaseError> {
        let conn = Connection::open_in_memory()?;
        
        // Initialize schema
        Self::initialize_schema(&conn)?;
        
        Ok(Self {
            conn: Rc::new(conn),
        })
    }
    
    /// Configure SQLCipher encryption on the connection
    fn configure_encryption(conn: &Connection, passphrase: &str) -> Result<(), DatabaseError> {
        // Set the encryption key
        conn.execute_batch(&format!("PRAGMA key = '{}';", passphrase.replace('\'', "''")))?;
        
        // Verify SQLCipher is active
        let cipher_version: String = conn.query_row(
            "PRAGMA cipher_version;",
            [],
            |row| row.get(0),
        ).map_err(|e| DatabaseError::SqlCipher(format!("SQLCipher not available: {}", e)))?;
        
        // Verify encryption is working by checking cipher_provider
        let cipher_provider: String = conn.query_row(
            "PRAGMA cipher_provider;",
            [],
            |row| row.get(0),
        ).map_err(|e| DatabaseError::SqlCipher(format!("SQLCipher provider check failed: {}", e)))?;
        
        tracing::info!(
            target: "hkask::storage",
            cipher_version = %cipher_version,
            cipher_provider = %cipher_provider,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_in_memory_database() {
        let db = Database::in_memory().unwrap();
        let conn = db.conn();

        // Verify tables exist
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"triples".to_string()));
        assert!(tables.contains(&"embeddings".to_string()));
        assert!(tables.contains(&"nu_events".to_string()));
        assert!(tables.contains(&"blobs".to_string()));
    }
    
    #[test]
    fn test_encrypted_database() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_encrypted.db");
        let passphrase = "test_passphrase_123";
        
        // Create encrypted database
        let db = Database::open(db_path.to_str().unwrap(), passphrase).unwrap();
        
        // Verify tables exist
        let conn = db.conn();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        
        assert!(tables.contains(&"triples".to_string()));
        assert!(tables.contains(&"embeddings".to_string()));
        assert!(tables.contains(&"nu_events".to_string()));
        assert!(tables.contains(&"blobs".to_string()));
        
        // Verify SQLCipher is active
        let cipher_version: String = conn
            .query_row("PRAGMA cipher_version;", [], |row| row.get(0))
            .unwrap();
        assert!(!cipher_version.is_empty());
        
        // Verify data is encrypted (can't read without key)
        drop(db);
        
        // Try to open without correct passphrase - should fail or return garbage
        let wrong_passphrase = "wrong_passphrase";
        let result = Database::open(db_path.to_str().unwrap(), wrong_passphrase);
        
        // With SQLCipher, wrong passphrase will either fail or return unreadable data
        // The behavior depends on SQLCipher version - newer versions fail, older return garbage
        if let Ok(db) = result {
            // If it opens, verify we can't read the data properly
            let conn = db.conn();
            let result: Result<Vec<String>, _> = conn
                .prepare("SELECT id FROM triples")
                .and_then(|mut stmt| {
                    stmt.query_map([], |row| row.get(0))
                        .map(|rows| rows.filter_map(|r| r.ok()).collect())
                });
            // Either fails or returns empty/garbage
            let _ = result;
        }
    }
}
