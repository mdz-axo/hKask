# Matrix Server Administration — hKask Principles Adaptation

**Status:** Design | **Version:** 0.1.0 | **Date:** 2026-06-27

## Matrix Best Practices → hKask Principles Mapping

### 1. Account Security

| Matrix Best Practice | hKask Adaptation | Principle |
|---------------------|-----------------|-----------|
| Strong password generation per account | ✅ UUID v4 passwords via `uuid::Uuid::new_v4()`. Already implemented in `register_pod_matrix()`. | P2 (Affirmative Consent — each agent has unique credentials) |
| Credential rotation | Credentials stored in OS keychain under `matrix-pod-{name}`. Rotation requires deleting keychain entry + re-registration. Design: add `kask matrix rotate <pod>` command. | P1 (User Sovereignty — user controls credential lifecycle) |
| No shared credentials | Each pod gets unique `@{pod_name}-bot:localhost` MXID. No admin account reuse. | P12 (accountable identity — every action has an authenticated author) |
| m.login.dummy auth for daemon-managed accounts | ✅ Already used. Appropriate for server-local pods where the daemon is the identity provider. | P4 (OCAP — daemon holds the capability to create agent identities) |

### 2. Room Management

| Matrix Best Practice | hKask Adaptation | Principle |
|---------------------|-----------------|-----------|
| Invitation-only rooms | Curator standing session rooms are invitation-only. Replicants invited by Curator after registration. | P2 (Affirmative Consent — explicit invitation = consent to participate) |
| Room version upgrades | Monitor Matrix.org security disclosures. When room versions are deprecated, Curator upgrades rooms. Design: CNS span `cns.communication.matrix.room_upgrade` triggers metacognition review. | P9 (Homeostatic Self-Regulation — Curator maintains system health) |
| Per-room access control | Each pod's Matrix session is scoped to its rooms. Pods cannot join rooms without Curator invitation. | P4 (Clear Boundaries — pod Matrix access is capability-scoped) |
| Ephemeral rooms for transient communication | Design: pods can request temporary rooms. Curator creates, monitors, and destroys after TTL. | P3 (Generative Space — pods can communicate within user-defined boundaries) |

### 3. Server Administration

| Matrix Best Practice | hKask Adaptation | Principle |
|---------------------|-----------------|-----------|
| Regular Conduit updates | Conduit deployed via K3s (`deploy/k8s/conduit/`). Version updates via `kubectl set image`. Curator monitors Conduit version and CNS alerts on known CVEs. | P9 (Homeostatic Self-Regulation) |
| Database backup | `deploy/k8s/conduit/pvc.yaml` uses `hcloud-volumes`. Litestream is configured for the main kask pod but NOT for Conduit. Gap: Conduit database backup needs Litestream or PVC snapshot. | P1 (User Sovereignty — data portability requires backup) |
| Federation security | Federation disabled by default. `CuratorDirective::InviteToFederation` gates peer discovery. Room version 12 required for CVE-2025-49090 mitigation. | P4 (OCAP — federation is capability-gated) |
| Monitoring | CNS spans under `cns.communication.matrix.*` track Matrix health. Algedonic alerts on registration failures, message send failures, listener stalls. | P9 (Homeostatic Self-Regulation) |

### 4. Content Safety

| Matrix Best Practice | hKask Adaptation | Principle |
|---------------------|-----------------|-----------|
| Trust & Safety team | Not applicable — hKask is a single-user system. Content is user-owned. | P1 (User Sovereignty — user decides what content enters their system) |
| Abuse reporting | Not applicable — federation is opt-in and peer-trusted. | P4 (OCAP — abuse is prevented by capability boundaries, not detected after) |
| Content retention policies | Episodic memory retention is user-configurable. Matrix room history is stored in Conduit's database. | P1 (User Sovereignty — user controls data retention) |

### 5. The Self-Healing Matrix Registration Loop

```
Pod Activation
  │
  ├─ TRY: register_pod_matrix(homeserver_url, pod_name).await
  │   │
  │   ├─ OK → delete pending marker, proceed to activation
  │   │
  │   └─ ERR → store pending marker in keychain
  │       │       matrix-pod-pending-{name} = homeserver_url
  │       │
  │       ├─ Continue activation (pod operates without Matrix)
  │       │
  │       └─ Daemon Self-Healing Loop (runs every N seconds):
  │           │
  │           ├─ Query keychain for pending registrations
  │           │   (all keys matching "matrix-pod-pending-*")
  │           │
  │           ├─ For each pending pod:
  │           │   ├─ TRY: register_pod_matrix(url, pod_name).await
  │           │   ├─ OK → delete pending marker, log CNS span
  │           │   └─ ERR → increment retry_count
  │           │       └─ retry_count > MAX → escalate to user
  │           │
  │           └─ MAX_RETRIES = 10, backoff = 2^retry_count * 30s
  │
  └─ ACTIVATE: pod.activate(mcp)
```

### 6. CNS Spans for Matrix Administration

| Span | Event | Purpose |
|------|-------|---------|
| `cns.communication.matrix.pod_registered` | Pod successfully registered on Conduit | Observability |
| `cns.communication.matrix.pod_registration` | Registration attempt (success/failure) | Health monitoring |
| `cns.communication.matrix.pod_pending` | Registration deferred to retry | Self-healing trigger |
| `cns.communication.matrix.room_upgrade` | Room version upgrade required | Security maintenance |
| `cns.communication.matrix.federation_invite` | Federation peer invited | Federation audit |
| `cns.communication.matrix.message_sent` | Message delivered to room | Communication health |
| `cns.communication.matrix.daemon` | Daemon Matrix connection status | System health |

### 7. Design Gaps

| Gap | Priority | What's Needed |
|-----|----------|--------------|
| Conduit database backup | High | Litestream config for Conduit's SQLite DB or PVC snapshot schedule |
| Self-healing loop implementation | High | Background task in daemon that polls pending registrations and retries |
| Room version monitoring | Medium | CNS watcher for Matrix.org security disclosures; trigger `room_upgrade` |
| Credential rotation command | Medium | `kask matrix rotate <pod>` CLI + keychain update |
| Pod deactivation cleanup | Medium | When pod is torn down, deactivate its Matrix account (API call to Conduit) |
| Ephemeral room support | Low | Pod-requested rooms with TTL; Curator creates + destroys |
| Federation hardening | Low | Room version enforcement, peer capability verification |
