---
title: "Condensation Continuation — Candidate #3: LoopMessage → tokio Channels"
audience: [architects, developers]
last_updated: 2026-06-09
version: "0.27.0"
status: "Complete — 2026-06-09"
domain: "Architecture"
mds_categories: [composition, lifecycle]
---

# Condensation Continuation — Candidate #3: LoopMessage/Signal → tokio Channels

**Status:** Complete. All three phases finished. All pathways migrated to direct `tokio::mpsc` channels. `LoopMessage`, `LoopPayload`, `MessageDispatch`, `CommunicationLoop`, `DispatchTarget`, `WorkerKind`, and `MessagePriority` all deleted. `LoopId` reduced from 7 variants to 4: `Inference`, `Memory`, `Curation`, `Cybernetics`. `CurationInput` unified inbox wired. `SnapshotLoop` registered with `LoopSystem`. `GoalTransitionEvent` producer wired in API route.

---

## Progress Summary

### ✅ Completed — Phase 1: Audit

Mapped every `LoopPayload` variant to its producer and consumer. See `docs/architecture/condensation/` for audit details.

### ✅ Completed — Phase 2a: New domain types

Created `hkask-types/src/loops/channels.rs` with:
- `RuntimeAlert` — algedonic alert (Cybernetics → Curation)
- `ToolConsumptionEvent` — gas usage report (GovernedTool → Cybernetics)
- `SpecEvent` — spec drift alert (SpecCurator → Curation)
- `GoalTransitionEvent` — goal state change (GoalStore → Curation)
- `CurationInput` — unified inbox enum (future use)

### ✅ Completed — Phase 2b: Strangler fig — 4 direct pathways wired

All 4 pathways have direct `tokio::mpsc` channels operating alongside the legacy `LoopMessage`/`CommunicationLoop` pipeline:

| # | Pathway | Channel Type | Producer | Consumer |
|---|---------|-------------|----------|----------|
| 1 | Alerts | `Sender<RuntimeAlert>` / `Receiver<RuntimeAlert>` | `CyberneticsLoop::act()` | `CurationLoop::sense()` |
| 2 | Tool consumption | `Sender<ToolConsumptionEvent>` / `Receiver<ToolConsumptionEvent>` | `GovernedTool::invoke()` | `CyberneticsLoop::process_inbox()` |
| 3 | Spec events | `Sender<SpecEvent>` / `Receiver<SpecEvent>` | `DefaultSpecCurator::evaluate()` | `CurationLoop::sense()` |
| 4 | Goal transitions | `Sender<GoalTransitionEvent>` / `Receiver<GoalTransitionEvent>` | (no producer yet) | `CurationLoop::sense()` |

### ✅ Completed — Phase 2c: Partial type deletions

These types/fields have been removed from the codebase:

| Type/Field | Status |
|---|---|
| `WorkerKind` | **Deleted** — removed from `dispatch.rs`, all consumers, re-exports |
| `HkaskLoop::worker_kind()` | **Removed** from trait |
| `DispatchTarget` | **Deleted** from public API (type definition still exists in `dispatch.rs` but is unused) |
| `LoopAction.target` | Changed `DispatchTarget` → `LoopId` |
| `LoopAction::new(target, ...)` | Simplified to accept `LoopId` directly |
| `LoopMessage.target_loop` | Changed `Option<DispatchTarget>` → `Option<LoopId>` |
| `LoopMessage.with_target()` | Takes `LoopId` directly |
| `CommunicationLoop` worker routing | `worker_senders` field, `register_worker_inbox()` removed |
| `LoopSystem::register_loop()` | Worker registration removed |
| `MetacognitionLoop::worker_kind()` | Removed |
| `CuratorContext::loop_dispatch_tx` | Field, `with_loop_dispatch_tx()` builder, `loop_dispatch_tx()` accessor all removed |
| `DefaultSpecCurator::dispatch_tx` | Field and `with_dispatch()` builder removed |
| `DefaultSpecCurator::evaluate()` | Old `LoopMessage`/`LoopPayload` path removed — uses direct `spec_tx` only |
| `InferenceLoop::dispatch_tx` | Field, `with_dispatch()` builder, and dead act() code removed |

### 🟡 Remaining — Phase 3: Complete legacy deletion + LoopId reduction

The following types still exist because they serve remaining pathways that haven't been migrated to direct channels:

