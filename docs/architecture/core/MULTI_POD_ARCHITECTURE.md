---
title: "Multi-Pod Architecture — CuratorPod + TeamPod + ReplicantPod"
audience: [architects, developers]
last_updated: 2026-06-18
version: "0.30.0"
status: "Design — Ready for Implementation"
domain: "Agent Pod Lifecycle"
principles: [P1, P4, P5, P6, P9, P11]
---

# Multi-Pod Architecture — CuratorPod + TeamPod + ReplicantPod

**Purpose:** Extend the per-pod SQLCipher model (Solid Pod isomorphism) to three pod tiers with lazy one-way semantic sync through the Curator.

**Post-interrogation synthesis:** Caveman compression + Grill-Me Socratic + Improv Plussing/YesBut/Riffing + Coding Guidelines framing.

---

## 1. Pod Tier Structure

```
Startup order:
  1. CuratorPod     ← singleton, SemanticIndex, CNS aggregation
  2. TeamPods        ← shared bot workspaces, one per team
  3. ReplicantPods   ← on demand, one per human+replicant pair

File layout:
  {data_dir}/agents/
    curator/pod.db                ← singleton, PodKind::Curator
    7r7/pod.db                    ← 7R7 TeamPod
    {team_name}/pod.db            ← arbitrary team pods
    {webid}/pod.db                ← one per human+replicant
```

### PodKind Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PodKind {
    Curator,    // singleton, SemanticIndex owner
    Team,       // shared bot workspace
    Replicant,  // per-user sovereign pod
}
```

No metadata file. No schema table for pod identity. The filename convention IS the identity. `PodRegistry.scan()` reads `*.db` files and parses the prefix. Deletion test: remove file → pod gone. No cleanup.

---

## 2. Data Flow — Lazy One-Way Sync

```
ReplicantPod (or TeamPod):
  store_semantic(triple)
    → write to local SQLCipher (source of truth, for backup/portability)
    → emit CNS event cns.semantic.published { pod_id, triple_id, entity, attribute }

CuratorPod sense loop:
  on CNS event cns.semantic.published
    → poll source pod's triples table: "new since last cursor"
    → insert into Curator SemanticIndex { triple, source_pod_id }
    → advance cursor

Any agent query:
  recall_semantic(entity)
    → CuratorPod SemanticIndex query
    → returns merged view with source_pod metadata
```

### Consistency Model

- Eventual, bounded by CNS event delivery (~ms in-process)
- Push-then-pull: pod writes local → fires CNS event → Curator polls pod's table
- CNS event carries triple ID + entity, not the full triple (CNS spans are small)
- Curator polls the pod's triples table directly — same deterministic passphrase
- On CuratorPod restart: cursor-based catch-up. Replays every triple published since last cursor.
- On source pod deletion while Curator was down: skip, log warning, advance cursor.

### Conflict Resolution

Two pods publish contradictory triples for the same entity+attribute. Both stored in SemanticIndex with `source_pod_id` metadata. Query returns:

```json
[
  { "entity": "Bitcoin", "attribute": "price", "value": "$100k", "source_pod": "replicant.alice", "confidence": 0.9 },
  { "entity": "Bitcoin", "attribute": "price", "value": "$50k",  "source_pod": "replicant.bob",   "confidence": 0.7 }
]
```

No merge. No winner. The consumer (agent, human) decides. Confidence weighting is a future curation concern.

---

## 3. New Types — Minimal Additions

### PodKind

```rust
// In hkask-agents/src/pod/types.rs (alongside PodLifecycleState, AgentMode)
pub enum PodKind {
    Curator,
    Team,
    Replicant,
}
```

### SemanticIndex

```rust
// In hkask-agents/src/curator/semantic_index.rs
pub struct SemanticIndex {
    store: TripleStore,           // backed by CuratorPod's own SQLCipher
    cursors: HashMap<PodID, u64>, // last-seen triple rowid per source pod
}
```

Not a new crate. Not a new database. Just a `TripleStore` on the CuratorPod's own `PerPodStorage`.

### PodDeployment extension

```rust
// Add to PodDeployment:
pub pod_kind: PodKind,

// Add to CuratorPod's PodDeployment:
pub semantic_index: Option<SemanticIndex>,
```

### PodRegistry extension

```rust
impl PodRegistry {
    /// Scan pods directory, classify by filename prefix
    pub fn scan_by_kind(&self) -> Vec<(PodKind, PodID, PathBuf)>;

