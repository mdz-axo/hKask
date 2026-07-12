---
title: "How to Configure a Database Backend — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Configure a Database Backend

hKask supports two database backends through the provider-agnostic `DatabaseDriver` trait in `crates/hkask-database/src/driver.rs`. This guide covers choosing a provider, configuring connections, and setting up encryption.

## Provider Comparison

| Provider | Status | Backend | Pooling | Vector Search | Encryption |
|----------|--------|---------|---------|---------------|------------|
| SQLite | v0.31 stable | rusqlite | r2d2 (8 conn) | sqlite-vec | SQLCipher |
| PostgreSQL | v0.32 planned | sqlx | sqlx pool | pgvector | pgcrypto |

**SQLite** is the default and only production-ready backend in v0.31. **PostgreSQL** support is implemented in `crates/hkask-database/src/postgres.rs` but gated behind the v0.32 milestone.

## SQLite Configuration

### Connection String

SQLite connects via a file path. The default location is `~/.config/hkask/data/kask.db`. Configure through environment variables:

```bash
export HKASK_DB_PATH="/path/to/kask.db"
```

The driver wraps a `r2d2::Pool<SqliteConnectionManager>` with WAL mode enabled for concurrent read access. The pool size defaults to 8 connections.

### Creating a Driver Programmatically

```rust
use hkask_database::{SqliteDriver, DatabaseDriver};
use r2d2_sqlite::SqliteConnectionManager;

let manager = SqliteConnectionManager::file("kask.db")
    .with_init(|conn| {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;"
        )
    });
let pool = r2d2::Pool::builder()
    .max_size(8)
    .build(manager)?;
let driver = SqliteDriver::new(pool);
```

### In-Memory (Testing)

```rust
let driver = SqliteDriver::in_memory_driver(); // Arc<dyn DatabaseDriver>
```

## Encryption Setup

Encryption is handled at the driver level via `Encryptor` in `crates/hkask-database/src/encrypt.rs`. It uses **AES-256-GCM** with a key derived via BLAKE3 from a user-provided passphrase.

### Configuration

Set the master passphrase as an environment variable or Kubernetes secret:

```bash
export HKASK_DB_PASSPHRASE="your-strong-passphrase-here"
```

In K8s, this goes in `deploy/k8s/secret.yaml` as `master-passphrase`.

### How It Works

1. The passphrase is hashed via BLAKE3 with a fixed domain-separation label, defined by `Encryptor::from_passphrase()`, to derive a 256-bit key.
2. Text values (`DbValue::Text`) are encrypted before storage with the format `ENCv1:<base64(nonce || tag || ciphertext)>`.
3. Plaintext values without the `ENCv1:` prefix pass through unmodified — enabling gradual migration of existing data.
4. The same passphrase on different machines produces the same key (deterministic key derivation).

### Testing Encryption

```rust
use hkask_database::encrypt::Encryptor;

let enc = Encryptor::from_passphrase("my-passphrase");
let encrypted = enc.encrypt("sensitive data");
assert!(encrypted.starts_with("ENCv1:"));
let decrypted = enc.decrypt(&encrypted);
assert_eq!(decrypted, "sensitive data");
```

## PostgreSQL (v0.32 Planned)

The `PostgresDriver` in `crates/hkask-database/src/postgres.rs` bridges async `sqlx` to the sync `DatabaseDriver` trait via `tokio::runtime::Handle::block_on`.

**Important:** PostgreSQL mode must be called from a non-async context or dedicated thread — calling `block_on` from within an async tokio context will panic.

SQLite-style `?N` placeholders are automatically translated to PostgreSQL `$N`.

### Planned Connection

```rust
// Planned for v0.32
let pool = sqlx::PgPool::connect("postgresql://user:pass@localhost/hkask").await?;
let driver = PostgresDriver::new(pool, tokio::runtime::Handle::current());
```

## Migration

hKask does not use a separate migration tool. Schema initialization happens in-store via `DatabaseDriver::execute_batch()`. Each store module (HMem, embeddings, etc.) calls its own `CREATE TABLE IF NOT EXISTS` statements on first access. The `AdapterStore` in `crates/hkask-adapter/src/adapter_store.rs` is a representative example with `init_schema()`.

To migrate from SQLite to PostgreSQL (when v0.32 ships), export the SQLite database to SQL, translate the schema, and import into PostgreSQL. No automated migration path exists yet.
