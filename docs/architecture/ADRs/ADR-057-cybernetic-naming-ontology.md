---
title: "ADR-057: Cybernetic Naming Ontology — Regulation → Regulation, Sensor Registry → Sensor Bus"
audience: [architects, developers, agents]
last_updated: 2026-07-21
version: "0.32.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, curation]
---

# ADR-057: Cybernetic Naming Ontology — Regulation → Regulation, Sensor Registry → Sensor Bus

**Date:** 2026-07-21  
**Status:** Active  
**Breaking:** Yes — no backward compatibility, no versioning transition. Clean atomic rename.

## Context

The current naming uses metaphors ("Regulation", "ν-event") that require reading the architecture docs to decode. A code reviewer seeing `CnsRuntime`, `NuEventStore`, or `SensorRegistry` for the first time cannot infer the cybernetic function from the name alone. The `CnsRuntime` is a god object that conflates afferent (sensing) and efferent (recording) functions under one name.

**Problem Statement:** The naming obscures the cybernetic function of each component, making the codebase harder to review and maintain.

**Constraints:**
- No backward compatibility requirement — persisted `NuEvent` records on disk will be recreated fresh.
- No versioning transition — single atomic rename across the workspace.
- The rename must be mechanical and complete — no half-renamed state.

## Decision

### Type Renames

| Current | Proposed | Cybernetic Function |
|---------|----------|---------------------|
| `SensorProvider` | `Sensor` | Afferent — senses current state |
| `SensorRegistry` (per-loop) | `SensorBus` | Afferent — actively walks sensors each tick |
| `SensorCatalog` (system-level) | `SensorRegistry` | Afferent — cross-loop registration and inventory |
| `CnsRuntime` | `RegulationLedger` | Efferent — records regulation outcomes, variety, alerts |
| `NuEvent` | `RegulationRecord` | Efferent — a single audit trail entry |
| `NuEventStore` | `RegulationArchive` | Efferent — persistent query store for historical records |
| `NuEventSink` | `RegulationSink` | Efferent — write-side trait |
| `PerPodCnsRuntime` | `PerPodLedger` | Efferent — per-pod regulation ledger |
| `CnsSpan` | `RegulationSpan` | Efferent — typed span enum |
| `CnsHealth` | `LedgerHealth` | Efferent — health of the regulation ledger |
| `CnsObserver` | `LedgerObserver` | Efferent — observer trait for ledger events |
| `CnsStoragePort` | `LedgerStoragePort` | Efferent — storage port for ledger data |
| `Loop` / `HkaskLoop` | `RegulationLoop` | Infrastructure — the sense→compare→compute→act→verify trait |
| `LoopSystem` | `LoopScheduler` | Infrastructure — schedules loop ticks |
| `LoopQuality` | `LoopMetrics` | Infrastructure — metrics about a loop's performance |
| `LoopAction` | `RegulatoryAction` | Infrastructure — an action produced by regulation |

### Namespace Rename

| Current | Proposed |
|---------|----------|
| `reg.*` | `reg.*` |
| `reg.tool.*` | `reg.tool.*` |
| `reg.inference` | `reg.inference` |
| `reg.gas` | `reg.gas` |
| `reg.curation` | `reg.curation` |
| `reg.cybernetics` | `reg.cybernetics` |
| `reg.regulation.*` | `reg.outcome.*` |
| `reg.variety` | `reg.variety` |
| `reg.algedonic` | `reg.alert` |
| `reg.agent_pod` | `reg.pod` |
| `reg.kata` | `reg.kata` |

### Crate Rename

| Current | Proposed |
|---------|----------|
| `hkask-regulation` | `hkask-regulation` |
| `hkask_cns` (in code) | `hkask_regulation` |

### Script Rename

| Current | Proposed |
|---------|----------|
| `scripts/check-regulation-canonical.sh` | `scripts/check-reg-canonical.sh` |

### What is NOT Renamed

- `SetPoints`, `SetPointCalibrator`, `RegulationPolicy`, `StagnationDetector`, `ImpactReport`, `ActionDecision` — already clear.
- `LoopId` — it's an identifier enum.
- `Signal`, `Deviation`, `SignalMetric` — standard cybernetic terms.
- `SpanNamespace`, `SpanKind`, `SpanCategory`, `ObservableSpan` — generic span infrastructure, not Regulation-specific.
- `Dampener`, `CircuitBreaker`, `GasBudget`, `GasBudgetManager` — already clear.

## Implementation Plan

### Phase 1: Crate rename (`hkask-regulation` → `hkask-regulation`)

1. Rename directory `crates/hkask-regulation` → `crates/hkask-regulation`
2. Update `Cargo.toml`: `name = "hkask-regulation"`
3. Update workspace `Cargo.toml` member list
4. Update all `Cargo.toml` files that depend on `hkask-regulation` → `hkask-regulation`
5. Update all `use hkask_cns::` → `use hkask_regulation::` in `.rs` files
6. Update all `extern crate hkask_cns` → `extern crate hkask_regulation`

