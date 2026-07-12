---
title: "How to Backup and Restore — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Backup and Restore

hKask uses SQLCipher-encrypted SQLite databases with Litestream for continuous backup to S3-compatible object storage. This guide covers creating snapshots, restoring from backups, and verifying backup integrity.

## Backup Architecture

Backups are managed at two levels:
- **Litestream**: Continuous WAL streaming to S3 (Kubernetes deployment)
- **CLI snapshots**: On-demand snapshots via `kask backup` commands

Backups are stored at `~/.config/hkask/backups/` in encrypted SQLCipher format (`.db` files).

## CLI Backup Commands

The following commands are available in the `kask` CLI and the TUI backup window (`crates/hkask-tui/src/windows/backup.rs`):

### Create a Snapshot

```bash
kask backup snapshot
```

Creates a point-in-time snapshot of all tracked storage types. The snapshot includes an artifact count, timestamp, and trigger reason.

### List Snapshots

```bash
kask backup list
```

Displays all stored snapshots with timestamps, artifact counts, and trigger types.

### Restore from Snapshot

```bash
kask backup restore
```

Restores the database from the most recent (or specified) snapshot. This is a destructive operation — it replaces the current database state.

### Verify Backup Integrity

```bash
kask backup verify
```

Checks that stored snapshots are not corrupted. The TUI backup window displays verification status with a green (verified) or yellow (needs attention) indicator.

### Prune Old Snapshots

```bash
kask backup prune
```

Removes snapshots older than the retention policy. Default retention: the configured number of daily and weekly snapshots.

## Backup Configuration

The `BackupDataBridge` in `crates/hkask-tui/src/bridges/backup.rs` exposes configuration fields:

- **Auto-Snapshot**: Enable/disable automatic snapshots on a schedule
- **Verify After Snapshot**: Run integrity verification after each snapshot
- **Encryption**: Enable/disable encryption of backup files (enabled by default with SQLCipher)
- **Tracked Types**: Number of storage artifact types included in backups
- **Retention**: Daily snapshot count and weekly snapshot count

## Kubernetes Litestream Backup

In the K8s deployment, Litestream runs as a sidecar in the kask Pod. Verify backups:

```bash
# Check Litestream snapshots
kubectl -n hkask exec deploy/kask -c litestream -- litestream snapshots /data/kask.db

# Check Litestream replication status
kubectl -n hkask logs deploy/kask -c litestream | grep "replicating"
```

Litestream configuration is in `deploy/k8s/configmap.yaml` (bucket, endpoint, region) and `deploy/k8s/secret.yaml` (access key, secret key).

## Backing Up the Keystore

The hKask keystore (`crates/hkask-keystore/`) stores cryptographic material. Back it up separately:

```bash
# The keystore is at ~/.config/hkask/keystore/
cp -r ~/.config/hkask/keystore/ ~/backups/hkask-keystore-$(date +%Y%m%d)/
```

The keystore path is configurable via environment:
```bash
export HKASK_KEYSTORE_PATH="/secure/path/keystore"
```

## Disaster Recovery

To fully restore from backup:

1. **Restore the database** from the most recent snapshot or Litestream S3 backup
2. **Restore the keystore** from your separate keystore backup
3. **Verify integrity**: Run `kask backup verify`
4. **Start kask**: The init container (K8s) or manual `litestream restore` (bare metal) ensures the database is complete before kask starts

The Litestream init container (`litestream restore`) in the K8s deployment runs before kask starts, guaranteeing the database is on disk when the application opens it.
