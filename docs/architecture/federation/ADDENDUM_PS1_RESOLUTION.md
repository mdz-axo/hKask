---
title: "Federation Design — Addendum E: PS-1 Resolution Walkthrough"
audience: [architects, developers]
last_updated: 2026-06-22
version: "0.30.0+fed"
status: "Design Decision — Resolved"
domain: "Cross-cutting"
---

# Addendum E: PS-1 Resolution — ν-Event Primacy and Canonical Time in Federation CRDTs

**Purpose:** Walk through the resolution of PS-1 (CRDT timestamps vs. ν-event canonical source) given the constraints: (a) no second-class triples, (b) single canonical time system. Resolves the apparent conflict between CRDT LWW convergence and hKask's ν-event primacy invariant.

---

## 1. The Apparent Conflict

The original federation design (FEDERATION_DESIGN.md §6.2) proposed:

> "Same triple from two replicas → LWW: highest timestamp + replica_id tiebreak"

This implies that when two servers independently publish triples about the same fact, wall-clock timestamps decide which one "wins." But hKask's foundational semantic invariant (stated in the pragmatic-semantics skill and PRINCIPLES.md P8) is:

> "The ν-event store is the sole canonical source. Semantic memory is derived — rebuildable from ν-events. If ν-events and semantic memory disagree, ν-events win."

With two servers, there are two ν-event stores. If CRDT LWW resolves conflicts via wall-clock timestamps, the "losing" server's ν-event is effectively ignored — violating ν-event primacy.

**The constraint:** No second-class triples. All triples must be first-class. Federation convergence must respect each server's ν-event authority.

---

## 2. What the Existing Code Already Provides

Four existing mechanisms make this problem simpler than it appears:

### 2.1 EAV Content Hashing (`recall_dedup::eav_hash`)

```rust
// crates/hkask-memory/src/recall_dedup.rs, line 26-33
pub fn eav_hash(triple: &Triple) -> [u8; 32] {
    let canonical = format!(
        "{}\x00{}\x00{}",
        triple.entity,
        triple.attribute,
        canonical_value(&triple.value)
    );
    *blake3::hash(canonical.as_bytes()).as_bytes()
}
```

**Key property:** The hash covers entity + attribute + canonical value ONLY. It is **metadata-independent** — timestamps, confidence, perspective, and visibility are excluded. Two triples with the same factual content always produce the same hash, regardless of which server published them or when.

### 2.2 Triple Identity (`TripleID`)

Every triple carries a `TripleID` (UUID v4). This is the triple's unique identity within its home server. Two servers independently observing the same fact will produce two different `TripleID` values — but the same EAV hash.

### 2.3 Provenance (`AccessControl::perspective`)

`SemanticIndex::insert()` stores the source pod's identity in `access.perspective`. This is already how the Curator tracks which pod contributed which triple. For federation, this extends naturally: the source **server** (replica) becomes the perspective.

### 2.4 Confidence

Every triple has a `Confidence` value (0.0–1.0). A server that observes a fact directly assigns `Confidence::full()` (1.0). A server that receives the fact via CRDT sync inherits the original confidence (which decays over time via `Confidence::decay()`).

---

## 3. The Resolution: OR-Set With EAV Hashes, Not Timestamps

### 3.1 The CRDT Key

The CRDT OR-Set key for triples is the **EAV hash**, not the triple's `TripleID` and not a timestamp:

```rust
/// Key for the federation semantic triple OR-Set.
/// Uses the EAV content hash — two servers publishing the same fact
/// produce the same key, converging automatically.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FederationTripleKey {
    /// BLAKE3 hash of (entity || \x00 || attribute || \x00 || canonical_value).
    eav_hash: [u8; 32],
}

impl FederationTripleKey {
    pub fn from_triple(triple: &Triple) -> Self {
        Self {
            eav_hash: hkask_memory::recall_dedup::eav_hash(triple),
        }
    }
}
```

### 3.2 The CRDT Dot

The dot is pure causal ordering — replica identity + monotonic counter:

```rust
/// CRDT dot — uniquely identifies a write in causal order.
/// No timestamp. No wall-clock dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dot {
    /// Which replica performed the write.
    pub replica: ReplicaId,
    /// Monotonic counter — increments on each write at this replica.
    /// Establishes causal ordering within and across replicas.
    pub counter: u64,
}
```

### 3.3 Walkthrough: Same-Fact Convergence

