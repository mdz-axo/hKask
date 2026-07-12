---
title: "hkask-storage — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

`hkask-storage` is the SQLite + SQLCipher storage backend facade. Foundation types (`Database`, `Store`, lock helpers) are re-exported from `hkask-storage-core`. Domain-specific storage modules live here or in sub-crates behind this facade.

## Public Modules

| Module | Purpose |
|---|---|
| `agent_registry` | `AgentRegistryStore` — agent identity and registration records |
| `embeddings` | `EmbeddingStore` — vector embeddings with similarity search |
| `goals` | `SqliteGoalRepository` — OCAP-gated goal records with quarantine |
| `nu_event_store` | `NuEventStore` — weighted event log with `DecayConfig` |
| `user_store` | User identity and authentication records |
| `wallet` | `WalletStore` — wallet data persistence |

## Re-exports from `hkask-storage-core`

| Type/Function | Purpose |
|---|---|
| `Database` | Core database wrapper |
| `DatabaseError` | Database error type |
| `open_database` | Open or create a database |
| `open_or_repair` | Open and verify the database without destructive recovery |
| `check_passphrase` | Verify database passphrase |
| `define_driver_store` | Macro for defining store types |
| `impl_from_db_error` | Error conversion macro |
| `sanitize_path` | Sanitize filesystem paths |

## Key Types

### `HMemStore`

Core episodic memory store in `hkask-storage::hmem`. Manages `HMem` records — the foundational unit of episodic memory in hKask. Error type: `HMemError`. Indexed by `HMemId`.

### `NuEventStore`

Weighted event log supporting temporal decay and event accumulation via `WeightedEvent`. Configured with `DecayConfig` to control how event relevance diminishes over time.

### `EmbeddingStore`

Vector embedding persistence with similarity search. Key types:

| Type | Purpose |
|---|---|
| `EmbeddingStore` | Store and query vector embeddings |
| `StoredEmbedding` | A stored embedding with metadata |
| `SimilarityResult` | Result of a similarity search query |
| `EmbeddingError` | Embedding store error type |

### `ConsentStore`

Stores user consent records (`StoredConsentRecord`) for sovereignty tracking (Magna Carta P1). Error type: `ConsentStoreError`.

### `EscalationQueue`

Manages escalation entries (`EscalationEntry`) with batching (`EscalationBatch`) and statistics (`EscalationStats`). Each entry has an `EscalationStatus`. Error type: `EscalationError`.

### `GalleryStore`

Image and tag persistence for the media gallery. Key types: `GalleryRecord`, `ImageRecord`, `TagRecord`, `GalleryMode`. Error type: `GalleryStoreError`.

### `KataHistoryStore`

Toyota Kata history tracking via `KataHistoryEntry`. Error type: `KataHistoryError`.

### `SovereigntyBoundaryStore`

Stores `SovereigntyBoundaryEntry` records representing defined sovereignty boundaries. Error type: `SovereigntyStoreError`.

### `BackupArchive`

Archive and migration support from `hkask-storage::hmem::archive`. Key types:

| Type | Purpose |
|---|---|
| `BackupArchive` | Create and restore encrypted backups |
| `BackupMeta` | Backup metadata (timestamp, size, checksum) |
| `MigrationReceipt` | Archive migration receipt |
| `ArchiveError` | Archive operation errors |

### `AgentRegistryStore`

Agent identity and registration records. Error type: `AgentRegistryError`.

### `SqliteGoalRepository`

Goal persistence with quarantine support. Key types: `QuarantinedGoal`. Error type: `GoalRepositoryError`.

### `TokenRegistryStore`

Delegation token persistence store in `hkask-storage::token_registry`.

### `WalletStore`

Wallet data persistence for rJoule balance and transaction records.

## Key Functions

| Function | Source | Purpose |
|---|---|---|
| `now_rfc3339` | `hkask_types::time` | Current time as RFC 3339 string |

## Feature Flags

No feature flags are defined. This crate is a core dependency.
