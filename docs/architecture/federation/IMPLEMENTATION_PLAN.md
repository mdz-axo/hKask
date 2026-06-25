---
title: "Federation v2 — Implementation Plan"
audience: [developers]
last_updated: 2026-06-22
version: "0.31.0"
status: "Ready for implementation"
domain: "Cross-cutting"
---

# Federation v2 — Detailed Implementation Plan

**Purpose:** Step-by-step coding plan for Phase 1 (Core Sync MVP), Phase 2 (Invitation & Lifecycle), and Phase 3 (Completeness). Each task includes concrete file paths, expected type signatures, test strategies, and verification criteria.

---

## Phase 0 — Foundation (Enable Federation)

Before federation code can be written, the type system must be extended.

### Task 0.1: Extend `OcapTokenKind` with `Federation`

**File:** `crates/hkask-types/src/curation.rs`

```rust
pub enum OcapTokenKind {
    Curation,       // existing
    SpecCurate,     // existing
    Federation,     // NEW — authority to establish and manage federation links
}
```

**Verification:** `cargo check -p hkask-types` — zero warnings. All existing matches on `OcapTokenKind` must be updated (add `Federation` arm).

**Affected files:** Search for `match.*OcapTokenKind` — likely `hkask-capability`, `hkask-cns`, `hkask-agents`.

### Task 0.2: Extend `CnsSpan` with Phase 1 Spans

**File:** `crates/hkask-types/src/cns.rs`

Add 5 variants:

```rust
pub enum CnsSpan {
    // ... existing variants ...
    FederationCrdtMerge,         // CRDT converge event with delta metadata
    FederationLinkEstablished,    // Two Curators establish link
    FederationLinkLost,           // Curator-to-Curator connection lost
    FederationLinkDegraded,       // Sync timeout — partition or peer death
    FederationMemberLeft,         // Server voluntarily left federation
}
```

**Verification:** Update `as_str()`, `Display`, `FromStr`, and test `cnsspan_exhaustive_match_covers_all_canonical`.

### Task 0.3: Create `hkask-federation` Crate

**File:** `crates/hkask-federation/Cargo.toml`

```toml
[package]
name = "hkask-federation"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Federation CRDT sync, link lifecycle, and merged registries for hKask"

[dependencies]
hkask-types = { path = "../hkask-types" }
hkask-ports = { path = "../hkask-ports" }
hkask-storage = { path = "../hkask-storage" }
hkask-memory = { path = "../hkask-memory" }
hkask-cns = { path = "../hkask-cns" }
hkask-agents = { path = "../hkask-agents" }
serde.workspace = true
serde_json.workspace = true
chrono.workspace = true
thiserror.workspace = true
tracing.workspace = true
tokio.workspace = true
blake3.workspace = true
uuid.workspace = true

[dev-dependencies]
hkask-test-harness = { path = "../hkask-test-harness" }
proptest = "1"
tokio = { workspace = true, features = ["macros", "rt", "time"] }

[[test]]
name = "crdt_contract"
path = "tests/contract/crdt_contract.rs"

[[test]]
name = "federation_integration"
path = "tests/integration/federation_integration.rs"
```

**File:** `crates/hkask-federation/src/lib.rs`

```rust
//! hKask Federation — CRDT-synced curator federations
//!
//! Three modules:
//! - `crdt`: General-purpose CRDT data structures (OR-Set, LWW-Map, G-Set)
//! - `sync`: FederationSync (sync loop) + FederationLinkManager (lifecycle)
//! - `registry`: FederationRegistry (merged user/agent resolution)

pub mod crdt;
pub mod registry;
pub mod sync;

pub use crdt::{Dot, FederationSemanticSet, LWWMap, GSet, VersionVector};
pub use registry::FederationRegistry;
pub use sync::{FederationLinkManager, FederationSync, LinkState};
```

**File:** `Cargo.toml` (workspace root) — add `"crates/hkask-federation"` to `[workspace].members`.

**Verification:** `cargo check -p hkask-federation` — skeleton compiles.

---

## Phase 1 — Core CRDT Sync (MVP)

### Task 1.1: Implement `VersionVector`

**File:** `crates/hkask-federation/src/crdt/version_vector.rs`

```rust
/// Causal ordering: maps replica → counter.
/// Merge is element-wise MAX.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionVector {
    entries: HashMap<ReplicaId, u64>,
}

impl VersionVector {
    pub fn new() -> Self;
    pub fn increment(&mut self, replica: ReplicaId) -> u64;
    pub fn get(&self, replica: &ReplicaId) -> u64;
    /// a dominates b if ∀r: a[r] ≥ b[r] and ∃r: a[r] > b[r]
    pub fn dominates(&self, other: &VersionVector) -> bool;
    /// Element-wise MAX
    pub fn merge(&self, other: &VersionVector) -> VersionVector;
}
```

**Tests** (`crates/hkask-federation/src/crdt/version_vector.rs` — `#[cfg(test)]`):

