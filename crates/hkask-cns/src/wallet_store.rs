//! WalletStore — SQLite persistence for agent gas/rJoule wallets.

use hkask_types::WebID;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct WalletStore {
    conn: Arc<Mutex<Connection>>,
}

impl WalletStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Result<Self, String> {
        {
            let c = conn.lock().map_err(|e| format!("Lock error: {e}"))?;
            c.execute(
                "CREATE TABLE IF NOT EXISTS agent_wallets (
                    agent_webid TEXT PRIMARY KEY NOT NULL,
                    wallet_id INTEGER NOT NULL,
                    gas_balance INTEGER NOT NULL DEFAULT 0,
                    rjoule_balance INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL
                )",
                [],
            )
            .map_err(|e| format!("Failed to create agent_wallets table: {e}"))?;
        }
        Ok(Self { conn })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.conn.lock().map_err(|e| format!("Lock error: {e}"))
    }

    pub fn insert_wallet(
        &self,
        agent: &WebID,
        wallet_id: i64,
        gas_balance: i64,
        rjoule_balance: i64,
    ) -> Result<(), String> {
        let c = self.lock()?;
        let webid_str = format!("{agent}");
        c.execute(
            "INSERT INTO agent_wallets (agent_webid, wallet_id, gas_balance, rjoule_balance, created_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            rusqlite::params![webid_str, wallet_id, gas_balance, rjoule_balance],
        )
        .map_err(|e| format!("Failed to insert wallet: {e}"))?;
        Ok(())
    }

    pub fn update_gas_balance(&self, agent: &WebID, gas_balance: i64) -> Result<(), String> {
        let c = self.lock()?;
        let webid_str = format!("{agent}");
        c.execute(
            "UPDATE agent_wallets SET gas_balance = ?1 WHERE agent_webid = ?2",
            rusqlite::params![gas_balance, webid_str],
        )
        .map_err(|e| format!("Failed to update gas balance: {e}"))?;
        Ok(())
    }

    pub fn get_wallet(&self, agent: &WebID) -> Result<Option<WalletRow>, String> {
        let c = self.lock()?;
        let webid_str = format!("{agent}");
        let mut stmt = c
            .prepare(
                "SELECT agent_webid, wallet_id, gas_balance, rjoule_balance, created_at
                 FROM agent_wallets WHERE agent_webid = ?1",
            )
            .map_err(|e| format!("Failed to prepare query: {e}"))?;

        let mut rows = stmt
            .query_map(rusqlite::params![webid_str], |row| {
                Ok(WalletRow {
                    agent: WebID::from_persona(row.get::<_, String>(0)?.as_bytes()),
                    wallet_id: row.get(1)?,
                    gas_balance: row.get(2)?,
                    rjoule_balance: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("Failed to query wallet: {e}"))?;

        match rows.next() {
            Some(Ok(row)) => Ok(Some(row)),
            Some(Err(e)) => Err(format!("Row error: {e}")),
            None => Ok(None),
        }
    }

    pub fn has_wallet(&self, agent: &WebID) -> Result<bool, String> {
        let c = self.lock()?;
        let webid_str = format!("{agent}");
        let count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM agent_wallets WHERE agent_webid = ?1",
                rusqlite::params![webid_str],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to check wallet: {e}"))?;
        Ok(count > 0)
    }

    pub fn next_wallet_id(&self) -> Result<i64, String> {
        let c = self.lock()?;
        let max_id: Option<i64> = c
            .query_row("SELECT MAX(wallet_id) FROM agent_wallets", [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("Failed to get max wallet id: {e}"))?;
        Ok(max_id.unwrap_or(0) + 1)
    }
}

#[derive(Debug, Clone)]
pub struct WalletRow {
    pub agent: WebID,
    pub wallet_id: i64,
    pub gas_balance: i64,
    pub rjoule_balance: i64,
    pub created_at: String,
}
