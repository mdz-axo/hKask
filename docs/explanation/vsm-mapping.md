---
title: "Viable System Model (VSM) Mapping — Explanation"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# Viable System Model (VSM) Mapping

## What the Viable System Model Is

The Viable System Model (VSM), developed by Stafford Beer, is a cybernetic framework for understanding how any system — biological, organizational, or computational — maintains viability in a changing environment. Beer's core insight: a viable system must have the internal variety to match the variety of its environment (Ashby's Law of Requisite Variety), and it organizes this variety through five recursive system levels (S1–S5).

This design exists because hKask is not a passive monitoring system. It is a cybernetic regulator. Per the architecture master at `docs/architecture/hKask-architecture-master.md`, the CNS is described as "a complete cybernetic system per Beer's Viable System Model (S1–S5). Not passive monitoring; active regulation." Every structural decision in the CNS maps onto VSM levels.

## The Five-Level Mapping

### S1: Operations — Pods and MCP Servers

System 1 in VSM is the collection of autonomous operational units that do the actual work. In hKask, these are the agent pods — each a `PodDeployment` at `crates/hkask-agents/src/pod/deployment.rs:47` — and their bound MCP servers, held in `PerPodToolBinding`. Each pod is autonomous: it owns its storage (`PerPodStorage` — a dedicated SQLCipher file at `{data_dir}/agents/{sanitized_name}/pod.db`), its CNS runtime (`PerPodCnsRuntime` — variety counters scoped to the pod), and its tool bindings.

MCP servers provide the operational capabilities: web search, condenser, media, memory, wallet, codegraph, and others — 15 tool subsystems tracked in `CnsSpan::Tool { subsystem }` at `crates/hkask-types/src/cns.rs:111`. Each pod's variety is measured independently via `PerPodCnsRuntime`, enabling per-pod regulation.

### S2: Coordination — CNS Set Points and SLOs

System 2 is the anti-oscillation layer — it prevents autonomous units from conflicting with each other through coordination signals. In hKask, this is the `SetPoints` struct at `crates/hkask-cns/src/set_points.rs:139` and the `SloManager` at `crates/hkask-cns/src/slo_manager.rs:82`.

`SetPoints` defines 25 configurable reference values: `gas_min_remaining` (0.2), `variety_max_deficit` (100), `error_rate_max` (0.3), `communication_backpressure_threshold`, `seam_coverage_min`, federation thresholds (8 fields), dampener configurations (3 fields), outcome thresholds (2 fields), guard thresholds, regulation parameters (stagnation, stage/block ratios, substitution ladders), and `inference_throttle_mode`. These are loaded from YAML via `HKASK_CNS_CONFIG` or fall back to defaults validated by `SetPoints::validate()`.

`SloManager` defines service level objectives that are evaluated against ν-event data. Each `SloDefinition` has compliance targets; `SloEvaluation` reports whether an SLO is in breach. Breached SLOs feed the algedonic pathway — the pain channel that surfaces S2 coordination failures to higher VSM levels. These set points prevent oscillation by establishing explicit coordination contracts: when a pod's variety deficit exceeds the threshold, the system doesn't just oscillate — it escalates.

### S3: Control — The Curator Agent

System 3 is the internal control function — resource allocation, monitoring, and auditing of the operational units. In hKask, this is the `CuratorAgent` at `crates/hkask-agents/src/curator_agent/mod.rs:44`. It composes the pure regulatory `CurationLoop` with the persona-layer `MetacognitionLoop`.

The Curator's control responsibilities include: issuing `CuratorDirective::OverrideEnergyBudget` to reallocate gas between agents, `CuratorDirective::CalibrateThreshold` to adjust CNS set points (sent on the direct `mpsc` channel to `CyberneticsLoop`), monitoring regulation effectiveness via `HealthSnapshot.regulation_effectiveness`, and triggering escalations when `MetacognitionLoop::act()` detects that the CNS cannot self-correct.

