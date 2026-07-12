# hkask-database

Provider-agnostic database driver abstraction for hKask. Enables storage code to work with SQLite, PostgreSQL, and future providers through a single `DatabaseDriver` trait.

## Public Modules

| Module | Purpose |
|--------|---------|
| `driver` | `DatabaseDriver` trait and ergonomic query helpers (`query_map`, `query_row`) |
| `types` | `DbProvider` enum, `DbError` error type |
| `value` | `DbValue` and `DbRow` — provider-agnostic value types |
| `sqlite` | `SqliteDriver` — rusqlite + r2d2 connection pooling with WAL mode |
| `postgres` | `PostgresDriver` — sqlx + pgvector with `?N` → `$N` placeholder translation |
| `transaction` | `TransactionHandle` — RAII transaction guard with auto-rollback on drop |
| `encrypt` | `Encryptor` — transparent AES-256-GCM encryption for `DbValue::Text` |

## Key Types

| Type | Description |
|------|-------------|
| `DatabaseDriver` | Dyn-compatible trait: `execute`, `query`, `query_optional`, `execute_batch`, `transaction`, provider accessors |
| `SqliteDriver` | SQLite/SQLCipher implementation via `r2d2::Pool<SqliteConnectionManager>` |
| `PostgresDriver` | PostgreSQL implementation via `sqlx::PgPool` (async, bridged via `block_on`) |
| `DbProvider` | Enum: `Sqlite`, `Postgres` |
| `DbError` | Error enum: `Database`, `NotFound`, `Constraint`, `Connection`, `Serialization`, `UnsupportedProvider`, `Migration` |
| `DbValue` | Provider-agnostic value: `Null`, `Integer(i64)`, `Real(f64)`, `Text(String)`, `Blob(Vec<u8>)`, `Bool(bool)` |
| `DbRow` | Query result row with indexed and named accessors (`get_str`, `get_int`, `get_json`, etc.) |
| `TransactionHandle` | RAII guard — `commit()` or auto-rollback on drop |
| `Encryptor` | AES-256-GCM encryption for text values with `ENCv1:` prefix detection |

## Usage

```rust
use hkask_database::{SqliteDriver, DatabaseDriver, DbValue};
use hkask_database::value::DbRow;
use std::sync::Arc;

// Create an in-memory SQLite driver
let driver: Arc<dyn DatabaseDriver> = SqliteDriver::in_memory_driver();

// Create schema
driver.execute_batch("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)")?;

// Insert
driver.execute("INSERT INTO users (name) VALUES (?1)", &[DbValue::Text("Alice".into())])?;

// Query
let rows = driver.query("SELECT * FROM users", &[])?;
assert_eq!(rows[0].get_str_named("name")?, "Alice");

// Transaction with auto-rollback
let tx = driver.transaction()?;
driver.execute("INSERT INTO users (name) VALUES (?1)", &[DbValue::Text("Bob".into())])?;
tx.commit()?; // or drop to roll back
```

### Encryption

```rust
use hkask_database::encrypt::Encryptor;

let enc = Encryptor::from_passphrase("my-secret");
let encrypted = enc.encrypt("sensitive data");
assert!(encrypted.starts_with("ENCv1:"));
let decrypted = enc.decrypt(&encrypted);
assert_eq!(decrypted, "sensitive data");
```

## Architecture

```
Store (HMemStore, EmbeddingStore, ...)
  └── Arc<dyn DatabaseDriver>           ← provider-agnostic handle
        ├── SqliteDriver(r2d2::Pool)    ← rusqlite + WAL + SQLCipher
        └── PostgresDriver(sqlx::PgPool) ← sqlx + pgvector + pgcrypto
```

## Dependencies

- `rusqlite` — SQLite bindings (bundled)
- `r2d2` / `r2d2_sqlite` — Connection pooling
- `sqlx` — PostgreSQL async driver
- `serde` / `serde_json` — Serialization
- `aes-gcm` / `blake3` / `rand` — Encryption
- `base64` — Encrypted value encoding
- `thiserror` — Error types
- `tokio` — PostgreSQL async runtime
- `hkask-types` — `InfrastructureError` integration
