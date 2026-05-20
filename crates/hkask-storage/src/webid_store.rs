//! WebID Capability Store — SQLite persistence layer

use hkask_ensemble::{OkapiCapability, OkapiOperation, WebID, macaroon::Macaroon};
use hkask_types::{TemplateID, Visibility};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum WebIDStoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("WebID not found: {0}")]
    WebIDNotFound(String),

    #[error("Hex decode error: {0}")]
    HexDecode(#[from] hex::FromHexError),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredWebIDEntry {
    pub webid: String,
    pub capabilities: Vec<StoredCapability>,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub active: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredCapability {
    pub macaroon_hex: String,
    pub operations: Vec<String>,
    pub issuer: String,
    pub subject: String,
    pub expires_at: Option<i64>,
    pub template_id: Option<String>,
    pub visibility: String,
}

impl StoredCapability {
    pub fn from_capability(cap: &OkapiCapability) -> Result<Self, WebIDStoreError> {
        let macaroon = cap.macaroon();
        let macaroon_bytes = bincode::serialize(macaroon).map_err(|e| WebIDStoreError::Serialization(e))?;
        let operations = cap.macaroon.caveats
            .iter()
            .filter(|c| c.caveat_id == "operation")
            .map(|c| c.data.clone())
            .collect();

        Ok(Self {
            macaroon_hex: hex::encode(macaroon_bytes),
            operations,
            issuer: cap.issuer().to_string(),
            subject: cap.subject().to_string(),
            expires_at: cap.expires_at().map(|dt| dt.timestamp()),
            template_id: cap.template_id().map(|t| t.to_string()),
            visibility: cap.visibility().as_str().to_string(),
        })
    }

    pub fn to_capability(&self) -> Result<OkapiCapability, WebIDStoreError> {
        let macaroon_bytes = hex::decode(&self.macaroon_hex)?;
        let macaroon: Macaroon = bincode::deserialize(&macaroon_bytes)
            .map_err(|e| WebIDStoreError::Serialization(e))?;

        let operations = self
            .operations
            .iter()
            .filter_map(|op| OkapiOperation::from_str(op.as_str()).ok())
            .collect();

        let issuer = WebID::from_string(&self.issuer);
        let subject = WebID::from_string(&self.subject);
        let template_id = self
            .template_id
            .as_ref()
            .map(|s| TemplateID::from_string(s.as_str()));

        Ok(OkapiCapability::from_macaroon(
            macaroon,
            operations,
            issuer,
            subject,
            self.expires_at,
            template_id,
            &self.visibility,
        ))
    }
}

pub struct WebIDStore {
    conn: Connection,
}

impl WebIDStore {
    pub fn new(path: &Path) -> Result<Self, WebIDStoreError> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.initialize_schema()?;
        Ok(store)
    }

    pub fn in_memory() -> Result<Self, WebIDStoreError> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.initialize_schema()?;
        Ok(store)
    }

    fn initialize_schema(&self) -> Result<(), WebIDStoreError> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS webid_capabilities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                webid TEXT NOT NULL UNIQUE,
                capabilities_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                last_used_at INTEGER,
                active INTEGER NOT NULL DEFAULT 1,
                updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_webid_active ON webid_capabilities(webid, active);
            CREATE INDEX IF NOT EXISTS idx_last_used ON webid_capabilities(last_used_at);
            CREATE INDEX IF NOT EXISTS idx_updated ON webid_capabilities(updated_at);
            ",
        )?;
        Ok(())
    }

    pub fn store(&self, entry: &StoredWebIDEntry) -> Result<(), WebIDStoreError> {
        let capabilities_json = serde_json::to_string(&entry.capabilities)?;
        let now = chrono::Utc::now().timestamp();

        self.conn.execute(
            "INSERT INTO webid_capabilities (webid, capabilities_json, created_at, last_used_at, active, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(webid) DO UPDATE SET
                capabilities_json = excluded.capabilities_json,
                last_used_at = excluded.last_used_at,
                active = excluded.active,
                updated_at = excluded.updated_at",
            params![
                entry.webid,
                capabilities_json,
                entry.created_at,
                entry.last_used_at,
                if entry.active { 1 } else { 0 },
                now
            ],
        )?;
        debug!("Stored capabilities for WebID: {}", entry.webid);
        Ok(())
    }

    pub fn get(&self, webid: &str) -> Result<Option<StoredWebIDEntry>, WebIDStoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT webid, capabilities_json, created_at, last_used_at, active FROM webid_capabilities WHERE webid = ?1"
        )?;

        let entry = stmt
            .query_row(params![webid], |row| {
                let webid: String = row.get(0)?;
                let capabilities_json: String = row.get(1)?;
                let created_at: i64 = row.get(2)?;
                let last_used_at: Option<i64> = row.get(3)?;
                let active: i32 = row.get(4)?;

                let capabilities: Vec<StoredCapability> = serde_json::from_str(&capabilities_json)?;

                Ok(StoredWebIDEntry {
                    webid,
                    capabilities,
                    created_at,
                    last_used_at,
                    active: active != 0,
                })
            })
            .optional()?;

        Ok(entry)
    }

    pub fn delete(&self, webid: &str) -> Result<(), WebIDStoreError> {
        self.conn.execute("DELETE FROM webid_capabilities WHERE webid = ?1", params![webid])?;
        debug!("Deleted capabilities for WebID: {}", webid);
        Ok(())
    }

    pub fn list_active(&self) -> Result<Vec<StoredWebIDEntry>, WebIDStoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT webid, capabilities_json, created_at, last_used_at, active
             FROM webid_capabilities WHERE active = 1 ORDER BY created_at DESC"
        )?;

        let entries = stmt
            .query_map([], |row| {
                let webid: String = row.get(0)?;
                let capabilities_json: String = row.get(1)?;
                let created_at: i64 = row.get(2)?;
                let last_used_at: Option<i64> = row.get(3)?;
                let active: i32 = row.get(4)?;

                let capabilities: Vec<StoredCapability> = serde_json::from_str(&capabilities_json)?;

                Ok(StoredWebIDEntry {
                    webid,
                    capabilities,
                    created_at,
                    last_used_at,
                    active: active != 0,
                })
            })?
            .filter_map(|result| result.ok())
            .collect();

        Ok(entries)
    }

    pub fn mark_used(&self, webid: &str) -> Result<(), WebIDStoreError> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "UPDATE webid_capabilities SET last_used_at = ?1, updated_at = ?2 WHERE webid = ?3",
            params![now, now, webid],
        )?;
        Ok(())
    }

    pub fn stats(&self) -> Result<WebIDStoreStats, WebIDStoreError> {
        let total: i64 = self.conn.query_row("SELECT COUNT(*) FROM webid_capabilities", [], |row| row.get(0))?;
        let active: i64 = self.conn.query_row("SELECT COUNT(*) FROM webid_capabilities WHERE active = 1", [], |row| row.get(0))?;

        Ok(WebIDStoreStats {
            total_entries: total as usize,
            active_entries: active as usize,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WebIDStoreStats {
    pub total_entries: usize,
    pub active_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use hkask_ensemble::OkapiCapability;

    fn test_key() -> [u8; 32] {
        [0x42; 32]
    }

    #[test]
    fn test_store_and_retrieve() {
        let store = WebIDStore::in_memory().unwrap();
        let webid = WebID::new();
        let key = test_key();

        let capability = OkapiCapability::new(
            vec![OkapiOperation::Generate, OkapiOperation::Chat],
            WebID::new(),
            webid,
            Duration::days(30),
            &key,
        );

        let stored_cap = StoredCapability::from_capability(&capability).unwrap();
        let entry = StoredWebIDEntry {
            webid: webid.to_string(),
            capabilities: vec![stored_cap],
            created_at: chrono::Utc::now().timestamp(),
            last_used_at: None,
            active: true,
        };

        store.store(&entry).unwrap();
        let retrieved = store.get(&webid.to_string()).unwrap().unwrap();
        assert_eq!(retrieved.webid, entry.webid);
        assert_eq!(retrieved.capabilities.len(), 1);
        assert!(retrieved.active);
    }

    #[test]
    fn test_delete() {
        let store = WebIDStore::in_memory().unwrap();
        let webid = WebID::new();

        let entry = StoredWebIDEntry {
            webid: webid.to_string(),
            capabilities: vec![],
            created_at: chrono::Utc::now().timestamp(),
            last_used_at: None,
            active: true,
        };

        store.store(&entry).unwrap();
        assert!(store.get(&webid.to_string()).unwrap().is_some());

        store.delete(&webid.to_string()).unwrap();
        assert!(store.get(&webid.to_string()).unwrap().is_none());
    }

    #[test]
    fn test_list_active() {
        let store = WebIDStore::in_memory().unwrap();

        for i in 0..3 {
            let webid = WebID::new();
            let entry = StoredWebIDEntry {
                webid: webid.to_string(),
                capabilities: vec![],
                created_at: chrono::Utc::now().timestamp(),
                last_used_at: None,
                active: true,
            };
            store.store(&entry).unwrap();
        }

        let active = store.list_active().unwrap();
        assert_eq!(active.len(), 3);
    }

    #[test]
    fn test_mark_used() {
        let store = WebIDStore::in_memory().unwrap();
        let webid = WebID::new();

        let entry = StoredWebIDEntry {
            webid: webid.to_string(),
            capabilities: vec![],
            created_at: chrono::Utc::now().timestamp(),
            last_used_at: None,
            active: true,
        };

        store.store(&entry).unwrap();
        let before = store.get(&webid.to_string()).unwrap().unwrap();
        assert!(before.last_used_at.is_none());

        store.mark_used(&webid.to_string()).unwrap();
        let after = store.get(&webid.to_string()).unwrap().unwrap();
        assert!(after.last_used_at.is_some());
    }
}