## Remaining Pathways (Need Direct Channels)

### Pathway A: CurationDirective (Curation → Cybernetics)

**Current flow:**
```
CurationLoop::act() → CuratorContext::issue_directive() → MessageDispatch::send_curator_directive()
  → CommunicationLoop → CyberneticsLoop::process_inbox() → handle_curation_directive()
```

**Target:** Direct `tokio::mpsc::Sender<CuratorDirective>` / `tokio::mpsc::Receiver<CuratorDirective>` channel

**Files to change:**
- `CuratorContext` — replace `dispatch: Arc<MessageDispatch>` with `curator_directive_tx: Sender<CuratorDirective>`
- `CyberneticsLoop` — add `curator_directive_rx` receiver, drain in `process_inbox()`
- `CuratorContext::issue_directive()` — send on direct channel instead of `dispatch.send_curator_directive()`
- `CurationLoop::act()` — send CuratorDirective on direct channel
- `MetacognitionLoop::issue_directive()` — send on direct channel
- `ServiceContext::build()` — create channel, wire to both
- `hkask-cli/src/commands/curator.rs` — update to use direct channel

### Pathway B: CyberneticsRegulation (Cybernetics → domain loops)

**Current flow:**
```
CyberneticsLoop::act() → dispatch_tx → MessageDispatch → CommunicationLoop → target loop inbox
```

**Target:** Direct `tokio::mpsc::Sender<Regulation>` channels

**Approach:** The remaining `LoopPayload::CyberneticsRegulation` variant carries throttle, calibrate, and circuit-break actions from Cybernetics to domain loops (Inference, Episodic, Semantic). These are infrequent regulatory signals. Each target loop could have its own `Sender<Regulation>` channel, or a unified `Sender<Regulation>` with target baked into the type.

**Files to change:**
- `CyberneticsLoop` — replace `dispatch_tx: Sender<LoopMessage>` with `regulation_txs: HashMap<LoopId, Sender<Regulation>>`
- Domain loops — add `regulation_rx` receivers
- `ServiceContext::build()` — create channels, wire to all loops

### Pathway C: GovernedTool → Cybernetics (legacy dispatch_tx)

**Current flow (legacy):** `GovernedTool::invoke()` sends `LoopMessage` via `dispatch_tx` alongside the direct `tool_consumption_tx`.

**Target:** Remove the legacy dispatch_tx send — the direct `tool_consumption_tx` channel fully replaces it.

**Files to change:**
- `GovernedTool` — remove `dispatch_tx` field, remove legacy send in `invoke()`
- `ServiceContext::build()` — stop passing `loop_system.dispatch_sender()` to `GovernedTool`
- `hkask-cli/src/repl/init.rs` — stop passing dispatch_tx

## Remaining Files to Delete

Once pathways A, B, and C are migrated:

| File | Action |
|------|--------|
| `crates/hkask-agents/src/communication/communication_loop.rs` | **Delete** — the entire CommunicationLoop |
| `crates/hkask-agents/src/communication/dispatch.rs` | **Delete** — MessageDispatch priority queue |
| `crates/hkask-agents/src/communication/mod.rs` | **Delete** (or reduce to empty) |

## Remaining Types to Delete from `hkask-types`

| Type | Action |
|------|--------|
| `LoopMessage` | **Delete** — replaced by per-pathway domain types |
| `LoopPayload` | **Delete** — variants migrated to domain types |
| `DispatchTarget` | **Delete** — type definition (already removed from public API) |
| `MessagePriority` | Keep (used by `LoopAction`) or inline into `ActionType` |

## Remaining LoopSystem Simplification

Once `MessageDispatch` and `CommunicationLoop` are deleted:

| Field/Method | Action |
|-------------|--------|
| `LoopSystem::dispatch: Arc<MessageDispatch>` | **Remove** |
| `LoopSystem::communication_loop: Arc<CommunicationLoop>` | **Remove** |
| `LoopSystem::inbox_senders` | **Remove** |
| `LoopSystem::inbox_receivers` | **Remove** |
| `LoopSystem::dispatch_tx` | **Remove** |
| `LoopSystem::dispatch_rx` | **Remove** |
| `LoopSystem::dispatch_sender()` | **Remove** |
| `LoopSystem::new(dispatch)` | Change to `LoopSystem::new()` — no parameter needed |
| `LoopSystem::start()` | Remove dispatch forwarder and CommLoop tick tasks |

