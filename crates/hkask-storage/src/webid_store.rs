//! WebID Capability Store — SQLite persistence layer

use hkask_ensemble::{OkapiCapability, OkapiOperation, macaroon::Macaroon};
use hkask_types::{TemplateID, WebID};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub enum WebIDStoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Bincode serialization error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

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
        let macaroon_bytes = bincode::serialize(macaroon).map_err(WebIDStoreError::Bincode)?;
        let operations = cap.granted_operations();

        Ok(Self {
            macaroon_hex: hex::encode(macaroon_bytes),
            operations,
            issuer: cap.issuer().to_string(),
            subject: cap.subject().to_string(),
            expires_at: cap.expires_at().map(|dt| dt.timestamp()),
            template_id: cap.template_id().map(|t| t.to_string()),
            visibility: "private".to_string(),
        })
    }

    pub fn to_capability(&self) -> Result<OkapiCapability, WebIDStoreError> {
        let macaroon_bytes = hex::decode(&self.macaroon_hex)?;
        let macaroon: Macaroon =
            bincode::deserialize(&macaroon_bytes).map_err(WebIDStoreError::Bincode)?;

        let operations = self
            .operations
            .iter()
            .filter_map(|op| OkapiOperation::from_str(op.as_str()).ok())
            .collect();

        let issuer = WebID::from_string(&self.issuer);
        let holder = WebID::from_string(&self.subject);
        let template_id = self
            .template_id
            .as_ref()
            .map(|s| TemplateID::from_string(s.as_str()));

        Ok(OkapiCapability::from_macaroon(
            macaroon,
            operations,
            issuer,
            holder,
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

                let capabilities: Vec<StoredCapability> = serde_json::from_str(&capabilities_json)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

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
        self.conn.execute(
            "DELETE FROM webid_capabilities WHERE webid = ?1",
            params![webid],
        )?;
        debug!("Deleted capabilities for WebID: {}", webid);
        Ok(())
    }

    pub fn list_active(&self) -> Result<Vec<StoredWebIDEntry>, WebIDStoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT webid, capabilities_json, created_at, last_used_at, active
             FROM webid_capabilities WHERE active = 1 ORDER BY created_at DESC",
        )?;

        let entries = stmt
            .query_map([], |row| {
                let webid: String = row.get(0)?;
                let capabilities_json: String = row.get(1)?;
                let created_at: i64 = row.get(2)?;
                let last_used_at: Option<i64> = row.get(3)?;
                let active: i32 = row.get(4)?;

                let capabilities: Vec<StoredCapability> = serde_json::from_str(&capabilities_json)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

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
        let total: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM webid_capabilities", [], |row| {
                    row.get(0)
                })?;
        let active: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM webid_capabilities WHERE active = 1",
            [],
            |row| row.get(0),
        )?;

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
