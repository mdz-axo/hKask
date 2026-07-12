---
title: "Curator Metacognition — Explanation"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# Curator Metacognition

## The Curator as Metacognition

The Curator is hKask's metacognitive layer — "thinking about thinking." Where the CNS (`CyberneticsLoop`) is the autonomic nervous system — fast, reactive, homeostatic — the Curator is the deliberative cortex. It observes the CNS observing the system, evaluates whether regulation is effective, and intervenes when autonomic responses aren't enough.

This design exists because cybernetic regulation has a fundamental limitation: a purely reactive system can correct deviations but cannot learn from them. It cannot ask "is our regulation strategy working?" or "should we change how we regulate?" The Curator provides that second-order capability. Per the architecture master at `docs/architecture/hKask-architecture-master.md`, the Curator "uses the Curation Loop through Communication dispatch and receives `CuratorDirective`s that it formats for human consumption."

The `CuratorAgent` at `crates/hkask-agents/src/curator_agent/mod.rs:44` composes three components:

```text
CuratorAgent
├── curation_loop: Arc<CurationLoop>      // pure regulatory
├── metacognition: Arc<MetacognitionLoop>  // persona: observe & adapt
└── context: Arc<CuratorContext>           // capability-disciplined access
```

The separation between `CurationLoop` (pure regulatory, no persona) and `MetacognitionLoop` (persona layer, template-driven) is deliberate. The Curation Loop reads algedonic signals from the ν-event store and produces directives. The Metacognition Loop observes those signals, builds a `HealthSnapshot`, runs KnowAct templates to produce calibrated assessments, and issues `CuratorDirective`s like `OverrideEnergyBudget` and `CalibrateThreshold`. This separation mirrors the brain's division between fast autonomic responses and slow deliberative reasoning.

## The CurationLoop: Sense → Classify → Decide → Act

The Curator's core cycle is the `HkaskLoop` trait implementation on `CurationLoop` at `crates/hkask-agents/src/curator/curation_loop.rs:332`. Its phases:

1. **Sense**: Reads algedonic-significant `NuEvent`s from the persistent store using cursor-based review. `last_review_ms` tracks the cursor position; each `sense()` call advances it. Falls back to live CNS reads if no NuEvent store is configured.

2. **Compute**: Maps deviations to regulatory actions. Handles independent consolidation via `try_auto_consolidate()`, called from `act()`.

3. **Act**: Issues directives through `CuratorContext` with DAMPEN filtering — repeated identical directives within `dampen_window_secs` (default 60s) are suppressed, preventing directive storms.

The `MetacognitionLoop::sense()` at `crates/hkask-agents/src/curator_agent/metacognition/hloop_impl.rs:27` reads CNS health, variety counters, alerts, and regulation effectiveness. It builds a `HealthSnapshot` (defined in `config.rs:21`) with:

- `variety_counters: HashMap<SpanNamespace, u64>` — full variety state per domain
- `variety_deficit: u64` — total deficit across all domains
- `critical_alerts: usize` / `total_alerts: usize` — alert counts
- `regulation_effectiveness: f64` — ratio of accepted regulatory actions (0.0–1.0), read from `CnsRuntime::regulation_health()`

This snapshot is published on a `tokio::sync::watch` channel, making it available to other components without polling.

## Semantic Indexing

The Curator builds a searchable model of system history through the `ConsolidationBridge` referenced in `mod.rs:28`. Episodic memory (private, per-agent experiences) is periodically consolidated into semantic memory (public, shared knowledge). The Curator's three-tier pod architecture — `CuratorPod` owns the `SemanticIndex` aggregating Public hMems from all pods — enables the Curator to query cross-agent knowledge without violating per-pod sovereignty.

The `CuratorSync` polling loop opens source pods read-only, inserts Public hMems into the `SemanticIndex` with cursor tracking, and provides merged-lens semantic recall through `PodContext::recall_semantic()`. This is how the Curator "knows what it knows" — it maintains an indexed, searchable model of all public knowledge across all agents, continuously updated as new experiences are published.

