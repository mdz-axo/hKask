//! SpecStore — SQLite-backed specification storage

use crate::spec_types::{Spec, SpecCategory, SpecError, SpecId, SpecStore};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct SqliteSpecStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSpecStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn init_schema(&self) -> Result<(), SpecError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SpecError::Storage(format!("Lock poisoned: {}", e)))?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS specs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                domain_anchor TEXT NOT NULL,
                signed_by TEXT,
                created_at TEXT NOT NULL,
                data TEXT NOT NULL
            )",
            [],
        )
        .map_err(|e| SpecError::Storage(e.to_string()))?;
        Ok(())
    }
}

impl SpecStore for SqliteSpecStore {
    fn load(&self, id: SpecId) -> Result<Spec, SpecError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SpecError::Storage(format!("Lock poisoned: {}", e)))?;
        let mut stmt = conn
            .prepare("SELECT data FROM specs WHERE id = ?1")
            .map_err(|e| SpecError::Storage(e.to_string()))?;
        let data: String = stmt
            .query_row(rusqlite::params![id.to_string()], |row| row.get(0))
            .map_err(|_| SpecError::NotFound(id))?;
        serde_json::from_str(&data).map_err(|e| SpecError::Storage(e.to_string()))
    }

    fn save(&self, spec: &Spec) -> Result<(), SpecError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SpecError::Storage(format!("Lock poisoned: {}", e)))?;
        let data = serde_json::to_string(spec).map_err(|e| SpecError::Storage(e.to_string()))?;
        let signed_by = spec.signed_by.map(|w| w.to_string());
        conn.execute(
            "INSERT OR REPLACE INTO specs (id, name, category, domain_anchor, signed_by, created_at, data)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                spec.id.to_string(),
                spec.name,
                spec.category.as_str(),
                spec.domain_anchor.as_str(),
                signed_by,
                spec.created_at.to_rfc3339(),
                data,
            ],
        )
        .map_err(|e| SpecError::Storage(e.to_string()))?;
        Ok(())
    }

    fn delete(&self, id: SpecId) -> Result<(), SpecError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SpecError::Storage(format!("Lock poisoned: {}", e)))?;
        let changed = conn
            .execute(
                "DELETE FROM specs WHERE id = ?1",
                rusqlite::params![id.to_string()],
            )
            .map_err(|e| SpecError::Storage(e.to_string()))?;
        if changed == 0 {
            return Err(SpecError::NotFound(id));
        }
        Ok(())
    }

    fn list_all(&self) -> Result<Vec<Spec>, SpecError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SpecError::Storage(format!("Lock poisoned: {}", e)))?;
        let mut stmt = conn
            .prepare("SELECT data FROM specs")
            .map_err(|e| SpecError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            })
            .map_err(|e| SpecError::Storage(e.to_string()))?;
        let mut specs = Vec::new();
        for row in rows {
            let data = row.map_err(|e| SpecError::Storage(e.to_string()))?;
            let spec: Spec =
                serde_json::from_str(&data).map_err(|e| SpecError::Storage(e.to_string()))?;
            specs.push(spec);
        }
        Ok(specs)
    }

    fn list_by_category(&self, cat: SpecCategory) -> Result<Vec<Spec>, SpecError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SpecError::Storage(format!("Lock poisoned: {}", e)))?;
        let mut stmt = conn
            .prepare("SELECT data FROM specs WHERE category = ?1")
            .map_err(|e| SpecError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![cat.as_str()], |row| {
                let data: String = row.get(0)?;
                Ok(data)
            })
            .map_err(|e| SpecError::Storage(e.to_string()))?;
        let mut specs = Vec::new();
        for row in rows {
            let data = row.map_err(|e| SpecError::Storage(e.to_string()))?;
            let spec: Spec =
                serde_json::from_str(&data).map_err(|e| SpecError::Storage(e.to_string()))?;
            specs.push(spec);
        }
        Ok(specs)
    }
}
