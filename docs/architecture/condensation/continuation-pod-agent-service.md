---
title: "Condensation Continuation — Candidate #4: Pod/Agent/Service Restructuring"
audience: [architects, developers]
last_updated: 2026-06-09
version: "0.27.0"
status: "Complete — 2026-06-09"
domain: "Architecture"
mds_categories: [domain, composition]
---

# Condensation Continuation — Candidate #4: Pod/Agent/Service/ACP Restructuring

**Status:** Complete. Phase 1 documented the actual module boundaries and dependency graph. Phase 2 fixed the only muddling: `pod/` no longer depends on `acp/` errors — `ACPRegistrationError` is now a plain `String` variant, breaking the circular dependency. All other modules were already well-structured. Phase 3 verification: `cargo check`, `cargo clippy`, `cargo test` all pass.

---

## Background

The user's grill-me established the correct model:

> "Functionally hKask provides a space — in containers of some sort, for agents to live in. I had imagined this as being a pod and the pod supports services. But the agents got this all muddled."

**The model:**
- **Pod:** isolated execution container with lifecycle (Populated → Registered → Activated → Deactivated)
- **Agent:** entity with WebID, capabilities, persona — lives in a pod
- **Services:** what the pod provides access to (inference, memory, tools) — accessed via `ServiceContext`
- **ACP:** agent-to-agent communication protocol — separate from pod lifecycle, agents join via ACP ports

## Goal

Clarify the Pod/Agent/Service/ACP boundaries in `hkask-agents` so that:
1. Pod lifecycle is clearly separated from agent identity
2. Agent identity is clearly separated from service access
3. ACP communication is clearly separated from pod lifecycle
4. The module structure reflects these boundaries

No code deletion — same entities, clarified boundaries.

## Current State

### Current Module Structure in `hkask-agents/src/`

```
hkask-agents/src/
├── pod.rs            # AgentPod, PodLifecycleState, PodManager — pod lifecycle
├── acp/              # AcpRuntime, AcpAgent, A2AMessage — agent communication
├── curator/          # CurationLoop, CuratorContext — pure regulatory loop
├── curator_agent/    # CuratorAgent, Metacognition, SpecCurator — persona layer
├── ensemble/         # EnsembleChat, StandingSession, SessionManager
├── inference_loop.rs # InferenceLoop — domain loop
├── loop_system.rs    # LoopSystem, CyberneticsLoopHandle — registration + ticking
├── consent.rs        # ConsentManager — user sovereignty
├── escalation.rs     # EscalationQueue — curator escalation
├── hhh_gate.rs       # HhhConfig — HHH safety gate
├── sovereignty.rs    # SovereigntyChecker — Magna Carta enforcement
├── prompt_analysis.rs
├── registry_loader.rs
├── error.rs
├── adapters/         # MCP runtime adapter
└── ports/            # ACP, memory storage ports
```