## LoopId Reduction (7 → 4)

**Target:** `Inference`, `Memory` (merges Episodic + Semantic), `Curation`, `Cybernetics`

**Files to change:**
- `hkask-types/src/loops/mod.rs` — remove `Communication`, `Snapshot`, merge `Episodic` + `Semantic` → `Memory`
- All `LoopId::Episodic` references → `LoopId::Memory`
- All `LoopId::Semantic` references → `LoopId::Memory`
- All `LoopId::Communication` references → delete (channel identity replaces it)
- All `LoopId::Snapshot` references → delete or merge into `Cybernetics`
- `AUTHORITY_ORDER` in `loop_system.rs`
- `default_tick_interval()` in `loop_system.rs`

## Verification Plan

After each migration step:
```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

Final verification:
```bash
# Verify no remaining references
grep -r "LoopMessage" crates/ --include="*.rs"
grep -r "LoopPayload" crates/ --include="*.rs"
grep -r "MessageDispatch" crates/ --include="*.rs"
grep -r "CommunicationLoop" crates/ --include="*.rs"
grep -r "DispatchTarget" crates/ --include="*.rs"
# Verify LoopId only has 4 variants
grep -c "LoopId::" crates/hkask-types/src/loops/mod.rs
```

## Key Files Modified So Far

| File | Changes |
|------|---------|
| `crates/hkask-types/src/loops/channels.rs` | **New** — 5 domain message types |
| `crates/hkask-types/src/loops/mod.rs` | Register channels, simplified LoopAction, removed worker_kind(), updated re-exports |
| `crates/hkask-types/src/loops/dispatch.rs` | Removed WorkerKind, DispatchTarget (type def still present) |
| `crates/hkask-types/src/lib.rs` | Updated re-exports |
| `crates/hkask-cns/src/cybernetics_loop.rs` | `alerts_tx`, `tool_consumption_rx`, switched to LoopId comparisons |
| `crates/hkask-cns/src/governed_tool.rs` | `tool_consumption_tx`, direct send |
| `crates/hkask-agents/src/communication/communication_loop.rs` | Removed WorkerKind routing |
| `crates/hkask-agents/src/loop_system.rs` | Removed worker_kind handling |
| `crates/hkask-agents/src/curator/context.rs` | Removed `loop_dispatch_tx` |
| `crates/hkask-agents/src/curator/curation_loop.rs` | `alerts_rx`, `spec_rx`, `goal_rx`, drains |
| `crates/hkask-agents/src/curator_agent/mod.rs` | Threaded channels, simplified constructors |
| `crates/hkask-agents/src/curator_agent/spec_curator.rs` | `spec_tx`, removed dispatch_tx |
| `crates/hkask-agents/src/curator_agent/metacognition.rs` | Removed worker_kind impl |
| `crates/hkask-agents/src/inference_loop.rs` | Removed dispatch_tx and dead code |
| `crates/hkask-services/src/context.rs` | Channel creation + wiring at ServiceContext::build() |

## Recommended Execution Order

1. **Pathway C** (GovernedTool legacy dispatch_tx) — simplest, already redundant
2. **Pathway A** (CurationDirective direct channel) — medium complexity, involves CLI
3. **Pathway B** (CyberneticsRegulation direct channels) — most complex, multi-target
4. **File deletions** — `CommunicationLoop`, `MessageDispatch`, `communication/mod.rs`
5. **Type deletions** — `LoopMessage`, `LoopPayload`, `DispatchTarget` (type def)
6. **LoopSystem simplification** — remove dispatch fields
7. **LoopId reduction** — 7 → 4

## Risks (unchanged from original)

1. **Algedonic alert pathway:** ✅ Already migrated (Pathway 1 — direct RuntimeAlert channel).
2. **Message semantics:** CurationDirective and CyberneticsRegulation need careful migration.
3. **TraceId:** Currently only used in `CuratorContext::issue_directive()`. Can be replaced with tracing or removed.
4. **Blast radius:** Remaining changes touch `hkask-services`, `hkask-cli`, `hkask-cns`, `hkask-agents`.

---

*This continuation prompt captures all context needed to resume the LoopMessage→tokio refactor at Phase 3.*
