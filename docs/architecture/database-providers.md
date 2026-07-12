---
title: "Database Provider Selection"
audience: [developers, operators]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "Infrastructure"
mds_categories: [domain, trust]
---

# Database Provider Selection

Select your database via HKASK_DB_PROVIDER env var.
Both providers support all storage: memory (h_mems), embeddings, wallet, registry.

## Encryption

hKask encrypts all agent memory at rest. The mechanism depends on provider:

- **SQLite**: SQLCipher (AES-256) — automatic when HKASK_DB_PASSPHRASE is set.
  Database file is encrypted; no plaintext ever touches disk.
- **PostgreSQL**: Client-side AES-256-GCM encryption of text values.
  When `HKASK_DB_PASSPHRASE` is set, all `DbValue::Text` params are
  encrypted before transmission and decrypted on retrieval. The
  encryption key is derived from the passphrase via BLAKE3. No plaintext
  values ever appear in the database or transit logs. Admin should also
  configure server-side TLS (`sslmode=require` in connection URL) for
  defense-in-depth.

## SQLite (Default, Stable)

No additional installation needed. SQLCipher encryption is built in.

export HKASK_DB_PROVIDER=sqlite
kask start

## PostgreSQL (Planned v0.32)

Requires PostgreSQL 15+ with pgvector extension.

export HKASK_DB_PROVIDER=postgres
export HKASK_DB_PATH=postgresql://user:pass@localhost:5432/hkask
kask start

## Migration (SQLite → PostgreSQL)

Planned for v0.32: export JSON, import via provider-specific INSERT,
re-index vectors via pgvector. Not available yet.
