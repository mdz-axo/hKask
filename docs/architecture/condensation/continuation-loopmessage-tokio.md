---
title: "Condensation Continuation — Candidate #3: LoopMessage → tokio Channels"
audience: [architects, developers]
last_updated: 2026-06-09
version: "0.27.0"
status: "Deferred"
domain: "Architecture"
mds_categories: [composition, lifecycle]
---

# Condensation Continuation — Candidate #3: LoopMessage/Signal → tokio Channels

**Status:** Deferred. This is a high-risk infrastructure refactor that should be done as a separate, focused task — not squeezed into the condensation pass.

---

## Background

The original 6-loop architecture required a custom inter-loop messaging system. Communication was treated as a first-class loop (Loop 4) owning message routing, MCP dispatch, and A2A/H2A protocol boundaries. This required typed message passing between loops with priority, tracing, and dispatch semantics.

The condensed 4-loop architecture **demotes Communication from a loop to transport infrastructure**. The custom messaging system (`LoopMessage`, `Signal`, `LoopId`, `WorkerKind`, `Deviation`, `DispatchTarget`, `ActionType`, `LoopPayload`) is a redundant projection of `tokio::mpsc` channels — Rust's standard async messaging primitives.

## Goal

Replace the custom loop messaging infrastructure with `tokio::mpsc` channels, preserving all message semantics while eliminating the redundant types.

## Current State

### Types in `hkask-types/src/loops/`

| Type | File | Purpose | Replacement |
|------|------|---------|-------------|
| `LoopId` | `mod.rs:45-54` | 7 loop identifiers (Inference, Episodic, Semantic, Communication, Curation, Cybernetics, Snapshot) | Reduce to 4 loop identifiers, or eliminate — channel identity replaces loop identity |
| `Signal` | `mod.rs:180-186` | Afferent signal with `SignalMetric` (27 variants) | `tokio::mpsc::Sender<MetricReading>` |
| `LoopMessage` | `dispatch.rs:230-238` | Inter-loop message with `LoopPayload` (9 variants), `TraceId`, `MessagePriority`, `DispatchTarget` | `tokio::mpsc::Sender<Message>` with a simplified payload enum |
| `WorkerKind` | `dispatch.rs:24-29` | Metacognition vs ToolDispatch workers | Eliminate — workers become `tokio::task` spawns |
| `LoopPayload` | `dispatch.rs` | 9 payload variants (AlgedonicAlert, CurationDirective, CyberneticsRegulation, ToolConsumption, GoalTransition, ToolInvocation, ToolResult, SpecDriftAlert) | Simplify to 4-5 variants, or use separate channels per concern |
| `DispatchTarget` | `dispatch.rs` | Loop or Worker routing target | Channel sender identity replaces target |
| `ActionType` | `mod.rs` | AdjustEnergyBudget, OverrideEnergyBudget, ReplenishBudget | These are regulatory concerns that should live in CNS, not in generic messaging |

### Consumers

| Consumer | Crate | What It Uses |
|----------|-------|-------------|
| `MessageDispatch` | `hkask-cns` | Routes `LoopMessage` between loops |
| `CyberneticsLoop::process_inbox()` | `hkask-cns` | Consumes `LoopPayload::AlgedonicAlert`, `ToolConsumption` |
| `CurationLoop` inbox | `hkask-agents` | Consumes `LoopPayload::CurationDirective`, `SpecDriftAlert` |
| `InferenceLoop` | `hkask-agents` | Consumes tool dispatch messages |
| `LoopSystem` | `hkask-agents` | Orchestrates all loop tick cycles |
| `EscalationQueue` | `hkask-agents` | Consumes algedonic escalation signals |

## Approach

### Phase 1 — Audit
1. Map every `LoopPayload` variant to its producer and consumer
2. Identify which variants can be replaced by direct function calls (no channel needed)
3. Identify which variants need channel-based delivery
4. Verify the algedonic alert pathway (Cybernetics → Curation) must survive unbroken

### Phase 2 — Simplify
1. Reduce `LoopId` from 7 to 4 (Inference, Memory, Curation, Cybernetics)
2. Remove `WorkerKind` — workers become async tasks
3. Replace `DispatchTarget` with channel sender handles
4. Replace `LoopMessage` with channel-specific message types per pathway:
   - Alerts channel: `tokio::mpsc::Sender<RuntimeAlert>` (Cybernetics → Curation)
   - Tool channel: `tokio::mpsc::Sender<ToolEvent>` (Inference → Cybernetics)
   - Spec channel: `tokio::mpsc::Sender<SpecEvent>` (Curation internal)

### Phase 3 — Replace
1. Replace `MessageDispatch` with `tokio::mpsc` channel setup in `ServiceContext::build()`
2. Replace `LoopSystem` tick orchestration with channel-based event loops
3. Remove all `LoopMessage`, `Signal`, `WorkerKind`, `DispatchTarget` types
4. Run `cargo check --workspace && cargo test --workspace` at each step

## Risks

1. **Algedonic alert pathway:** The unidirectional Cybernetics → Curation signal is a Prohibition-level constraint. The channel replacement must preserve this path exactly.
2. **Message semantics:** `LoopPayload` variants may carry data that doesn't map cleanly to simpler types. Each variant needs individual migration.
3. **TraceId:** Cross-loop correlation via `TraceId` must be preserved or replaced with equivalent tracing.
4. **Blast radius:** Loop messaging touches `hkask-types`, `hkask-cns`, `hkask-agents`, `hkask-cli` — a change in one crate ripples to all.

## Verification

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
# Verify algedonic alert pathway end-to-end
# Verify no remaining references to LoopMessage, Signal, WorkerKind, DispatchTarget
```

## Predecessor Tasks

All preceding condensation work should be complete before starting this:
- [x] Candidate #5: EnergyBudget rename
- [x] Candidate #1: Visibility 3→2
- [x] Candidate #2: NuEvent/Span — resolved (complementary, no action)
- [x] Documentation cleanup (DDMVSS→MDS, 9→5 categories, 6→4 loops)
- [x] MDS specification (5 categories, 5 tools, 3 curation decisions)

---

*This continuation prompt captures all context needed to resume the LoopMessage→tokio refactor as a standalone task.*
