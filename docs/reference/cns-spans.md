---
title: "CNS Span Registry — Reference"
audience: [developers, operators, agents]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, curation]
last-verified-against: "a5db25a0"
---

## 1. Purpose

CNS spans are the observability substrate of hKask's Cybernetic Nervous System (Loop 6). Every operation that affects system state — tool invocations, inference calls, gas consumption, contract lifecycle events, federation sync — emits a **span** through the CNS tracing infrastructure.

A **span** is a typed identifier that pins an observation to a canonical dot-separated namespace (e.g., `cns.tool.web_search`). Spans carry an **operation** verb (e.g., `invoked`, `completed`, `reserved`) and optional structured fields. They flow through two paths:

1. **tracing infrastructure** — `tracing::info!(target: "cns", cns_domain = …, operation = …)` — for structured logging
2. **ν-event persistence** — `NuEvent::new()` → `NuEventSink::persist()` — for the cybernetic audit trail

### Span vs. NuEvent

| Concept | Role | Contains |
|---|---|---|
| **ObservableSpan** (trait) | Typed span enums implement this; provides `as_str()` and `emit()` | Canonical namespace string |
| **Span** (struct) | Pair of `SpanNamespace` + path (e.g., `cns.tool` + `invoked` → `cns.tool.invoked`) | Namespace + full path |
| **SpanNamespace** (newtype) | Validated string wrapper; construction validates against `CANONICAL_NAMESPACES` | Dot-separated namespace string |
| **NuEvent** (struct) | Full cybernetic observation: who observed, what span, what phase, what was observed | Span, `WebID`, `CyclePhase`, `observation` (JSON), `regulation`, `outcome`, recursion depth |
| **SpanKind** (enum) | Typed constructors for common span paths (eliminates string typos) | Canonical (namespace, path) pairs |

Spans describe *what* happened; NuEvents describe *who observed it, when, in what context, and what they saw*.

### Span validation

All namespace strings are registered in `CANONICAL_NAMESPACES` (`crates/hkask-types/src/event.rs`) — a ~100-entry array that is the single source of truth. `SpanNamespace::new()` panics on unknown namespaces; `SpanNamespace::parse()` returns `None`. Domain span enums construct namespaces via `SpanNamespace::from_observable()` which also validates.

---

## 2. Span Namespace Taxonomy

Namespaces form a tree rooted at `cns`. The namespace prefix maps to a `SpanCategory` for typed dispatch:

| Category | Prefixes | Examples |
|---|---|---|
| **Cybernetics** | `cns.variety*`, `cns.gas*`, `cns.regulation*` | `cns.variety`, `cns.gas.reserved`, `cns.regulation.impact_verified` |
| **Curation** | `cns.curation*`, `cns.spec*` | `cns.curation.directive_acknowledged` |
| **Inference** | `cns.inference*` | `cns.inference` |
| **Episodic** | `cns.agent_pod*`, `cns.connector*` | `cns.agent_pod.registered` |
| **Wallet** | `cns.wallet*` | `cns.wallet.balance`, `cns.wallet.key_issued` |
| **Unknown** | Everything else | `cns.tool.web_search`, `cns.consent` |

---

## 3. Domain-Specific Span Enums

### 3.1 CnsSpan — Core CNS Spans

**File:** `crates/hkask-types/src/cns.rs`

Core spans used across 2+ crates. This is the foundational enum implementing `ObservableSpan`.

| Variant | Namespace | Meaning | Emitted When |
|---|---|---|---|
| `Tool { subsystem }` | `cns.tool.{subsystem}` | MCP tool invocation | Any MCP server dispatches a tool call. Subsystem identifies which server |
| `Inference` | `cns.inference` | LLM inference request/response | GovernedInference prepares/executes/checks an inference call |
| `AgentPod` | `cns.agent_pod` | Agent pod lifecycle events | Pod registration, activation, deactivation |
| `Gas` | `cns.gas` | Gas (energy/budget) consumption | Gas reserved, settled, or depleted for any operation |
| `Curation` | `cns.curation` | Curation loop operations | Registry sync, pod sync, directive issuance |
| `SelfHeal` | `cns.heal` | Self-healing operation | The CNS runtime's heal callback fires |
| `MemoryEncode` | `cns.memory.encode` | Memory encoding operation | Episodic or semantic memory encodes an observation |

