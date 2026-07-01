use super::WalletStore;
use crate::Store;
use crate::collect_rows_strict;
use hkask_types::time::now_rfc3339;
use hkask_types::{ApiKeyId, Ed25519PublicKey, InfrastructureError, WalletId};
use hkask_wallet_types::{ApiKeyCapability, PrivacyMode, RJoule, RateLimitConfig, WalletError};
use std::str::FromStr;

// ── Row type for query mapping ─────────────────────────────────────────────────

#[allow(dead_code)] // fields populated by rusqlite query mapping
struct ApiKeyRow {
    key_id: String,
    privacy_mode: String,
    preferred_chain: Option<String>,
    wallet_id: String,
    public_key: Vec<u8>,
    spending_limit_rj: i64,
    spent_rj: i64,
    scope: String,
    purpose: String,
    rate_limit_json: Option<String>,
    expires_at: Option<String>,
    issued_at: String,
}

// ── API Key methods ────────────────────────────────────────────────────────────

impl WalletStore {
    /// Store a newly issued API key capability.
    /// Store an API key capability.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — store API key capability
    /// pre:  capability has valid key_id and wallet_id
    /// post: API key stored
    pub fn store_api_key(&self, capability: &ApiKeyCapability) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        let scope_json =
            serde_json::to_string(&capability.scope).unwrap_or_else(|_| "[]".to_string());
        let rate_limit_json = capability
            .rate_limit
            .as_ref()
            .and_then(|rl| serde_json::to_string(rl).ok());
        conn.execute(
            "INSERT INTO api_keys (key_id, wallet_id, public_key, spending_limit_rj, spent_rj, scope, purpose, rate_limit_json, privacy_mode, preferred_chain, expires_at, issued_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                capability.key_id.to_string(),
                capability.wallet_id.to_string(),
                capability.public_key.as_bytes(),
                capability.spending_limit_rj.as_u64() as i64,
                capability.spent_rj.as_u64() as i64,
                scope_json,
                capability.purpose,
                rate_limit_json,
                capability.privacy_mode.to_string(),
                capability.preferred_chain.map(|c| c.to_string()),
                capability.expiry.map(|e| e.to_rfc3339()),
                capability.issued_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Look up an API key by its ID.
    /// Get an API key by key ID.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — get API key by ID
    /// pre:  key_id is valid
    /// post: returns Some(capability) if found, None otherwise
    #[must_use = "result must be used"]
    pub fn get_api_key(&self, key_id: ApiKeyId) -> Result<Option<ApiKeyCapability>, WalletError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT key_id, wallet_id, public_key, spending_limit_rj, spent_rj, scope, purpose, rate_limit_json, privacy_mode, preferred_chain, expires_at, issued_at, revoked_at FROM api_keys WHERE key_id = ?1",
        )?;
        let rows: Vec<ApiKeyCapability> = collect_rows_strict!(
            stmt,
            rusqlite::params![key_id.to_string()],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<ApiKeyRow> {
                Ok(ApiKeyRow {
                    key_id: row.get(0)?,
                    wallet_id: row.get(1)?,
                    public_key: row.get(2)?,
                    spending_limit_rj: row.get(3)?,
                    spent_rj: row.get(4)?,
                    scope: row.get(5)?,
                    purpose: row.get(6)?,
                    rate_limit_json: row.get(7)?,
                    privacy_mode: row.get(8)?,
                    preferred_chain: row.get(9)?,
                    expires_at: row.get(10)?,
                    issued_at: row.get(11)?,
                })
            },
            |r: ApiKeyRow| -> Result<ApiKeyCapability, WalletError> {
                row_to_api_key_capability(r)
            }
        );
        Ok(rows.into_iter().next())
    }

    /// Look up an API key by its Ed25519 public key (for Bearer token auth).
    /// Get an API key by public key.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — get API key by public key
    /// pre:  public_key is valid
    /// post: returns Some(capability) if found, None otherwise
    pub fn get_api_key_by_public_key(
        &self,
        public_key: &[u8],
    ) -> Result<Option<ApiKeyCapability>, WalletError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT key_id, wallet_id, public_key, spending_limit_rj, spent_rj, scope, purpose, rate_limit_json, privacy_mode, preferred_chain, expires_at, issued_at, revoked_at FROM api_keys WHERE public_key = ?1 AND revoked_at IS NULL",
        )?;
        let rows: Vec<ApiKeyCapability> = collect_rows_strict!(
            stmt,
            rusqlite::params![public_key],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<ApiKeyRow> {
                Ok(ApiKeyRow {
                    key_id: row.get(0)?,
                    wallet_id: row.get(1)?,
                    public_key: row.get(2)?,
                    spending_limit_rj: row.get(3)?,
                    spent_rj: row.get(4)?,
                    scope: row.get(5)?,
                    purpose: row.get(6)?,
                    rate_limit_json: row.get(7)?,
                    privacy_mode: row.get(8)?,
                    preferred_chain: row.get(9)?,
                    expires_at: row.get(10)?,
                    issued_at: row.get(11)?,
                })
            },
            |r: ApiKeyRow| -> Result<ApiKeyCapability, WalletError> {
                row_to_api_key_capability(r)
            }
        );
        Ok(rows.into_iter().next())
    }

    /// List all active (non-revoked) API keys for a wallet.
    /// List API keys for a wallet.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — list API keys
    /// pre:  wallet_id is valid
    /// post: returns Vec of API key capabilities
    #[must_use = "result must be used"]
    pub fn list_api_keys(&self, wallet_id: WalletId) -> Result<Vec<ApiKeyCapability>, WalletError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(
            "SELECT key_id, wallet_id, public_key, spending_limit_rj, spent_rj, scope, purpose, rate_limit_json, privacy_mode, preferred_chain, expires_at, issued_at, revoked_at FROM api_keys WHERE wallet_id = ?1 AND revoked_at IS NULL ORDER BY issued_at DESC",
        )?;
        let rows: Vec<ApiKeyCapability> = collect_rows_strict!(
            stmt,
            rusqlite::params![wallet_id.to_string()],
            |row: &rusqlite::Row<'_>| -> rusqlite::Result<ApiKeyRow> {
                Ok(ApiKeyRow {
                    key_id: row.get(0)?,
                    wallet_id: row.get(1)?,
                    public_key: row.get(2)?,
                    spending_limit_rj: row.get(3)?,
                    spent_rj: row.get(4)?,
                    scope: row.get(5)?,
                    purpose: row.get(6)?,
                    rate_limit_json: row.get(7)?,
                    privacy_mode: row.get(8)?,
                    preferred_chain: row.get(9)?,
                    expires_at: row.get(10)?,
                    issued_at: row.get(11)?,
                })
            },
            |r: ApiKeyRow| -> Result<ApiKeyCapability, WalletError> {
                row_to_api_key_capability(r)
            }
        );
        Ok(rows)
    }

    /// Revoke an API key. Returns unspent rJoules to the wallet.
    /// Idempotent — revoking an already-revoked key is a no-op.
    /// Revoke an API key.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — revoke API key
    /// pre:  key_id is valid
    /// post: API key revoked, unspent rJ returned to wallet
    pub fn revoke_api_key(&self, key_id: ApiKeyId) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        let now = now_rfc3339();
        let rows = conn.execute(
            "UPDATE api_keys SET revoked_at = ?1 WHERE key_id = ?2 AND revoked_at IS NULL",
            rusqlite::params![now, key_id.to_string()],
        )?;
        if rows == 0 {
            return Ok(()); // already revoked or doesn't exist — no-op
        }
        // Return unspent rJoules to wallet
        let (wallet_id_str, spent, limit): (String, i64, i64) = conn.query_row(
            "SELECT wallet_id, spent_rj, spending_limit_rj FROM api_keys WHERE key_id = ?1",
            rusqlite::params![key_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let unspent = limit - spent;
        if unspent > 0 {
            conn.execute(
                "UPDATE wallet_balances SET balance_rj = balance_rj + ?1, updated_at = ?2 WHERE wallet_id = ?3",
                rusqlite::params![unspent, now, wallet_id_str],
            )?;
        }
        Ok(())
    }

    /// Update the spent_rj counter on an API key (called after each tool invocation).
    /// Update spent rJoules for an API key.
    ///
    /// expect: "The system provides durable storage for wallet data"
    /// \[P3\] Motivating: Generative Space — update spent rJ for key
    /// pre:  key_id is valid
    /// post: spent_rj updated
    pub fn update_spent_rj(&self, key_id: ApiKeyId, spent: RJoule) -> Result<(), WalletError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE api_keys SET spent_rj = ?1 WHERE key_id = ?2",
            rusqlite::params![spent.as_u64() as i64, key_id.to_string()],
        )?;
        Ok(())
    }
}

