---
title: "ν-Event Semantics — Explanation"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# ν-Event Semantics

## What a ν-Event Is

A ν-event (nu-event) is a thin domain event — a timestamped, attributed, namespaced observation that enters the cybernetic nervous system. It is the atomic unit of observability in hKask. The `NuEvent` struct at `crates/hkask-types/src/event.rs:16` carries:

- `id: EventID` — unique identifier
- `timestamp: DateTime<Utc>` — when the event occurred
- `observer_webid: WebID` — who observed it (the agent or system component)
- `span: Span` — a (namespace, path) pair identifying where in the system
- `phase: CyclePhase` — which phase of the cybernetic cycle (Sense, Compute, Compare, Act, Verify)
- `observation: Value` — arbitrary JSON payload describing what was observed
- `regulation: Option<Value>` — optional regulatory metadata
- `outcome: Option<Value>` — optional outcome data
- `recursion_depth: u8` — for nested/recursive operations
- `parent_event: Option<EventID>` — causal chain link
- `visibility: String` — `"private"` by default

A ν-event is an **assertion**, not a trace. It says "at time T, observer O witnessed fact F in domain D during phase P." It is persisted, queryable, and replayable. It feeds the CNS homeostatic loop.

## ObservableSpan vs NuEvent

This distinction is crucial. `ObservableSpan` (at `crates/hkask-types/src/observable_span.rs:47`) is a trait that typed span enums implement — it produces a canonical dot-separated namespace string like `"cns.tool.web_search"`. `CnsSpan` is the primary implementor, but the trait is designed to be domain-extensible: `FederationSpan`, `WalletSpan`, and other domain span enums can implement it. A span is a **trace** — it marks where in the system something happened.

A `NuEvent` contains a `Span`, but it adds: *who*, *when*, *what*, *which phase*, and *what was the regulatory outcome*. A span says "tool invoked"; a ν-event says "Agent A invoked the web_search tool in the Sense phase, observing {server, tool, estimated_cost}, with no regulation applied, at recursion depth 0."

The bridging function is `SpanNamespace::from_observable()` — it takes any `impl ObservableSpan`, validates against the canonical namespace set in `CANONICAL_NAMESPACES` (94 entries at v0.31.0, listed in `event.rs:111-284`), and produces a validated `SpanNamespace` for `NuEvent` construction. This design decouples domain span definitions from namespace validation: domain crates define their spans; the event system validates.

## The Emission Contract

The emission contract has three participants:

- **Emitter** — Any system component that creates a `NuEvent`. `GovernedTool::invoke()` is the canonical emitter for tool invocations; `CyberneticsLoop` emits regulation spans; the Curator emits curation spans. The emitter constructs the event with `NuEvent::new(observer_webid, span, phase, observation, recursion_depth)`, optionally chaining `.with_outcome()`, `.with_regulation()`, `.with_parent()`, and `.with_visibility()`.

- **Sink** — `NuEventSink` (line 640) is the persistence trait. It has a single method: `fn persist(&self, event: &NuEvent) -> Result<(), InfrastructureError>`. The production implementation is `NuEventStore` in `hkask-storage`. The sink is the durable boundary — once persisted, the event is available for CNS sensing, Curator review, and forensic audit.

- **Observer** — The CNS itself. `CurationLoop::sense()` reads algedonic-significant events from the store using cursor-based review. `CyberneticsLoop::sense()` reads via sensor providers (`SensorProvider` trait). Events are also replayed with decay weighting via `CnsStoragePort::replay_weighted()`.

## CNS Span Namespaces

The `CnsSpan` enum at `crates/hkask-types/src/cns.rs:111` defines the core span identifiers. Each variant maps to a canonical namespace string:

| Variant | Namespace | Purpose |
|---------|-----------|---------|
| `Tool { subsystem }` | `cns.tool.{subsystem}` | 15 MCP subsystems: web_search, condenser, training, replica, research, communication, registry, wallet, media, kanban, memory, companies, docproc, filesystem, curator |
| `Inference` | `cns.inference` | LLM request/response |
| `AgentPod` | `cns.agent_pod` | Pod lifecycle events |
| `Gas` | `cns.gas` | Energy consumption tracking |
| `Curation` | `cns.curation` | Registry sync, pod sync, directive issuance |
| `SelfHeal` | `cns.heal` | Self-healing operations |
| `MemoryEncode` | `cns.memory.encode` | Memory encoding events |

The `SpanKind` enum at `event.rs:523` provides typed construction for common spans, eliminating string typos: `ToolInvoked`, `ToolCompleted`, `GasReserved`, `GasSettled`, `GasDepleted`, `CurationDirectiveAcknowledged`, `CurationEscalation`, `AgentPodRegistered`, `AgentPodActivated`, `VarietyAlgedonicAlert`, and the v0.31.0 regulation spans (`ImpactVerified`, `ActionSubstituted`, `ActionBlocked`, `RegulatoryPlateauDetected`, `LoopQualityTelemetry`).

Beyond `CnsSpan`, the `CANONICAL_NAMESPACES` array registers 94 total namespace strings spanning architecture seams, chat, CI, classification, condenser, consent, consolidation, contracts, curation, cybernetics, deploy, federation (14 spans), gas, guard, healing, inference, kata, MCP media, memory, multi-agent, platform metrics (11 spans for PaaP/DORA/SPACE/Loyalty), QA (4 spans), regulation, replicant, semantic, skills, SLOs, sovereignty (5 spans), specs, storage, tools, variety, and wallet (10 sub-spans).

## How ν-Events Feed the CNS Homeostatic Loop

The CNS loop is `sense → compare → compute → act → verify`. ν-events enter at `sense` — they are the afferent signals. `CyberneticsLoop::sense()` (at `cybernetics_loop.rs:733`) reads via pluggable `SensorProvider` implementations: `EnergyBudgetSensor`, `VarietySensor`, `WalletKeyHealthSensor`. Each sensor queries the ν-event store for relevant events and produces `Signal` values with metrics and set-points.

In the `compare` phase, signals are measured against set-points to produce `Deviation` values with direction (`AboveSetPoint` / `BelowSetPoint`). In `compute`, deviations map to `LoopAction` with action types like `Calibrate`, `Escalate`, `Throttle`, `Notify`. In `act`, actions are executed — directive issuance, budget adjustments, algedonic alerts. In `verify_impact`, the `ImpactReport` records whether actions were effective, producing `ActionDecision::Accept | Stage | Block`.

The `parent_event` field creates causal chains: the `cns.tool.completed` event has `parent_event` set to the `cns.tool.invoked` event's ID. This enables causality tracing through the event graph.

## WeightedEvent and Decay

Events don't persist at full weight forever. `WeightedEvent` at `crates/hkask-ports/src/cns.rs:106` pairs a `NuEvent` with a `weight: f64`. `DecayConfig` (line 116) defines per-category exponential decay constants: cybernetics has a 5-minute half-life, curation 15 minutes, inference 2 minutes, episodic 10 minutes. Events below `weight_threshold` (default 0.001) are not replayed. This implements episodic memory — recent events matter more than ancient ones — and prevents the CNS from drowning in historical noise.

The `CnsStoragePort::replay_weighted()` method provides time-decayed event replay, enabling the CNS to reconstruct system state from recent history without loading the entire event store. This is the computational expression of the least-action principle applied to observability: only the computationally cheapest (most recent, most salient) events factor into regulation decisions.