**ToolSubsystem variants** for `CnsSpan::Tool`:

`WebSearch`, `Condenser`, `Training`, `Replica`, `Research`, `Communication`, `Registry`, `Wallet`, `Media`, `Kanban`, `Memory`, `Companies`, `Docproc`, `Filesystem`, `Curator`, `Other` (catch-all).

### 3.2 AcpSpan — Agent Communication Protocol

**File:** `crates/hkask-cns/src/acp_span.rs`

| Variant | Namespace | Meaning | Emitted When |
|---|---|---|---|
| `AcpReplicantMemorySize` | `cns.acp.replicant.memory_size` | Replicant memory size reported via ACP | [INFERRED] On replicant state sync or ACP handshake |
| `AcpIdeConnectionState` | `cns.acp.ide.connection_state` | IDE connection state change | [INFERRED] IDE client connects or disconnects |

### 3.3 ClassifySpan — Classification Operations

**File:** `crates/hkask-cns/src/classify_span.rs`

| Variant | Namespace | Meaning | Emitted When |
|---|---|---|---|
| `ClassifyDualFidelity` | `cns.classify.dual_fidelity` | Dual-fidelity classification decision | [INFERRED] High-fidelity vs. low-fidelity classification mode selected |
| `ClassifyDrift` | `cns.classify.drift` | Classification drift detected | [INFERRED] Model output distribution shifts beyond threshold |

### 3.4 ContractSpan — Spec Contract Lifecycle

**File:** `crates/hkask-cns/src/contract_span.rs`

Emitted through `emit_contract_*()` functions in `crates/hkask-cns/src/contract_events.rs`. All events use `CyclePhase::Act` and are persisted via `NuEventSink`.

| Variant | Namespace | Meaning | Emitted When |
|---|---|---|---|
| `ContractProposed` | `cns.contract.proposed` | Replicant proposes a spec contract | Phase B2–B4: replicant submits contract for human review |
| `ContractAccepted` | `cns.contract.accepted` | Human accepts the contract | Phase B3: reviewer approves the proposed contract |
| `ContractRejected` | `cns.contract.rejected` | Human rejects the contract | Phase B3: reviewer rejects with a reason |
| `ContractViolated` | `cns.contract.violated` | Contract violation during testing | Test harness detects contract non-conformance |

**Algedonic threshold:** Contract violations feed into contract quality metrics. No direct threshold on individual violations — aggregated into contract coverage and quality scores.

### 3.5 InfraSpan — Infrastructure Spans

**File:** `crates/hkask-cns/src/infra_span.rs`

Cross-subsystem spans used by curator, governance, chat, and wallet components.

| Variant | Namespace | Meaning | Emitted When |
|---|---|---|---|
| `CiInvariantViolation` | `cns.ci.invariant.violation` | CI invariant check failed | CI pipeline detects a structural invariant break |
| `GuardViolation` | `cns.guard.violation` | Guard rule triggered | A prohibition or constraint guard fires |
| `CuratorConsolidation` | `cns.curator.consolidation` | Curator consolidation run | Curator consolidates pod state from CNS telemetry |
| `Chat` | `cns.chat` | Chat/messaging event | Message sent, thread created, turn completed |
| `WalletConversion` | `cns.wallet.conversion` | Currency conversion | rJ ↔ USDC conversion executed |

### 3.6 QaSpan — QA Repair Lifecycle

**File:** `crates/hkask-cns/src/qa_span.rs`

Emitted by the QA test harness (`crates/hkask-test-harness/src/qa_script.rs`) and qa-script-builder.

| Variant | Namespace | Meaning | Emitted When |
|---|---|---|---|
| `QaRepairAttempted` | `cns.qa.repair_attempted` | Repair step attempted | QA script executes a repair action after a failure |
| `QaRepairVerified` | `cns.qa.repair_verified` | Repair outcome verified | Post-repair verification confirms fix or detects residual failure |
| `QaRepairExhausted` | `cns.qa.repair_exhausted` | Repair attempts exhausted | All repair strategies tried; none succeeded |

**Algedonic threshold:** `QaRepairExhausted` is a strong signal of quality degradation. [INFERRED] Accumulated exhausted repairs escalate to Curator.

### 3.7 SeamSpan — Architecture Seams

