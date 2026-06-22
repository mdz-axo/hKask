---
title: "Federation Design — Addendum D: Multi-Skill Gap Analysis"
audience: [architects, developers]
last_updated: 2026-06-22
version: "0.30.0+fed"
status: "Findings — Gaps Requiring Resolution"
domain: "Cross-cutting"
---

# Addendum D: Multi-Skill Federation Gap Analysis

**Purpose:** Systematic audit of the federation design against four evaluation frameworks: pragmatic-semantics (IS/OUGHT classification, provenance, constraint conflicts), pragmatic-cybernetics (VSM, feedback loops, variety engineering, Good Regulator), deep-module (deletion test, depth score, 7-function rule), and improve-codebase-architecture (coupling, seams, locality, testability).

---

## 1. Pragmatic-Semantics Gaps

### PS-1: CRDT Timestamps vs. ν-Event Canonical Source (Provenance Ambiguity — High)

**Statement under review:** "Same triple from two replicas → LWW: highest timestamp + replica_id tiebreak" (FEDERATION_DESIGN.md §6.2 conflict resolution table).

**Classification error:** This is presented as a Declarative (certain) claim, but it's **Subjunctive** (projection). The resolution depends on wall-clock synchronization between servers, which is an assumption, not a measurement.

**Correct epistemic mode:** Hypothesis (IS + Subjunctive): "We assume bounded clock skew; under this assumption, LWW with replica_id tiebreak converges. If clock skew exceeds the CRDT merge latency window, the 'wrong' server's triple may win."

**The deeper conflict:** hKask's semantic invariant states: "The ν-event store is the sole canonical source. If ν-events and semantic memory disagree, ν-events win." But with cross-server CRDTs, there are two ν-event stores. Which one is canonical? The current design resolves this with timestamps (a data-level resolution), not with ν-event priority (the semantic-level invariant). This is a **Provenance conflict**: the federation's conflict resolution model (LWW timestamps) contradicts the system's semantic invariant (ν-events are canonical).

**Resolution required:** Either:
- A) Federation CRDTs must carry ν-event provenance, and conflict resolution must prefer the ν-event with higher causal ordering (not wall-clock time). This requires extending CRDT dots to carry ν-event IDs, not just timestamps.
- B) Federation explicitly states that cross-server triples are "second-class" — they are derived from ν-events but not themselves canonical. SemanticIndex on each server is authoritative for locally-published triples; CRDT-received triples carry a `provenance: federated` flag and can be overridden by locally-published triples with higher confidence.

**Recommendation: Option A** — extend CRDT dots to carry ν-event provenance (`(replica_id, ν_event_id, counter)` instead of `(replica_id, counter)`). This makes CRDT convergence causal-order-aware and preserves the ν-event primacy invariant.

---

### PS-2: Federation Security Claims — Constraint Force Misclassification (Medium)

**Statement under review:** "Episodic memory never crosses federation boundary — CRDT syncs only from SemanticMemory" (ADDENDUM_PROTOCOL.md §9).

**Classification error:** This is classified as equivalent to P1 (Prohibition: "Episodic memory never exposed without consent"). But the enforcement mechanism differs:

| Claim | Actual Enforcement | Correct Force |
|-------|-------------------|---------------|
| "Episodic memory never crosses" | `EpisodicMemory::store()` rejects `Visibility::Public` at write time. `FederationSync` reads only from `SemanticMemory` by code convention. | Prohibition for write path. **Guardrail** for sync path — the sync path could be changed to include episodic data without compiler error. |
| "CRDT syncs only from SemanticMemory" | `FederationSync::run()` reads from `SemanticMemory::query_deduped()`. No type-level enforcement. | **Evidence** (IS + Probabilistic): "We observe that the current code reads from SemanticMemory. This is enforced by code review, not the type system." |

**Correct classification per pragmatic-semantics decision tree:**
```
"CRDT syncs only from SemanticMemory"
├── NOT a direct measurement → not Declarative
├── Statistical inference from code review → Probabilistic
├── IS statement (describes what the code does today) → IS
└── IS + Probabilistic → Evidence (not Prohibition, not Guardrail)
```

**Implication:** A future code change that accidentally adds an episodic query to `FederationSync` would NOT be caught by the compiler. The Prohibition-level guarantee exists only at the `EpisodicMemory::store()` level. The federation sync path should add a type-level guard: `FederationSync` should hold a `FederationSyncPort` trait that only exposes `SemanticMemory` queries, not `EpisodicMemory`.

