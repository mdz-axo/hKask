---
title: "Public Surface Justification — hkask-storage"
audience: [architects, developers]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# Public Surface Justification — hkask-storage

**Crate:** `hkask-storage`  
**Public items in lib.rs:** 39  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-storage` is the **persistence foundation** — SQLite + SQLCipher + sqlite-vec backing every domain's storage needs. Its public surface is large because it provides storage adapters for multiple domains:

1. **Multi-domain storage adapters** — Agent registry, user store, consent store, goal repository, triple store, gallery store, NuEvent store, wallet store, backup store, and episodic/semantic memory pipelines. Each adapter is a focused module with its own error type.

2. **Database abstraction** — `Database::conn_arc()` provides the shared `Arc<Mutex<Connection>>` pattern used by all storage consumers. The in-memory and file-based constructors serve different deployment modes.

3. **Encryption infrastructure** — SQLCipher integration, passphrase management, and key derivation are public because they're used by the keystore and wallet crates for encrypted-at-rest storage.

## Mitigations

- **Per-domain modules:** Each storage domain (gallery, goals, triples, wallet, etc.) is a separate module with its own error enum — they pass the deep-module test individually.
- **Shared connection pattern:** `conn_arc()` eliminates per-adapter connection management duplication.
- **Feature-gated SQL:** The `sql` feature gates rusqlite integration, allowing the crate to be used without SQL dependencies where only types are needed.

## Deletion Test

If you delete `hkask-storage`, the SQLite schema management, encrypted-at-rest persistence, vector storage (sqlite-vec), and per-domain repository patterns reappear scattered across every crate that needs persistence. The crate earns its existence.