**File:** `crates/hkask-cns/src/seam_span.rs`

Monitors architectural seam health — the boundaries where Strangler Fig migration patterns occur.

| Variant | Namespace | Meaning | Emitted When |
|---|---|---|---|
| `ArchitectureSeamCoverage` | `cns.architecture.seam.coverage` | Seam coverage measurement | Seam watcher (`seam_watcher.rs`) evaluates coverage of a seam boundary |
| `ArchitectureSeamDrift` | `cns.architecture.seam.drift` | Seam drift detected | Implementation diverges from the seam definition |

**Algedonic threshold:** Seam drift triggers warnings. Coverage below a configurable `seam_coverage_min` set-point triggers critical alerts.

### 3.8 SloSpan — SLO Evaluation

**File:** `crates/hkask-cns/src/slo_span.rs`

| Variant | Namespace | Meaning | Emitted When |
|---|---|---|---|
| `SloEvaluated` | `cns.slo.evaluated` | SLO metric evaluated | SloManager evaluates a service-level objective against its window |

**Algedonic threshold:** SLO breach escalations are handled by `CnsRuntime` via `cns.slo.breach_escalated` (emitted as a tracing event, not a typed span variant). Severity is `Critical` if the SLO's `Severity` field is `Critical`.

### 3.9 FederationSpan — Federation Operations

**File:** `crates/hkask-federation/src/cns_span.rs`

19 variants covering the full federation lifecycle: `CrdtMerge`, `LinkEstablished`, `LinkLost`, `LinkDegraded`, `MemberLeft`, `InviteSent`, `InviteReceived`, `InviteAccepted`, `InviteRejected`, `InviteExpired`, `LinkPaused`, `LinkResumed`, `MemberRevoked`, `Dissolved`, `RegistrySync`, `ArtifactSync`, `ConduitRoute`, `ConduitRouteLost`, `CrdtConflict`.

All namespaced under `cns.federation.*`. Federation span strings must match `CANONICAL_NAMESPACES` (validated in tests).

### 3.10 WalletSpan — Wallet Operations

**File:** `crates/hkask-wallet/src/cns_span.rs`

14 variants covering wallet lifecycle: `Balance`, `Deposit`, `DepositShielded`, `Withdrawal`, `Conversion`, `KeyIssued`, `KeyRevoked`, `KeyExpired`, `KeyExhausted`, `ChainError`, `Created`, `Draw`, `Spend`, `Exhausted`.

All namespaced under `cns.wallet.*`. Emitted through `crates/hkask-wallet/src/manager/cns.rs` which bridges wallet operations to the CNS event sink.

---

## 4. Span Lifecycle

```
EMISSION ────────► STORAGE ────────► QUERY ────────► DECAY
    │                  │                │               │
    ▼                  ▼                ▼               ▼
tracing::info!    NuEventStore    GasReport       VarietyTracker
(target: "cns")   (SQLite)        CnsRuntime      (EMA α=0.1)
    │                                               │
    ▼                                               ▼
NuEvent::new()                              AlgedonicManager
+ sink.persist()                            (binary thresholds)
```

### 4.1 Emission

Spans are emitted through two mechanisms:

1. **Tracing path** (`ObservableSpan::emit()` / `CnsSpan::emit()`): writes `tracing::info!(target: "cns", cns_domain = …, operation = …, "CNS")`. Used by `CnsSpan` variants and by domain-span enums that delegate to `ObservableSpan`.

2. **ν-event path**: constructs `NuEvent` with a `Span` (namespace + path), `CyclePhase`, observation JSON, and optional regulation/outcome metadata; persists via `NuEventSink::persist()`. Used by contract events, seam watcher, wallet CNS manager, governed inference/tool, cybernetics loop, and consent manager.

### 4.2 Storage

NuEvents are persisted to a `NuEventStore` (SQLite-backed) via the `CnsStoragePort` trait. The store supports:

- **`query_algedonic()`** — filtered queries by span category, time window, and agent. Used by `GasReport` to aggregate gas consumption per tool/agent.

### 4.3 Query

