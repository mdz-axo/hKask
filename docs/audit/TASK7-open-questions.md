# Task 7: Open Questions

Seven questions identified during the audit that require design decisions or code changes before the simplification changelog (Task 4) is implemented.

---

## Q1: Loop Priority Under Resource Contention

### Problem

The inference and memory loops compete for the same SQLite connection (via `hkask-storage`). When inference is assembling context (reading) and memory is consolidating (writing), they contend for the single connection pool. SQLite's WAL mode allows concurrent readers with a single writer, but a long-running read can block the writer, and a writer blocks all subsequent readers until it commits.

In a resource-constrained scenario, which loop wins?

### Questions to Answer

1. Is this a governance concern (which loop should the system prioritize) or an infrastructure scheduling concern (which lock acquisition ordering prevents deadlocks)?
2. Should the system preempt a context assembly read for a pending consolidation write?
3. Should the priority be configurable per-agent, or is it a system-wide invariant?

### Recommendation

Implement a **priority-tagged lock** in `hkask-storage`:

```rust
pub enum LockPriority {
    Critical,   // Revocation writes, capability verification
    High,       // Consolidation writes, sovereignty event writes
    Normal,     // Context assembly reads, span emission
    Low,        // Batch deduplication, cache eviction
}
```

The lock acquisition function checks priority: a `Critical` or `High` writer can preempt `Normal` readers by failing their acquisition with a `LockPreempted` error that the caller retries. This prevents starvation of governance-critical writes (revocation, sovereignty) while allowing inference reads to proceed when no higher-priority work is pending.

**Rationale:** This is fundamentally a governance concern — the system must ensure that capability revocation and sovereignty enforcement cannot be blocked by inference reads. The `LockPriority` enum makes the priority discipline explicit and auditable.

---

## Q2: Membrane vs. Capability Boundary

### Problem

Task 4 collapses the `Membrane` abstraction — the concept of a membrane as a distinct entity is removed. But membranes enforced the boundary between a bot's public actions and a replicant's private episodic store. If membranes are gone, what enforces this boundary?

The `template_type` discriminator (`Bot` vs. `Replicant`) alone is **not sufficient** because:
- It can be checked incorrectly (missed at a call site)
- It's a data-level discriminator, not an authority boundary
- A bug in one place can leak private data across all pods

### Questions to Answer

1. Does the capability handle system (Task 5) provide enough enforcement, or does visibility boundary enforcement need a separate mechanism?
2. How do `DataCategory` caveats derived from the keystore's key-derivation scheme compose with capability handles?

### Recommendation

Visibility enforcement moves to `MemoryReadHandle`'s capability boundary, using `DataCategory` caveats:

```rust
impl MemoryReadHandle {
    pub fn query_visible(&self, entity: &str) -> Vec<Triple> {
        let raw = self.store.query(entity);
        raw.into_iter()
            .filter(|t| self.is_visible(t))
            .collect()
    }

    fn is_visible(&self, triple: &Triple) -> bool {
        match triple.data_category {
            DataCategory::Public => true,
            DataCategory::Shared => self.capability.allows_read(triple.owner_domain()),
            DataCategory::Private => self.observer_webid == triple.owner_webid,
            DataCategory::Semantic => self.capability.allows_memory_access(),
        }
    }
}
```

The `DataCategory` is derived from the keystore's key-derivation scheme:
- `Public` — no key derivation needed, stored plaintext
- `Shared` — encrypted with a domain key (HKDF-derived from master key + domain label)
- `Private` — encrypted with the agent's individual key (HKDF-derived from master key + WebID)
- `Semantic` — encrypted with the system's semantic store key (stripped of perspective)

**Rationale:** The keystore's HKDF key hierarchy already encodes the visibility discipline cryptographically. The capability handle enforces it programmatically. The `template_type` becomes a descriptive label (for registry filtering) rather than a security boundary.

---

## Q3: Minimum CNS for Variety Deficit Detection

### Problem

The CNS crate has ~130 abstractions, but the **minimum viable path** for variety deficit detection — the algedonic mechanism that detects when the system is stuck and triggers escalation — requires far fewer.

### Recommendation

Reduce to 4 essential functions:

| Function | Purpose |
|---|---|
| `increment_variety(domain: &str)` | Atomic increment of counter per domain |
| `check_variety(domain: &str) -> Option<AlgedonicAlert>` | Compare counter to expected variety, return alert if deficit |
| `determine_severity(deficit: usize, threshold: usize) -> AlertSeverity` | Classify deficit as Info / Warning / Critical |
| `process_alert(alert: &AlgedonicAlert) -> EscalationAction` | Route to log / queue / Curator |

The supporting infrastructure — currently scattered across `SovereigntyObserver`, `GoalVarietyMonitor`, and `BotMetricsCollector` — can be **unified into a single `VarietyTracker`**:

```rust
pub struct VarietyTracker {
    /// Per-domain variety counters, keyed by (domain, observer_webid)
    counters: HashMap<(String, WebID), VarietyCounter>,
    /// Expected variety per domain
    thresholds: HashMap<String, usize>,
}
```

