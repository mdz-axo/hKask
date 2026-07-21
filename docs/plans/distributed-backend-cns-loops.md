---
title: "hKask Distributed Backend & CNS Loop Architecture Plan"
audience: [architects, developers]
last_updated: 2026-07-01
version: "0.31.0"
status: "Draft — Reviewed via essentialist, grill-me, pragmatic-semantics, pragmatic-cybernetics"
domain: "Cross-cutting (Storage, CNS, Deployment)"
mds_categories: [composition, trust, lifecycle]
anchored_on: [PRINCIPLES.md P1, P2, P4, P5, P9, P12]
reviewed_via: [essentialist, grill-me, pragmatic-semantics, pragmatic-cybernetics, improve-codebase-architecture]
---

# hKask Distributed Backend & CNS Loop Architecture Plan

**Purpose:** Define the path from single-writer SQLite to a per-store sync architecture (HMem via CRDT; registries via event log), the autonomous and curator-mediated CNS loop closure architecture, and the monitoring surface for a multi-pod agent deployment.

**Open questions that shape this plan:** See §7. The plan assumes eventual consistency for HMem, strong consistency for user/sovereignty data, and single-writer for wallet operations.

---

## 1. Per-Store Sync Architecture

### 1.1 Motivation

The current architecture is a single-writer SQLite database (kask.db) replicated to S3 via Litestream WAL shipping. This gives us:

- **One pod maximum** — K8s rolling updates, node maintenance, and scaling are unavailable
- **Pod restart = all state lost until S3 restore completes** — in-flight sessions terminate
- **Litestream is backup, not replication** — it's a recovery mechanism, not a concurrency primitive

The goal is to enable horizontally scaled pods where any pod can accept writes and all pods converge to the same state. But "state" is not one thing — the system has 8 distinct stores with different sync requirements.

### 1.2 What Stores Exist

The `StorageContext` in `hkask-services-context` exposes 6 stores in the main `kask.db`. HMem data lives in separate per-pod and per-agent databases (`memory.db`):