**Recommendation:** Extract a `FederationSyncPort` trait in `hkask-ports` that exposes only the public-memory query surface. `FederationSync` depends on the trait, not on concrete `SemanticMemory`. This makes the Prohibition enforceable at the type level.

---

### PS-3: Temporal Semantics of CRDT "Now" (Medium)

**Statement under review:** The SYNC_REQUEST/SYNC_RESPONSE protocol uses version vectors to determine "what's new since last sync." But "new" depends on wall-clock time in the LWW-Map for user profiles.

**The problem:** hKask's temporal semantics define multiple time layers:
- Valid from: ν-event timestamp
- Valid to: Until superseded by newer ν-event
- Supersession: Newer ν-event replaces older

But CRDT LWW uses wall-clock `timestamp` as the conflict resolver. If Server A's clock is 30 seconds ahead of Server B's, every user profile update from A will "win" over B's. This is **not** semantic supersession — it's clock-supremacy.

**Correct epistemic framing:** "LWW-Map for user profiles uses wall-clock timestamps. This is a **Guideline** (OUGHT + Probabilistic), not a Prohibition. Acceptable for profiles because they are low-stakes data. NOT acceptable for triples (where ν-event causal ordering should dominate)."

**Recommendation:** Explicitly document the temporal contract:
- **Triples (OR-Set):** Use causal ordering (ν-event-based version vectors). Timestamps are advisory only.
- **User profiles (LWW-Map):** Use wall-clock timestamps. Acceptable for low-stakes data; clock skew tolerance documented.
- **Agent registrations (G-Set):** No temporal conflict (additive only).

---

## 2. Pragmatic-Cybernetics Gaps

### PC-1: Federation VSM Recursion — Missing Second-Order Cybernetics (Critical)

**The VSM recursion principle:** "Every component should be viable at its own level." A federation of hKask servers IS a viable system and needs its own S1–S5 recursion.

**Current state:** Federation has CNS spans (sensor) and CRDTs (model), but the feedback loop is incomplete:

| VSM System | Federation Implementation | Status |
|------------|--------------------------|--------|
| **S1 (Operations)** | CRDT sync + Matrix conduit message routing | ✅ Specified |
| **S2 (Coordination)** | SYNC_REQUEST/SYNC_RESPONSE protocol + version vectors | ✅ Specified |
| **S3 (Control)** | CNS spans (`cns.federation.*`) — observe but don't regulate | ⚠️ **Sensors only, no comparators** |
| **S3\* (Audit)** | `kask cns federation health` | ⚠️ **Not specified** |
| **S4 (Intelligence)** | Curator evaluates federation health via metacognition | ⚠️ **No federation-specific metacognition model** |
| **S5 (Policy)** | `OcapTokenKind::Federation` + invitation policies | ✅ Specified |

**The gap:** S3 has sensors (18 CNS spans) but no **comparators** (algedonic thresholds for federation). Without comparators, the Curator (S4) receives raw spans but can't distinguish "normal" from "abnormal" federation behavior.

**Missing algedonic thresholds:**

| Threshold | Metric | Warning | Critical | CNS Span |
|-----------|--------|---------|----------|----------|
| `fed_sync_latency` | CRDT sync round-trip time (ms) | > 5000ms | > 30000ms | `FederationCrdtMerge` |
| `fed_crdt_divergence` | Elements in CRDT delta > baseline × factor | > 2× baseline | > 10× baseline | `FederationCrdtMerge` |
| `fed_link_downtime` | Duration link has been in Paused/Degraded state (s) | > 3600s | > 86400s | `FederationLinkPaused` |
| `fed_member_count_change` | Federation member count change (absolute) | ±1 | ±N/2 | `FederationMemberJoined` / `FederationMemberLeft` |
| `fed_invitation_rate` | Invitations received per hour | > 5 | > 20 | `FederationInviteReceived` |
| `fed_registry_divergence` | User/agent entries differed in last registry sync | > 10 | > 100 | `FederationRegistrySync` |

**Recommendation:** Add federation-specific set-points alongside the existing CNS `SetPoints`. The Curator's metacognition model must include federation health as a dimension alongside variety, energy, and error rate.

---