| Test | Property |
|------|----------|
| `merge_is_commutative` | a.merge(b) == b.merge(a) |
| `merge_is_associative` | a.merge(b).merge(c) == a.merge(b.merge(c)) |
| `merge_is_idempotent` | a.merge(a) == a |
| `dominates_reflexive` | a.dominates(a) |
| `dominates_transitive` | a.dominates(b) ∧ b.dominates(c) ⇒ a.dominates(c) |
| `merge_advances_both` | a.merge(b).dominates(a) ∧ a.merge(b).dominates(b) |
| `empty_dominated_by_all` | VersionVector::new().dominates(x) is false (unless x is also empty) |

### Task 1.2: Implement `Dot` and `FederationTripleKey`

**File:** `crates/hkask-federation/src/crdt/dot.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dot {
    pub replica: ReplicaId,
    pub counter: u64,
}
```

**File:** `crates/hkask-federation/src/crdt/triple_key.rs`

```rust
/// CRDT key for semantic triples — EAV content hash.
/// Same entity+attribute+value → same key → automatic convergence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FederationTripleKey {
    eav_hash: [u8; 32],
}

impl FederationTripleKey {
    pub fn from_triple(triple: &hkask_storage::Triple) -> Self {
        Self { eav_hash: hkask_memory::recall_dedup::eav_hash(triple) }
    }
}
```

**Test:** `from_triple_produces_same_hash_for_same_eav` — two triples with same entity+attribute+value but different `TripleID`, timestamps, and confidence produce identical keys.

### Task 1.3: Implement `ORSet<T>`

**File:** `crates/hkask-federation/src/crdt/or_set.rs`

```rust
/// Observed-Remove Set — elements can be added and removed.
/// Removals observe specific adds. Concurrent add+remove → add wins (add-bias).
pub struct ORSet<T: Hash + Eq + Clone> {
    add_set: HashMap<T, Vec<Dot>>,
    remove_set: HashMap<T, Vec<Dot>>,
    replica: ReplicaId,
    counter: AtomicU64,
}

impl<T: Hash + Eq + Clone> ORSet<T> {
    pub fn new(replica: ReplicaId) -> Self;
    pub fn add(&mut self, element: T) -> Dot;
    pub fn remove(&mut self, element: &T);
    pub fn contains(&self, element: &T) -> bool;
    pub fn elements(&self) -> HashSet<&T>;
    pub fn merge(&mut self, other: &Self);
    pub fn version_vector(&self) -> VersionVector;
}
```

**Merge algorithm** (core correctness):

```rust
pub fn merge(&mut self, other: &Self) {
    // For each (element, dots) in other.add_set:
    for (element, other_dots) in &other.add_set {
        let my_removed = self.remove_set.get(element);
        let surviving: Vec<Dot> = other_dots.iter()
            .filter(|dot| {
                // Element survives if it's not in self.remove_set with a
                // causally-greater-or-equal dot
                my_removed.map_or(true, |removed_dots| {
                    !removed_dots.iter().any(|rd| rd.counter >= dot.counter && rd.replica == dot.replica)
                })
            })
            .copied()
            .collect();
        if !surviving.is_empty() {
            self.add_set.entry(element.clone()).or_default().extend(surviving);
        }
    }
    // Union remove sets
    for (element, dots) in &other.remove_set {
        self.remove_set.entry(element.clone()).or_default().extend(dots.clone());
    }
}
```

**Property-based tests** (`crates/hkask-federation/tests/contract/crdt_contract.rs`):

| Test | Property | Strategy |
|------|----------|----------|
| `merge_commutative` | a.merge(b) == b.merge(a) | Random sequences of adds/removes on two replicas |
| `merge_associative` | a.merge(b).merge(c) == a.merge(b.merge(c)) | Three-replica random sequences |
| `merge_idempotent` | a.merge(a) == a | Single replica, random ops |
| `add_then_contains` | after add(x), contains(x) == true | Deterministic |
| `remove_then_not_contains` | after remove(x), contains(x) == false | Deterministic |
| `concurrent_add_remove_add_wins` | add(x) on A, remove(x) on B, merge → contains(x) == true | Two-replica concurrent |
| `causal_remove_wins` | add(x) on A, sync, remove(x) on A, add(x) on B (before sync), sync → contains(x) == false | Causal ordering |
| `elements_is_consistent` | elements() returns exactly the add_set minus removed | Deterministic |

### Task 1.4: Implement `LWWMap<K,V>` and `GSet<T>`

**File:** `crates/hkask-federation/src/crdt/lww_map.rs`

```rust
pub struct LWWMap<K: Hash + Eq, V: Clone> {
    entries: HashMap<K, LwwEntry<V>>,
}

struct LwwEntry<V> {
    value: V,
    timestamp: DateTime<Utc>,
    replica: ReplicaId,
}

impl<K: Hash + Eq, V: Clone> LWWMap<K, V> {
    pub fn new() -> Self;
    pub fn insert(&mut self, key: K, value: V, timestamp: DateTime<Utc>, replica: ReplicaId);
    pub fn get(&self, key: &K) -> Option<&V>;
    pub fn remove(&mut self, key: &K);
    pub fn merge(&mut self, other: &Self);  // LWW: highest timestamp wins
}
```

**File:** `crates/hkask-federation/src/crdt/g_set.rs`

