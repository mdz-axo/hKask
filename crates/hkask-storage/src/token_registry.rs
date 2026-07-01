//! Token Registry — SQLite persistence for DelegationToken lifecycle
//!
//! Persists Ed25519-signed OCAP delegation tokens so the platform can
//! audit consent (P2 — Affirmative Consent). OCAP gates enforce consent
//! at runtime; this store proves it after the fact.
//!
//! CNS spans record token *usage*; this store records token *issuance*.
//! Together they enable the full consent audit picture.

use crate::Store;
use hkask_capability::{
    DelegationAction, DelegationResource, DelegationToken, TokenRegistry, TokenRegistryError,
};
use hkask_types::WebID;
use rusqlite::params;

define_store!(TokenRegistryStore);

impl TokenRegistryStore {
    /// Initialize the delegation_tokens table.
    ///
    /// expect: "Token issuance is persisted for consent audit"
    /// [P2] Motivating: Affirmative Consent — audit trail for delegation tokens
    /// post: delegation_tokens table created if not exists
    pub fn initialize_schema(&self) -> Result<(), hkask_types::InfrastructureError> {
        let conn = self.lock_conn()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS delegation_tokens (
                id TEXT PRIMARY KEY,
                resource TEXT NOT NULL,
                resource_id TEXT NOT NULL,
                action TEXT NOT NULL,
                delegated_from TEXT NOT NULL,
                delegated_to TEXT NOT NULL,
                signature_hex TEXT NOT NULL,
                public_key_hex TEXT NOT NULL,
                expires_at INTEGER,
                attenuation_level INTEGER NOT NULL DEFAULT 0,
                max_attenuation INTEGER NOT NULL DEFAULT 7,
                context_nonce TEXT NOT NULL DEFAULT '',
                caveats_json TEXT NOT NULL DEFAULT '[]',
                revoked INTEGER NOT NULL DEFAULT 0,
                issued_at INTEGER NOT NULL DEFAULT (unixepoch())
            );
            CREATE INDEX IF NOT EXISTS idx_tokens_issuer ON delegation_tokens(delegated_from, issued_at);
            CREATE INDEX IF NOT EXISTS idx_tokens_recipient ON delegation_tokens(delegated_to, issued_at);
            CREATE INDEX IF NOT EXISTS idx_tokens_revoked ON delegation_tokens(revoked);
            ",
        )?;
        Ok(())
    }

    fn row_to_token(row: &rusqlite::Row) -> rusqlite::Result<DelegationToken> {
        let id: String = row.get("id")?;
        let resource_str: String = row.get("resource")?;
        let resource_id: String = row.get("resource_id")?;
        let action_str: String = row.get("action")?;
        let from_str: String = row.get("delegated_from")?;
        let to_str: String = row.get("delegated_to")?;
        let sig_hex: String = row.get("signature_hex")?;
        let pk_hex: String = row.get("public_key_hex")?;
        let expires_at: Option<i64> = row.get("expires_at")?;
        let attenuation_level: u8 = row.get("attenuation_level")?;
        let max_attenuation: u8 = row.get("max_attenuation")?;
        let context_nonce: String = row.get("context_nonce")?;
        let caveats_json: String = row.get("caveats_json")?;

        let resource = match resource_str.as_str() {
            "Registry" => DelegationResource::Registry,
            "Template" => DelegationResource::Template,
            "Memory" => DelegationResource::Memory,
            "Inference" => DelegationResource::Inference,
            "Tool" => DelegationResource::Tool,
            "Wallet" => DelegationResource::Wallet,
            other => DelegationResource::Other(other.to_string()),
        };
        let action = match action_str.as_str() {
            "Execute" => DelegationAction::Execute,
            "Read" => DelegationAction::Read,
            "Write" => DelegationAction::Write,
            "Delegate" => DelegationAction::Delegate,
            other => DelegationAction::Other(other.to_string()),
        };

        let signature = {
            let bytes = hex::decode(&sig_hex).unwrap_or(vec![0u8; 64]);
            let mut arr = [0u8; 64];
            arr.copy_from_slice(&bytes[..64.min(bytes.len())]);
            hkask_capability::token_types::TokenSignature(arr)
        };
        let public_key = {
            let bytes = hex::decode(&pk_hex).unwrap_or(vec![0u8; 32]);
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes[..32.min(bytes.len())]);
            hkask_types::Ed25519PublicKey(arr)
        };

        let from_wid = WebID::from_string(&from_str);
        let to_wid = WebID::from_string(&to_str);

        Ok(DelegationToken {
            id,
            resource,
            resource_id,
            action,
            delegated_from: from_wid,
            delegated_to: to_wid,
            signature,
            public_key,
            expires_at,
            attenuation_level,
            max_attenuation,
            context_nonce,
            caveats: vec![], // caveats are internal; not serialized to SQL
        })
    }
}

