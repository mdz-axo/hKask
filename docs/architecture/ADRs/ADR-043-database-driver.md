---
title: "ADR-043: Database Driver Abstraction"
audience: [developers, architects]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "Infrastructure"
mds_categories: [domain, composition, trust]
---

# ADR-043: Database Driver Abstraction

Status: Accepted | Date: 2026-07-31 | Version: 0.31.0 | Updated: 2026-08-01

## Context

hKask was hardcoded to SQLite/SQLCipher via rusqlite. Every store accepted
raw Arc<Mutex<rusqlite::Connection>>. Users could not choose their database
infrastructure — violating Magna Carta P1 (User Sovereignty).

## Decision

Introduce hkask-database crate with DatabaseDriver trait abstracting over
SQLite and PostgreSQL. All stores (HMemStore, EmbeddingStore, wallet, registry)
construct via `from_driver(driver)`. Provider selected via HKASK_DB_PROVIDER.

- **HMemStore**: Both providers. SQL path dispatches on `driver.provider()`.
  SQLite uses raw rusqlite; Postgres uses driver with translated `?N→$N`.
- **EmbeddingStore**: Both providers. sqlite-vec for SQLite, pgvector for Postgres.
- **Transactions**: RAII `TransactionHandle` — auto-rollback on drop.

## Providers

| Provider | Status | Features |
|----------|--------|----------|
| SQLite/SQLCipher | Stable | Embedded, AES-256 encrypted at rest, sqlite-vec |
| PostgreSQL/pgvector | Stable | Server, multi-user, pgvector, admin-managed encryption |

## Encryption

- **SQLite**: SQLCipher — automatic when passphrase set. No plaintext on disk.
- **PostgreSQL**: Client-side AES-256-GCM via BLAKE3-derived key from passphrase.
  Text values encrypted before storage, decrypted on retrieval. Also supports
  TLS via connection URL (`sslmode=require`).