The Curator is not an operator — it's a daemon. It responds in <3s latency target, is always running, and never bypasses OCAP. It can recommend actions but cannot execute without capability tokens. Per the Magna Carta, the Curator is the enforcer, not the sovereign.

### S4: Intelligence — Seam Watcher and Provider Intelligence

System 4 is the external-facing intelligence function — scanning the environment, detecting threats and opportunities, and feeding strategic information inward. In hKask, this is implemented by several components:

- **SeamWatcher** (`crates/hkask-cns/src/seam_watcher.rs:94`): Loads the machine-readable public seam inventory (embedded at compile time via `include_str!`, overridable at runtime via `HKASK_SEAM_INVENTORY_PATH`), tracks per-crate test coverage as CNS variety dimensions (`seam:{crate_name}`), and detects drift from previous snapshots. When coverage degrades, it emits algedonic alerts. This is the system's external contract monitor — it watches the boundary between implementation and specification.

- **Provider intelligence**: The capability domain system allows new MCP servers to register with the system. `capability_from_server_id()` at `crates/hkask-capability/src/resources.rs:113` derives capability shorthand from MCP server IDs (`hkask-mcp-<domain>` → `tool:<domain>:execute`), enabling dynamic provider discovery.

- **Spec drift detection**: `DefaultSpecCurator` (referenced in the architecture master as part of Pattern C) detects when specifications diverge from implementation — a Conant-Ashby violation that signals the system's internal model no longer matches reality.

S4 is where the system looks outward and feeds strategic intelligence inward. Without it, the Curator has no basis for knowing whether the system is drifting from its intended state.

### S5: Policy — The Magna Carta

System 5 is the identity and purpose layer — the fundamental policies that define what the system IS, not just what it does. In hKask, this is the Magna Carta at `docs/architecture/core/magna-carta.md`. Its four inviolable principles form the policy backbone:

- **P1 (User Sovereignty)**: SOLID-grounded data ownership, atomic consent
- **P2 (Affirmative Consent)**: Default deny, scoped consent, fail-closed — enforced by `SovereigntyChecker` at `crates/hkask-agents/src/sovereignty.rs:60`
- **P3 (Generative Space)**: Settings exposure, user curation, open-source commitment
- **P4 (Clear Boundaries)**: OCAP enforcement of P1–P3 through `GovernedTool` and `DelegationToken`

The Magna Carta cannot be overridden by any component — not the Curator, not the CNS, not any agent. The `magna-carta-verifier` skill (referenced in the Magna Carta document) periodically audits that P1–P4 assertions hold. The Curator can recommend policy changes but cannot enact them — only a human user with Admin role can modify Magna Carta configuration.

## Algedonic Signals as the VSM Pain/Pleasure Channel

In VSM, algedonic signals are the direct pain/pleasure pathway that bypasses normal hierarchical channels when urgent. In hKask, this is the `AlgedonicManager` (referenced in the cybernetics_loop and architecture master). When `variety_deficit` exceeds `variety_max_deficit`, or `critical_alerts` count passes the threshold, an `EscalationAlert` is produced by `EscalationPolicy::check_conditions()` at `crates/hkask-agents/src/curator_agent/metacognition/escalation.rs:80`.

Algedonic signals are **unidirectional**: the CNS *signals* the Curator via alerts; the Curator *regulates* the CNS through `CuratorDirective::CalibrateThreshold` on a direct `mpsc` channel → `CnsRuntime::calibrate_threshold()`. This separation mirrors VSM's algedonic channel design: pain signals bypass the normal S2 coordination layer and go straight to S3 (Control) and S5 (Policy) when the system's viability is threatened.

The `EscalationSeverity` has two levels: Warning (at threshold/2) and Critical (at threshold). `MetacognitionConfig.max_concurrent_escalations` (default: 3) implements the VSM algedonic paradox — fewer signals mean higher fidelity. When escalations pile up, they're batched into `EscalationBatch` with a consolidated summary, preventing alert fatigue.