```
Server A (replica=alpha)                    Server B (replica=beta)
──────────────────────                      ──────────────────────

Agent observes: "sensor1 temp = 25°C"       Agent observes: "sensor1 temp = 25°C"

Triple T_a created:                         Triple T_b created:
  id: uuid-a-123                             id: uuid-b-456
  entity: "sensor1"                          entity: "sensor1"    
  attribute: "temperature"                   attribute: "temperature"
  value: "25"                                value: "25"
  confidence: 1.0                            confidence: 1.0
  temporal.valid_from: 2026-06-22T12:00:00Z  temporal.valid_from: 2026-06-22T12:00:05Z
  access.owner: webid-A                      access.owner: webid-B

NuEvent ν_a emitted:                        NuEvent ν_b emitted:
  id: event-a-1                              id: event-b-1
  timestamp: 2026-06-22T12:00:00Z            timestamp: 2026-06-22T12:00:05Z
  observer_webid: webid-A                    observer_webid: webid-B

EAV hash: 0x3f7a...                          EAV hash: 0x3f7a... (SAME!)
                                           
CRDT dot: (alpha, 42)                       CRDT dot: (beta, 17)
                                           
── SYNC ───────────────────────────────────→
                                            OR-Set merge:
                                              element 0x3f7a has dots [(alpha,42), (beta,17)]
                                              → contains() = true
                                              → Both servers agree: the fact exists
                                              → No conflict. No timestamp comparison.
```

**Result:** Both servers converge. The fact is present in both SemanticIndexes. No server's ν-event was ignored. The CRDT doesn't need to "pick a winner" because there's nothing to win — the fact is the same.

### 3.4 Walkthrough: Divergent-Fact Retention

```
Server A                                    Server B
────────                                    ────────

"sensor1 temp = 25°C" (confidence 1.0)      "sensor1 temp = 26°C" (confidence 0.8)

EAV hash: 0x3f7a...                          EAV hash: 0x8b2c... (DIFFERENT — value differs)
CRDT dot: (alpha, 42)                       CRDT dot: (beta, 17)

── SYNC ───────────────────────────────────→
                                            OR-Set merge:
                                              element 0x3f7a has dots [(alpha, 42)]
                                              element 0x8b2c has dots [(beta, 17)]
                                              → BOTH retained
                                              → SemanticIndex query returns both values
```

**Result:** Both triples exist in the merged state. The Curator's merged-lens query returns both. Resolution moves to the semantic layer:

```rust
// In CuratorAgent metacognition — resolving divergent sensor readings:
fn resolve_divergent_readings(triples: &[Triple]) -> Option<Temperature> {
    triples
        .iter()
        .max_by(|a, b| {
            // Prefer higher confidence.
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(Ordering::Equal)
                // Tiebreak: prefer local provenance.
                .then_with(|| is_local(&a.access).cmp(&is_local(&b.access)))
        })
        .map(|t| parse_temperature(&t.value))
}
```

---

## 4. The Single Canonical Time System

### 4.1 What IS the Canonical Time?

Each server's ν-event timestamps (`DateTime<Utc>`) are the canonical time for THAT server's observations. The system already uses `DateTime<Utc>` for:
- ν-event `timestamp` fields
- Triple `TemporalBounds::valid_from` and `valid_to`
- Confidence decay (`Confidence::decay()` uses `Utc::now()` at recall time)

### 4.2 Why Cross-Server Timestamp Comparison Is Never Needed

| Operation | Time Used | Cross-Server? |
|-----------|-----------|---------------|
| CRDT causal ordering | Version vectors (counters) | No — counters are per-replica |
| Same-fact convergence | EAV hash (content-addressed) | No — hash is deterministic |
| Divergent-fact retention | OR-Set add-wins (both kept) | No — no conflict to resolve |
| Confidence comparison | `Confidence` value (0.0–1.0) | No — compared by magnitude, not timestamp |
| Temporal bounds | `valid_from` / `valid_to` per triple | No — each triple carries its own bounds |
| ν-event ordering | `DateTime<Utc>` per server | No — each server is authoritative for its own events |
| User profile LWW | `DateTime<Utc>` wall-clock | Yes — ONLY here, and only for metadata |

### 4.3 The LWW Exception (User Profiles)

User profiles use LWW-Map with wall-clock timestamps. This is acceptable because:

1. **Profiles are metadata, not ν-event-grounded facts.** There is no "ν-event primacy" for profiles — they're configuration, not observations.
2. **Profile conflicts are rare.** Two admins editing the same user's display name simultaneously is an edge case.
3. **LWW is a Guideline, not a Prohibition.** Per the constraint hierarchy, Guidelines can be relaxed with reason stated.