```rust
pub struct GSet<T: Hash + Eq> {
    elements: HashSet<T>,
}

impl<T: Hash + Eq> GSet<T> {
    pub fn new() -> Self;
    pub fn insert(&mut self, element: T);
    pub fn contains(&self, element: &T) -> bool;
    pub fn elements(&self) -> impl Iterator<Item = &T>;
    pub fn merge(&mut self, other: &Self);  // union
}
```

**Tests:** Same property-based approach as OR-Set. LWW: test `latest_timestamp_wins` and `replica_tiebreak`. GSet: test `union_is_commutative` and `insert_is_idempotent`.

### Task 1.5: Implement `TriplePayloadStore`

**File:** `crates/hkask-federation/src/sync/payload_store.rs`

```rust
/// Maps EAV hash → full Triple. OR-Set determines existence.
/// PayloadStore upserts by confidence.
pub struct TriplePayloadStore {
    payloads: HashMap<FederationTripleKey, Triple>,
}

impl TriplePayloadStore {
    pub fn new() -> Self;
    pub fn upsert(&mut self, triple: Triple);
    pub fn get(&self, key: &FederationTripleKey) -> Option<&Triple>;
    pub fn remove(&mut self, key: &FederationTripleKey);
    pub fn iter(&self) -> impl Iterator<Item = (&FederationTripleKey, &Triple)>;
}
```

**Test:** `upsert_keeps_higher_confidence` — insert triple with confidence 0.5, upsert same EAV with confidence 0.9, verify stored confidence is 0.9. `upsert_preserves_lower` — insert 0.9, upsert 0.5, verify stored is still 0.9.

### Task 1.6: Implement `FederationTransport` Trait and Test Adapter

**File:** `crates/hkask-ports/src/federation.rs` (NEW)

```rust
use serde::{Deserialize, Serialize};

/// Replica identifier — unique per hKask server in the federation.
pub type ReplicaId = String;

/// Messages exchanged between federation peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationMessage {
    SyncRequest { version_vector: HashMap<ReplicaId, u64> },
    SyncResponse { deltas: FederationDelta, version_vector: HashMap<ReplicaId, u64> },
    Invitation(FederationInvitation),
    Accept(FederationAcceptance),
    Reject(FederationRejection),
    LinkPause { paused_by: ReplicaId, reason: String },
    LinkResume { resumed_by: ReplicaId },
    MembershipRevoked { revoked_by: ReplicaId, reason: String },
    FederationGoodbye { leaver: ReplicaId, reason: String },
}

pub trait FederationTransport: Send + Sync {
    async fn send(&self, peer: ReplicaId, message: FederationMessage) -> Result<(), FederationTransportError>;
    async fn recv(&self) -> Result<(ReplicaId, FederationMessage), FederationTransportError>;
    fn simulate_partition(&self, peer: ReplicaId);
    fn heal_partition(&self, peer: ReplicaId);
}

#[derive(Debug, thiserror::Error)]
pub enum FederationTransportError {
    #[error("peer not found: {0}")]
    PeerNotFound(ReplicaId),
    #[error("peer partitioned: {0}")]
    PeerPartitioned(ReplicaId),
    #[error("transport error: {0}")]
    Transport(String),
}
```

**File:** `crates/hkask-federation/src/sync/transport.rs`

```rust
/// In-memory transport for unit testing — no Matrix dependency.
/// Supports partition simulation and healing.
pub struct InMemoryFederationTransport {
    /// Per-replica message queues. Keyed by (from, to).
    queues: Arc<RwLock<HashMap<(ReplicaId, ReplicaId), VecDeque<FederationMessage>>>>,
    /// Partitioned replicas — messages are dropped.
    partitions: Arc<RwLock<HashSet<ReplicaId>>>,
    /// This replica's own ID.
    local_replica: ReplicaId,
}
```

**Test:** `partition_drops_messages` — send during partition, verify not received. `heal_delivers_queued` — queue during partition, heal, verify received.

### Task 1.7: Implement `FederationSyncPort` Trait

**File:** `crates/hkask-ports/src/federation.rs` (append)

```rust
pub trait FederationSyncPort: Send + Sync {
    fn query_public_since(&self, cursor: u64, limit: usize) -> Result<Vec<Triple>, FederationSyncError>;
    fn insert_federated(&self, triple: &Triple, source: ReplicaId) -> Result<(), FederationSyncError>;
    fn cursor_for(&self, source: &ReplicaId) -> u64;
    fn advance_cursor(&mut self, source: ReplicaId, cursor: u64);
}

#[derive(Debug, thiserror::Error)]
pub enum FederationSyncError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("not found")]
    NotFound,
}
```

**File:** `crates/hkask-ports/src/lib.rs` — add `pub mod federation;` and re-export `FederationTransport`, `FederationSyncPort`, `FederationSyncError`, `FederationMessage`, `ReplicaId`.

**Adapter:** Implement `FederationSyncPort` for `SemanticIndex`:

**File:** `crates/hkask-agents/src/curator/semantic_index.rs` (append)