### PC-2: Broken Feedback Closure During Network Partition (Critical)

**Scenario:** Curator A pauses link to Curator B. A sends `LINK_PAUSE`. Network partition occurs — message is lost. B never receives the pause notification.

**What the design says:** "Peer knows the pause is intentional, not a failure. No CURATOR_SYNC_DEGRADED alert."

**What actually happens:** B sees sync timeout. B has no way to distinguish "intentional pause" from "network failure." B marks the link as... what? The design doesn't define a `Degraded` state. B's only options are:
- Stay in `Linked` state (but sync is broken — model-reality divergence)
- Transition to `Revoked` (too drastic for a transient network issue)
- Invent a new state (but no `Degraded` state exists)

**The feedback loop analysis:**

| Property | Assessment |
|----------|------------|
| **Polarity** | Unknown — if B retries sync, this could become positive feedback (amplifying retries) |
| **Delay** | B detects timeout at sync interval (5s default) |
| **Gain** | Unknown — no dampener for retry storms |
| **Closure** | **BROKEN** — B's state diverges from reality. No path to recover without A's intervention. |
| **Fidelity** | LOW — B can't distinguish pause from partition from peer death |

**Recommendation:** Add a `Degraded` state to the `LinkState` enum:

```rust
pub enum LinkState {
    // ... existing states ...
    /// Sync has failed for longer than the configured timeout.
    /// The peer may be paused, partitioned, or dead — unknown.
    Degraded {
        /// When the degradation was detected.
        degraded_at: DateTime<Utc>,
        /// Consecutive failed sync attempts.
        failed_attempts: u64,
        /// Last successful sync timestamp (for staleness calculation).
        last_success_at: DateTime<Utc>,
    },
}
```

Transitions:
- `Linked → Degraded`: sync timeout (consecutive failures > threshold)
- `Degraded → Linked`: sync resumes successfully (partition healed, peer resumed from pause)
- `Degraded → Revoked`: extended degradation (e.g., 7 days without recovery)
- `Paused → Degraded`: paused peer stops responding (the LINK_PAUSE was lost)
- `Degraded` → CNS: `cns.federation.link_degraded` with `{failed_attempts, last_success_age_secs}`

---

### PC-3: Federation Variety Engineering — Amplification Deficit (Medium)

**System variety (federation disturbance modes):**

