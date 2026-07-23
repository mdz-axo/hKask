---
title: "ADR-047: Storage Crate Modularization"
audience: [architects, developers]
last_updated: 2026-07-03
version: "0.31.0"
status: "Active"
domain: "Technology"
mds_categories: [lifecycle]
---

# ADR-047: Storage Crate Modularization

**Date:** 2026-07-03
**Status:** Active

## Context

`hkask-storage` was a 29-file, 9,856-line monolith containing SQLite-backed
stores for every domain. This violated deep-module discipline: a single crate
with 22 public modules and 50+ re-exported types.

**Problem Statement:** `hkask-storage` was too large to navigate and too coupled
to compile efficiently.

## Decision

**Chosen Approach:** Extract the storage foundation into `hkask-storage`
and organize domain storage behind the `hkask-storage` facade.

**Storage modules:**

| Module/Crate | Contents |
|--------------|----------|
| `hkask-storage` | `Database`, `Store` trait, lock helpers, SQL macros |
| `hkask-storage::gallery` | `GalleryStore`, `GalleryRecord`, image/tag types |
| `hkask-storage::kata` | `KataHistoryStore`, `KataHistoryEntry` |
| `hkask-storage::hmem` | `HMemStore`, `HMem`, `HMemError` |
| `hkask-storage::hmem::archive` | `BackupArchive`, `BackupMeta`, `MigrationReceipt` |
| `hkask-storage::token_registry` | `TokenRegistryStore` |
| `hkask-storage::consent_store` | `ConsentStore`, `StoredConsentRecord` |
| `hkask-storage::sovereignty` | `SovereigntyBoundaryStore` |
| `hkask-storage::escalation` | `EscalationQueue`, `EscalationEntry` |

**Facade pattern:** `hkask-storage` re-exports domain storage types at the
crate root. Downstream consumers see no API change.

**Organization order:** `core` → `gallery` → `kata` → `hmem` → `archive` →
`token_registry` + `consent_store` + `sovereignty` + `escalation` (batch).

**Remaining in facade:** `agent_registry`, `embeddings`, `goals`,
`nu_event_store`, `spec_store`, `spec_types`, `user_store`, `wallet`
(8 modules, candidates for future extraction).

## Consequences

### Positive
- Storage domains are independently testable
- Clear dependency direction: storage domains depend on core, never reverse
- 11 files were removed from the monolith during modularization

### Negative
- More internal module boundaries to navigate
- `hkask-storage` is now a thin facade

### Neutral
- Consumers migrated from `hkask_storage::database::in_memory_db` to
  `hkask_storage::in_memory_db` (8 callers updated)

## Compliance

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P5** (Essentialism) | ✅ | Each storage domain has a single responsibility |
| **P7** (Prefer deletion over deprecation) | ✅ | 11 files deleted from monolith |
| **G2** (Surface ≤ 7) | ✅ | `hkask-storage`: 4 public modules |

## Related Documents

- [ADR-046: REPL Extraction Path](ADR-046-repl-extraction-path.md) — Same extraction pattern applied to REPL