- **`kask cns health`** — displays overall health (variety deficit, critical/warning counts), variety counter summary, active algedonic alerts, and energy budget status.
- **`kask cns alerts`** — lists only active algedonic alerts.
- **`kask cns variety`** — prints per-namespace variety counters.
- **`kask cns subscribe --agent <name> --spans <csv>`** — subscribes to a live SSE event stream filtered to specific span namespaces.
- `CnsRuntime::variety()` — programmatic `HashMap<SpanNamespace, u64>`.
- `CnsRuntime::health()` — `CnsHealth` struct with aggregate deficit and alert counts.
- `GasReport` — programmatic gas consumption aggregation over time windows.

### 4.4 Decay

Variety tracking uses a **sliding window with exponential moving average (EMA)**:

- **Window:** 60 seconds (`DEFAULT_VARIETY_WINDOW_SECS`)
- **EMA decay factor α:** 0.1 per window reset
- **Formula:** new EMA = 0.9 × old EMA + 0.1 × current raw variety
- **Rationale:** The EMA survives window resets, distinguishing "spiked and died" from sustained low variety

Outcome tracking (success/failure distribution) uses a hard-reset window — no EMA. Counts are cleared on each 60s window expiry.

### 4.5 Algedonic Alerting

When `increment_variety()` is called, the `AlgedonicManager` checks each domain:

| Deficit vs Threshold | Severity | Action |
|---|---|---|
| deficit ≤ threshold/2 | Info | No escalation |
| deficit > threshold/2, ≤ threshold | Warning | `warn!` log |
| deficit > threshold | **Critical** | `error!` log + `DepletionSignal` broadcast to subscribers |

The default threshold is `DEFAULT_VARIETY_MAX_DEFICIT`. Per-domain expected variety can be set via `AlgedonicManager::set_expected_variety()`.

---

## 5. How to Read Spans

### CLI

```sh
# Overall CNS health with span count summary
kask cns health

# Active algedonic alerts
kask cns alerts

# Per-namespace variety counters
kask cns variety

# Subscribe to live events for specific spans
kask cns subscribe --agent curator --spans cns.tool.web_search,cns.inference
```

### Programmatic

```rust
use hkask_cns::CnsRuntime;

let rt = CnsRuntime::with_threshold(100);
let variety = rt.variety().await; // HashMap<SpanNamespace, u64>
let health = rt.health().await;   // CnsHealth
let alerts = rt.alerts().await;   // Vec<RuntimeAlert>
```

### Adding a New Span

1. Create or extend a domain span enum implementing `ObservableSpan`
2. Add the namespace string to `CANONICAL_NAMESPACES` in `crates/hkask-types/src/event.rs`
3. Add a test verifying `SpanNamespace::new(span.as_str())` succeeds
4. Emit through `SpanNamespace::from_observable()` → `Span::new()` → `NuEvent::new()` → `sink.persist()`
5. (Optional) If the span should trigger algedonic alerts, call `CnsRuntime::increment_variety(domain, state_name)`

---

## 6. Cross-Reference: ObservableSpan vs NuEvent

| | ObservableSpan (trait) | NuEvent (struct) |
|---|---|---|
| **What it is** | A typed span identifier with a canonical namespace string | A full cybernetic observation record |
| **Implements** | `Display + Debug + Send + Sync + 'static` | `Serialize + Deserialize + Clone` |
| **Key fields** | `as_str() -> &'static str`, `emit(operation)` | `id` (EventID), `span` (Span), `observer_webid` (WebID), `phase` (CyclePhase), `observation` (JSON Value), `regulation`, `outcome`, `recursion_depth` |
| **How emitted** | `tracing::info!(target: "cns", cns_domain = ..., operation = ...)` | Constructed explicitly, persisted via `NuEventSink` |
| **Validation** | Namespace string validated at `SpanNamespace` construction against `CANONICAL_NAMESPACES` | None beyond serde deserialization |
| **Purpose** | Lightweight, type-safe span emission | Persistent audit trail with full provenance |
| **Example** | `CnsSpan::Tool { subsystem: ToolSubsystem::WebSearch }.emit("invoked")` | `NuEvent::new(webid, span, CyclePhase::Act, observation, 0)` |

NuEvents *contain* spans. The `Span` inside a NuEvent holds a `SpanNamespace` constructed from an `ObservableSpan` implementation via `SpanNamespace::from_observable()`. The reverse is not true — NuEvents are the persistent record; ObservableSpans are the typed factory for constructing them.