The three separate observer/collector/monitor types all do the same thing: count events, compare to a threshold, emit alerts. They differ only in the **key** they count by:
- `SovereigntyObserver` — keyed by `(WebID, DataCategory)`
- `GoalVarietyMonitor` — keyed by `GoalID`
- `BotMetricsCollector` — keyed by `WebID`

A unified `VarietyTracker` keyed by a generic `(domain: String, observer_webid: WebID)` handles all three cases. The domain string encodes the category: `"sovereignty:{category}"`, `"goal:{id}"`, `"bot:{webid}"`.

**Rationale:** Three nearly-identical counting infrastructures violate P6 (no excess complexity). Unifying them reduces the CNS abstraction count from ~130 to ~40 while preserving all essential subloops (variety tracking, algedonic escalation, bot metrics, sovereignty observation).

---

## Q4: Recursive Simplification — Inference Micro-Governance

### Problem

The inference loop at `dispatch_action()` contains `verify_capability()` — a governance check embedded in the inference path. An eager refactorer might try to "simplify" this by moving the check to the governance loop and having inference call it via handle delegation.

### Decision

**Do NOT simplify this away.** The micro-governance check is **constitutive, not accidental:**

1. **OCAP must be enforced at the point of use.** Delegating the check to a governance loop introduces a time-of-check-to-time-of-use (TOCTOU) window. Between the governance loop verifying the capability and the inference loop using it, the token could be revoked, the capability could be exhausted, or the agent's sovereignty boundary could change.

2. **The micro-check is not redundant with the macro-check.** The governance loop verifies that *this pod* has authority to dispatch actions. The inference loop's `verify_capability` verifies that *this specific action* is authorized by the capability token presented with it. They operate at different granularity.

3. **Folding it would break Task 2's inference loop diagram.** The `AUTH → DENY → Deny + Emit Span` path is an essential subloop boundary. Removing `verify_capability` from `dispatch_action` breaks the loop's ability to reject unauthorized tool calls at the point of dispatch.

**Rationale:** This is an example of *essential* recursive complexity — a smaller governance cycle embedded within the inference cycle. The composition graph (Task 3) shows this as the `Inference → Governance` edge (`verify_capability()`), which is capability-restricted via `GovernanceHandle`. The edge is correct; the embedding is correct; no simplification is needed.

---

## Q5: Extensibility Threshold for `template_type`

### Problem

The registry's `template_type` discriminator currently has variants. How many new agent types can be added before the registry becomes a dumpster?

### Recommendation

**Hard cap of 5 variants.** A 6th requires passing the **"new category test":**

| Question | Rationale |
|---|---|
| Does it need its **own handler pipeline?** | If it can reuse an existing pipeline (e.g., a new bot subtype), fold it under an existing type with a subtype field. |
| Does it need its **own data model?** | If it can use the existing `BotData` or `ReplicantData` with additional optional fields, use serde `flatten` or `Option` instead of a new variant. |
| Does it need its **own capability boundary?** | If it shares the capability model of an existing type, don't create a new type — attenuate the existing one. |

**If all three hold**, it's a genuinely new agent category that warrants its own `template_type` variant.

**Rationale:** Each new `template_type` variant requires matching arms in every registry operation, every capability check, every pod lifecycle handler, and every ensemble deliberation coordinator. Adding a variant imposes a **linear cost on every operation** that branches on `template_type`. The cap prevents unbounded growth and forces subtype reuse through capability attenuation rather than type proliferation.

---

## Q6: Security — Admin Passphrase Timing Attack

### Problem

`hkask-keystore::admin::verify_admin_passphrase()` at `crates/hkask-keystore/src/admin.rs:65` uses `==` for string comparison:

```rust
// Line 64-65
// Constant-time comparison to prevent timing attacks
stored_hash == computed_hash
```

The comment explicitly states "Constant-time comparison to prevent timing attacks" but the code uses Rust's standard string `==`, which short-circuits on the first differing byte. This makes the comparison timing-variant — an attacker can measure response times to progressively narrow down the hash value byte by byte.

### Recommendation

Replace `==` with `subtle::ConstantTimeEq`:

```rust
use subtle::ConstantTimeEq;

// In verify_admin_passphrase:
stored_hash.as_bytes().ct_eq(computed_hash.as_bytes()).into()
```

`subtle` is already a workspace dependency (`Cargo.toml` line 70), used by `hkask-storage` and `hkask-types`. It needs to be added to `hkask-keystore/Cargo.toml`:

```toml
[dependencies]
subtle.workspace = true
```

The `hash_admin_passphrase()` function returns hex-encoded strings (likely `blake3` output via `hex::encode`). If the stored and computed values are hex strings, `ct_eq` on their `.as_bytes()` is correct because both will have the same length. If one could be shorter (e.g., a truncated hash stored in a different format), this would need an additional length check (also constant-time).

**Rationale:** This is a fix-before-implement situation. The simplification changelog (Task 4) should not proceed until this vulnerability is addressed, because capability handles (Task 5) depend on correct key derivation, and the admin passphrase gates keychain access.