```rust
/// User profile LWW entry — the one place wall-clock timestamps are used
/// for cross-server conflict resolution.
#[derive(Debug, Clone)]
struct LwwEntry<T> {
    value: T,
    /// Wall-clock timestamp. Acceptable because profiles are metadata,
    /// not ν-event-grounded observations.
    timestamp: DateTime<Utc>,
    /// Replica that wrote this entry (tiebreak if timestamps are equal).
    replica: ReplicaId,
}

impl<T: Clone> LwwEntry<T> {
    fn merge(&self, other: &Self) -> Self {
        match self.timestamp.cmp(&other.timestamp) {
            Ordering::Greater => self.clone(),
            Ordering::Less => other.clone(),
            Ordering::Equal => {
                // Timestamp tiebreak: higher replica_id wins (arbitrary but deterministic)
                if self.replica > other.replica { self.clone() } else { other.clone() }
            }
        }
    }
}
```

---

## 5. Constraint Verification

### 5.1 No Second-Class Triples ✅

Every triple in the CRDT is a first-class triple. The OR-Set doesn't distinguish "local" from "federated" — all elements are equal. Provenance is carried in `access.perspective` (same mechanism as intra-server SemanticIndex), not in a separate "second-class" flag.

```rust
// Both local and federated triples go through the same insert path:
semantic_index.insert(&triple, source_replica)?;
// source_replica is ReplicaId — could be local or remote.
// The triple itself carries no "class" distinction.
```

### 5.2 Single Canonical Time System ✅

The canonical time system is each server's ν-event timestamps (`DateTime<Utc>`). Cross-server temporal comparison is unnecessary because:
- CRDT convergence uses causal ordering (version vectors) and content addressing (EAV hash)
- Divergent facts are both retained — no LWW needed for triples
- Confidence provides the semantic resolution layer

### 5.3 ν-Event Primacy Preserved ✅

Each server's ν-event store remains the canonical source for that server's observations. The CRDT merged state is a **derived view** — it can be rebuilt from ν-events by replaying them through the CRDT. If the CRDT state diverges from what ν-events would produce, rebuild from ν-events.

```rust
/// FederationSync's reconcile step — ν-event-driven, not CRDT-driven:
///
/// 1. Query local ν-events since last reconciliation cursor
/// 2. Convert ν-events → triples → OR-Set adds
/// 3. Query remote deltas via CRDT sync
/// 4. Merge remote deltas into OR-Set
/// 5. Materialize OR-Set → SemanticIndex (upsert all elements)
///
/// The ν-events are the source. The CRDT is the convergence mechanism.
/// The SemanticIndex is the materialized view.
```

### 5.4 No Timestamp-Based Conflict Resolution for Triples ✅

The conflict resolution table now reads:

| Data Type | CRDT | Conflict Strategy | Rationale |
|-----------|------|-------------------|-----------|
| **Semantic triples** | OR-Set (keyed by EAV hash) | Add-wins (both retained). Same EAV hash → same element → natural convergence. Different EAV hash → both elements kept → Curator resolves. | ν-event primacy. No cross-server time comparison. |
| **User profiles** | LWW-Map | Highest wall-clock timestamp + replica_id tiebreak. | Metadata, not ν-event-grounded. Guideline-level constraint. |
| **Agent registrations** | G-Set | Union (additive). | No conflicts possible. |
| **Artifacts** | OR-Set (keyed by content hash) | Same as triples — content-addressed, both retained if different. | Content-addressed storage. |

---

## 6. Impact on FederationSync Implementation

### 6.1 CRDT Module Changes