```rust
impl FederationSyncPort for SemanticIndex {
    fn query_public_since(&self, cursor: u64, limit: usize) -> Result<Vec<Triple>, FederationSyncError> {
        // SELECT * FROM triples WHERE rowid > cursor AND visibility = 'public' LIMIT limit
    }
    fn insert_federated(&self, triple: &Triple, source: ReplicaId) -> Result<(), FederationSyncError> {
        self.insert(triple, PodID::from_uuid(/* convert ReplicaId */))?;
        Ok(())
    }
    // ...
}
```

### Task 1.8: Implement `FederationSync`

**File:** `crates/hkask-federation/src/sync/federation_sync.rs`

```rust
/// Background CRDT sync loop — the core of federation.
pub struct FederationSync {
    semantic_set: ORSet<FederationTripleKey>,
    payload_store: TriplePayloadStore,
    user_map: LWWMap<WebID, UserProfile>,
    agent_set: GSet<RegisteredAgent>,
    artifact_set: ORSet<ArtifactKey>,
    transport: Arc<dyn FederationTransport>,
    sync_port: Arc<dyn FederationSyncPort>,
    peers: HashMap<ReplicaId, FederationPeer>,
    interval: Duration,
    event_sink: Arc<dyn NuEventSink>,
    reconciliation_cursor: EventID,
    local_replica: ReplicaId,
}

impl FederationSync {
    /// Start the sync loop.
    ///
    /// expect: "Federated Curators converge on public memory"
    /// [P3] Motivating: Generative Space — cross-server knowledge sharing
    /// [P9] Constraining: Homeostatic Self-Regulation — CNS-observed sync
    /// pre:  sync_port initialized with local SemanticIndex data
    /// pre:  At least one peer in Linked state
    /// post: On each interval, deltas sent to Linked peers, received deltas merged
    /// post: CNS span FederationCrdtMerge emitted per successful merge
    /// post: CNS span FederationLinkDegraded emitted for consecutive failures > 3
    /// test: InMemoryFederationTransport, two replicas, insert triple on A,
    ///       verify triple appears on B within sync_interval * 2
    pub async fn run(&self, cancel: watch::Receiver<bool>) { ... }

    /// Query federation health.
    pub fn health(&self) -> FederationHealth { ... }

    /// Query federation status (all links).
    pub fn status(&self) -> FederationStatus { ... }

    // ── Internal ──
    async fn tick(&self) -> Result<(), FederationSyncError> { ... }
    async fn sync_with_peer(&self, peer: &ReplicaId) -> Result<FederationDelta, FederationSyncError> { ... }
    fn reconcile(&mut self, deltas: FederationDelta) { ... }
    fn emit_cns_span(&self, span: CnsSpan, metadata: Value) { ... }
}
```

**Tick loop pseudocode:**

```rust
async fn tick(&self) -> Result<(), FederationSyncError> {
    // 1. Pull local ν-events since last reconciliation cursor
    let local_events = self.query_local_events().await?;
    let local_deltas = self.extract_deltas(local_events);

    // 2. For each Linked peer, send SYNC_REQUEST
    for peer in self.linked_peers() {
        match self.sync_with_peer(&peer.replica).await {
            Ok(deltas) => {
                self.reconcile(deltas);
                self.emit_cns_span(CnsSpan::FederationCrdtMerge, json!({
                    "peer": peer.replica,
                    "triples_added": deltas.triples_added,
                    "latency_ms": deltas.latency_ms,
                }));
            }
            Err(_) => {
                let failures = peer.failed_attempts.fetch_add(1, Relaxed) + 1;
                if failures > self.max_sync_failures {
                    peer.transition_to(LinkState::Degraded { ... });
                    self.emit_cns_span(CnsSpan::FederationLinkDegraded, json!({
                        "peer": peer.replica,
                        "failed_attempts": failures,
                    }));
                }
            }
        }
    }

    // 3. Check Degraded peers for recovery
    for peer in self.degraded_peers() {
        if peer.downtime() > self.max_degraded_duration {
            peer.transition_to(LinkState::Revoked { ... });
            self.emit_cns_span(CnsSpan::FederationMemberLeft, json!({
                "peer": peer.replica,
                "reason": "extended_degradation",
            }));
        }
    }

    Ok(())
}
```

### Task 1.9: Integration Test — Two Replicas Converge

**File:** `crates/hkask-federation/tests/integration/federation_integration.rs`

```rust
#[tokio::test]
async fn two_replicas_converge_on_same_fact() {
    let transport = Arc::new(InMemoryFederationTransport::new());
    let transport_a = transport.for_replica("alpha");
    let transport_b = transport.for_replica("beta");

    let (sync_a, sync_b) = setup_two_replicas(transport_a, transport_b).await;

    // Establish link
    sync_a.link_manager().establish_link("beta").await.unwrap();
    assert_eq!(sync_a.link_state("beta"), LinkState::Linked);
    assert_eq!(sync_b.link_state("alpha"), LinkState::Linked);

    // Insert triple on A
    let triple = make_triple("sensor1", "temperature", "25", webid_a);
    sync_a.sync_port().insert_federated(&triple, "alpha".into()).unwrap();

    // Run one sync cycle
    sync_a.tick().await.unwrap();
    sync_b.tick().await.unwrap();

    // Verify triple exists on B
    let results = sync_b.sync_port().query_public_since(0, 10).unwrap();
    assert!(results.iter().any(|t| t.entity == "sensor1" && t.attribute == "temperature"));
}

#[tokio::test]
async fn divergent_facts_both_retained() {
    // A publishes (sensor1, temp, 25), B publishes (sensor1, temp, 26)
    // After sync, both values are present in both SemanticIndexes
}

#[tokio::test]
async fn partition_triggers_degraded_state() {
    // Simulate partition, verify Degraded, heal, verify recovery to Linked
}
```

