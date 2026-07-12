---
title: "Database Driver Abstraction — Explanation"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# Database Driver Abstraction

## Why DatabaseDriver Exists

hKask's storage layer faces a common tension: the local-first architecture demands SQLite with encryption for pod databases, but remote deployments may need PostgreSQL. Without an abstraction, every store would contain provider-specific connection logic, and adding a new backend would touch every crate.

The `DatabaseDriver` trait (`crates/hkask-database/src/driver.rs`) solves this. It defines a dyn-compatible trait that all stores code against. Stores hold `Arc<dyn DatabaseDriver>` instead of raw `rusqlite::Connection`. Provider-specific logic is isolated in two implementations: `SqliteDriver` and `PostgresDriver`. A third provider could be added without changing any store code.

```rust
pub trait DatabaseDriver: Send + Sync {
    fn execute(&self, sql: &str, params: &[DbValue]) -> Result<usize, DbError>;
    fn execute_batch(&self, sql: &str) -> Result<(), DbError>;
    fn query(&self, sql: &str, params: &[DbValue]) -> Result<Vec<DbRow>, DbError>;
    fn query_optional(&self, sql: &str, params: &[DbValue]) -> Result<Option<DbRow>, DbError>;
    fn provider(&self) -> DbProvider;
    // ... transaction, pool access
}
```

Two free functions provide ergonomic query patterns that work with `&dyn DatabaseDriver`: `query_map()` maps rows to domain types via closures, and `query_row()` returns an optional single row. These replace the raw `stmt.query_map(params, |row| { ... })` pattern.

## SQLite: The Local Backend

`SqliteDriver` (`crates/hkask-database/src/sqlite.rs`) wraps an `r2d2::Pool<SqliteConnectionManager>` with WAL mode enabled. Connection pooling via r2d2 enables concurrent read access — each `execute`/`query` acquires a connection from the pool and returns it on completion. The pool is configured with `max_size(4)` and `SqliteConnectionManager` with WAL mode, busy timeout (5000ms), synchronous=NORMAL, and foreign keys ON.

The driver translates between hKask's type-agnostic `DbValue` enum (`Null`, `Integer`, `Real`, `Text`, `Blob`, `Bool`) and `rusqlite`'s native types. Parameter conversion goes `DbValue` → `Box<dyn ToSql>`. Row conversion goes `rusqlite::Row` → `DbRow` (column names + values). The `acquire_raw()` method provides escape-hatch access to the raw `rusqlite::Connection` for stores that need it — sqlite-vec virtual tables, for instance, don't work through the `DbValue` abstraction.

For testing, `in_memory_driver()` returns an `Arc<dyn DatabaseDriver>` backed by an in-memory SQLite database in a single line.

## PostgreSQL: The Remote Backend

`PostgresDriver` (`crates/hkask-database/src/postgres.rs`) bridges async `sqlx` to the sync `DatabaseDriver` trait via `tokio::runtime::Handle::block_on`. This has an important constraint: calling from within a tokio context panics with "Cannot start a runtime from within a runtime." Callers must ensure PostgresDriver operations happen from a non-async context or a dedicated thread.

The driver translates SQLite-style `?N` placeholders to PostgreSQL `$N`. Parameters are serialized to `Option<String>` — PostgreSQL coerces text to the target column type (int, float, bool, bytea). Row decoding tries integer first (most common), then real, bool, blob, text — matching PostgreSQL's internal type representation.

## Schema Auto-Initialization

The `from_driver()` pattern is how stores bootstrap themselves. Rather than each store checking whether tables exist, the driver handles schema initialization at construction time:

```rust
// WalletStore example
pub fn from_driver(driver: Arc<dyn DatabaseDriver>) -> Self {
    driver.execute_batch("CREATE TABLE IF NOT EXISTS wallets (...)").unwrap();
    Self { driver }
}
```

For file-backed databases, `Database::connect()` (`crates/hkask-storage-core/src/database.rs`) handles this at a lower level. Schema SQL is embedded via `include_str!("sql/schema.sql")` and executed on the first connection from the pool. The embedding dimension is templated (`$DIM` → configured value) at runtime. This means a fresh database file is fully initialized — tables, indexes, foreign keys — the moment the first connection is opened.

## SQLCipher Encryption

hKask's threat model requires local databases to be encrypted at rest. SQLCipher provides this at the SQLite engine level. The `Database` struct in `hkask-storage-core` manages the encryption lifecycle:

1. **Salt generation** — When a database is created, a 16-byte random salt is written to `{path}.salt`. The salt is stored alongside the database file.
2. **Key derivation** — The user's passphrase + salt → Argon2id → 256-bit key. The key is held in memory only during pool construction.
3. **PRAGMA key** — On new databases, `PRAGMA cipher_plaintext_header_size = 32` is set before `PRAGMA key` to reserve space for the encryption header. On existing databases, only `PRAGMA key` is needed.
4. **Passphrase verification** — Schema initialization doubles as passphrase verification. A wrong passphrase produces a "file is not a database" error, which is mapped to `DatabaseError::PassphraseMismatch`.

The `open_or_repair()` function verifies that the database opens with the supplied passphrase. A passphrase failure is returned to the caller; it never deletes or replaces the database or salt file. Destructive recovery must be an explicit, separately authorized operation.

## Column-Level Encryption

Beyond full-database SQLCipher encryption, `hkask-database::encrypt` (`crates/hkask-database/src/encrypt.rs`) provides column-level AES-256-GCM encryption for `DbValue::Text` values. When a passphrase is configured, text values are encrypted before storage and decrypted on retrieval. The format `ENCv1:<base64(nonce || tag || ct)>` enables automatic detection — plaintext passes through unchanged. The key is derived from the passphrase via BLAKE3, not shared with the SQLCipher key. This provides defense-in-depth: even if the database file is decrypted, individually encrypted columns remain opaque without the column-level passphrase.

## Transactions

`TransactionHandle` in `hkask-database` provides RAII-guarded transactions. `driver.transaction()` begins a transaction and returns a guard. If the guard is dropped without calling `.commit()`, the transaction is rolled back. This prevents leaked transactions from leaving the database in an inconsistent state.

```rust
let tx = driver.transaction()?;
driver.execute("INSERT INTO t VALUES (?)", &[DbValue::Integer(42)])?;
tx.commit()?;  // or drop(tx) for rollback
```

## Migration Receipts and Backup

Migration receipts are handled at the `Database` level. The schema is versioned and `CREATE TABLE IF NOT EXISTS` ensures migrations are idempotent — running them on an already-migrated database is a no-op. Backup and restore are enabled by the file-based nature of SQLite: the `.db` file and its `.salt` companion can be copied as a single unit, providing a consistent snapshot under WAL mode.