```rust
// Before (LWW-based, problematic):
pub struct FederationSemanticSet {
    add_set: HashMap<TripleHash, VersionVector>,     // TripleHash = hash of TripleID? Ambiguous.
    remove_set: HashMap<TripleHash, VersionVector>,
    // No EAV hash. No content-addressing.
}

// After (OR-Set with EAV hashing, correct):
pub struct FederationSemanticSet {
    /// OR-Set keyed by EAV content hash.
    /// Same entity+attribute+value → same key → automatic convergence.
    add_set: HashMap<FederationTripleKey, Vec<Dot>>,
    /// Tombstones: elements that have been removed, keyed by EAV hash.
    remove_set: HashMap<FederationTripleKey, Vec<Dot>>,
    replica: ReplicaId,
    counter: AtomicU64,
}

impl FederationSemanticSet {
    /// Add a triple. The EAV hash is computed from content.
    /// Confidence and temporal bounds are stored as metadata on the payload.
    pub fn add(&mut self, triple: &Triple) -> Dot {
        let key = FederationTripleKey::from_triple(triple);
        let dot = Dot { replica: self.replica, counter: self.counter.fetch_add(1, Relaxed) };
        self.add_set.entry(key).or_default().push(dot);
        dot
    }

    /// Merge another replica's OR-Set state.
    /// Elements in other.add_set not tombstoned in self → added.
    /// Elements in other.remove_set → tombstoned in self.
    /// No timestamp comparison. Pure causal CRDT merge.
    pub fn merge(&mut self, other: &Self) {
        // For each (key, dots) in other.add_set:
        //   If key not in self.remove_set with causally-greater-or-equal dots:
        //     Add dots to self.add_set
        // For each (key, dots) in other.remove_set:
        //   If key not in self.remove_set with causally-greater-or-equal dots:
        //     Add dots to self.remove_set
        //     Remove corresponding dots from self.add_set
    }
}
```

### 6.2 Triple Payload (Separate from CRDT Key)

The CRDT tracks which EAV hashes exist. The actual triple data (confidence, temporal bounds, provenance) is stored separately:

```rust
/// Payload store — maps EAV hash → full triple data.
/// The OR-Set determines which hashes exist.
/// The payload store provides the rich triple data for those hashes.
pub struct TriplePayloadStore {
    /// EAV hash → Triple (most recent by confidence)
    payloads: HashMap<FederationTripleKey, Triple>,
}

impl TriplePayloadStore {
    /// Upsert a triple. If the same EAV hash already exists,
    /// keep the one with higher confidence.
    pub fn upsert(&mut self, triple: Triple) {
        let key = FederationTripleKey::from_triple(&triple);
        self.payloads
            .entry(key)
            .and_modify(|existing| {
                if triple.confidence > existing.confidence {
                    *existing = triple.clone();
                }
            })
            .or_insert(triple);
    }
}
```

### 6.3 ν-Event → CRDT Pipeline

```rust
impl FederationSync {
    /// Reconciliation tick — ν-event-driven, CRDT-converged.
    async fn reconcile(&self) -> Result<(), FederationSyncError> {
        // 1. Query local ν-events since last reconciliation cursor
        let local_events = self.nu_event_store
            .query_since(self.reconciliation_cursor, EventFilter::SemanticPublished)?;

        // 2. Convert ν-events → triples → OR-Set adds
        for event in &local_events {
            if let Some(triple) = self.extract_triple_from_event(event) {
                let key = FederationTripleKey::from_triple(&triple);
                self.semantic_set.add(&triple);
                self.payload_store.upsert(triple);
                self.reconciliation_cursor = event.id;
            }
        }

        // 3. CRDT sync with peers
        for peer in self.linked_peers() {
            let remote_deltas = self.sync_with_peer(peer).await?;
            self.semantic_set.merge(&remote_deltas.semantic_state);
            // Merge remote payloads
            for triple in &remote_deltas.triples {
                self.payload_store.upsert(triple.clone());
            }
        }

        // 4. Materialize OR-Set → SemanticIndex
        for key in self.semantic_set.elements() {
            if let Some(triple) = self.payload_store.get(&key) {
                self.semantic_index.insert(triple, key.source_replica())?;
            }
        }

        Ok(())
    }
}
```

---

## 7. Summary

The PS-1 conflict was an **artifact of the original LWW design**, not a fundamental tension. The resolution:

1. **OR-Set keyed by EAV hash** — same fact from two servers produces the same key, converging automatically without any timestamp comparison.
2. **Divergent facts both retained** — OR-Set add-wins keeps both. The Curator's semantic layer resolves interpretation using confidence and provenance.
3. **Single canonical time system** — each server's ν-event timestamps. Cross-server temporal comparison is never needed for triples.
4. **No second-class triples** — all triples go through the same insert path. Provenance is carried in `access.perspective`, not a class flag.
5. **ν-event primacy preserved** — the CRDT merged state is a derived view rebuildable from ν-events. ν-events remain the canonical source.
6. **LWW reserved for user profiles only** — where it's a Guideline, not a Prohibition, and wall-clock timestamps are an acceptable heuristic for metadata.

The CRDT module now implements pure causal ordering. The only clock dependency is `AtomicU64` counters. No wall-clock comparison enters the triple merge path.