---

## Phase 2 — Invitation and Lifecycle

### Task 2.1: Implement `FederationLink` and `LinkState`

**File:** `crates/hkask-federation/src/sync/link.rs`

```rust
pub struct FederationLink {
    pub peer_replica: ReplicaId,
    pub peer_server_domain: String,
    pub peer_matrix_domain: String,
    pub peer_curator_matrix_id: String,
    pub state: RwLock<LinkState>,
}

pub enum LinkState {
    Isolated,
    Invited { invited_at: DateTime<Utc>, expires_at: DateTime<Utc> },
    Linked { established_at: DateTime<Utc> },
    Paused { paused_at: DateTime<Utc>, reason: String, initiated_by: ReplicaId },
    Degraded { degraded_at: DateTime<Utc>, failed_attempts: u64, last_success_at: DateTime<Utc> },
    Revoked { revoked_at: DateTime<Utc>, reason: String, initiated_by: ReplicaId, scope: RevocationScope },
}

pub enum RevocationScope {
    SingleMember,
    FederationDissolved,
    VoluntaryDeparture,
}
```

### Task 2.2: Implement `FederationLinkManager`

**File:** `crates/hkask-federation/src/sync/link_manager.rs`

```rust
pub struct FederationLinkManager {
    links: RwLock<HashMap<ReplicaId, FederationLink>>,
    transport: Arc<dyn FederationTransport>,
    local_replica: ReplicaId,
    event_sink: Arc<dyn NuEventSink>,
}

impl FederationLinkManager {
    // ── Invitation ──
    pub async fn invite(&self, peer: ReplicaId, server_domain: &str, matrix_domain: &str, matrix_id: &str, message: Option<&str>) -> Result<(), LinkError>;
    pub async fn accept(&self, invitation_id: &str) -> Result<(), LinkError>;
    pub async fn reject(&self, invitation_id: &str, reason: Option<&str>) -> Result<(), LinkError>;

    // ── Pause/Resume ──
    pub async fn pause(&self, peer: ReplicaId, reason: &str) -> Result<(), LinkError>;
    pub async fn resume(&self, peer: ReplicaId) -> Result<(), LinkError>;

    // ── Revocation ──
    pub async fn revoke_member(&self, peer: ReplicaId, reason: &str) -> Result<(), LinkError>;
    pub async fn leave(&self, reason: &str) -> Result<(), LinkError>;
    pub async fn dissolve(&self, reason: &str) -> Result<(), LinkError>;

    // ── Query ──
    pub fn link_state(&self, peer: &ReplicaId) -> LinkState;
    pub fn linked_peers(&self) -> Vec<ReplicaId>;
    pub fn pending_invitations(&self) -> Vec<(ReplicaId, DateTime<Utc>)>;
}
```

**State transition validation** (`FederationLink::transition_to`):

```rust
impl FederationLink {
    pub fn transition_to(&mut self, new_state: LinkState) -> Result<(), LinkError> {
        match (&self.state, &new_state) {
            (LinkState::Isolated, LinkState::Invited { .. }) => Ok(()),
            (LinkState::Invited { .. }, LinkState::Linked { .. }) => Ok(()),
            (LinkState::Invited { .. }, LinkState::Isolated) => Ok(()),  // timeout/reject
            (LinkState::Linked { .. }, LinkState::Paused { .. }) => Ok(()),
            (LinkState::Paused { .. }, LinkState::Linked { .. }) => Ok(()),
            (LinkState::Linked { .. }, LinkState::Degraded { .. }) => Ok(()),
            (LinkState::Degraded { .. }, LinkState::Linked { .. }) => Ok(()),
            (LinkState::Degraded { .. }, LinkState::Revoked { .. }) => Ok(()),
            (LinkState::Paused { .. }, LinkState::Degraded { .. }) => Ok(()),
            (LinkState::Paused { .. }, LinkState::Revoked { .. }) => Ok(()),
            (LinkState::Linked { .. }, LinkState::Revoked { .. }) => Ok(()),
            (same, _) if std::mem::discriminant(same) == std::mem::discriminant(&new_state) => Ok(()),
            _ => Err(LinkError::InvalidTransition { from: self.state.to_string(), to: new_state.to_string() }),
        }
    }
}
```

**Tests:** Each valid transition verified. Each invalid transition produces `InvalidTransition` error. `Degraded → Linked` after successful sync. `Degraded → Revoked` after timeout.

### Task 2.3: Implement `InvitationPolicy` Trait