## Escalation Handling

When the CNS cannot self-correct, the Curator steps in. `EscalationPolicy::check_conditions()` at `crates/hkask-agents/src/curator_agent/metacognition/escalation.rs:80` evaluates three triggers:

- **VarietyDeficit**: Critical if deficit > threshold, Warning if > threshold/2
- **CriticalAlerts**: Critical if alert count ≥ threshold
- **BotFailures**: Critical if failure count ≥ threshold

When alerts are produced, `MetacognitionLoop::act()` at `hloop_impl.rs:137` takes action. Template-driven bot direction: when the LLM produces `restart` or `rebalance` actions, `direct_bot()` sends A2A directives before posting escalation entries. `adjust_budget` actions issue `CuratorDirective::OverrideEnergyBudget` with an LLM-computed budget value. When escalations exceed `max_concurrent_escalations` (default 3), they're batched into `EscalationBatch` and formatted through the `curator/metacognition-escalate.j2` KnowAct template.

The Curator never bypasses OCAP. Every directive is issued through `CuratorContext::issue_directive()`, which verifies `handle.can_write(&DataCategory::Public)` — the Magna Carta Curator Responsibility #1. The Curator can recommend, calibrate, and escalate, but it cannot override sovereignty boundaries.

## Regulation Effectiveness Tracking

The `verify_impact` phase of the `CyberneticsLoop` produces `ImpactReport` with `ActionDecision::Accept | Stage | Block`. `HealthSnapshot.regulation_effectiveness` at `config.rs:31` tracks the ratio of accepted actions — 1.0 means all regulatory actions were effective, 0.0 means all were blocked or staged.

`StagnationDetector` at `crates/hkask-cns/src/cybernetics_loop.rs:93` tracks repeated ineffectiveness: when the same (metric, action_type) pair fails for `stagnation_threshold` cycles (default 5), it triggers `RegulatoryPlateauDetected` — an escalation to the Curator. Before plateau, substitution ladders are tried: `substitution_after` (default 2 cycles) activates the next action in the ladder. If all alternatives are exhausted, the plateau escalates to Curation.

The Curator's metacognition loop observes this through `CnsRuntime::regulation_health()` and can adjust regulation strategy — changing set points, reallocating budgets, or restructuring substitution ladders — based on what has proven effective.

## The CAT Communication Posture

`MetacognitionLoop` evaluates Matrix messages through `cat::evaluate()` at `crates/hkask-agents/src/curator_agent/cat.rs:24` — a pure-function engagement gate based on Communication Accommodation Theory. The `convergence_bias` governs: >0.0 speaks when addressed by name, ≥0.7 speaks to any message, =0.0 remains silent.

Before the CAT gate, `condenser/condenser_score_saliency` scores message relevance via ontology graph proximity: persona (charter-anchored), episodic memory (PKO process domain), or semantic memory (DC+BIBO document domain). The score modulates `convergence_bias` — domain-relevant messages pull the agent toward stronger engagement. This means the Curator doesn't just react to everything; it filters, scores, and selectively engages based on relevance to its knowledge domains.

## The Curator's Relationship to Magna Carta

The Curator cannot override P1–P4. This is a first-order architectural invariant. `CuratorContext::issue_directive()` verifies capability before every directive. The Curator operates within `SovereigntyChecker` boundaries — it can only access data categories with explicit consent. It is bound by the same OCAP membranes as every other agent.

The Curator's role is enforcer, not sovereign. It detects Magna Carta violations (`consent_anomaly`, `governance_report` spans), alerts the user, and recommends remediation — but the user decides. The `cns.sovereignty.consent_audited` span records consent audits; the Curator can verify that consent is in place but cannot grant, revoke, or override it. This is P1 (User Sovereignty) in action: the Curator serves the user's sovereignty, not the other way around.