| Disturbance | CNS Span Coverage | Curator Response Path |
|------------|------------------|----------------------|
| Network partition | ❌ No `Degraded` state exists | ⚪ Can't detect |
| Clock skew | ❌ Not measured | ⚪ Can't detect |
| Malicious peer injecting bad data | ⚠️ `FederationCrdtMerge` (observes deltas but doesn't classify) | ⚪ Classifier missing |
| Peer offline permanently | ⚠️ `FederationLinkLost` (proposed but not wired to comparator) | ⚪ Escalation path missing |
| Invitation spam | ✅ `FederationInviteReceived` | ⚠️ Rate limit not specified |
| CRDT state divergence | ❌ No checksum/hash comparison | ⚪ Can't detect |
| Slow peer (high latency) | ⚠️ `FederationCrdtMerge` carries latency | ⚪ Comparator missing |

**Variety deficit:** 7 disturbance modes, only 3 have CNS span coverage, 0 have comparator thresholds, 0 have Curator response paths.

**Recommendation:** For Phase 1 (MVP), prioritize:
1. `Linked → Degraded` state (covers partition, peer offline)
2. Federation algedonic thresholds (covers latency, divergence, member count)
3. Invitation rate limiting (covers spam)
4. Defer malicious-data detection to Phase 2 (requires content classification)

---

### PC-4: Good Regulator — Curator's Federation Model (Medium)

**The Good Regulator theorem:** "Every good regulator of a system must be a model of that system."

**Question:** Does the Curator have a model of what "healthy federation" looks like?

**Current state:** The Curator receives CNS federation spans. But CNS spans are ν-events — raw observations. The Curator doesn't have a **model** of normal federation behavior to compare against.

**What the model should include:**

| Model Parameter | Meaning | Source |
|----------------|---------|--------|
| Expected sync latency | Normal CRDT sync round-trip time | Rolling average of `FederationCrdtMerge` latency |
| Expected merge frequency | Normal number of merges per interval | Rolling count per sync window |
| Expected member count | Known peers in federation | `FederationSync.peers.len()` |
| Expected delta size | Normal number of triples per sync | Rolling average of delta sizes |
| Baseline confidence | Confidence that the model is accurate | Decays with staleness (like episodic confidence) |

**Recommendation:** Add a `FederationHealthModel` to the Curator's metacognition. This is a lightweight struct updated each CNS tick:

```rust
/// The Curator's model of what "healthy federation" looks like.
/// Updated each CNS tick. Used by metacognition to detect anomalies.
pub struct FederationHealthModel {
    /// Rolling window of sync latency samples (ms).
    latency_window: Vec<u64>,
    /// Expected merge frequency per interval.
    expected_merge_frequency: f64,
    /// Known peer count (changes trigger variety signals).
    expected_member_count: usize,
    /// Model confidence — decays with staleness.
    confidence: Confidence,
    /// Last model update timestamp.
    last_updated: DateTime<Utc>,
}
```

---

## 3. Deep-Module Gaps

### DM-1: `hkask-federation` Crate — Interface Size Assessment (Medium)

**Proposed public surface:**

| Module | Public Types | Public Functions (est.) | Total Items |
|--------|-------------|------------------------|-------------|
| `crdt` | `VersionVector`, `Dot`, `ORSet<T>`, `LWWMap<K,V>`, `GSet<T>` | ~15 methods | ~20 |
| `sync` | `FederationSync` | `run()`, `pause()`, `resume()`, `revoke()`, `status()`, `health()` (~6 methods) | ~7 |
| `registry` | `FederationRegistry` | `resolve_user()`, `resolve_agent()`, `resolve_matrix_target()` (~3 methods) | ~4 |
| `conduit` | `ConduitFederation` | `federate_with()`, `route_message()`, `status()` (~3 methods) | ~4 |
| `link` | `FederationLink`, `LinkState`, `LinkSyncState`, `RevocationScope` | ~3 methods | ~7 |
| **Total** | **11 types** | **~30 methods** | **~42 items** |

**Depth score estimate:**
- Implementation lines: ~800–1200 (CRDT logic, sync orchestration, Matrix integration, state machine)
- Public items: ~42
- **Depth score: ~800/42 ≈ 19** → **Very Shallow** by the depth-score matrix

**Problem:** The crate has a large interface surface but most of the behavior is wiring (connecting CRDTs to stores to CNS). The "depth" comes from the CRDT algorithms and the sync state machine — but these are split across 5 modules.

**Recommendation:** Consolidate. The 7-function rule suggests > 7 public items per module is a signal of shallowness. Options:
1. Merge `link` + `sync` into a single `sync` module (link state is part of sync lifecycle)
2. Merge `conduit` into `sync` (Matrix federation is part of the sync transport)
3. Keep `crdt` separate (it's general-purpose and independently testable)
4. Keep `registry` separate (it's the merged-registry abstraction, consumed by non-federation code)

**After consolidation:**
- `crdt` module: ~20 items (acceptable — general-purpose library)
- `sync` module (link + sync + conduit): ~12 items (still high, but behavior-rich)
- `registry` module: ~4 items (deep — few methods, substantial behavior)

---

### DM-2: Deletion Test for CRDT Module (Medium)

**Direction 1 — Callers:** If we deleted the `crdt` module, would complexity reappear in `FederationSync`?
- **Yes.** FederationSync would need to inline OR-Set merge, version vector comparison, LWW-Map resolution. Complexity reappears → the module earns its keep.

**Direction 2 — Module:** If we deleted the `crdt` module and replaced it with an external CRDT library (e.g., `crdts` crate), would behavior be lost?
- **Partially.** The external CRDT library provides OR-Set, LWW-Map, G-Set. But the federation-specific extensions (ν-event-aware dots, cross-server transport serialization, CNS span emission during merge) would need to be built as wrappers.
- If an external CRDT library is used, `hkask-federation/src/crdt.rs` becomes a thin wrapper → **shallow module** (pass-through risk).

**Recommendation:** Evaluate external CRDT libraries (e.g., the `crdts` Rust crate) before implementing the CRDT module. If a library exists that provides OR-Set, LWW-Map, and G-Set with the required properties (commutative, associative, idempotent merge), use it. Build federation-specific wrappers in `hkask-federation/src/crdt.rs` that add ν-event provenance and CNS span emission. This avoids reinventing well-tested algorithms and keeps the module deep (wrappers add behavior on top of the library).

**If no suitable library exists:** The CRDT module is justified. Implement with comprehensive property-based tests.

---

### DM-3: `FederationSync` — The 7-Function Rule (Medium)

`FederationSync` is the central orchestrator. It currently has one method (`run()`), but the lifecycle protocol (ADDENDUM_PROTOCOL.md) implies additional methods:

```rust
impl FederationSync {
    pub async fn run(&self, cancel: watch::Receiver<bool>);     // 1. Main loop
    pub async fn pause(&self, peer: ReplicaId, reason: &str);    // 2. Pause link
    pub async fn resume(&self, peer: ReplicaId);                  // 3. Resume link
    pub async fn revoke(&self, peer: ReplicaId, reason: &str);   // 4. Revoke member
    pub async fn leave(&self, reason: &str);                      // 5. Leave federation
    pub async fn invite(&self, ...);                              // 6. Send invitation
    pub async fn accept(&self, invitation_id: &str);             // 7. Accept invitation
    pub fn status(&self) -> FederationStatus;                     // 8. Query status
    pub fn health(&self) -> FederationHealth;                     // 9. Health check
}
```

**9 public methods** — exceeds the 7-function heuristic. This is a signal that `FederationSync` may be two modules:
1. `FederationSync` — the sync loop and CRDT management (run, status, health) — 3 methods
2. `FederationLinkManager` — link lifecycle (invite, accept, pause, resume, revoke, leave) — 6 methods

**Recommendation:** Split into two types. `FederationSync` manages the background CRDT sync. `FederationLinkManager` handles the invitation and revocation protocol. Both are consumed by the Curator, which issues directives to either.

---

## 4. Improve-Codebase-Architecture Gaps

### IA-1: Tight Coupling — FederationSync Depends on 5 Concrete Domain Types (High)

**The dependency list:**

```rust
pub struct FederationSync {
    semantic_set: ORSet<FederationTripleKey>,           // CRDT (self)
    user_map: LWWMap<WebID, UserProfile>,               // CRDT (self)
    agent_set: GSet<RegisteredAgent>,                    // CRDT (self)
    artifact_set: ORSet<ArtifactKey>,                     // CRDT (self)
    peers: HashMap<ReplicaId, FederationPeer>,            // self
    interval: Duration,                                    // std
    local_index: Arc<RwLock<SemanticIndex>>,              // hkask-agents (concrete)
    local_agents: Arc<AgentRegistry>,                      // hkask-communication (concrete)
    local_users: Arc<UserStore>,                           // hkask-storage (concrete)
    event_sink: Arc<dyn NuEventSink>,                      // hkask-types (trait — good!)
}
```

**The problem:** `local_index` depends on `SemanticIndex` — a concrete type defined in `hkask-agents/src/curator/semantic_index.rs`. `local_agents` depends on `AgentRegistry` — a concrete type in `hkask-communication`. `local_users` depends on `UserStore` — a concrete type in `hkask-storage`.

**This creates a hexagonal ports violation:** FederationSync depends on concrete implementations, not port traits. Testing FederationSync requires a real SemanticIndex, real AgentRegistry, and real UserStore.

**The seam is missing.** There should be trait abstractions:

```rust
/// In hkask-ports:
pub trait FederationSyncPort: Send + Sync {
    /// Query public triples since a cursor (for CRDT sync).
    fn query_public_since(&self, cursor: u64, limit: usize) -> Result<Vec<Triple>, FederationSyncError>;
    /// Insert a federated triple into the local SemanticIndex.
    fn insert_federated(&self, triple: &Triple, source: ReplicaId) -> Result<(), FederationSyncError>;
    /// Get the current cursor for a source replica.
    fn cursor_for(&self, source: ReplicaId) -> u64;
    /// Advance the cursor for a source replica.
    fn advance_cursor(&mut self, source: ReplicaId, cursor: u64);
}

pub trait FederationRegistryPort: Send + Sync {
    fn resolve_user(&self, webid: &WebID) -> Option<UserProfile>;
    fn resolve_agent(&self, webid: &WebID) -> Option<AgentInfo>;
    fn list_local_users(&self) -> Vec<UserProfile>;
    fn list_local_agents(&self) -> Vec<AgentInfo>;
}
```

**Recommendation:** Add `FederationSyncPort` and `FederationRegistryPort` traits to `hkask-ports`. `SemanticIndex` and the merged registry implement these traits. `FederationSync` depends on the traits, enabling mock-based testing.

---

### IA-2: Missing Seam — Invitation Acceptance Policy (Medium)

**The problem:** The invitation acceptance flow is hardcoded as "admin manually accepts" or "auto_accept configured." There's no trait abstraction for custom acceptance policies.

**Recommendation:** Add an `InvitationPolicy` trait:

```rust
/// In hkask-ports:
pub trait InvitationPolicy: Send + Sync {
    /// Evaluate an incoming federation invitation.
    /// Returns the decision: Accept, Reject(reason), or DeferToAdmin.
    fn evaluate(&self, invitation: &FederationInvitation) -> InvitationDecision;
}

pub enum InvitationDecision {
    /// Auto-accept — link establishment proceeds immediately.
    Accept,
    /// Auto-reject with reason.
    Reject { reason: String },
    /// Defer to human admin for manual review (default).
    DeferToAdmin,
}
```

Default implementations:
- `ManualInvitationPolicy`: Always returns `DeferToAdmin` (P2 default)
- `AllowListInvitationPolicy`: Accepts only from configured peers
- `RateLimitingInvitationPolicy`: Wraps another policy, adds rate limiting

This creates a **real seam** (multiple adapters: manual, allowlist, rate-limiting).

---

### IA-3: Locality — Directives Scattered Across Curator and FederationSync (Medium)

**The problem:** Federation directives (pause, resume, revoke, invite, accept) are defined as `CuratorDirective` variants in `hkask-types/src/curator.rs`. But the implementation of these directives lives in `hkask-federation/src/sync.rs` in `FederationSync` (or `FederationLinkManager`).

**The dispersione:** Understanding "what happens when I pause a link" requires reading:
1. `CuratorDirective::PauseFederationLink` (the directive type) — in `hkask-types`
2. `FederationSync::pause()` (the implementation) — in `hkask-federation`
3. `CuratorAgent::compute()` (the dispatch) — in `hkask-agents`
4. `CnsSpan::FederationLinkPaused` (the CNS event) — in `hkask-types`

**This is a locality problem.** A single operation spans 4 files across 3 crates.

**Recommendation:** Accept this as inherent to the hexagonal architecture. The directive type lives in the foundation (hkask-types) because multiple crates need it. The implementation lives in the domain crate (hkask-federation). The dispatch lives in the curator (hkask-agents). This is NOT a design flaw — it's the cost of separation of concerns. The alternative (putting everything in one crate) would create a monolith.

**Mitigation:** Document the cross-crate trace for each federation operation in a `FEDERATION_OPERATIONS.md` reference document.

---

### IA-4: Untestable Without Multi-Server Harness (Critical)

**The problem:** The federation design has no test strategy. Federation operations require at least two servers with Matrix transport. The existing test infrastructure (`hkask-test-harness`) provides in-memory databases and mock CNS runtimes, but doesn't simulate cross-server communication.

**What's needed for testability:**

```rust
/// In hkask-ports (or hkask-test-harness):
pub trait FederationTransport: Send + Sync {
    /// Send a federation message to a peer.
    async fn send(&self, peer: ReplicaId, message: FederationMessage) -> Result<(), TransportError>;
    /// Receive federation messages addressed to this replica.
    async fn recv(&self) -> Result<FederationMessage, TransportError>;
    /// Simulate a network partition (for testing).
    fn simulate_partition(&self, peer: ReplicaId);
    /// Heal a simulated partition.
    fn heal_partition(&self, peer: ReplicaId);
}
```

With this trait:
- **Real adapter:** `MatrixFederationTransport` — wraps Matrix SDK for production
- **Test adapter:** `InMemoryFederationTransport` — simulates message passing in tests
- **Chaos adapter:** `ChaosFederationTransport` — introduces latency, drops, partitions for robustness testing

**Recommendation:** Add `FederationTransport` to `hkask-ports` as Phase 1 of federation implementation. This enables unit-testing the CRDT merge logic, sync protocol, and state machine WITHOUT a running Matrix server. Multi-server integration tests can follow in Phase 2.

---

## 5. Coding-Guidelines Gaps

### CG-1: Goal-Driven Execution — Missing Success Criteria for Federation

**Principle 4:** "Define success criteria. Loop until verified."

The federation design has no verifiable success criteria. "Federation works" is not verifiable. What does "works" mean?

**Recommendation:** Define behavioral contracts with `expect:` annotations:

```rust
/// Start the federation sync loop.
///
/// expect: "Federated Curators converge on public memory"
/// [P3] Motivating: Generative Space — cross-server knowledge sharing
/// [P9] Constraining: Homeostatic Self-Regulation — CNS-observed sync
/// pre:  CRDT state is initialized with local SemanticIndex data.
/// pre:  At least one peer is configured and Linked.
/// post: On each sync interval, local deltas are sent to all Linked peers.
/// post: Received deltas are merged into local CRDT state.
/// post: CNS span `FederationCrdtMerge` is emitted for each successful merge.
/// post: CNS span `FederationLinkDegraded` is emitted for consecutive failures > threshold.
/// test: InMemoryFederationTransport with two replicas, insert triple on A, verify triple appears on B within sync_interval * 2.
```

### CG-2: Simplicity First — Are All 18 CNS Spans Necessary?

**Principle 2:** "Minimum code that solves the problem. Nothing speculative."

The federation design proposes 18 CNS span variants. This is the full lifecycle coverage. But for MVP (Phase 1), which spans are actually needed?

**Recommendation:** Prioritize:

| Priority | CNS Span Variants | Rationale |
|----------|------------------|-----------|
| **Phase 1 (MVP)** | `FederationCrdtMerge`, `FederationLinkEstablished`, `FederationLinkLost`, `FederationLinkDegraded` (new), `FederationMemberLeft` | Core sync observability + failure detection |
| **Phase 2** | `FederationInviteSent`, `FederationInviteReceived`, `FederationInviteAccepted`, `FederationInviteRejected`, `FederationLinkPaused`, `FederationLinkResumed` | Invitation lifecycle |
| **Phase 3** | `FederationMemberRevoked`, `FederationDissolved`, `FederationRegistrySync`, `FederationArtifactSync`, `FederationConduitRoute`, `FederationConduitRouteLost`, `FederationCrdtConflict` | Advanced operations + data sync |

---

## Summary: Gap Severity Matrix

| # | Gap | Framework | Severity | Phase |
|---|-----|-----------|----------|-------|
| PS-1 | CRDT timestamps vs. ν-event canonical source | Pragmatic Semantics | **Critical** | Must resolve before implementation |
| PC-1 | Missing federation algedonic thresholds | Pragmatic Cybernetics | **Critical** | Must resolve before implementation |
| PC-2 | Broken feedback closure during partition (no Degraded state) | Pragmatic Cybernetics | **Critical** | Must resolve before implementation |
| IA-4 | Untestable without multi-server harness | Architecture | **Critical** | Must resolve before implementation |
| PS-2 | Federation security claims misclassified (Evidence, not Prohibition) | Pragmatic Semantics | **High** | Phase 1 |
| IA-1 | FederationSync tight coupling to 5 concrete types | Architecture | **High** | Phase 1 |
| PC-3 | Variety deficit: 7 disturbances, 0 comparators | Pragmatic Cybernetics | **High** | Phase 1 (prioritize top 4) |
| DM-1 | `hkask-federation` depth score ~19 (Very Shallow) | Deep Module | **Medium** | Phase 1 (consolidate modules) |
| DM-3 | FederationSync 9 methods (exceeds 7-function rule) | Deep Module | **Medium** | Phase 1 (split into two types) |
| PS-3 | Temporal semantics of CRDT "now" | Pragmatic Semantics | **Medium** | Phase 1 (document, use causal ordering for triples) |
| PC-4 | Good Regulator — missing federation health model | Pragmatic Cybernetics | **Medium** | Phase 2 |
| IA-2 | Missing InvitationPolicy seam | Architecture | **Medium** | Phase 2 |
| DM-2 | CRDT module: build vs. use external library | Deep Module | **Medium** | Pre-implementation evaluation |
| IA-3 | Locality: directives scattered across 3 crates | Architecture | **Low** | Document, accept |
| CG-1 | Missing success criteria / behavioral contracts | Coding Guidelines | **Medium** | Phase 1 |
| CG-2 | 18 CNS spans — phased delivery recommended | Coding Guidelines | **Low** | Phase 1 (prioritize 5 spans) |