**File:** `crates/hkask-ports/src/federation.rs` (append)

```rust
pub trait InvitationPolicy: Send + Sync {
    fn evaluate(&self, invitation: &FederationInvitation) -> InvitationDecision;
}

pub enum InvitationDecision {
    Accept,
    Reject { reason: String },
    DeferToAdmin,
}
```

**File:** `crates/hkask-federation/src/sync/invitation_policy.rs`

```rust
pub struct ManualInvitationPolicy;
pub struct AllowListInvitationPolicy { allowed: HashSet<ReplicaId> }
pub struct RateLimitingInvitationPolicy<P: InvitationPolicy> { inner: P, window: Duration, max_invites: u64 }
```

### Task 2.4: Wire `FederationLinkManager` into `CuratorAgent`

**File:** `crates/hkask-agents/src/curator_agent/mod.rs` (append)

```rust
impl CuratorAgent {
    pub fn with_federation(
        mut self,
        link_manager: Arc<FederationLinkManager>,
    ) -> Self {
        self.link_manager = Some(link_manager);
        self
    }
}
```

**File:** `crates/hkask-agents/src/curator/curation_loop.rs` — handle federation directives in `compute()`:

```rust
CuratorDirective::InviteToFederation { .. } => {
    self.link_manager.as_ref().map(|lm| lm.invite(...));
}
CuratorDirective::PauseFederationLink { .. } => {
    self.link_manager.as_ref().map(|lm| lm.pause(...));
}
// ... etc.
```

### Task 2.5: Extend CNS Spans (Phase 2)

**File:** `crates/hkask-types/src/cns.rs` — add 7 variants:

```rust
FederationInviteSent,
FederationInviteReceived,
FederationInviteAccepted,
FederationInviteRejected,
FederationInviteExpired,
FederationLinkPaused,
FederationLinkResumed,
```

---

## Phase 3 — Completeness

### Task 3.1: Implement `FederationRegistry`

**File:** `crates/hkask-federation/src/registry/federation_registry.rs`

```rust
pub struct FederationRegistry {
    local_users: Arc<UserStore>,
    local_agents: Arc<AgentRegistry>,
    remote_users: LWWMap<WebID, FederatedUserProfile>,
    remote_agents: GSet<FederatedAgentEntry>,
}

impl FederationRegistry {
    pub fn resolve_user(&self, webid: &WebID) -> Option<UserProfile>;
    pub fn resolve_agent(&self, webid: &WebID) -> Option<AgentInfo>;
    pub fn resolve_matrix_target(&self, webid: &WebID) -> Option<String>;
    pub fn merge_remote_users(&mut self, users: LWWMap<WebID, FederatedUserProfile>);
    pub fn merge_remote_agents(&mut self, agents: GSet<FederatedAgentEntry>);
}
```

### Task 3.2: Implement Federation Algedonic Thresholds

**File:** `crates/hkask-cns/src/set_points.rs` (append)

```rust
pub struct SetPoints {
    // ... existing fields ...
    pub fed_sync_latency_warning_ms: u64,       // 5000
    pub fed_sync_latency_critical_ms: u64,      // 30000
    pub fed_crdt_divergence_warning_factor: f64, // 2.0
    pub fed_link_downtime_warning_secs: u64,     // 3600
    pub fed_link_downtime_critical_secs: u64,    // 86400
    pub fed_max_pause_duration_hours: u64,       // 24
    pub fed_invitation_rate_warning_per_hour: u64, // 5
    pub fed_registry_divergence_warning: u64,    // 10
}
```

**File:** `crates/hkask-federation/src/sync/federation_sync.rs` — integrate with health check:

```rust
pub fn health(&self) -> FederationHealth {
    let latency = self.rolling_latency.average();
    let divergence = self.last_delta_size as f64 / self.baseline_delta_size.max(1) as f64;
    FederationHealth {
        sync_latency_ms: latency,
        sync_latency_warning: latency > self.set_points.fed_sync_latency_warning_ms,
        sync_latency_critical: latency > self.set_points.fed_sync_latency_critical_ms,
        crdt_divergence_factor: divergence,
        crdt_divergence_warning: divergence > self.set_points.fed_crdt_divergence_warning_factor,
        member_count: self.peers.len(),
        degraded_count: self.degraded_peers().len(),
        last_sync: self.last_sync_at,
    }
}
```

### Task 3.3: Implement `FederationHealthModel`

**File:** `crates/hkask-agents/src/curator_agent/federation_health.rs` (NEW)

```rust
pub struct FederationHealthModel {
    latency_window: Vec<u64>,
    expected_merge_frequency: f64,
    expected_member_count: usize,
    confidence: Confidence,
    last_updated: DateTime<Utc>,
}

impl FederationHealthModel {
    pub fn update(&mut self, health: &FederationHealth);
    pub fn anomaly_score(&self, current: &FederationHealth) -> f64;
}
```

### Task 3.4: Implement `MatrixFederationTransport`

**File:** `crates/hkask-federation/src/sync/matrix_transport.rs`

