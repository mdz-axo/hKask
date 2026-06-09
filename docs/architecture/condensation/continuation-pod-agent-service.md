---
title: "Condensation Continuation — Candidate #4: Pod/Agent/Service Restructuring"
audience: [architects, developers]
last_updated: 2026-06-09
version: "0.27.0"
status: "Deferred"
domain: "Architecture"
mds_categories: [domain, composition]
---

# Condensation Continuation — Candidate #4: Pod/Agent/Service/ACP Restructuring

**Status:** Deferred. `hkask-agents` is the most overloaded crate in the codebase — it contains Curation, Inference, Pod management, ACP communication, Ensemble, Sovereignty enforcement, Consent management, and Escalation. This is a structural clarification, not a deletion: the entities are correct, but their boundaries are muddled.

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
├── pod/           # AgentPod, PodLifecycleState, PodManager
├── acp/           # AcpRuntime, AcpAgent, A2AMessage
├── curator/       # CuratorAgent, DefaultSpecCurator, CurationLoop
├── curator_agent/ # Metacognition, spec curator
├── ensemble/      # EnsembleChat, StandingSession, ImprovMode
├── communication/ # MessageDispatch
├── inference_loop/# InferenceLoop
├── loop_system/   # LoopSystem, CyberneticsLoopHandle
├── hhh_gate/      # HhhConfig, HhhMode
├── escalation/    # EscalationQueue
├── consent/       # ConsentManager
├── sovereignty/   # SovereigntyChecker
├── adapters/      # MCP runtime adapter
├── ports/         # ACP, memory storage ports
├── prompt_analysis/
└── registry_loader/
```

### Muddling Examples

| Entity | Where It Lives | Problem |
|--------|---------------|---------|
| `AgentPod` lifecycle | `pod/mod.rs` | Pod lifecycle owns agent registration, which should be a separate concern |
| `AcpRuntime` | `acp/` | ACP lives in a submodule but agents join ACP through pod lifecycle — circular dependency |
| `PodManager` | `pod/` | Manages pods but also handles service access delegation |
| `CurationLoop` | `curator/` | Curation regulates pods but lives in the same crate as pods |
| `InferenceLoop` | `inference_loop/` | Inference is a separate loop but lives in agents crate |
| `EnsembleChat` | `ensemble/` | Multi-agent chat coordination lives in agents but should be separate from pod lifecycle |
| `Communication` | `communication/` | Should be demoted to transport (per 4-loop model) |

### The Deletion Test Applied

If we mentally delete `hkask-agents`:
- Pod lifecycle → reappears in every surface that creates agents (CLI, API) → DEEP, keep
- Agent identity → reappears in registration, inference, ensemble → DEEP, keep
- Service access → reappears in every tool dispatch → DEEP, keep
- ACP → reappears in multi-agent communication → DEEP, keep
- Curation → reappears in every decision — but should Curation live in agents at all? → BOUNDARY QUESTION

## Approach

### Phase 1 — Document the Model

1. Write a clear specification of Pod/Agent/Service/ACP boundaries in a single document
2. Map every existing type to its correct boundary
3. Identify types that cross boundaries (the muddling)

### Phase 2 — Clarify Module Boundaries

1. Reorganize `hkask-agents/src/` to reflect the model:
   ```
   hkask-agents/src/
   ├── pod/       # Pod lifecycle only (AgentPod, PodLifecycleState, PodManager)
   ├── agent/     # Agent identity (AgentDefinition, WebID, Charter, Persona, AgentKind)
   ├── acp/       # ACP communication (AcpRuntime, AcpAgent, A2AMessage) — separate from pod
   ├── curator/   # Curation (CuratorAgent, CurationLoop — may belong in separate crate)
   ├── ensemble/  # Multi-agent chat
   └── inference/ # InferenceLoop (may belong closer to templates or CNS)
   ```

2. Move Curation and Inference to their own modules with clear boundaries
3. Ensure ServiceContext (in hkask-services) is the single entry point for service access — pods don't reach into domain crates directly

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
- [x] Candidate #3: LoopMessage→tokio — deferred (separate continuation prompt)
- [x] Documentation cleanup (DDMVSS→MDS, 9→5 categories, 6→4 loops)
- [x] MDS specification (5 categories, 5 tools, 3 curation decisions)

## Dependencies

Candidate #3 (LoopMessage→tokio) should be completed before #4 if both are pursued, because #3 changes the messaging infrastructure that pods and agents rely on.

---

*This continuation prompt captures all context needed to resume the Pod/Agent/Service restructuring as a standalone task.*