impl TokenRegistry for TokenRegistryStore {
    fn store(&self, token: &DelegationToken) -> Result<(), TokenRegistryError> {
        let conn = self
            .lock_conn()
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        let sig_hex = hex::encode(token.signature.0);
        let pk_hex = hex::encode(token.public_key.0);
        let resource_str = match &token.resource {
            DelegationResource::Registry => "Registry",
            DelegationResource::Template => "Template",
            DelegationResource::Memory => "Memory",
            DelegationResource::Inference => "Inference",
            DelegationResource::Tool => "Tool",
            DelegationResource::Wallet => "Wallet",
            DelegationResource::Other(s) => s.as_str(),
        };
        let action_str = match &token.action {
            DelegationAction::Execute => "Execute",
            DelegationAction::Read => "Read",
            DelegationAction::Write => "Write",
            DelegationAction::Delegate => "Delegate",
            DelegationAction::Other(s) => s.as_str(),
        };

        conn.execute(
            "INSERT INTO delegation_tokens
             (id, resource, resource_id, action, delegated_from, delegated_to,
              signature_hex, public_key_hex, expires_at, attenuation_level,
              max_attenuation, context_nonce, issued_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, unixepoch())",
            params![
                token.id,
                resource_str,
                token.resource_id,
                action_str,
                token.delegated_from.to_string(),
                token.delegated_to.to_string(),
                sig_hex,
                pk_hex,
                token.expires_at,
                token.attenuation_level,
                token.max_attenuation,
                token.context_nonce,
            ],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint") {
                TokenRegistryError::Duplicate(token.id.clone())
            } else {
                TokenRegistryError::Storage(e.to_string())
            }
        })?;
        Ok(())
    }

    fn get(&self, token_id: &str) -> Result<Option<DelegationToken>, TokenRegistryError> {
        let conn = self
            .lock_conn()
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT * FROM delegation_tokens WHERE id = ?1")
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        let result = stmt
            .query_row(params![token_id], Self::row_to_token)
            .optional()
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        Ok(result)
    }

    fn query_by_issuer(
        &self,
        webid: &WebID,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<DelegationToken>, TokenRegistryError> {
        let conn = self
            .lock_conn()
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        let since_ts = since.timestamp();
        let mut stmt = conn
            .prepare(
                "SELECT * FROM delegation_tokens WHERE delegated_from = ?1 AND issued_at >= ?2 AND revoked = 0",
            )
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![webid.to_string(), since_ts], Self::row_to_token)
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        let mut tokens = Vec::new();
        for row in rows {
            tokens.push(row.map_err(|e| TokenRegistryError::Storage(e.to_string()))?);
        }
        Ok(tokens)
    }

    fn query_by_recipient(
        &self,
        webid: &WebID,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<DelegationToken>, TokenRegistryError> {
        let conn = self
            .lock_conn()
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        let since_ts = since.timestamp();
        let mut stmt = conn
            .prepare(
                "SELECT * FROM delegation_tokens WHERE delegated_to = ?1 AND issued_at >= ?2 AND revoked = 0",
            )
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![webid.to_string(), since_ts], Self::row_to_token)
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        let mut tokens = Vec::new();
        for row in rows {
            tokens.push(row.map_err(|e| TokenRegistryError::Storage(e.to_string()))?);
        }
        Ok(tokens)
    }

    fn query_all(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<DelegationToken>, TokenRegistryError> {
        let conn = self
            .lock_conn()
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        let since_ts = since.timestamp();
        let mut stmt = conn
            .prepare("SELECT * FROM delegation_tokens WHERE issued_at >= ?1 AND revoked = 0")
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map(params![since_ts], Self::row_to_token)
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        let mut tokens = Vec::new();
        for row in rows {
            tokens.push(row.map_err(|e| TokenRegistryError::Storage(e.to_string()))?);
        }
        Ok(tokens)
    }

    fn revoke(&self, token_id: &str) -> Result<(), TokenRegistryError> {
        let conn = self
            .lock_conn()
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        let affected = conn
            .execute(
                "UPDATE delegation_tokens SET revoked = 1 WHERE id = ?1",
                params![token_id],
            )
            .map_err(|e| TokenRegistryError::Storage(e.to_string()))?;
        if affected == 0 {
            return Err(TokenRegistryError::NotFound(token_id.to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use hkask_capability::DelegationTokenBuilder;
    use hkask_types::WebID;
    use rand::rngs::OsRng;

    fn test_token(from: WebID, to: WebID) -> DelegationToken {
        let signing_key = SigningKey::generate(&mut OsRng);
        DelegationTokenBuilder::new(
            DelegationResource::Registry,
            "test-skill".into(),
            DelegationAction::Execute,
            from,
            to,
            &signing_key,
        )
        .sign()
    }

    fn test_db() -> TokenRegistryStore {
        let db = crate::database::in_memory_db().expect("in-memory db");
        let store = TokenRegistryStore::new(db);
        store.initialize_schema().expect("schema init");
        store
    }

    #[test]
    fn store_and_retrieve_token() {
        let store = test_db();
        let from = WebID::from_persona(b"issuer");
        let to = WebID::from_persona(b"recipient");
        let token = test_token(from, to);
        let token_id = token.id.clone();

        store.store(&token).unwrap();
        let retrieved = store.get(&token_id).unwrap().expect("token should exist");

        assert_eq!(retrieved.id, token.id);
        assert_eq!(retrieved.delegated_from.to_string(), from.to_string());
        assert_eq!(retrieved.delegated_to.to_string(), to.to_string());
    }

    #[test]
    fn query_by_issuer_filters_correctly() {
        let store = test_db();
        let alice = WebID::from_persona(b"alice");
        let bob = WebID::from_persona(b"bob");
        let carol = WebID::from_persona(b"carol");

        store.store(&test_token(alice, bob)).unwrap();
        store.store(&test_token(alice, carol)).unwrap();
        store.store(&test_token(bob, carol)).unwrap();

        let alice_tokens = store
            .query_by_issuer(&alice, chrono::Utc::now() - chrono::Duration::hours(1))
            .unwrap();
        assert_eq!(alice_tokens.len(), 2);

        let bob_tokens = store
            .query_by_issuer(&bob, chrono::Utc::now() - chrono::Duration::hours(1))
            .unwrap();
        assert_eq!(bob_tokens.len(), 1);
    }

    #[test]
    fn query_by_recipient_filters_correctly() {
        let store = test_db();
        let alice = WebID::from_persona(b"alice");
        let bob = WebID::from_persona(b"bob");
        let carol = WebID::from_persona(b"carol");

        store.store(&test_token(alice, bob)).unwrap();
        store.store(&test_token(carol, bob)).unwrap();

        let bob_tokens = store
            .query_by_recipient(&bob, chrono::Utc::now() - chrono::Duration::hours(1))
            .unwrap();
        assert_eq!(bob_tokens.len(), 2);
    }

    #[test]
    fn revoke_removes_from_queries() {
        let store = test_db();
        let alice = WebID::from_persona(b"alice");
        let bob = WebID::from_persona(b"bob");
        let token = test_token(alice, bob);
        let token_id = token.id.clone();

        store.store(&token).unwrap();
        store.revoke(&token_id).unwrap();

        // Revoked token should not appear in queries
        let results = store
            .query_all(chrono::Utc::now() - chrono::Duration::hours(1))
            .unwrap();
        assert!(results.is_empty());

        // Direct get should still work (for audit trail)
        let retrieved = store.get(&token_id).unwrap();
        assert!(retrieved.is_some());
    }

    #[test]
    fn duplicate_store_returns_error() {
        let store = test_db();
        let alice = WebID::from_persona(b"alice");
        let bob = WebID::from_persona(b"bob");
        let token = test_token(alice, bob);

        store.store(&token).unwrap();
        let result = store.store(&token);
        assert!(result.is_err());
    }

    #[test]
    fn revoke_nonexistent_returns_not_found() {
        let store = test_db();
        let result = store.revoke("nonexistent");
        assert!(matches!(result, Err(TokenRegistryError::NotFound(_))));
    }
}