```rust
pub struct MatrixFederationTransport {
    local_matrix: Arc<Mutex<MatrixTransport>>,
    local_replica: ReplicaId,
    inbox: mpsc::UnboundedReceiver<(ReplicaId, FederationMessage)>,
}

impl FederationTransport for MatrixFederationTransport {
    async fn send(&self, peer: ReplicaId, message: FederationMessage) -> Result<(), FederationTransportError> {
        // Serialize message → Matrix custom event → send to @curator:<peer-domain>
    }
    async fn recv(&self) -> Result<(ReplicaId, FederationMessage), FederationTransportError> { ... }
    fn simulate_partition(&self, _peer: ReplicaId) { /* no-op in production */ }
    fn heal_partition(&self, _peer: ReplicaId) { /* no-op in production */ }
}
```

### Task 3.5: Extend CNS Spans (Phase 3)

**File:** `crates/hkask-types/src/cns.rs` — add remaining 7 variants:

```rust
FederationMemberRevoked,
FederationDissolved,
FederationRegistrySync,
FederationArtifactSync,
FederationConduitRoute,
FederationConduitRouteLost,
FederationCrdtConflict,
```

### Task 3.6: Multi-Server Integration Test Harness

**File:** `crates/hkask-federation/tests/integration/multi_server.rs`

```rust
#[tokio::test]
async fn three_servers_gossip_revocation() {
    // Setup: A, B, C all federated
    // A revokes B → A transitions B to Revoked
    // A gossips to C → C transitions B to Revoked
    // B receives MEMBERSHIP_REVOKED → transitions A to Revoked
    // A and C remain federated
}

#[tokio::test]
async fn invitation_expiry() {
    // A invites B, B never responds, 24h passes, A transitions to Isolated
}

#[tokio::test]
async fn pause_during_partition_triggers_degraded() {
    // A pauses B, LINK_PAUSE lost due to partition
    // B sees sync timeout → Degraded
    // A sees B's sync requests stop → Degraded (or stays Paused with no peer feedback)
}
```

### Task 3.7: Chaos Testing

**File:** `crates/hkask-federation/src/sync/chaos_transport.rs`

```rust
/// Wraps any FederationTransport and injects:
/// - Latency (configurable distribution)
/// - Packet loss (configurable percentage)
/// - Partition (configurable duration)
/// - Duplication (configurable percentage)
pub struct ChaosFederationTransport<T: FederationTransport> {
    inner: T,
    latency: ChaosParam<Duration>,
    packet_loss: ChaosParam<f64>,     // 0.0–1.0
    duplication: ChaosParam<f64>,     // 0.0–1.0
    rng: StdRng,
}

#[tokio::test]
async fn crdt_converges_under_chaos() {
    // 3 replicas, chaos transport with 10% packet loss, 50ms latency
    // Each replica adds 100 random triples
    // After 100 sync intervals, verify all 3 replicas have identical OR-Set state
}
```

---

## Dependency Graph (Post-Implementation)

```
hkask-types (extended: OcapTokenKind::Federation, CnsSpan::Federation*, CuratorDirective::*)
    ↑
hkask-ports (extended: FederationTransport, FederationSyncPort, FederationRegistryPort, InvitationPolicy)
    ↑
hkask-federation ─────────────┐
    ↑                         │
hkask-memory (eav_hash)       │
    ↑                         │
hkask-storage (Triple, TripleStore)  │
    ↑                         │
hkask-agents (SemanticIndex impl FederationSyncPort) ← hkask-cns (SetPoints extended)
    ↑
hkask-services-context (wires FederationSync into AgentService)
    ↑
hkask-cli / hkask-api (federation commands)
```

---

## Task Dependency Graph

```
Phase 0 (Foundation)
├── 0.1 OcapTokenKind::Federation ──────────────────────┐
├── 0.2 CnsSpan Phase 1 variants ───────────────────────┤
└── 0.3 hkask-federation crate skeleton ────────────────┤
                                                        │
Phase 1 (Core Sync)                                     │
├── 1.1 VersionVector ──────────────────┐               │
├── 1.2 Dot + FederationTripleKey ──────┤               │
├── 1.3 ORSet<T> (depends on 1.1, 1.2) ─┤               │
├── 1.4 LWWMap + GSet ──────────────────┤               │
├── 1.5 TriplePayloadStore ─────────────┤               │
├── 1.6 FederationTransport + InMemory  │               │
├── 1.7 FederationSyncPort + impl ──────┤               │
├── 1.8 FederationSync ─────────────────┤               │
└── 1.9 Integration tests ──────────────┘               │
                                                        │
Phase 2 (Invitation & Lifecycle)                        │
├── 2.1 FederationLink + LinkState ─────┐               │
├── 2.2 FederationLinkManager ──────────┤               │
├── 2.3 InvitationPolicy ───────────────┤               │
├── 2.4 Wire into CuratorAgent ─────────┤               │
└── 2.5 CnsSpan Phase 2 variants ───────┘               │
                                                        │
Phase 3 (Completeness)                                  │
├── 3.1 FederationRegistry ─────────────┐               │
├── 3.2 Algedonic thresholds ───────────┤               │
├── 3.3 FederationHealthModel ──────────┤               │
├── 3.4 MatrixFederationTransport ──────┤               │
├── 3.5 CnsSpan Phase 3 variants ───────┤               │
├── 3.6 Multi-server integration ───────┤               │
└── 3.7 Chaos testing ──────────────────┘               │
```

