---
title: "Agent Pod Lifecycle State Machine"
audience: [architects, developers, agents, replicants]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active"
domain: "Pod"
mds_categories: [domain, lifecycle, curation]
diataxis: "reference"
---

# Agent Pod Lifecycle State Machine

## Description

The `PodLifecycleState` in `hkask-agents` governs every agent pod through a strict linear progression: `Populated → Registered → Activated → Deactivated`. The `can_transition_to()` method enforces this linear model with idempotent restate (re-stating current state is always permitted). `Deactivated` is terminal — no further transitions are possible. When in `Activated` state, the pod operates in one of two mutually exclusive `AgentMode` variants: `Chat` (conversational H2A interaction) or `Server` (presenting as MCP server for A2A tool dispatch). Each state carries distinct properties: `Populated` establishes identity and capabilities from the persona template, `Registered` mints the OCAP capability token, `Activated` grants MCP access and sovereign memory, and `Deactivated` revokes all capabilities.

**Key source:** `crates/hkask-agents/src/pod/types.rs:57-66` (`PodLifecycleState` enum), `types.rs:68-96` (`can_transition_to`), `types.rs:15-21` (`AgentMode`), `active_pods.rs:148-157` (`with_factory_and_ports`).

```mermaid
stateDiagram-v2
    [*] --> Populated : instantiate from template

    state Populated {
        [*] --> templated : persona loaded
        --
        note left of templated
            identity: AgentPersona
            capabilities: from YAML
            charter: purpose + scope
            WebID cached
            PodKind: Curator/Team/Replicant
        end note
    }

    Populated --> Registered : register()

    state Registered {
        [*] --> minted : capability token created
        --
        note left of minted
            identity: confirmed
            OCAP: DelegationToken minted
            A2A: registered with runtime
            capabilities: token-gated
        end note
    }

    Registered --> Activated : activate()

    state Activated {
        [*] --> Chat : AgentMode::Chat
        [*] --> Server : AgentMode::Server
        Chat --> Chat : H2A conversation
        Server --> Server : A2A tool dispatch
        --
        note left of Chat
            conversational mode
            tool-augmented inference
            GovernedTool membrane
            episodic + semantic memory
            sovereign memory active
        end note
        note right of Server
            MCP server mode
            incoming tool calls
            OCAP token verification
            energy budget enforced
            CNS observability
        end note
        note left of Activated
            modes are mutually exclusive
            concurrency planned for future
            CNS spans active
        end note
    }

    Activated --> Deactivated : deactivate()

    state Deactivated {
        [*] --> terminal : capabilities revoked
        --
        note left of terminal
            identity: archived
            capabilities: revoked
            OCAP token: invalidated
            no further transitions
            can_transition_to() → false
        end note
    }

    Deactivated --> [*]
```

## Transition Table

| From | To | Trigger | Guard |
|------|----|---------|-------|
| `[*]` | `Populated` | Pod instantiated from template crate | AgentPersona parsed, WebID cached |
| `Populated` | `Registered` | `register()` — A2A runtime registration | Capability token minted by `CapabilityChecker::grant()` |
| `Registered` | `Activated` | `activate()` — MCP access granted | `GovernedTool`, episodic/semantic storage wired |
| `Activated` | `Deactivated` | `deactivate()` — capabilities revoked | Token invalidated, memory frozen |
| `Deactivated` | `[*]` | Terminal — no further transitions | `can_transition_to()` returns `false` for all `next` |

## State Properties Summary

| State | Identity | Capabilities | Memory | OCAP Token |
|-------|----------|-------------|--------|------------|
| Populated | AgentPersona + WebID | From YAML capability list | None | Not yet minted |
| Registered | Confirmed by A2A runtime | Token-gated | None | Minted (`DelegationToken`) |
| Activated | Active agent mode | Token-gated + MCP access | Episodic + Semantic | Active, verified per-tool-call |
| Deactivated | Archived | All revoked | Frozen | Invalidated |

## Operating Modes (Activated)

The `AgentMode` enum determines how the pod interacts:

- **Chat** (`AgentMode::Chat`): Conversational H2A mode. Tool-augmented inference via `GovernedTool` membrane, with episodic memory for private experience and semantic memory for shared knowledge.
- **Server** (`AgentMode::Server`): MCP server mode for A2A tool dispatch. Incoming tool calls verified against OCAP token, energy budget enforced per-call.

Modes are mutually exclusive per the current design, with concurrency support planned.

---

<!-- DIAGRAM_ALIGNMENT
id: DIAG-DC-007
verified_date: 2026-07-01
verified_against: crates/hkask-agents/src/pod/types.rs (PodLifecycleState:57-66, can_transition_to:68-96, AgentMode:15-21, AgentPersona:110-131), crates/hkask-agents/src/pod/active_pods.rs (ActivePods:22-32, with_factory_and_ports:148-157), crates/hkask-agents/src/pod/deployment.rs (PerPodToolBinding:109-112, deploy:239-249), crates/hkask-agents/src/pod/context.rs (PodContext:48-64)
status: VERIFIED
-->

## Cross-Reference

- [`hKask-architecture-master.md` § Agent Pods](architecture/hKask-architecture-master.md#agent-pods)
- [`types.rs`](crates/hkask-agents/src/pod/types.rs) — `PodLifecycleState`, `AgentMode`, `AgentPersona`, `PodKind`
- [`active_pods.rs`](crates/hkask-agents/src/pod/active_pods.rs) — `ActivePods` registry, activation wiring
- [`deployment.rs`](crates/hkask-agents/src/pod/deployment.rs) — `PodFactory::deploy()`, `PerPodToolBinding`
- [`context.rs`](crates/hkask-agents/src/pod/context.rs) — `PodContext`, `GovernedTool` membrane
