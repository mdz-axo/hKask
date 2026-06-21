//! # hKask Ledger — Double-Entry Accounting
//!
//! Immutable double-entry ledger backed by SQLite. Three domain ledgers (cost,
//! crypto, securities) share this crate with separate database files.
//!
//! ## Invariants
//!
//! 1. **Idempotency** — same `reference` committed twice is a no-op.
//! 2. **Double-entry** — every transaction's postings must sum to 0.
//! 3. **Immutability** — committed transactions are never modified or deleted.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors the ledger can produce.
#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("account '{0}' already exists in namespace '{1}'")]
    AccountAlreadyExists(String, String),
    #[error("double-entry violation: postings sum to {0}, must sum to 0")]
    DoubleEntryViolation(i64),
}

/// A named account in the ledger. Account IDs use colon-separated paths
/// like `cost:api/deepinfra` or `wallet:hedera/main`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub namespace: String,
    pub created_at: String,
}

/// A single entry in a transaction — moves `amount` of `asset` from
/// `source` account to `destination` account. Amount is in the asset's
/// smallest integer unit (µrJ for rJ, µUSD for USD, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Posting {
    pub source: String,
    pub destination: String,
    pub asset: String,
    pub amount: i64,
}

/// An immutable transaction containing one or more postings. The `reference`
/// field provides idempotency — committing the same reference twice is a no-op.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerTransaction {
    pub id: String,
    pub timestamp: String,
    pub reference: String,
    pub postings: Vec<Posting>,
    pub metadata: serde_json::Value,
}

/// A computed balance for an account + asset pair.
#[derive(Debug, Clone, Serialize)]
pub struct AccountBalance {
    pub account: String,
    pub asset: String,
    pub balance: i64,
}

/// A time range for querying transactions.
#[derive(Debug, Clone)]
pub struct DateRange {
    pub start: String, // ISO 8601
    pub end: String,   // ISO 8601
}

/// Filters for transaction queries.
#[derive(Debug, Clone, Default)]
pub struct QueryFilter {
    pub asset: Option<String>,
    pub account: Option<String>,
    pub namespace: Option<String>,
}

/// The double-entry ledger.
pub struct Ledger {
    db: Connection,
}

impl Ledger {
    /// REQ: P8-ledger-open
    /// expect: "I can open a ledger database and it creates the schema if needed" [P8]
    /// pre:  path is a valid filesystem path
    /// post: returns Ledger with accounts, transactions, postings tables created
    /// inv:  idempotent — opening the same path twice creates the same tables
    /// [P8] Constraining: Persistence — data survives process restarts
    pub fn open(path: &std::path::Path) -> Result<Self, LedgerError> {
        let db = Connection::open(path)?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                namespace TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS transactions (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                reference TEXT UNIQUE NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS postings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                transaction_id TEXT NOT NULL REFERENCES transactions(id),
                source TEXT NOT NULL,
                destination TEXT NOT NULL,
                asset TEXT NOT NULL,
                amount INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_postings_destination_asset
                ON postings(destination, asset);
            CREATE INDEX IF NOT EXISTS idx_postings_source_asset
                ON postings(source, asset);
            CREATE INDEX IF NOT EXISTS idx_transactions_reference
                ON transactions(reference);",
        )?;
        Ok(Self { db })
    }