### Phase 2: Type renames (compile-checked)

For each type rename, use `sed` to replace across all `.rs` files:
1. `CnsRuntime` → `RegulationLedger`
2. `NuEventStore` → `RegulationArchive`
3. `NuEventSink` → `RegulationSink`
4. `NuEvent` → `RegulationRecord`
5. `CnsSpan` → `RegulationSpan`
6. `CnsHealth` → `LedgerHealth`
7. `CnsObserver` → `LedgerObserver`
8. `CnsStoragePort` → `LedgerStoragePort`
9. `PerPodCnsRuntime` → `PerPodLedger`
10. `SensorProvider` → `Sensor`
11. `SensorRegistry` → `SensorBus` (per-loop)
12. `SensorCatalog` → `SensorRegistry` (system-level)
13. `HkaskLoop` → `RegulationLoop`
14. `Loop` trait → `RegulationLoop` (careful: only the trait, not the word "loop" in general)
15. `LoopSystem` → `LoopScheduler`
16. `LoopQuality` → `LoopMetrics`
17. `LoopAction` → `RegulatoryAction`

### Phase 3: Namespace rename (`reg.*` → `reg.*`)

1. Update `CANONICAL_NAMESPACES` array in `crates/hkask-types/src/event.rs`
2. Update all `tracing::info!(target: "reg...", ...)` → `tracing::info!(target: "reg...", ...)`
3. Update `scripts/check-regulation-canonical.sh` → `scripts/check-reg-canonical.sh`
4. Update CI workflow references to the script name

### Phase 4: Module and file renames

1. `crates/hkask-types/src/reg.rs` → `crates/hkask-types/src/regulation.rs`
2. `crates/hkask-types/src/regulation/` directory → `crates/hkask-types/src/regulation/` (if exists)
3. Update `pub mod regulation` → `pub mod regulation` in `hkask-types/src/lib.rs`
4. Update all `use hkask_types::regulation::` → `use hkask_types::regulation::`

### Phase 5: Documentation updates

1. Update all `docs/**/*.md` files that reference "Regulation", "NuEvent", "SensorRegistry", etc.
2. Update `AGENTS.md`
3. Update architecture docs
4. Update ADRs (add superseded note to old ADRs, not rewrite them)

### Phase 6: Validation

1. `cargo check --workspace` — zero errors
2. `cargo test --workspace` — all tests pass
3. `scripts/check-reg-canonical.sh` — passes
4. `grep -r "CnsRuntime\|NuEvent\|SensorCatalog\|regulation\." crates/ --include="*.rs"` — zero matches (except in historical ADRs)

## Consequences

### Positive

- **Reviewer clarity:** A new contributor can infer the cybernetic function from the name alone.
- **Afferent/efferent distinction:** `SensorBus` (afferent) vs `RegulationLedger` (efferent) makes the cybernetic loop structure visible in the type names.
- **God object decomposition:** `CnsRuntime` → `RegulationLedger` makes it clear this is a recording surface, not a sensing surface. The sensing is in `SensorBus`.

### Negative

- **Large diff:** ~84 files reference `hkask-regulation`/`hkask_cns`, ~88 files have `reg.` tracing targets, ~57 files reference `NuEvent`. The rename touches a significant portion of the codebase.
- **Fresh NuEventStore:** Persisted regulation records on disk will be invalidated by the namespace rename. This is acceptable — no backward compatibility requirement.

### Neutral

- The rename is mechanical — no logic changes, just naming.
- The `reg.*` namespace is shorter than `reg.*` — slightly less typing in tracing calls.

## Verification

```bash
# Verify no old names remain in code
grep -rn "CnsRuntime\|NuEvent\|NuEventStore\|NuEventSink\|CnsSpan\|CnsHealth\|CnsObserver\|CnsStoragePort\|PerPodCnsRuntime\|SensorCatalog\|SensorProvider\|HkaskLoop\|LoopSystem\|LoopQuality\|LoopAction" crates/ mcp-servers/ --include="*.rs" | grep -v "^.*:.*//.*" | wc -l
# Expected: 0

# Verify no old namespace remains
grep -rn 'target:.*"regulation' crates/ mcp-servers/ --include="*.rs" | wc -l
# Expected: 0

# Verify workspace compiles
cargo check --workspace

# Verify tests pass
cargo test --workspace
```

## References

[^ousterhout]: Ousterhout, J. (2018). *A Philosophy of Software Design.* Yaknyam Press. — deep modules: the interface should reveal the implementation's purpose.
[^beer-vsm]: Beer, S. (1979). *The Heart of Enterprise.* Wiley. — VSM: afferent (sensory) vs efferent (motor) pathways.
[^ashby]: Ashby, W. R. (1956). *An Introduction to Cybernetics.* Chapman & Hall. — the cybernetic loop: sense → compare → act → verify.

---

*ℏKask v0.31.0 — A Sovereign Chat Client for Human Users — ADR-057*