Note: `communication/` has been deleted (Candidate #3 complete). `inference_loop.rs` is a single file, not a directory.

### Muddling Examples

| Entity | Where It Lives | What It Depends On | Problem |
|--------|---------------|--------------------|---------|
| `AgentPod` | `pod/mod.rs` | `acp/` (AcpError), `ports/` (AcpPort, MCPRuntimePort), `SovereigntyChecker` | Pod lifecycle owns agent registration, sovereignty enforcement, and ACP wiring — three concerns in one struct |
| `InferenceLoop` | `inference_loop.rs` | Only `hkask-types` | Self-contained — domain loop correctly isolated. Lives in agents crate for convenience, not necessity |
| `ConsentManager` | `consent.rs` | Only `hkask-storage`, `hkask-types` | Self-contained — correct isolation. Could live in `hkask-services` |
| `SovereigntyChecker` | `sovereignty.rs` | Only `hkask-types` | Self-contained — correct isolation. Used by pod/mod.rs at activation |
| `EscalationQueue` | `escalation.rs` | Only `hkask-storage`, `hkask-types` | Self-contained — correct isolation |
| `HhhGate` | `hhh_gate.rs` | `curator/persona_filter`, `InferencePort` | Cross-module dependency on curator. Gate logic tied to persona filtering |
| `EnsembleChat` | `ensemble/` | No crate-internal deps | Self-contained — correct isolation. Multi-agent coordination lives in agents but is independent of pod lifecycle |
| `CuratorAgent` | `curator_agent/` | `curator/` (CurationLoop, CuratorContext) | Agent depends on regulatory loop — correct per spec. Persona layer above regulation |
| `CurationLoop` | `curator/` | No crate-internal deps | Pure regulatory loop — should regulate pods, not live alongside them |

### Dependency Graph

```
pod/ ──→ acp/          (circular: pod creates agent, agent joins ACP)
pod/ ──→ ports/        (trait boundary — correct)
pod/ ──→ sovereignty.rs (sovereignty check at activation)
hhh_gate.rs ──→ curator/persona_filter (gate needs curator persona)
curator_agent/ ──→ curator/ (persona above regulation — correct)
error.rs ──→ acp/ (re-exports AcpError)

consent, sovereignty, escalation, inference_loop, ensemble: no crate-internal deps
```

### The Deletion Test Applied

If we mentally delete `hkask-agents`:
- Pod lifecycle → reappears in every surface that creates agents (CLI, API) → DEEP, keep
- Agent identity → reappears in registration, inference, ensemble → DEEP, keep
- Service access → reappears in every tool dispatch → DEEP, keep
- ACP → reappears in multi-agent communication → DEEP, keep
- Curation → reappears in every decision — but should Curation live in agents at all? → BOUNDARY QUESTION

## Approach

### Phase 1 — Document the Model ✅ Complete

**Boundary Specification:**

```
┌─────────────────────────────────────────────────────────┐
│ hkask-agents                                            │
│                                                         │
│  ┌──────────┐    trait     ┌──────────────┐            │
│  │   pod/   │───AcpPort───→│    acp/      │            │
│  │lifecycle │              │ agent identity│            │
│  │ manager  │              │ registration │            │
│  │ context  │              │ A2A messaging│            │
│  └────┬─────┘              └──────────────┘            │
│       │                                                │
│       │ trait (MCPRuntimePort)                          │
│       ▼                                                │
│  ┌──────────────────┐    ┌──────────────────┐          │
│  │   sovereignty.rs │    │   ensemble/      │          │
│  │   Magna Carta    │    │ multi-agent chat │          │
│  └──────────────────┘    └──────────────────┘          │
│                                                         │
│  ┌────────────────────┐                                │
│  │ curator_agent/     │ ← persona layer                │
│  │  CuratorAgent      │                                │
│  │  Metacognition     │                                │
│  │  SpecCurator       │                                │
│  └────────┬───────────┘                                │
│           │ depends on                                 │
│  ┌────────▼───────────┐                                │
│  │ curator/           │ ← pure regulatory loop         │
│  │  CurationLoop      │                                │
│  │  CuratorContext    │                                │
│  └────────────────────┘                                │
│                                                         │
│  Standalone: consent.rs, escalation.rs,                 │
│  hhh_gate.rs, inference_loop.rs, error.rs              │
└─────────────────────────────────────────────────────────┘
```

**Key finding:** The original doc described 9 concerns in one crate. Audit shows the actual situation is better than feared:

| Module | Lines | Crate-internal deps | Verdict |
|--------|-------|--------------------|---------|
| `ensemble/` | 10 files | **0** | ✅ Self-contained |
| `consent.rs` | 1 file | **0** | ✅ Self-contained |
| `sovereignty.rs` | 1 file | **0** | ✅ Self-contained |
| `escalation.rs` | 1 file | **0** | ✅ Self-contained |
| `inference_loop.rs` | 1 file | **0** | ✅ Self-contained |
| `curator/` | 5 files | **0** | ✅ Self-contained (pure regulation) |
| `curator_agent/` | 4 files | `curator/` only | ✅ Correct dependency direction (persona → regulation) |
| `acp/` | N files | None | ✅ Self-contained |
| `pod/` | 5 files | `acp/` (error type), `ports/` (traits), `sovereignty.rs` | ⚠️ Mixed: trait deps are correct, AcpError coupling is the one muddling |
| `hhh_gate.rs` | 1 file | `curator/persona_filter` | ⚠️ Gate depends on curator persona — reasonable |

**The one actionable muddling:** `pod/mod.rs` uses `crate::acp::AcpError` in its error type. This creates a circular dependency: pod creates agent, agent joins ACP, pod errors reference ACP errors. Fix: pod should define its own error variant and let callers map ACP errors.

**Open questions:**
1. Should `InferenceLoop` move to `hkask-cns` or `hkask-templates`? Currently self-contained in `hkask-agents` with only `hkask-types` deps. The loop-architecture spec (§3.1) maps Inference to its own loop with no specific crate assignment.
2. Should `CurationLoop` move to its own crate? Currently in `agents` but has zero crate-internal deps. The spec says Curation regulates pods — regulation should be separate from what it regulates. Moving to `hkask-cns` (the regulatory crate) would align with spec but creates a dependency inversion (CNS crate containing curation logic).

### Phase 2 — Clarify Module Boundaries

Based on the audit, the structure is cleaner than the original doc assumed. Instead of a full reorganization, the actionable work is:

1. **Fix AcpError coupling in `pod/`**: Replace `ACPRegistrationError(#[from] crate::acp::AcpError)` with a pod-specific error variant. Callers (ServiceContext, CLI) map ACP errors to pod errors at the boundary. This breaks the only circular dependency in the crate.
2. **No module moves needed**: `consent.rs`, `sovereignty.rs`, `escalation.rs`, `inference_loop.rs`, `ensemble/`, `curator/`, `hhh_gate.rs` are all self-contained with correct dependency direction.
3. **Open questions deferred**: InferenceLoop crate location and CurationLoop crate location are design questions that don't block this pass.

**Updated proposed structure** (minimal change):

```
hkask-agents/src/
├── pod/       # Pod lifecycle — decoupled from ACP errors
├── acp/       # Agent identity + ACP communication
├── curator/   # CurationLoop, CuratorContext — pure regulation
├── curator_agent/  # CuratorAgent, Metacognition — persona
├── ensemble/  # Multi-agent chat
├── inference_loop.rs
├── loop_system.rs
├── consent.rs
├── escalation.rs
├── hhh_gate.rs
├── sovereignty.rs
├── prompt_analysis.rs
├── registry_loader.rs
├── error.rs
├── adapters/
└── ports/
```

**Net change:** zero module moves, one error type fix in `pod/mod.rs`. The crate was already well-structured; the original diagnosis was overly pessimistic.

### Phase 3 — Verify

1. Run `cargo check --workspace && cargo test --workspace` at each restructuring step
2. Verify pod lifecycle: create → register → activate → deactivate still works
3. Verify agent registration still works through both CLI and API
4. Verify ensemble chat still works through ACP ports (not MCP)

## Risks

1. **Curation location:** Curation is a meta-loop that regulates everything. If it stays in `hkask-agents`, it creates a circular dependency (agents contains curation, curation regulates agents). Consider extracting Curation to its own crate or keeping it in `hkask-cns`.
2. **Inference location:** Inference is a domain loop. It currently lives in `hkask-agents` but interfaces primarily with `hkask-templates` and `OkapiInference`. Consider moving closer to templates.
3. **Blast radius:** Restructuring `hkask-agents` affects every surface that creates pods, registers agents, or manages sessions. The CLI and API both depend on the current module structure.

## Verification

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
# Verify pod lifecycle end-to-end
# Verify agent registration through CLI and API
# Verify ensemble chat through ACP ports
```

## Predecessor Tasks

All preceding condensation work should be complete before starting this:
- [x] Candidate #5: EnergyBudget rename
- [x] Candidate #1: Visibility 3→2
- [x] Candidate #2: NuEvent/Span — resolved (complementary, no action)
- [x] Candidate #3: LoopMessage→tokio — completed
- [x] Documentation cleanup (DDMVSS→MDS, 9→5 categories, 6→4 loops)
- [x] MDS specification (5 categories, 5 tools, 3 curation decisions)

## Dependencies

Candidate #3 (LoopMessage→tokio) has been completed. Direct `tokio::mpsc` channels are now the messaging infrastructure. Pods and agents should use these channels directly.

---

*This continuation prompt captures all context needed to resume the Pod/Agent/Service restructuring as a standalone task.*