| Store | Location | Crate | Write Rate | Content |
|-------|----------|-------|-----------|---------|
| **HMem (episodic + semantic)** | memory.db | `hkask-storage-hmem` | High (every tool call, inference result, recall) | Agent experiences and consolidated knowledge |
| **SqliteRegistry** | kask.db | `hkask-templates` | Low (admin deploys skills) | Template/skill registrations and versions |
| **SqliteGoalRepository** | kask.db | `hkask-storage` | Low (user creates/modifies goals) | Goal definitions, state transitions, criteria |
| **UserStore** | kask.db | `hkask-storage` | Very low (sign-up, profile changes) | Human users, userpod identities, OAuth mappings, sessions |
| **SovereigntyBoundaryStore** | kask.db | `hkask-storage` | Very low (consent grant/revoke) | Affirmative consent boundaries (P2) |
| **WalletStore** | kask.db | `hkask-storage` | Medium (rJoule credits/debits) | Agent-local rJoule balances, API keys, transaction history |
| **EmbeddingStore** | memory.db | `hkask-storage` | Medium (semantic consolidation) | Vector embeddings for semantic recall |
| **UserPod files** | userpods/{name}/ (filesystem) | — | Low-medium (userpod creates artifacts) | adapters/, gallery/, documents/, library/, sessions/, portfolios/, artifacts/ |
| **Style/Kanban/Training DBs** | userpods/{name}/*.db | — | Low (configuration, tracking) | style profiles, kata kanban state, training data |

### 1.3 Per-Store Sync Strategy

A single CRDT cannot cover all stores and artifact types. Each has different conflict semantics, write rates, and consistency requirements. The sync strategy is **five-tier**:

#### Tier 1: OR-Set CRDT — HMem (episodic + semantic)

HMem is the only high-write store and the best fit for CRDTs. `HMemStore` entries are naturally commutative: two pods storing the same entity+attribute observation converge to one entry. Tombstone semantics already exist (the forgetting curve marks entries for decay). The OR-Set identity key is `(entity, attribute, access.owner_webid)` — two HMem entries with the same identity are the same observation, regardless of confidence or dimension differences.

**Merge semantics:**
- Two pods store the same entity+attribute+owner concurrently → one survives (latest `observed_at` wins; older becomes a soft tombstone)
- Pod A stores, Pod B deletes (forgetting curve decay) concurrently → delete wins if decay threshold crossed; store wins otherwise
- Confidence updates commute — highest confidence from any pod wins for semantic consolidation

**Sync transport:** Matrix room per pod group. Each `HMem` operation (store, recall, decay) serializes as a Matrix event. Pods replay the event DAG to rebuild their local `HMemStore`.

#### Tier 2: Event Log Replication — Registry, Goals, Userpods

These stores have low write rates and simple data. Last-write-wins is acceptable — there's no "wrong" merge for a template version bump or a goal state transition. Each write produces a Matrix event. Pods replay events in order on startup and subscribe to new events during runtime. No merge logic needed — ordered replay with timestamp-based conflict resolution.

**Per-store considerations:**
- **SqliteRegistry:** Template deploys are idempotent (same skill + version → no-op). Events carry the full template manifest.
- **SqliteGoalRepository:** Goal state transitions are serialized by the Curator (one Curator pod at a time via leader election). No concurrent writes to the same goal.
- **UserStore:** Userpod registrations are per-user unique. No conflict possible (different users register different userpods).

#### Tier 3: Single-Writer with Read Replicas — Users, Sovereignty

These stores require strong consistency. Consent revocation must not have merge conflicts — if a user revokes consent, every pod must see it immediately. Sovereignty data changes rarely (single-digit writes per session) but each change has high stakes. The pattern: one pod is the **primary** for these stores. All writes go through the primary. Read replicas subscribe to the event log but do not accept writes. If the primary pod fails, a new primary is elected (leader election via Matrix room membership).

This is the same leader-follower pattern databases have used for decades. It adds latency for writes (one extra hop to the primary) but eliminates the consistency risk of CRDT merges for sovereignty data.

#### Tier 4: Agent-Local, Not Synced — Wallet, Embeddings, Style/Kanban/Training

- **WalletStore:** rJoule balances and gas budgets are **agent-local.** Cryptocurrency balances are verified from the source of truth (blockchain/ledger), not synced between pods. Each agent tracks its own consumption. Wallet operations are single-writer by definition (the agent that owns the wallet is the only writer). Syncing wallet state between pods would be a security error — it creates a second source of truth that can diverge from the blockchain.
- **EmbeddingStore:** Vector embeddings are derived from HMem data. They can be rebuilt from the HMem event log rather than synced directly. Each pod computes its own embeddings from the replicated HMem store.
- **Style/Kanban/Training DBs:** These are agent-local configuration and tracking data (style profiles, kata coaching state, training data). They are low-write, agent-scoped, and have no cross-pod merge semantics. Each pod maintains its own copy; the agent.yaml definition records which styles a pod should load.

#### Tier 5: Content-Addressed File Sync — Adapters, Gallery, Documents, Artifacts

Agent files (LoRA adapters, generated images, document artifacts, portfolio exports) are **binary blobs** — not database records. They need a different sync mechanism than event log replication. hKask already has the infrastructure: `GitCASPort` provides content-addressed storage with `put_blob(content) → ContentHash`, `get_blob(hash) → content`.

**How it works:**
1. Agent creates a file (e.g., a generated logo in `gallery/`, a trained LoRA adapter in `adapters/`)
2. File is stored in the local CAS via `put_blob()` → returns a `ContentHash` (BLAKE3)
3. The file's manifest entry (path + hash) is published as a Matrix event in the agent's sync room
4. Other pods observe the event, check their local CAS for the hash, and `get_blob()` if missing
5. Same content = same hash = natural deduplication. Two agents generating the same file don't duplicate storage

**What syncs:**
- **adapters/:** LoRA adapters, training checkpoints — content-addressed, agent-created
- **gallery/:** Generated images, media — content-addressed, deduplicated by BLAKE3
- **documents/:** Parsed documents, generated reports — content-addressed
- **portfolios/:** Curated work collections — content-addressed
- **artifacts/:** General agent artifacts — content-addressed

**What doesn't sync:**
- **sessions/:** Session recordings are ephemeral — agent-local, not replicated
- **library/:** Reference materials can be fetched from source (web search, document processing) — cache, not sync
- **Template files:** Already handled by SqliteRegistry (Tier 2) — the registry records versions; the template cache is a local optimization

**Transport:** File manifest events flow through Matrix rooms (one room per agent for file discovery). Actual blob transfer can use Matrix's own file storage (`m.file` events) or direct pod-to-pod HTTP for large files (>10MB adapters). Matrix provides the discovery layer; the CAS provides the deduplication layer.

### 1.4 Sync Transport: Matrix Rooms

Matrix is already required (Conduit homeserver runs alongside kask). Different store tiers use different room topologies:

| Tier | Room Topology | Event Type |
|------|--------------|------------|
| Tier 1 (HMem) | One room per pod group | `HMemOp` (store/recall/decay) |
| Tier 2 (Registry/Goals/Agents) | One room per store type | `RegistryOp`, `GoalOp`, `AgentOp` |
| Tier 3 (Users/Sovereignty) | Primary pod writes to room; replicas read | `UserOp`, `SovereigntyOp` |

Matrix provides for free: ordered delivery (event DAG with sequence numbers), federation (`.well-known` discovery), persistence (room history = transaction log), access control (room membership = who can sync which store), and E2E encryption (Olm/Megolm for P1 sovereignty in transit).

### 1.5 Tier Summary

| Tier | Stores | Mechanism | Consistency | Write Rate |
|------|--------|-----------|-------------|------------|
| **Tier 1** | HMem (episodic + semantic) | OR-Set CRDT via Matrix rooms | Eventual (commutative merge) | High |
| **Tier 2** | Registry, Goals, Agents | Event log replication via Matrix rooms | Eventual (LWW) | Low |
| **Tier 3** | Users, Sovereignty | Single-writer, leader-follower via Matrix | Strong (writes go to primary) | Very low |
| **Tier 4** | Wallet, Embeddings, Style/Kanban/Training | Agent-local, not synced | N/A (local only) | Varies |
| **Tier 5** | Adapters, Gallery, Documents, Portfolios, Artifacts | Content-addressed file sync via GitCAS + Matrix | Eventual (hash-verified) | Low-medium |

### 1.6 Migration Path

```
Phase 1 — Read Replicas (today→weeks)
├── Single writer + litestream → S3 for ALL stores
├── Read-only replicas via S3 restore (every 30s WAL sync)
├── Horizontal read scaling, no sync complexity
├── Wallet, Users, Sovereignty remain single-writer indefinitely
└── Breaks when: write throughput exceeds one SQLite connection

Phase 2 — HMem CRDT + Tier 2 Event Log (months)
├── HMem (Tier 1): OR-Set CRDT backed by local SQLite, synced via Matrix room
├── Registry, Goals, Agents (Tier 2): Event log replication via per-store Matrix rooms
├── Users, Sovereignty (Tier 3): Single-writer with read replicas, leader election via Matrix
├── Wallet, Embeddings (Tier 4): Agent-local, not synced
├── Litestream/S3 retained for disaster recovery (cold backup of full state)
├── K8s: N replicas for HMem + Tier 2 stores; leader-follower for Tier 3
└── New: custom OR-Set crate for HMem, Matrix sync adapter per store tier

Phase 3 — Distributed Agents + Federation (beyond)
├── Agents get their own pods with per-agent HMem CRDT stores
├── Curator federation via Matrix (cross-instance room sync)
├── OCAP chains span instances via federated trust model
├── Per-agent wallet stays agent-local (no cross-pod wallet sync)
└── New: federated trust model, cross-instance delegation tokens
```

### 1.6 What Changes Per Phase

| Component | Today | Phase 1 | Phase 2 | Phase 3 |
|-----------|-------|---------|---------|---------|
| HMem store | SQLite (single writer) | SQLite (writer) + read replicas | OR-Set CRDT over local SQLite, Matrix sync | Per-agent OR-Set |
| Registry/Goals/Agents | SQLite (single writer) | SQLite (writer) + read replicas | Event log replication via Matrix rooms | Per-instance event logs |
| Users/Sovereignty | SQLite (single writer) | Single writer + read replicas | Leader-follower, leader election via Matrix | Leader-follower per instance |
| Wallet | SQLite (agent-local) | Agent-local (unchanged) | Agent-local (unchanged) | Agent-local (unchanged) |
| Backup | Litestream → S3 (all stores) | Litestream → S3 | CRDT + event log replication; Litestream for disaster recovery | CRDT + Matrix room history; Litestream for DR |
| Sync transport | None (single writer) | Litestream WAL shipping | Matrix rooms (per-tier) | Matrix federation |
| Deployment | 1 replica, RWO PVC | 1 writer + N readers | N HMem replicas + Tier 3 leader-follower | N curator + M agent pods |
| K8s scaling | Not supported | Read scaling only | Full horizontal scaling (HMem + Tier 2) | Full horizontal scaling |
| Federation | N/A | N/A | Same-instance pods | Cross-instance curator sync |
| OCAP authority | Single root | Single root | Per-instance roots, delegated | Federated trust model |

---

## 2. CNS Loop Architecture — Autonomous & Curator-Mediated

### 2.1 The Problem

The CNS today has variety counters and algedonic thresholds but the loops don't fully close:
- **Sense** works: CNS spans are collected
- **Compare** works: variety counters, threshold checks
- **Compute** works: algedonic alerts fire
- **Act** is missing: alerts are informational, no automated corrective action
- **Verify** is missing: no post-action re-measurement to confirm the fix worked

Additionally, all alerts follow the same path (human notification). There's no distinction between "the thermostat should handle this" (autonomous) and "this needs human judgment" (curator-mediated).

### 2.2 Dual-Loop Model

```
┌──────────────────────────────────────────────────┐
│                  CNS Model                        │
│                                                  │
│  Sense ──→ Compare ──→ Compute ──→ Act ──→ Verify│
│    │                                   │    │    │
│    │         ┌─────────────────────────┘    │    │
│    │         │                              │    │
│    │    ┌────┴────┐                    ┌────┴───┐│
│    │    │Autonomous│                   │Curator ││
│    │    │  Path    │                   │ Path    ││
│    │    │          │                   │         ││
│    │    │Pre-auth'd│                   │Human-in-││
│    │    │guardrails│                   │the-loop ││
│    │    │          │                   │         ││
│    │    │Seconds   │                   │Minutes  ││
│    │    └──────────┘                   └─────────┘│
└──────────────────────────────────────────────────┘
```

**Autonomous loops** close in seconds without human involvement. They handle operational guardrails that are pre-authorized:
- Disk pressure → prune old exports
- MCP server crash → restart subprocess
- Litestream sidecar dead → restart pod
- Inference rate approaching quota → throttle

**Curator-mediated loops** close in minutes to hours with human judgment. They handle decisions that need user context:
- Budget running low → present options (increase / switch model / pause)
- Variety deficit → suggest underused tools
- Pod degraded → explain impact and recovery estimate
- Export archive ready → notify for download

### 2.3 Loop Definitions

#### Autonomous Loops

| Loop | Sense Signal | Threshold | Action | Verify |
|------|-------------|-----------|--------|--------|
| **Storage Guard** | `/health` disk_usage_pct | >80% warn, >95% critical | warn: log CNS span; critical: prune exports older than 7d | Re-check after 5min |
| **Litestream Guard** | litestream sidecar liveness | Liveness probe failure × 3 | Restart pod (k8s does this; CNS records the event) | Confirm sidecar healthy after restart |
| **MCP Server Guard** | Child process health check | Process dead or unresponsive > 30s | Restart MCP subprocess via McpRuntime | Verify process responds to health ping |
| **Inference Throttle** | Gas consumption rate | Projected exhaustion < 15min at current rate | Reduce max_tokens, switch to classifier model (`DI/Qwen/Qwen3-235B-A22B-Instruct-2507`), notify Curator | Re-check consumption rate after 5min |

#### Curator-Mediated Loops

| Loop | Sense Signal | Threshold | Curator Assessment | User Decision |
|------|-------------|-----------|-------------------|---------------|
| **Budget Guard** | rJoule balance, consumption rate | Balance < 50 rJ or projected exhaustion < 1hr | "You have ~30min of inference at current rate. Options: [1] Add funds, [2] Switch to classifier model (cheaper, lower reasoning depth), [3] Continue at current rate" | User picks option |
| **Variety Deficit** | CNS variety counter | Deficit > 100 (critical) | "Your agent used 3 of 15 tools this session. Underused: [list]. Consider: [suggestions]" | User decides to explore or dismiss |
| **Pod Health Escalation** | `/health` shows degraded (DB or Conduit) | Any non-OK status | "Matrix connectivity lost — chat works but agent communication is offline. Conduit pod restarting (~30s)." | User waits or takes manual action |
| **Sovereignty Export** | Scheduled export completes | Export ready | "Your sovereignty archive is ready (1,247 triples, 4.2 MB). Download available for 7 days." | User downloads or defers |

### 2.4 Autonomous Guardrail Authorization

Autonomous actions are not "the machine decides on its own." They are **pre-authorized** by the user during onboarding or configuration:

```bash
# User authorizes autonomous guardrails
kask config set cns.autonomous.prune_exports true
kask config set cns.autonomous.restart_mcp true
kask config set cns.autonomous.throttle_inference true

# User sets thresholds
kask config set cns.autonomous.prune_threshold_pct 80
kask config set cns.autonomous.throttle_budget_minutes 15
```

The Magna Carta principle **P2 (Affirmative Consent)** applies here: autonomous actions require prior consent. The user explicitly opts in, with clear documentation of what each guardrail does and doesn't do.

### 2.5 CNS Signal Registry — New Signals

The following signals are proposed additions to the CNS span registry. They are design targets, not committed API — implementation in Phase B will determine the final shape.

| Signal | Fields | Purpose |
|--------|--------|---------|
| **DiskUsage** | `dir`, `used_bytes`, `available_bytes`, `pct` | Storage Guard sense input |
| **LitestreamHealth** | `sidecar_alive`, `last_replication_secs`, `s3_reachable` | Litestream Guard sense input |
| **McpServerHealth** | `server_name`, `pid`, `responsive`, `restart_count` | MCP Server Guard sense input |
| **InferenceBudget** | `session_id`, `rjoules_remaining`, `consumption_rate_rj_per_min`, `projected_exhaustion_minutes` | Inference Throttle + Budget Guard sense input |
| **AutonomousAction** | `loop_name`, `action`, `reason`, `pre_state`, `post_state`, `success` | Loop closure record — was the autonomous action effective? |
| **CuratorEscalation** | `escalation_id`, `loop_name`, `severity`, `assessment`, `options`, `user_decision`, `time_to_decision_secs` | Curator-mediated loop record |

**Naming migration:** The existing `SignalMetric::TripleCount` variant (doc: "Semantic h_mem count") should be renamed to `HMemCount` as part of Phase B, aligning the CNS signal registry with the `HMem` data model. The string form `"triple_count"` is visible in the CNS health API — the rename is a breaking change to be coordinated with the API surface.

**Deployment note:** The `HMemCount` signal is distinct from the monitoring signals above. `HMemCount` measures agent activity (how many memories are stored). The monitoring signals above measure infrastructure health (is the disk full, is Litestream alive). Different consumers, different retention, different urgency.

---

## 3. Monitoring Surface — What `/health` Reports

### 3.1 Current State

`GET /health` returns:
```json
{
  "healthy": true,
  "db": true,
  "conduit": true,
  "disk_usage_pct": 42,
  "disk_status": "ok"
}
```

### 3.2 Monitoring Coverage Matrix

| Failure Mode | Detected By | Detection Latency | Action |
|-------------|------------|-------------------|--------|
| DB unreachable | `/health` DB query | 10s (readiness probe) | Pod marked Not Ready |
| Conduit unreachable | `/health` HTTP check | 10s (readiness probe) | Pod marked Not Ready |
| Disk > 80% | `/health` disk check | 10s (readiness probe) | CNS autonomous prune |
| Litestream sidecar dead | K8s liveness probe (litestream container) | 30s | Pod restart |
| S3 unreachable | CNS LitestreamHealth span (to be implemented) | Depends on Litestream retry interval | CNS alert → Curator |
| OAuth misconfigured | Login attempt fails | User-visible | User reports |
| Inference provider down | Inference API error | Per-request | CNS InferenceBudget span |
| MCP server dead | Child process exit (to be implemented) | Process exit signal | CNS autonomous restart |
| Agent pod deadlock | No health signal improvement (to be implemented) | ~5min stall | Curator escalation |

### 3.3 Future Monitoring Additions

| Signal | Implementation | Priority |
|--------|---------------|----------|
| Litestream replication lag | Check `/data/kask.db-wal` age vs S3 last sync | After Phase 1 CRDT work |
| Inference provider status | Periodic health ping to configured providers | After autonomous inference throttle |
| MCP server responsiveness | Health ping to each child MCP process | After MCP server guard loop |
| Agent pod heartbeat | Per-pod liveness signal to CNS | Phase 3 with distributed agents |

---

## 4. Resilience — What Happens on Pod Restart

### 4.1 Current Behavior

When the pod restarts (voluntary or crash):
1. Init container `wait-for-conduit` polls until Matrix is ready
2. Init container `litestream-restore` downloads latest DB from S3
3. kask starts with the restored database
4. Litestream sidecar begins replicating

**Lost on restart:**
- In-flight LLM calls (gas budget may be partially debited)
- Active terminal sessions (WebSocket disconnects)
- CNS short-term memory (variety counters reset)

### 4.2 Phase 2 Resilience (CRDT)

When CRDT replication replaces Litestream:
- No restore step needed — every pod has a full replica
- New pod joins the Matrix sync room and replays events to catch up
- In-flight operations may be duplicated (CRDT merge handles idempotency)

### 4.3 Session Recovery (Future)

To recover terminal sessions after restart:
- Session state serialized to CRDT store periodically
- On reconnect, client sends last-known cursor (event ID)
- Server replays events since cursor, resumes from checkpoint

This requires changes to the terminal WebSocket handler and the `kask repl` session model. Deferred to Phase 3.

---

## 5. Verification — Closing the Feedback Loop

Every loop, autonomous or curator-mediated, must verify that its action had the intended effect:

```
Act → Wait → Sense again → Compare → Record outcome

If fixed → log CnsSpan::AutonomousAction { success: true }
If not fixed → escalate (autonomous → curator, curator → higher-severity alert)
If worse → rollback if possible, otherwise critical alert
```

The **dampener pattern** from the existing CNS applies: each loop has a minimum interval between actions to prevent thrashing. The `Dampener` type in `hkask-cns` already implements this — each loop definition specifies its cooldown.

---

## 6. Implementation Sequence

| Phase | Work | Depends On | Estimated Effort |
|-------|------|-----------|-----------------|
| **A — Monitoring Surface** | Extend `/health` with disk space, define CNS spans for new signals | Phase 5 | ✅ Done (disk space in `/health`; CNS spans defined in §2.5) |
| **B — Autonomous Guardrails** | Implement Storage Guard, Litestream Guard, MCP Server Guard loops in `hkask-cns` | Phase A | 2–3 weeks |
| **C — Curator-Mediated Loops** | Wire Curator assessment pipeline for Budget Guard, Variety Deficit, Pod Health Escalation | Phase B | 2–3 weeks |
| **D — CRDT Prototype** | Custom OR-Set triple store, Matrix sync transport, dual-write with SQLite fallback | Phase C | 4–6 weeks |
| **E — CRDT Migration** | Replace Litestream with CRDT replication, enable multi-replica K8s deployment | Phase D | 2–3 weeks |
| **F — Distributed Agents** | Per-agent CRDT stores, curator federation, cross-instance OCAP | Phase E | Beyond current plan horizon |

---

## 7. Open Questions

1. **OR-Set identity key for HMem.** The plan proposes `(entity, attribute, access.owner_webid)` as the identity key. Is `attribute` sufficient, or does the identity need to include `value` for full deduplication? Two pods storing the same entity+attribute with different values (e.g., "temperature: 72" vs "temperature: 68") — are these the same observation at different times, or different observations?

2. **Matrix room topology per tier.** Tier 1 (HMem) uses one room per pod group. Tier 2 uses one room per store type. How many rooms total? For 3 pods: 1 HMem room + 1 registry room + 1 goals room + 1 agents room = 4 rooms. Acceptable overhead? Does each room's event history grow unboundedly, or do we need compaction/truncation?

3. **Tier 3 leader election.** Leader election for Users/Sovereignty via Matrix room membership — what's the failover latency? If the primary pod dies, how long before a replica is promoted? Is this fast enough for consent revocation (P2: must deny access immediately upon revocation)?

4. **Autonomous authority scope.** Which guardrails should be opt-in vs always-on? Disk pruning feels safe to always-enable. Inference throttling feels like it needs affirmative consent (spending the user's rJoules without asking). Where's the line?

5. **Backup relationship.** Does CRDT + event log replication *replace* Litestream/S3 backup, or complement it? Recommendation: keep S3 for disaster recovery (cold storage, survives cluster loss), use CRDT for operational resilience (hot replicas, survive pod loss). But this means maintaining two backup systems.

6. **Storage Guard actuator mismatch.** When disk usage exceeds 80%, the plan says "prune exports older than 7d." But the PVC also contains the growing kask.db and per-agent databases. If the database itself is the space consumer, pruning exports achieves nothing. The Storage Guard needs a fallback escalate path for when the primary action is insufficient.

7. **Session recovery contract.** What's the minimum acceptable behavior on terminal disconnect? "Reconnect and start fresh" vs "Reconnect and resume where you left off"? The latter requires session state serialization to the HMem store.

8. **Curator liveness.** In Phase 3 (distributed agents), if the Curator pod crashes, who assesses CNS signals? The plan assumes a Curator is always available. A distributed system needs Curator failover or degraded-mode operation.

9. **Phase parallelization.** The Implementation Sequence (§6) is strictly sequential. But the monitoring surface (Phase A) and CRDT prototype (Phase D) have no dependency on each other — they could be developed in parallel. Should the sequence reflect this?

10. **Per-store deployment model.** Phase 2 says "no PVC dependency" for HMem replicas but requires local SQLite for the OR-Set store. Does each pod still need a PVC for local state, or is the OR-Set in-memory with the Matrix event log as the durable source of truth?

---

## 8. References

- `docs/diagrams/flowchart-deployment-architecture.md` — Current K8s deployment architecture
- `docs/diagrams/flowchart-pod-startup.md` — Pod startup sequence with init containers and probes
- `docs/plans/deployment-and-backup.md` — Deployment plan with Phase 1–6 implementation status
- `docs/architecture/core/FUNCTIONAL_SPECIFICATION.md` §3.18 — Deployment domain production contracts (FR-DP1–FR-DP17)
- `crates/hkask-cns/src/cybernetics_loop.rs` — Existing CNS loop implementation (sense→compare→compute→act)
- `crates/hkask-cns/src/dampener.rs` — Dampener for loop cooldown/thrashing prevention
- `crates/hkask-cns/src/algedonic.rs` — Algedonic alert severity and threshold definitions
- `crates/hkask-api/src/routes/health.rs` — Health check endpoint with DB + Conduit + disk monitoring
