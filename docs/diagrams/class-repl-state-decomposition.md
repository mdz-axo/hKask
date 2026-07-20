---
title: "ReplState Decomposition — Type Hierarchy"
audience: [architects, developers]
last_updated: 2026-07-20
version: "0.32.0"
status: "Active"
domain: "Surface"
mds_categories: [composition]
---

# ReplState Decomposition — Type Hierarchy

Class diagram of `ReplState` and its sub-structs in `crates/hkask-repl/src/lib.rs`. `ReplState` is the central state object for the REPL — all infrastructure (inference, memory, tool dispatch, gas tracking) is accessed through `service_context: Arc<AgentService>`. The sub-structs group paired state to enforce invariants at the type level.

```mermaid
classDiagram
    class ReplState {
        +WebID agent_webid
        +String current_model
        +String current_agent
        +Option~String~ active_session
        +Option~ResolvedSecrets~ resolved_secrets
        -Option~PersonaConstraints~ persona_constraints
        +ToolPrompt tool_prompt
        +ManifestState manifest_state
        +Arc~AgentService~ service_context
        +ReplSettings repl_settings
        +bool is_first_run
        +TalkConfig talk_config
        +Option~ImprovMode~ improv_mode
        +Option~KanbanService~ kanban_service
        +Vec degraded_servers
        +ThreadRegistry thread_registry
        +Arc~dyn ReplHost~ host
    }
    class ToolPrompt {
        +String section
        +Vec~ChatToolDefinition~ definitions
    }
    class ManifestCascade {
        +BundleManifest manifest
        +ManifestExecutor executor
    }
    class TalkConfig {
        +TalkMode mode
        +Option~String~ voice_design
    }
    class TalkMode {
        <<enumeration>>
        On
        Off
    }
    class ThreadRegistry {
        +BTreeMap threads
        +Option~String~ active_thread_id
        +bool seeded
    }
    class ReplHost {
        <<interface>>
        +resolve_user_webid() WebID
        +run_onboarding(rt) Result
        +list_templates_local() Vec
        +run_sovereignty_status()
    }
    class AgentService {
        <<external>>
        +governed_tool(webid) Arc
        +inference_port() Option
        +per_agent_memory(name) Result
        +cns() CnsContext
        +gas_remaining() Option
        +gas_cap() Option
    }

    ReplState --> ToolPrompt : tool_prompt
    ReplState --> ManifestCascade : manifest_state (Option)
    ReplState --> TalkConfig : talk_config
    ReplState --> ThreadRegistry : thread_registry
    ReplState --> ReplHost : host (Arc dyn)
    ReplState --> AgentService : service_context (Arc)
    TalkConfig --> TalkMode : mode
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-REPL-002
verified_date: 2026-07-20
verified_against: crates/hkask-repl/src/lib.rs:100-159; crates/hkask-services-context/src/context_impl.rs:103-474
status: VERIFIED
-->

## Design Notes

- **`ManifestState` is a type alias** for `Option<ManifestCascade>`, not a struct. This enforces the "both present or both absent" invariant at the type level — the invalid state `Some(manifest) + None(executor)` is unrepresentable. The previous struct shape with two `Option<T>` fields permitted this invalid state.
- **`TalkMode` is an enum**, not a `bool`. The `enabled: bool` field was replaced with `mode: TalkMode` so the on/off decision is explicit at the type level. The invalid state "off but has voice_design" is still representable (the user can set a voice while talk is off), but the on/off check in the turn pipeline is now a pattern match, not a boolean test.
- **`tool_prompt` is a cache.** It exists because `ToolPort` uses `impl Trait` returns, making `Arc<dyn ToolPort>` infeasible. The cache is refreshed during MCP server start/stop. A future refactor making `ToolPort` dyn-compatible would eliminate this cache.
- **`host: Arc<dyn ReplHost>`** bridges the REPL crate to the CLI binary. The REPL crate cannot depend on `hkask-cli` (dependency direction violation), so `ReplHost` is a trait implemented by `CliHost` in `hkask-cli`. After onboarding, the host is only used for `resolve_user_webid()` in tool invocation — a value already available in `agent_webid` and `service_context.webid()`.
- **Manual `Debug` impl** redacts `resolved_secrets`, `manifest_state`, `service_context`, and `host` so the central state object can be inspected in diagnostics without leaking secrets or hitting non-Debug trait-object bounds.

## Cross-References

- [REPL Specification §3.2 — ReplState](../specifications/REPL-specification.md#32-replstate--central-state-object)
- [ADR-046: REPL Extraction Path](../architecture/ADRs/ADR-046-repl-extraction-path.md)
- [REPL Turn Pipeline Flowchart](flowchart-repl-turn-pipeline.md)
