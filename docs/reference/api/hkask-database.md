---
title: "hkask-database — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

`hkask-database` provides a provider-agnostic database driver abstraction enabling hKask to work with SQLite, PostgreSQL, and future database providers. User sovereignty requires database optionality — users must be able to choose their infrastructure.

## Architecture

```
Store (HMemStore, EmbeddingStore, ...)
  └── Arc<dyn DatabaseDriver>           ← provider-agnostic handle
        ├── SqliteDriver(r2d2::Pool)    ← rusqlite + WAL + SQLCipher
        └── PostgresDriver(sqlx::PgPool) ← sqlx + pgvector + pgcrypto
```

## Public Modules

| Module | Purpose |
|---|---|
| `driver` | `DatabaseDriver` trait — the port stores code against |
| `encrypt` | Encryption support for database-level protections |
| `postgres` | `PostgresDriver` — sqlx-based PostgreSQL implementation |
| `sqlite` | `SqliteDriver` — r2d2-pooled rusqlite with WAL + SQLCipher |
| `transaction` | `TransactionHandle` — RAII transaction guard |
| `types` | `DbProvider` enum, `DbError`, and supporting types |
| `value` | `DbRow`, `DbValue` — database value abstraction layer |

## Key Types

### `DatabaseDriver` (trait)

Dyn-compatible trait (all methods are object-safe) that stores code against using `&dyn DatabaseDriver` or `Arc<dyn DatabaseDriver>`.

| Method | Signature | Purpose |
|---|---|---|
| `execute` | `(sql: &str, params: &[DbValue]) -> Result<usize, DbError>` | Execute INSERT/UPDATE/DELETE/DDL |
| `execute_batch` | `(sql: &str) -> Result<(), DbError>` | Execute batch SQL (schema creation) |
| `query` | `(sql: &str, params: &[DbValue]) -> Result<Vec<DbRow>, DbError>` | Query returning all rows |
| `query_optional` | `(sql: &str, params: &[DbValue]) -> Result<Option<DbRow>, DbError>` | Query single optional row |
| `provider` | `() -> DbProvider` | Which provider backs this driver |
| `commit_tx` | `() -> Result<(), DbError>` | Commit current transaction (internal, `#[doc(hidden)]`) |
| `rollback_tx` | `() -> Result<(), DbError>` | Rollback current transaction (internal, `#[doc(hidden)]`) |
| `as_any` | `() -> &dyn Any` | **Deprecated since 0.32.0.** Use `sqlite_pool()` or `postgres_pool()` instead |
| `sqlite_pool` | `() -> Option<&r2d2::Pool<...>>` | Access SQLite pool if `SqliteDriver` (default: `None`) |
| `postgres_pool` | `() -> Option<&sqlx::PgPool>` | Access PostgreSQL pool if `PostgresDriver` (default: `None`) |
| `transaction` | `() -> Result<TransactionHandle<'_>, DbError>` | Start a transaction; auto-rollback on drop; only on concrete types |

Ergonomic free functions are available in the `driver` module: `query_map()` maps query results via a closure; `query_row()` queries a single row.

### `SqliteDriver`

rusqlite-backed SQLite driver wrapped in an `r2d2::Pool` with WAL journal mode and SQLCipher encryption support. Vector search uses sqlite-vec.

### `PostgresDriver`

sqlx-backed PostgreSQL driver with pgvector for vector search and pgcrypto for encryption. Available in v0.32+.

### `DbProvider`

Enum identifying the database backend:

| Variant | Backend |
|---|---|
| `Sqlite` | SQLite via rusqlite/r2d2 |
| `Postgres` | PostgreSQL via sqlx |

### `TransactionHandle`

RAII guard for database transactions. Created via `DatabaseDriver::transaction()`. Auto-rollbacks on drop if not explicitly committed. Only available on concrete driver types (not `dyn DatabaseDriver`).

## Provider Coverage

| Provider | v0.31 | v0.32 | Backend | Pool | Vector | Encryption |
|---|---|---|---|---|---|---|
| SQLite | ✅ | ✅ | rusqlite | r2d2 (8) | sqlite-vec | SQLCipher |
| PostgreSQL | — | ✅ | sqlx | sqlx pool | pgvector | pgcrypto |

## Feature Flags

No feature flags are defined. This crate is a core dependency.