---

## Q7: Dead Code — Standing Session Fields

### Problem

Four `#[allow(dead_code)]` fields in `hkask-ensemble::standing_session` are parsed from YAML configuration but never wired to runtime behavior:

| Field | Location | Type |
|---|---|---|
| `StandingSessionConfig.rules` | Line 33 | `SessionRules` |
| `SessionRules.consensus_required` | Line 62 | `bool` |
| `SessionRules.orchestration_model` | Line 64 | `String` |
| `BootstrapConfig.auto_start` | Line 70 | `bool` |
| `ParticipantEntry.voting` | Line 51 | `bool` |

That's 5 fields, 4 struct-level `#[allow(dead_code)]` annotations (the `rules` field plus 3 interior fields + voting).

### Recommendation

**Wire them or remove them:**

| Field | Wire To | Rationale |
|---|---|---|
| `rules.consensus_required` | `EnsembleChatManager::deliberate()` | If `true`, require consensus before producing final response. If `false`, use majority vote. |
| `rules.orchestration_model` | `DeliberationCoordinator::choose_strategy()` | Dispatch to round-robin / consensus / Curator-mediated strategy based on model name. |
| `bootstrap.auto_start` | `StandingSession::start()` | If `true`, start the session immediately on load. If `false`, wait for Curator initiation. |
| `participant.voting` | `DeliberationCoordinator::collect_votes()` | If `true`, this participant casts a vote. If `false`, they observe only. |

**If not wired, remove** the fields and the `SessionRules` struct entirely (since both fields would be unused). This would be delete candidate D4 in Task 1.

**Rationale:** Dead code violates P6 (no excess complexity). Either the ensemble features these fields represent are part of the essential system (wire them), or they're speculative features that haven't been implemented (remove them). The audit's position is that consensus, orchestration model, auto-start, and voting rights are **essential ensemble governance functions** — they should be wired.

---

## Summary

| # | Question | Status | Action Required |
|---|---|---|---|
| Q1 | Loop priority under resource contention | Design decision | `LockPriority` enum in `hkask-storage` |
| Q2 | Membrane replacement via capability boundary | Design decision | `DataCategory` caveats in `MemoryReadHandle` |
| Q3 | Minimum CNS for variety deficit | Implementation | Unify 3 counters into single `VarietyTracker` |
| Q4 | Recursive inference micro-governance | **Resolved — keep** | No action needed |
| Q5 | `template_type` extensibility cap | **Policy established** | Enforce in code review |
| Q6 | Admin passphrase timing attack | **Bug — fix required** | Add `subtle` to keystore, use `ct_eq` |
| Q7 | Dead standing session fields | Implementation | Wire or remove the 5 fields |
| Q8 | Bot/Replicant entity collapse | **Resolved — reject** | Keep `template_type` variants; C2 capability collapse accepted |
| Q9 | CnsGovernReadHandle/CnsGovernWriteHandle split | **Resolved — apply** | Governance reads, Curation writes; type system enforces |

Questions Q4, Q5, Q8, and Q9 are resolved with clear decisions. Q6 is a concrete fix. Q1, Q2, Q3, and Q7 require implementation work before the simplification changelog can be fully applied.

---

## Q8: Bot/Replicant — Separate Types or `interaction_mode` Attribute?

### Source

Alternative simplification analysis (system-simplification-core-loops.md Q1) proposes collapsing Bot and Replicant into a single `AgentPod` with `interaction_mode: A2A | H2A`, arguing: *"The distinction is a usage pattern (machine-to-machine vs human-to-agent), not a structural difference. Visibility/ownership is already handled by `WebID` + `DataCategory`."*

### Analysis

This goes further than our TASK1 collapse candidate C2 (`BotCapabilities` + `ReplicantCapabilities` → `AgentCapabilities`). The alternative proposes collapsing the entity model itself, not just the capability types.

### Recommendation: Reject

**Keep Bot and Replicant as distinct `template_type` variants.** The entity-level collapse is rejected for three reasons:

1. **`template_type` drives branching logic across 4 crates.** Registry filtering, pod lifecycle, capability derivation, and ensemble deliberation all branch on `template_type`. Collapsing to `interaction_mode` moves the branching from compile-time (enum matching) to runtime (field checking), which is *less* safe, not more.

2. **Storage model differs structurally.** Replicants have private episodic stores (`DataCategory::Private`); bots do not. The `MemoryWriteHandle` authority matrix (`store_episodic` only for own WebID) encodes this. A single `AgentPod` with `interaction_mode` would need runtime gating on write operations — the exact pattern capability handles are designed to eliminate.

3. **The capability handle system already isolates the distinction.** `MemoryWriteHandle::store_episodic` only works for the handle's own WebID. Whether that WebID belongs to a Bot or Replicant is irrelevant at the handle level — the type system enforces the boundary regardless.

**Accepted:** The capability-level collapse (C2: `BotCapabilities` + `ReplicantCapabilities` → `AgentCapabilities` with `MemoryAccess` enum) remains in TASK4. This achieves the same simplification without touching the entity model.