// ── Row conversion helper ──────────────────────────────────────────────────────

fn row_to_api_key_capability(r: ApiKeyRow) -> Result<ApiKeyCapability, WalletError> {
    let public_key_bytes: [u8; 32] = r.public_key.try_into().map_err(|_| {
        WalletError::Infra(InfrastructureError::Database(
            "public_key must be 32 bytes".into(),
        ))
    })?;
    let scope: Vec<String> = serde_json::from_str(&r.scope).unwrap_or_default();
    let rate_limit: Option<RateLimitConfig> = r
        .rate_limit_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok());
    Ok(ApiKeyCapability {
        wallet_id: WalletId::from_str(&r.wallet_id)?,
        key_id: ApiKeyId::from_str(&r.key_id)?,
        public_key: Ed25519PublicKey(public_key_bytes),
        spending_limit_rj: RJoule::new(r.spending_limit_rj as u64),
        spent_rj: RJoule::new(r.spent_rj as u64),
        scope,
        purpose: r.purpose,
        rate_limit,
        expiry: r.expires_at.map(|e| {
            chrono::DateTime::parse_from_rfc3339(&e)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now())
        }),
        issued_at: chrono::DateTime::parse_from_rfc3339(&r.issued_at)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now()),
        privacy_mode: PrivacyMode::Transparent,
        preferred_chain: None,
    })
}
