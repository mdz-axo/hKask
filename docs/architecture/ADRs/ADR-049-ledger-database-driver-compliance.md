---
title: "ADR-049: Ledger DatabaseDriver Compliance"
audience: [architects, developers]
last_updated: 2026-07-11
version: "0.31.0"
status: "Active"
domain: "Technology"
mds_categories: [lifecycle]
---

# ADR-049: Ledger DatabaseDriver Compliance

**Date:** 2026-07-11
**Status:** Resolved — remediation complete

## Resolution

The ledger has been refactored to depend on `hkask_database::DatabaseDriver` (2026-07-11):

- `Ledger::from_driver(driver: Arc<dyn DatabaseDriver>)` — sole constructor per ADR-043
- `Ledger::open(path)` **removed** — no backward compatibility wrapper; consumers migrated to `from_driver()`
- `Mutex<Connection>` replaced with `Arc<dyn DatabaseDriver>` (connection pool managed by driver)
- `rusqlite::Error` replaced with `DbError` in `LedgerError`
- `rusqlite::params!` replaced with `DbValue` arrays
- `query_row`/`prepare`/`query_map` replaced with `query`/`query_optional` driver methods
- All 16 existing tests pass unchanged
- Consumers migrated:
  - `hkask-services-runtime::provider_intel` — `SelfTrackedProvider` and `FirecrawlProvider` now store `Arc<dyn DatabaseDriver>` instead of `PathBuf`; `create_provider` accepts `Option<Arc<dyn DatabaseDriver>>`
  - `hkask-mcp-companies::portfolio` — `PortfolioManager` stores `Option<Arc<dyn DatabaseDriver>>` instead of `Option<PathBuf>`

## Context

ADR-043 established the `DatabaseDriver` abstraction to decouple storage from SQLite/SQLCipher. The mandate: "All stores construct via `from_driver(driver)`." Every store in `hkask-storage` complies — `HMemStore`, `EmbeddingStore`, `WalletStore`, etc.

The `hkask-ledger` crate does **not** comply. It uses raw `rusqlite::Connection` directly:

```rust
pub struct Ledger {
    db: Mutex<Connection>,  // raw rusqlite, bypasses DatabaseDriver
}
```

Its `Cargo.toml` depends on `rusqlite.workspace = true` but does not depend on `hkask-database`. This means:

1. The ledger cannot use PostgreSQL — it is hardcoded to SQLite.
2. Multi-user deployments that select PostgreSQL cannot use the ledger.
3. The ledger does not benefit from the RAII `TransactionHandle` auto-rollback pattern.
4. The ledger maintains its own `Mutex<Connection>` instead of using the driver's connection pool.

## Decision

**Remediate:** Refactor `hkask-ledger` to depend on `hkask-database::DatabaseDriver` and construct via `from_driver(driver)`, matching every other store in the codebase.

### Remediation Steps

1. Add `hkask-database` dependency to `hkask-ledger/Cargo.toml`.
2. Replace `Mutex<Connection>` with `DatabaseDriver` (or `Arc<DatabaseDriver>` for shared access).
3. Refactor `Ledger::open(path)` → `Ledger::from_driver(driver: &DatabaseDriver)`.
4. Keep the SQL DDL identical — `execute_batch` works on both SQLite and Postgres via the driver.
5. Replace `Mutex<Connection>` lock pattern with driver's transaction API (`TransactionHandle` for atomic commits).
6. Update consumers (`hkask-services-runtime`, `hkask-mcp-companies`) to pass the driver.

### Why not document this as an accepted exception?

The ledger's double-entry invariant (postings sum to zero, idempotency by reference) is not incompatible with `DatabaseDriver`. The driver supports transactions via `TransactionHandle` — exactly what atomic double-entry commits need. There is no technical reason for the bypass.

## Consequences

### Positive
- Ledger gains PostgreSQL support for multi-user deployments.
- Consistent with ADR-043 and every other store.
- Benefits from connection pooling and RAII transactions.

### Negative
- Breaking change to `Ledger::open()` API — consumers must migrate to `from_driver()`.
- SQL dialect differences may surface (e.g., `INSERT OR IGNORE` is SQLite-specific; Postgres uses `ON CONFLICT DO NOTHING`).

### Neutral
- The `hkask-database` driver already handles `?N` → `$N` parameter translation for Postgres.

## Compliance

| Principle | Current | After Remediation |
|-----------|---------|-------------------|
| **ADR-043** (DatabaseDriver) | ❌ Non-compliant | ✅ Compliant |
| **P5** (Essentialism) | ❌ Redundant connection management | ✅ Delegates to driver |
| **P7** (Evolutionary Architecture) | ❌ Hardcoded to SQLite | ✅ Provider-agnostic |

## Related Documents

- [ADR-043: Database Driver Abstraction](ADR-043-database-driver.md) — The mandate this ADR enforces
- Database Driver Class Diagram (inlined in `architecture/core/hKask-architecture-master.md`) — DatabaseDriver trait hierarchy