    /// Find the CuratorPod (there must be exactly one)
    pub fn find_curator(&self) -> Option<(PodID, PathBuf)>;

    /// Find all TeamPods
    pub fn find_teams(&self) -> Vec<(PodID, PathBuf)>;
}
```

---

## 4. CNS Event — `cns.semantic.published`

New CnsSpan variant:

```rust
// In hkask-types/src/cns.rs
pub enum CnsSpan {
    // ...
    SemanticPublished,  // NEW: cns.semantic.published
}
```

Emitted in `PodContext::store_semantic()` after successful local write. Payload: `{ pod_id, triple_id, entity, attribute }`. Lightweight — Curator uses triple_id to poll the source pod's table.

---

## 5. Implementation Steps — Goal-Driven

### Step 1: PodKind + Filename Convention
**Contract:** `PodRegistry.scan_by_kind()` classifies every `*.db` file by prefix.
- [ ] Add `PodKind` enum to `types.rs`
- [ ] Add `scan_by_kind()`, `find_curator()`, `find_teams()` to `PodRegistry`
- [ ] Update `PodFactory::deploy()` to accept `PodKind` and produce correct filename
- [ ] Test: deploy curator → `curator.db`; deploy team → `team.7r7.db`; deploy replicant → `replicant.{webid}.db`
- [ ] Verify: `cargo test -p hkask-agents`

### Step 2: SemanticIndex + CuratorPod
**Contract:** CuratorPod has a `SemanticIndex` backed by its own SQLCipher.
- [ ] Add `SemanticIndex` struct with `TripleStore` + `cursors: HashMap<PodID, u64>`
- [ ] Add `pod_kind` and `semantic_index` fields to `PodDeployment`
- [ ] Add `CnsSpan::SemanticPublished`
- [ ] Test: create CuratorPod → verify SemanticIndex exists and is empty

### Step 3: CNS Event on Semantic Write
**Contract:** `store_semantic()` fires CNS event after local write.
- [x] Emit `cns.semantic.published` in `PodContext::store_semantic()`
- [ ] Test: write semantic triple → verify CNS span emitted with correct payload

### Step 4: Curator Sense Loop — Poll and Index
**Contract:** Curator polls source pod's triples on CNS event, inserts into SemanticIndex within 1 second.
- [x] CuratorPod subscribes to `cns.semantic.published` events
- [x] On event: open source pod's DB with deterministic passphrase, query triples since cursor
- [x] Insert into SemanticIndex with `source_pod_id` metadata
- [x] Advance cursor
- [ ] Test: Pod A writes semantic triple → verify Curator sees it within 1s
- [ ] Test: CuratorPod restart → verify catch-up replays all missed triples

### Step 5: recall_semantic() → Curator
**Contract:** `PodContext::recall_semantic()` queries CuratorPod's SemanticIndex.
- [x] Route recall_semantic through CuratorPod when available
- [ ] Test: 2 pods write contradictory triples → recall returns both with source metadata

### Step 6: TeamPod
**Contract:** Bots get delegated OCAP tokens into a shared TeamPod.
- [x] `PodFactory::deploy(PodKind::Team, "7r7")`
- [x] TeamPod stores bot episodic data in shared SQLCipher (bots can see each other's episodic)
- [x] Semantic published to Curator same as ReplicantPod
- [ ] Test: 2 bots write to TeamPod → both visible in team episodic, both published to Curator

---

## 6. What We Don't Build (Yet)

- **Cross-pod A2A query protocol** — defer to ζ.1 (open questions). Curator is the query surface for now.
- **Confidence-weighted merge** — both triples stored, consumer decides.
- **Pod deletion cleanup** — log warning on stale cursor, skip.
- **PodKind migration** — existing pods without pod_kind default to `Replicant`.

---

## 7. Principle Traceability

| Principle | How Enforced |
|-----------|-------------|
| P1 User Sovereignty | ReplicantPod owns its data. Semantic publish is one-way — Curator never writes to replicant pods. |
| P4 Clear Boundaries | Curator opens pods read-only. OCAP token gates all access. |
| P5 Essentialism | 1 new enum, 1 new struct. No new crates. No new databases. |
| P6 Space for Replicants | Each replicant gets own pod. Bots share TeamPod. |
| P9 Homeostasis | CNS event `cns.semantic.published` drives sync loop. Curator monitors sync lag. |
| P11 Digital Sphere | Episodic stays local. Semantic published lazily. Filename IS identity. |
