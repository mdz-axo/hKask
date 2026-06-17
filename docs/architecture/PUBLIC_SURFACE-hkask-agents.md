---
title: "Public Surface Justification — hkask-agents"
audience: [architects, developers]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition]
---

# Public Surface Justification — hkask-agents

**Crate:** `hkask-agents`  
**Public items in lib.rs:** 26  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-agents` is the **agent orchestration crate** — pod lifecycle, ACP integration, curation loop, and sovereignty enforcement. Its surface is large because it spans multiple agent concerns:

1. **Pod lifecycle** — `AgentPod`, `PodLifecycleState`, `AgentMode`, `AgentPersona` are public because they're used by CLI, API, and MCP servers for agent management.
2. **ACP integration** — `A2ARuntime`, `A2AError`, and ACP adapters are public for registration and capability management.
3. **Curation loop** — `CurationLoop`, curator agent, and persona filter are public for metacognitive monitoring.
4. **Sovereignty** — `SovereigntyConsent` trait and `DenyAllConsent` are public for OCAP enforcement.

## Mitigations

- **Submodule organization:** pod/, acp/, curator/, curator_agent/, adapters/ each have focused concerns.
- **Trait-based ports:** `A2APort`, `MCPRuntimePort` enable testability without exposing implementation details.

## Deletion Test

Delete `hkask-agents` and pod lifecycle, ACP registration, curation loop orchestration, and sovereignty enforcement reappear scattered across CLI, API, and daemon. The crate earns its existence.
