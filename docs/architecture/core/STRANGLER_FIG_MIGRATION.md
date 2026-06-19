---
title: "Strangler-Fig Migration Plan — PodManager → PodDeployment"
audience: [architects, developers]
last_updated: 2026-06-18
version: "0.29.0"
status: "Complete — PodManager deleted. PodDeployment is canonical."
domain: "Agent Pod Lifecycle"
---

# Strangler-Fig Migration — PodManager → PodDeployment

**Status:** Phase 1 (coexistence) implemented. `PodDeployment`, `PodFactory`, `PerPodStorage`, `PerPodCnsRuntime`, `PerPodToolBinding` types exist in `crates/hkask-agents/src/pod/deployment.rs`. All tests pass alongside existing `PodManager`.

---

## Phase 1 — Coexistence ✅ COMPLETE

- [x] Introduce `PodDeployment` type with `PerPodStorage`, `PerPodCnsRuntime`, `PerPodToolBinding`
- [x] Introduce `PodFactory` as a stateless constructor
- [x] Keep `PodManager` as-is
- [x] Both paths can coexist in the same crate
- [x] All tests pass: `cargo test -p hkask-agents`

**Files changed:**
- `crates/hkask-agents/src/pod/deployment.rs` (new)
- `crates/hkask-agents/src/pod/mod.rs` (module registration + exports)

---

## Phase 2 — CNS Per-Pod (Next Sprint)

**Goal:** Extract `CnsRuntime` into a constructable-per-pod component.

### Step 2.1 — Create PerPodCnsRuntime

Currently `PerPodCnsRuntime` is a placeholder (pod_id + span_namespace). Wire it to an actual CNS runtime:

```rust
pub struct PerPodCnsRuntime {
    pod_id: PodID,
    span_namespace: String,
    inner: CnsRuntime,  // Actual CNS instance, scoped to this pod
}
```

- The server-global CNS aggregator reads from all per-pod counters
- Each pod's variety counters are isolated
- CNS spans carry `pod_id` in their metadata

### Step 2.2 — Verification

```bash
# Per-pod CNS health
kask cns health --pod-id <id>

# All tests pass with both CNS paths
cargo test -p hkask-agents
cargo test -p hkask-cns
```

---

## Phase 3 — Storage Per-Pod

**Goal:** Migrate from shared TripleStore (scoped by `owner_webid`) to per-pod SQLCipher files.

### Step 3.1 — Per-Pod SQLCipher File

```rust
impl PerPodStorage {
    /// Open or create the pod's SQLCipher database file
    pub fn open(db_path: PathBuf, key: &[u8]) -> Result<Self, PodDeployError> {
        let conn = SqlCipherConnection::open(&db_path, key)?;
        // Initialize schema (triples table, CNS spans, episodic/semantic indices)
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(Self { conn, db_path })
    }
}
```

### Step 3.2 — Strangler-Fig on Storage

- **New pods:** `PodFactory::deploy()` creates a per-pod SQLCipher file
- **Existing pods:** Continue with shared TripleStore
- **Migration command:** `kask pod migrate-storage <pod_id>` exports from shared store into per-pod file

### Step 3.3 — Verification

```bash
# Migrate an existing pod
kask pod migrate-storage <pod_id>

# Verify data integrity
kask pod verify-storage <pod_id>

# All tests pass
cargo test -p hkask-agents
cargo test -p hkask-storage
```

---

## Phase 4 — MCP Binding Per-Pod

**Goal:** Each pod deployment binds its own MCP server instances.

### Step 4.1 — Virtual vs. Separate Instances

| Server Type | Binding | Mechanism |
|-------------|---------|-----------|
| Stateless (inference, research, web search) | Virtual instance — same process, different OCAP token | `GovernedTool` with pod-scoped capability |
| Stateful (memory, kanban) | Separate instance with pod-scoped storage | Per-pod SQLCipher file (already Phase 3) |

### Step 4.2 — Verification

```bash
# Two pods, two users, no collision
kask test pod-collision-isolation

# All tests pass
cargo test -p hkask-agents
cargo test -p hkask-mcp
```

---

## Phase 5 — Delete PodManager (Final Phase)

**Goal:** Remove PodManager's HashMap, shared pod cache, and centralized lifecycle methods.

### Step 5.1 — Prerequisites

- [ ] All active pods migrated to PodDeployment
- [ ] No callers of PodManager::create_pod (replaced by PodFactory::deploy)
- [ ] No callers of PodManager::activate_pod (replaced by PodDeployment activation)
- [ ] No callers of PodManager::deactivate_pod
- [ ] No callers of PodManager::list_pods (replaced by PodIndexEntry directory listing)

### Step 5.2 — Deletion

```bash
# Verify no references to PodManager outside test fixtures
grep -rn "PodManager" crates/ --include="*.rs" | grep -v "test\|mock\|#[cfg(test)]"

# Delete PodManager
rm crates/hkask-agents/src/pod/manager.rs
```

### Step 5.3 — Keep

- `PodFactory` — canonical pod constructor
- `PodIndexEntry` — lightweight directory listing index
- `AgentPod` — pod identity and lifecycle (still needed by PodDeployment)
- `PodContext` — runtime context for active pods (adapt to work with PodDeployment instead of PodManager)

### Step 5.4 — Verification

```bash
# Full workspace build
cargo check --workspace

# Full test suite
cargo test --workspace

# Contract audit
scripts/ci/contract-audit.sh --summary
```

---

## Acceptance Tests

### Test 1 — Pod Portability (η.2)

> "Create a pod on server A. Export it as a SQLCipher file. Import it on server B. Activate it. The agent retains its memory, identity, capabilities, and CNS history."

```bash
# Server A
kask pod create --template replicant --persona alice.yaml
kask pod export --pod-id <id> --output alice-pod.db

# Server B
kask pod import --file alice-pod.db
kask pod activate --pod-id <id>
kask pod verify --pod-id <id>  # Verifies memory, identity, capabilities, CNS history
```

### Test 2 — No Service Collisions (η.3)

> "Two pods, two different users, both running server mode. They do not collide on tool dispatch. They do not share memory. They do not leak CNS spans across pods."

```bash
# Two pods in server mode simultaneously
kask pod create --template replicant --persona alice.yaml
kask pod create --template bot --persona bob.yaml
kask pod mode server alice --role research &
kask pod mode server bob --role condenser &

# Verify no collision
kask test pod-collision-isolation
```

---

## Principle Traceability

| Phase | Principle | What it enforces |
|-------|-----------|-----------------|
| Phase 2 (CNS per-pod) | P9 Homeostatic Self-Regulation | Per-pod variety tracking; Curator aggregates |
| Phase 3 (Storage per-pod) | P11 Digital Public/Private Sphere | SQLCipher file IS private sphere boundary |
| Phase 4 (MCP per-pod) | P4 Clear Boundaries | Pod boundary IS OCAP enforcement perimeter |
| Phase 5 (Delete PodManager) | P5 Essentialism | Delete pass-through cache; keep PodFactory |
| Acceptance Test 1 | P1 User Sovereignty | Pod portability — user's agent moves freely |
| Acceptance Test 2 | P4 + P11 | No service collisions; no cross-pod data access |
