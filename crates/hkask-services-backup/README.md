# hkask-services-backup — Backup Service

Backup service policy layer: snapshot scheduling, restore orchestration, scope management, and pod-level backup coordination. Implements the backup loop on top of `GitCASPort`.

**Version:** v0.30.0 | **Crate:** `hkask-services-backup`

## Modules

| Module | Purpose |
|--------|---------|
| `service` | `BackupService` — snapshot, restore, list, prune, verify |
| `loop` | `BackupLoop` — periodic backup scheduling |
| `scope` | Backup scope configuration (full, agent-only, wallet-only) |
| `config` | Backup config (retention, schedule, CAS target) |
| `metadata` | Snapshot metadata and manifest types |
| `pod_ops` | Per-pod backup lifecycle management |
| `producers` | Artifact producers (config, memory, wallet, ledger) |
| `serialization` | Snapshot serialization format |

## Key Types

- `BackupService` — primary service interface
- `BackupLoop` — scheduled background backup loop
- `SnapshotManifest` — snapshot metadata and integrity hash

## Dependencies

- `hkask-ports` — `GitCASPort` for content-addressable storage
- `hkask-services-core` — `ServiceConfig`, `ServiceError`
- `hkask-cns` — CNS span emission for backup operations