    /// REQ: P8-ledger-ensure-account
    /// expect: "I can create a named account and doing it twice is harmless" [P8]
    /// pre:  id and namespace are non-empty strings
    /// post: account exists in the database; second call with same id is a no-op
    /// inv:  idempotent — calling ensure_account twice with same id does not error
    /// [P8] Constraining: Persistence — account survives restarts
    pub fn ensure_account(&self, id: &str, namespace: &str) -> Result<(), LedgerError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.db.execute(
            "INSERT OR IGNORE INTO accounts (id, namespace, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, namespace, now],
        )?;
        Ok(())
    }

    /// REQ: P8-ledger-commit
    /// expect: "I can commit a transaction and the postings are stored immutably" [P8]
    /// pre:  tx.id is unique, tx.reference is unique, tx.postings is non-empty
    /// post: transaction and all postings are stored; balances reflect new postings
    /// inv:  idempotent by reference; empty postings rejected
    /// [P4] Constraining: Clear Boundaries — committed transactions cannot be modified
    /// [P8] Constraining: Persistence — committed data survives restarts
    pub fn commit(&self, tx: &LedgerTransaction) -> Result<(), LedgerError> {
        // Double-entry invariant is structurally guaranteed:
        // each posting debits source and credits destination by the same amount.
        // Require at least one posting.
        if tx.postings.is_empty() {
            return Err(LedgerError::DoubleEntryViolation(0));
        }

        let now = chrono::Utc::now().to_rfc3339();

        // Insert transaction row (idempotent by reference)
        self.db.execute(
            "INSERT OR IGNORE INTO transactions (id, timestamp, reference, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                tx.id,
                tx.timestamp,
                tx.reference,
                tx.metadata.to_string(),
                now
            ],
        )?;

        // Insert postings only if the transaction was new (check rows_changed)
        if self.db.changes() > 0 {
            for posting in &tx.postings {
                self.db.execute(
                    "INSERT INTO postings (transaction_id, source, destination, asset, amount, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        tx.id,
                        posting.source,
                        posting.destination,
                        posting.asset,
                        posting.amount,
                        now,
                    ],
                )?;
            }
        }

        Ok(())
    }

    /// REQ: P9-ledger-balance
    /// expect: "I can query the balance of any account and see all credits minus debits" [P9]
    /// pre:  account is a valid account ID (may or may not exist)
    /// post: returns sum(destination amounts) - sum(source amounts) for matching asset
    /// inv:  read-only — does not modify the ledger; non-existent account returns 0
    /// [P9] Constraining: Observability — balances are visible to the user
    pub fn balance(&self, account: &str, asset: Option<&str>) -> Result<i64, LedgerError> {
        let (query, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(a) = asset {
            (
                "SELECT
                        COALESCE(SUM(CASE WHEN destination = ?1 THEN amount ELSE 0 END), 0)
                      - COALESCE(SUM(CASE WHEN source = ?1 THEN amount ELSE 0 END), 0)
                     FROM postings WHERE (source = ?1 OR destination = ?1) AND asset = ?2",
                vec![Box::new(account.to_string()), Box::new(a.to_string())],
            )
        } else {
            (
                "SELECT
                        COALESCE(SUM(CASE WHEN destination = ?1 THEN amount ELSE 0 END), 0)
                      - COALESCE(SUM(CASE WHEN source = ?1 THEN amount ELSE 0 END), 0)
                     FROM postings WHERE source = ?1 OR destination = ?1",
                vec![Box::new(account.to_string())],
            )
        };
        let balance: i64 = self.db.query_row(
            query,
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| row.get(0),
        )?;
        Ok(balance)
    }

    /// REQ: P9-ledger-namespace-balances
    /// expect: "I can see all balances in a domain (cost, wallet, portfolio) at once" [P9]
    /// pre:  namespace is a valid namespace string
    /// post: returns all (account, asset, balance) triples for accounts in the namespace
    /// inv:  read-only; returns empty vec for unknown namespace
    /// [P9] Constraining: Observability — all domain balances are visible at once
    pub fn namespace_balances(&self, namespace: &str) -> Result<Vec<AccountBalance>, LedgerError> {
        let mut stmt = self.db.prepare(
            "SELECT a.id,
                    COALESCE(p.asset, '') AS asset,
                    COALESCE(SUM(CASE WHEN p.destination = a.id THEN p.amount ELSE 0 END), 0)
                  - COALESCE(SUM(CASE WHEN p.source = a.id THEN p.amount ELSE 0 END), 0) AS balance
             FROM accounts a
             LEFT JOIN postings p ON (p.source = a.id OR p.destination = a.id)
             WHERE a.namespace = ?1
             GROUP BY a.id, p.asset
             ORDER BY a.id, p.asset",
        )?;
        let rows = stmt.query_map([namespace], |row| {
            Ok(AccountBalance {
                account: row.get(0)?,
                asset: row.get(1)?,
                balance: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
            })
        })?;
        let mut balances = Vec::new();
        for row in rows {
            balances.push(row?);
        }
        Ok(balances)
    }

    /// Count unique transactions with a posting to the given destination account.
    pub fn transaction_count(&self, destination: &str) -> Result<u64, LedgerError> {
        let count: i64 = self.db.query_row(
            "SELECT COUNT(DISTINCT transaction_id) FROM postings WHERE destination = ?1",
            rusqlite::params![destination],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }

    /// REQ: P9-ledger-query
    /// expect: "I can query transactions by time range and filter by account or asset" [P9]
    /// pre:  range.start <= range.end (ISO 8601 strings)
    /// post: returns all transactions whose timestamp falls within the range,
    ///       filtered by optional account/asset/namespace criteria
    /// inv:  read-only; returns empty vec if no matches
    /// [P9] Constraining: Observability — transaction history is queryable
    pub fn query(
        &self,
        range: &DateRange,
        filter: &QueryFilter,
    ) -> Result<Vec<LedgerTransaction>, LedgerError> {
        // Build the query dynamically based on filters
        let mut sql = String::from(
            "SELECT DISTINCT t.id, t.timestamp, t.reference, t.metadata, t.created_at
             FROM transactions t
             JOIN postings p ON p.transaction_id = t.id",
        );
        let mut conditions = vec![format!(
            "t.timestamp >= '{}' AND t.timestamp <= '{}'",
            range.start, range.end
        )];

        if let Some(ref account) = filter.account {
            conditions.push(format!(
                "(p.source = '{}' OR p.destination = '{}')",
                account, account
            ));
        }
        if let Some(ref asset) = filter.asset {
            conditions.push(format!("p.asset = '{}'", asset));
        }
        if let Some(ref ns) = filter.namespace {
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM accounts a WHERE (a.id = p.source OR a.id = p.destination) AND a.namespace = '{}')",
                ns
            ));
        }

        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
        sql.push_str(" ORDER BY t.timestamp, t.id");

        let mut stmt = self.db.prepare(&sql)?;
        let tx_rows: Vec<(String, String, String, String, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        // For each transaction, load its postings
        let mut result = Vec::new();
        for (id, timestamp, reference, metadata, created_at) in tx_rows {
            let mut pstmt = self.db.prepare(
                "SELECT source, destination, asset, amount
                 FROM postings WHERE transaction_id = ?1 ORDER BY id",
            )?;
            let postings: Vec<Posting> = pstmt
                .query_map([&id], |row| {
                    Ok(Posting {
                        source: row.get(0)?,
                        destination: row.get(1)?,
                        asset: row.get(2)?,
                        amount: row.get(3)?,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            result.push(LedgerTransaction {
                id,
                timestamp,
                reference,
                postings,
                metadata: serde_json::from_str(&metadata).unwrap_or_default(),
            });
            // `created_at` is internal metadata, not exposed on LedgerTransaction
            let _ = created_at;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: P8-ledger-open — ledger opens and schema is created
    #[test]
    fn ledger_open_creates_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");

        let ledger = Ledger::open(&path).unwrap();

        // Verify all expected tables exist
        let tables: Vec<String> = ledger
            .db
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(
            tables.contains(&"accounts".to_string()),
            "accounts table missing"
        );
        assert!(
            tables.contains(&"transactions".to_string()),
            "transactions table missing"
        );
        assert!(
            tables.contains(&"postings".to_string()),
            "postings table missing"
        );
    }

    // REQ: P8-ledger-open — idempotent open
    #[test]
    fn ledger_open_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");

        // Open twice — second open should not error
        let _first = Ledger::open(&path).unwrap();
        let second = Ledger::open(&path);
        assert!(
            second.is_ok(),
            "re-opening an existing ledger should succeed"
        );
    }

    // REQ: P8-ledger-ensure-account — creates an account
    #[test]
    fn ensure_account_creates() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join("test.db")).unwrap();

        ledger.ensure_account("cost:api/deepinfra", "cost").unwrap();

        let count: i64 = ledger
            .db
            .query_row(
                "SELECT COUNT(*) FROM accounts WHERE id = ?1",
                ["cost:api/deepinfra"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "account should exist after ensure_account");
    }

    // REQ: P8-ledger-ensure-account — idempotent
    #[test]
    fn ensure_account_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join("test.db")).unwrap();

        ledger.ensure_account("cost:api/deepinfra", "cost").unwrap();
        // Second call with same id should succeed (no-op)
        let result = ledger.ensure_account("cost:api/deepinfra", "cost");
        assert!(result.is_ok(), "duplicate ensure_account should succeed");

        // Verify only one row exists
        let count: i64 = ledger
            .db
            .query_row(
                "SELECT COUNT(*) FROM accounts WHERE id = ?1",
                ["cost:api/deepinfra"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            count, 1,
            "duplicate ensure_account should not create second row"
        );
    }

    fn sample_tx(reference: &str) -> LedgerTransaction {
        use uuid::Uuid;
        let now = chrono::Utc::now().to_rfc3339();
        LedgerTransaction {
            id: Uuid::new_v4().to_string(),
            timestamp: now.clone(),
            reference: reference.to_string(),
            postings: vec![Posting {
                source: "cost:qa/run".into(),
                destination: "cost:api/deepinfra".into(),
                asset: "rJ".into(),
                amount: 100,
            }],
            metadata: serde_json::json!({}),
        }
    }

    // REQ: P8-ledger-commit — commits a transaction
    #[test]
    fn commit_stores_transaction() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join("test.db")).unwrap();
        ledger.ensure_account("cost:qa/run", "cost").unwrap();
        ledger.ensure_account("cost:api/deepinfra", "cost").unwrap();

        let tx = sample_tx("test-commit-1");
        ledger.commit(&tx).unwrap();

        // Verify transaction exists
        let count: i64 = ledger
            .db
            .query_row(
                "SELECT COUNT(*) FROM transactions WHERE id = ?1",
                [&tx.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "transaction should be stored");

        // Verify postings exist
        let posting_count: i64 = ledger
            .db
            .query_row(
                "SELECT COUNT(*) FROM postings WHERE transaction_id = ?1",
                [&tx.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(posting_count, 1, "posting should be stored");
    }

    // REQ: P8-ledger-commit — idempotent by reference
    #[test]
    fn commit_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join("test.db")).unwrap();
        ledger.ensure_account("cost:qa/run", "cost").unwrap();
        ledger.ensure_account("cost:api/deepinfra", "cost").unwrap();

        let tx = sample_tx("test-commit-idem");
        ledger.commit(&tx).unwrap();

        // Same reference, different id — should be no-op (reference is unique)
        let tx2 = {
            let mut t = sample_tx("test-commit-idem");
            t.id = uuid::Uuid::new_v4().to_string();
            t
        };
        let result = ledger.commit(&tx2);
        assert!(result.is_ok(), "duplicate reference should succeed (no-op)");

        // Verify still only one transaction with that reference
        let count: i64 = ledger
            .db
            .query_row(
                "SELECT COUNT(*) FROM transactions WHERE reference = ?1",
                ["test-commit-idem"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            count, 1,
            "duplicate reference should not create second transaction"
        );
    }

    // REQ: P8-ledger-commit — rejects empty postings
    #[test]
    fn commit_rejects_empty_postings() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join("test.db")).unwrap();

        let tx = LedgerTransaction {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            reference: "empty".into(),
            postings: vec![], // empty — violates invariant
            metadata: serde_json::json!({}),
        };

        let result = ledger.commit(&tx);
        assert!(result.is_err(), "empty transaction should fail");
        match result.unwrap_err() {
            LedgerError::DoubleEntryViolation(sum) => assert_eq!(sum, 0),
            e => panic!("expected DoubleEntryViolation, got {:?}", e),
        }
    }

    // REQ: P9-ledger-balance — computes balance from postings
    #[test]
    fn balance_after_commit() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join("test.db")).unwrap();
        ledger.ensure_account("cost:qa/run", "cost").unwrap();
        ledger.ensure_account("cost:api/deepinfra", "cost").unwrap();

        // Commit: move 100 rJ from qa/run → api/deepinfra
        let tx = sample_tx("test-balance");
        ledger.commit(&tx).unwrap();

        // qa/run should be at -100 (debited 100, not credited)
        let qa_balance = ledger.balance("cost:qa/run", Some("rJ")).unwrap();
        assert_eq!(qa_balance, -100);

        // api/deepinfra should be at +100 (credited 100, not debited)
        let api_balance = ledger.balance("cost:api/deepinfra", Some("rJ")).unwrap();
        assert_eq!(api_balance, 100);
    }

    // REQ: P9-ledger-balance — non-existent account returns 0
    #[test]
    fn balance_nonexistent_returns_zero() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join("test.db")).unwrap();

        let balance = ledger.balance("cost:nonexistent", Some("rJ")).unwrap();
        assert_eq!(balance, 0);
    }

    // REQ: P9-ledger-balance — balances sum to zero (conservation)
    #[test]
    fn balances_sum_to_zero() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join("test.db")).unwrap();
        ledger.ensure_account("cost:qa/run", "cost").unwrap();
        ledger.ensure_account("cost:api/deepinfra", "cost").unwrap();

        let tx = sample_tx("test-conservation");
        ledger.commit(&tx).unwrap();

        let qa = ledger.balance("cost:qa/run", Some("rJ")).unwrap();
        let api = ledger.balance("cost:api/deepinfra", Some("rJ")).unwrap();
        assert_eq!(qa + api, 0, "balances across all accounts must sum to zero");
    }

    // REQ: P9-ledger-namespace-balances — all balances for a namespace
    #[test]
    fn namespace_balances_returns_all_accounts() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join("test.db")).unwrap();
        ledger.ensure_account("cost:qa/run", "cost").unwrap();
        ledger.ensure_account("cost:api/deepinfra", "cost").unwrap();

        let tx = sample_tx("test-ns-bal");
        ledger.commit(&tx).unwrap();

        let balances = ledger.namespace_balances("cost").unwrap();

        // Should have rows for (account, asset) with postings
        assert!(
            !balances.is_empty(),
            "namespace balances should not be empty"
        );

        // Find qa/run balance
        let qa = balances
            .iter()
            .find(|b| b.account == "cost:qa/run" && b.asset == "rJ")
            .expect("cost:qa/run rJ balance should exist");
        assert_eq!(qa.balance, -100);

        // Find api/deepinfra balance
        let api = balances
            .iter()
            .find(|b| b.account == "cost:api/deepinfra" && b.asset == "rJ")
            .expect("cost:api/deepinfra rJ balance should exist");
        assert_eq!(api.balance, 100);
    }

    // REQ: P9-ledger-namespace-balances — unknown namespace returns empty
    #[test]
    fn namespace_balances_empty_for_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join("test.db")).unwrap();

        let balances = ledger.namespace_balances("nonexistent").unwrap();
        assert!(balances.is_empty(), "unknown namespace should return empty");
    }

    // REQ: P9-ledger-query — time-range query returns matching transactions
    #[test]
    fn query_by_time_range() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join("test.db")).unwrap();
        ledger.ensure_account("cost:qa/run", "cost").unwrap();
        ledger.ensure_account("cost:api/deepinfra", "cost").unwrap();

        let now = chrono::Utc::now();
        let start = (now - chrono::Duration::hours(1)).to_rfc3339();
        let end = (now + chrono::Duration::hours(1)).to_rfc3339();

        let tx = sample_tx("test-query");
        ledger.commit(&tx).unwrap();

        let range = DateRange { start, end };
        let filter = QueryFilter::default();
        let results = ledger.query(&range, &filter).unwrap();

        assert_eq!(results.len(), 1, "should find one transaction in range");
        assert_eq!(results[0].reference, "test-query");
        assert_eq!(results[0].postings.len(), 1);
    }

    // REQ: P9-ledger-query — empty result for no matches
    #[test]
    fn query_empty_when_no_matches() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join("test.db")).unwrap();

        // Query a time range where nothing exists
        let range = DateRange {
            start: "2020-01-01T00:00:00Z".into(),
            end: "2020-01-02T00:00:00Z".into(),
        };
        let filter = QueryFilter::default();
        let results = ledger.query(&range, &filter).unwrap();
        assert!(results.is_empty());
    }

    // REQ: P9-ledger-query — filter by account
    #[test]
    fn query_filter_by_account() {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::open(&dir.path().join("test.db")).unwrap();
        ledger.ensure_account("cost:qa/run", "cost").unwrap();
        ledger.ensure_account("cost:api/deepinfra", "cost").unwrap();

        let now = chrono::Utc::now();
        let start = (now - chrono::Duration::hours(1)).to_rfc3339();
        let end = (now + chrono::Duration::hours(1)).to_rfc3339();

        let tx = sample_tx("test-query-acct");
        ledger.commit(&tx).unwrap();

        let range = DateRange { start, end };
        let filter = QueryFilter {
            account: Some("cost:api/deepinfra".into()),
            ..Default::default()
        };
        let results = ledger.query(&range, &filter).unwrap();
        assert_eq!(results.len(), 1, "should find tx involving api/deepinfra");
    }
}
