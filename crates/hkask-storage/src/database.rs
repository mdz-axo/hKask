//! Database connection with SQLCipher encryption

use rusqlite::Connection;
use std::rc::Rc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// Database wrapper with SQLCipher support
pub struct Database {
    conn: Rc<Connection>,
}

impl Database {
    /// Open database with passphrase for encryption
    pub fn open(_path: &str, _passphrase: &str) -> Result<Self, DatabaseError> {
        // Stub - returns in-memory for now
        Self::in_memory()
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
    pub fn conn(&self) -> Rc<Connection> {
        Rc::clone(&self.conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