---

## Verification Checklist Per Phase

### Phase 1 Verification

```
[ ] cargo check --workspace — 0 errors, 0 warnings
[ ] cargo test -p hkask-federation — all CRDT property tests pass
[ ] cargo test -p hkask-federation --test crdt_contract — commutativity, associativity, idempotence
[ ] cargo test -p hkask-federation --test federation_integration — two replicas converge
[ ] Same-fact convergence: insert triple on A, verify appears on B within 2× sync interval
[ ] Divergent-fact retention: insert different values on A and B, verify both present
[ ] Degraded detection: partition, verify Degraded state, heal, verify recovery to Linked
[ ] CNS span emission: grep logs for "cns.federation.crdt_merge"
```

### Phase 2 Verification

```
[ ] cargo test -p hkask-federation — link state machine tests pass
[ ] Valid transitions: all 12 valid transitions succeed
[ ] Invalid transitions: all invalid transitions produce LinkError::InvalidTransition
[ ] Invitation expiry: verify timeout → Isolated
[ ] Pause notification: verify peer transitions to Paused
[ ] Lost LINK_PAUSE: verify peer detects Degraded
[ ] ManualInvitationPolicy: verify DeferToAdmin on unknown peer
[ ] AllowListInvitationPolicy: verify auto-accept on configured peer
```

### Phase 3 Verification

```
[ ] FederationRegistry: resolve user from remote server
[ ] Algedonic thresholds: CNS alert fires when sync latency > 5000ms
[ ] Health model: anomaly_score > threshold when latency spikes
[ ] Matrix transport: send/receive via real Matrix homeserver
[ ] Chaos: 3 replicas, 10% packet loss, 100 triples each, converge within 100 sync cycles
[ ] Gossip: revoke member on A, verify C also revokes
```

---

## File Inventory (New and Modified)

### New Files

| File | Phase |
|------|-------|
| `crates/hkask-federation/Cargo.toml` | 0 |
| `crates/hkask-federation/src/lib.rs` | 0 |
| `crates/hkask-federation/src/crdt/mod.rs` | 1 |
| `crates/hkask-federation/src/crdt/version_vector.rs` | 1 |
| `crates/hkask-federation/src/crdt/dot.rs` | 1 |
| `crates/hkask-federation/src/crdt/triple_key.rs` | 1 |
| `crates/hkask-federation/src/crdt/or_set.rs` | 1 |
| `crates/hkask-federation/src/crdt/lww_map.rs` | 1 |
| `crates/hkask-federation/src/crdt/g_set.rs` | 1 |
| `crates/hkask-federation/src/sync/mod.rs` | 1 |
| `crates/hkask-federation/src/sync/payload_store.rs` | 1 |
| `crates/hkask-federation/src/sync/transport.rs` (InMemory) | 1 |
| `crates/hkask-federation/src/sync/federation_sync.rs` | 1 |
| `crates/hkask-federation/src/sync/link.rs` | 2 |
| `crates/hkask-federation/src/sync/link_manager.rs` | 2 |
| `crates/hkask-federation/src/sync/invitation_policy.rs` | 2 |
| `crates/hkask-federation/src/registry/mod.rs` | 3 |
| `crates/hkask-federation/src/registry/federation_registry.rs` | 3 |
| `crates/hkask-federation/src/sync/matrix_transport.rs` | 3 |
| `crates/hkask-federation/src/sync/chaos_transport.rs` | 3 |
| `crates/hkask-federation/tests/contract/crdt_contract.rs` | 1 |
| `crates/hkask-federation/tests/integration/federation_integration.rs` | 1 |
| `crates/hkask-federation/tests/integration/multi_server.rs` | 3 |
| `crates/hkask-ports/src/federation.rs` | 1 |

### Modified Files

| File | Phase | Change |
|------|-------|--------|
| `Cargo.toml` (workspace) | 0 | Add `hkask-federation` member |
| `crates/hkask-types/src/curation.rs` | 0 | Add `OcapTokenKind::Federation` |
| `crates/hkask-types/src/cns.rs` | 0,2,3 | Add 5+7+7 federation spans |
| `crates/hkask-ports/src/lib.rs` | 1 | Add `pub mod federation` + re-exports |
| `crates/hkask-agents/src/curator/semantic_index.rs` | 1 | impl `FederationSyncPort` |
| `crates/hkask-agents/src/curator_agent/mod.rs` | 2 | Add `link_manager` field and federation directive handling |
| `crates/hkask-agents/src/curator/curation_loop.rs` | 2 | Handle federation directives in `compute()` |
| `crates/hkask-agents/src/curator_agent/federation_health.rs` | 3 | NEW — `FederationHealthModel` |
| `crates/hkask-cns/src/set_points.rs` | 3 | Add federation algedonic thresholds |
| `crates/hkask-services-context/src/context_impl.rs` | 3 | Wire `FederationSync` into `AgentService` |
