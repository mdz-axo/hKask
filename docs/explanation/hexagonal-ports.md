---
title: "Hexagonal Ports and Adapters — Explanation"
audience: [architects, developers]
last_updated: 2026-07-12
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, composition]
last-verified-against: "3d1a876f"
---

# Hexagonal Ports and Adapters

The hexagonal architecture in hKask is not a pattern adopted for aesthetics. It exists because the system's core regulatory logic — the CNS, the Curator, the Infer loop — must function identically whether it runs against a local SQLite database in a developer's laptop or against a federated cluster of PostgreSQL-backed replicas. More importantly, it must be testable without any of those backends at all.

## What "Hexagonal" Means in hKask

In standard hexagonal architecture, the domain core is surrounded by ports (interfaces the core defines) and adapters (implementations that satisfy those interfaces). In Rust, ports are **traits**, and adapters are **structs that implement those traits**. The rule is simple: domain crates define the traits; infrastructure crates provide the implementations.

This design exists because hKask's dependency graph imposes a strict Authority DAG. Domain crates (`hkask-cns`, `hkask-agents`, `hkask-inference`) must not depend on infrastructure crates (`hkask-storage`, `hkask-mcp`, `hkask-federation`). The port traits in `hkask-ports` are the only shared dependency — every domain crate imports from `hkask-ports`, and every infrastructure crate implements against it. There is no other coupling path.

As the crate-level documentation in `crates/hkask-ports/src/lib.rs` states: "Port traits that enable crates to depend on abstractions rather than concrete implementations. Per the Authority DAG, domain crates depend on these port traits (not on each other)."

## The Port Traits

The `hkask-ports` crate defines **17 trait contracts**, each guarding a distinct architectural boundary. They group into five concerns:

| Concern | Traits |
|---------|--------|
| CNS regulation | `CircuitBreakerPort`, `CnsStoragePort`, `CnsObserver` |
| Federation | `FederationTransport`, `FederationSyncPort`, `FederationDispatch` |
| Storage and registry | `EmbeddingPort`, `RegistryPort`, `RegistryIndex`, `SkillRegistryIndex`, `GitCASPort` |
| Inference and tools | `InferencePort`, `ToolPort` |
| Governance and pipelines | `ConsentPort`, `EscalationPort`, `WalletBudgetPort`, `StepExecutor` |

The eight traits documented in detail below are the primary infrastructure boundaries. The remaining nine are listed in §9.

### 1. `InferencePort` (`crates/hkask-ports/src/inference_port.rs`)

The LLM invocation boundary. This is the most heavily used port — every agent pod, every Curator reflection, every template cascade eventually calls `generate()`. It uses `Pin<Box<dyn Future>>` rather than `async_trait` for object safety, enabling `Arc<dyn InferencePort>` dispatch at construction time. The trait provides default implementations for `generate_n()`, `generate_stream()`, `generate_with_model()`, and `generate_vision()` — all fall back to `generate()`, so a new backend only needs to implement one method.

The concrete implementor is the `InferenceRouter` in `hkask-inference`, which multiplexes across multiple providers (DeepSeek, Anthropic, Groq, OpenAI, etc.) with model routing, failover, and concurrency control.

### 2. `ToolPort` (`crates/hkask-ports/src/tool.rs`)

The **governance membrane** for MCP tool invocation. Unlike `InferencePort`, this port has an authentication asymmetry: `discover_tools()` and `get_tool_info()` are intentionally unauthenticated — tool schemas are public metadata — but `invoke()` requires a `DelegationToken`. OCAP enforcement applies at the actuator boundary, not the sensor boundary. The concrete implementor is `McpDispatcher` in `hkask-mcp`.

The error type, `ToolPortError`, encodes the governance envelope directly: `CapabilityDenied` (OCAP rejection), `EnergyBudgetExceeded` (gas depletion), `NotFound`, and `InvocationFailed`.

### 3. `CircuitBreakerPort` (`crates/hkask-ports/src/cns.rs`)

The circuit breaker boundary for the Cybernetics membrane. A minimal trait — `allow_request()`, `record_success()`, `record_failure()`, `state()` — that allows the Inference loop to use circuit breaking without depending on `hkask-cns`. The concrete implementor is `CircuitBreaker` in `hkask-cns`. When the CNS detects elevated error rates above the `error_rate_max` set-point (default: 30%), it opens the circuit and the inference loop stops sending requests.

### 4. `CnsStoragePort` (`crates/hkask-ports/src/cns.rs`)

Storage abstraction for CNS event queries. While `CircuitBreakerPort` is the actuator boundary, `CnsStoragePort` is the memory boundary — it abstracts the `NuEventStore` behind a trait so the cybernetic regulation layer (`GasReport`, `CalibratedEnergyEstimator`, `WalletGasCalibrator`) can be tested without a real SQLite database. It provides `query_algedonic()` for alert retrospectives, `replay_weighted()` for temporal decay-weighted event replay, and `persist_cursor()`/`load_cursor()` for crash recovery.

### 5. `CnsObserver` (`crates/hkask-ports/src/cns.rs`)

The subscriber interface for CNS events. Observers declare an `interest_mask()` of `SpanNamespace` values they care about, then receive `on_event()`, `on_depletion()`, and `on_backpressure()` callbacks. The concrete implementor in `hkask-inference` uses this to react to throttle and circuit-break signals.

### 6. `ConsentPort` (`crates/hkask-ports/src/consent_port.rs`)

Decouples agent pods from the concrete `ConsentStore` in `hkask-storage`. A simple CRUD trait for consent records — `initialize_schema()`, `store()`, `list_active()` — that ensures the Affirmative Consent (P2) verification layer can be tested independently of the database schema.

### 7. `EmbeddingPort` (`crates/hkask-ports/src/embedding_port.rs`)

The vector embedding storage boundary. Abstracts the concrete `EmbeddingStore` in `hkask-storage`. Provides `store()`, `get()`, `search()` (cosine similarity), and `delete()` — the four operations needed by the semantic memory loop to anchor triples in embedding space.

### 8. Federation ports (`crates/hkask-ports/src/federation.rs`)

Two traits for cross-instance federation: `FederationTransport` (async send/receive with partition simulation for testing) and `FederationSyncPort` (CRDT cursor-based triple synchronization). These decouple the Curator's federation logic from the concrete `FederationLinkManager` in `hkask-federation`.

Additionally, `EscalationPort` (`crates/hkask-ports/src/escalation.rs`) and `RegistryPort` (`crates/hkask-ports/src/registry_port.rs`) provide the escalation queue and agent registry boundaries, respectively.

## How Dependency Inversion Works

Consider the `AgentService` pattern. When an agent pod needs to run inference, it does not call `InferenceRouter::new()`. It receives a `Box<dyn InferencePort>` at construction time. In production, this is an `InferenceRouter` backed by real API keys. In tests, it is a `MockInferencePort` that returns canned responses. The agent pod's logic — the prompt assembly, the response parsing, the template cascade — is identical in both contexts.

The same pattern applies to every boundary. The CNS's `CyberneticsLoop` stores an `Option<Arc<dyn NuEventSink>>` — in production it is the `NuEventStore`, in tests it is a `NoopEventSink` that discards events. The regulation logic does not care.

This is not a theoretical benefit. The CNS test suite (`crates/hkask-cns/src/cybernetics_loop.rs`, `mod tests`) creates a `CyberneticsLoop` with a `CnsRuntime` backed by in-memory state, runs it through `tick()` cycles, and asserts on the resulting `LoopQuality` and action vectors — no database, no network, no API keys.

## How Ports Compose

The ports do not exist in isolation. At runtime, three ports compose into the regulated inference pathway:

```
InferenceLoop
    │
    ├─▶ CircuitBreakerPort::allow_request()
    │       └── state = Open → return Err, short-circuit
    │
    ├─▶ InferencePort::generate()
    │       └── actual LLM call, returns InferenceResult
    │
    └─▶ ToolPort::invoke(tool, args, token)
            └── OCAP check → gas reservation → MCP dispatch
```

The `CircuitBreakerPort` gates the `InferencePort`: if the circuit is open (too many recent failures), the inference loop skips the LLM call entirely and returns an error. The `ToolPort` governs tool execution: even if the LLM produces a tool call, the OCAP membrane checks whether the agent's delegation token authorizes that specific tool before dispatching.

This composition is not wired by magic — it is wired by the `InferenceLoop` in `hkask-inference`, which holds references to all three ports and sequences them explicitly. The CNS observes the results (`CnsObserver::on_event()`) and may decide to open the circuit or escalate based on the outcome.

## The GovernedTool Decorator Pattern

`ToolPort` on its own would be a simple dispatch interface: "call this tool with these arguments." But hKask's P4 (Object Capability) principle requires that every tool invocation be capability-gated. Rather than polluting every call site with authorization logic, the system uses a **decorator pattern**: `GovernedTool` wraps `ToolPort` with OCAP checking, energy reservation, span emission, and cost accounting.

The decorator's `invoke()` method:
1. Checks the `DelegationToken` against the tool's `required_capability`
2. Reserves gas from the agent's `GasBudget`
3. Emits a `cns.tool.pre` span
4. Delegates to the inner `ToolPort::invoke()`
5. Accounts for the actual cost against the budget
6. Emits a `cns.tool.post` span with outcome

From the caller's perspective, it still calls `invoke()` on a `ToolPort` — the decorator makes the governance membrane invisible to the consumer while enforcing it at every invocation. This is the cybernetic equivalent of a capability-secure dispatch: the agent can only call tools it holds tokens for, and every call is metered.

## Why This Design Exists

The hexagonal pattern in hKask serves three purposes, each grounded in a specific project constraint:

**Testability.** The CNS must be testable without external dependencies. `CnsStoragePort` means the `CyberneticsLoop` test suite runs against in-memory data. `ToolPort` means the OCAP enforcement tests don't need running MCP servers. `InferencePort` means the prompt assembly tests don't burn API credits.

**Provider independence.** The `InferencePort` abstraction means the system can route to any LLM provider (DeepSeek, Anthropic, Groq, OpenAI, Ollama, or a local model) without changing agent logic. The `CircuitBreakerPort` means the circuit breaker implementation can be swapped — today it is a simple failure-counting breaker in `hkask-cns`, but it could become an adaptive breaker without touching the inference loop.
> Provider list updated: DeepInfra, Together AI, fal.ai, OpenRouter, KiloCode.

**OCAP enforcement at boundaries.** The `ToolPort` is not just a convenience abstraction — it is a **security boundary**. The `DelegationToken` requirement is not advisory; it is enforced by the trait's contract. Any implementor of `ToolPort` must reject unauthenticated invocations. The hexagon's perimeter is also the capability security perimeter.

For a visual reference, see the [Ports Trait Hierarchy Class Diagram](../diagrams/class-ports-trait-hierarchy.md) (DIAG-IC-002 in the Diagram Index), which renders the complete trait hierarchy with method signatures and implementor relationships.

---

## Additional Port Traits (§9)

The following nine traits are defined in `hkask-ports` but are not given full section treatment above. They are listed here for completeness and agent-correctness.

| Trait | File | Purpose |
|-------|------|---------|
| `FederationDispatch` | `federation.rs` | High-level federation orchestration: `register_peer`, `invite`, `sync`, `remove_peer`. The primary federation trait referenced in AGENTS.md Key Docs. |
| `GitCASPort` | `git_cas/port.rs` | Content-addressed storage boundary: `store_blob`, `get_blob`, `hash_exists`. Guards the Git object store abstraction. |
| `WalletBudgetPort` | `wallet_budget_port.rs` | Wallet-backed gas budgeting: `get_balance`, `reserve_gas`, `release_gas`. Enables CNS energy management to query wallet state. |
| `StepExecutor` | `pipeline_runner.rs` | Pipeline step execution boundary for multi-step agent workflows. |
| `SkillRegistryIndex` | `registry.rs` | Read-only skill registry access: `list_skills`, `get_skill_metadata`. Used by `SkillAuditor` and bundle composition. |
| `RegistryIndex` | `registry.rs` | Read-only template registry access: `list_templates`, `get_template`. Used by the cascade resolver. |
| `RegistryPort` | `registry_port.rs` | Full registry access (read + write): `insert_template`, `get_template`, `list_templates`. The mutable counterpart to `RegistryIndex`. |
| `EscalationPort` | `escalation.rs` | Escalation queue access: `push_escalation`, `list_escalations`, `resolve_escalation`. Bridges CNS algedonic alerts to Curator action. |
| `ConsentPort` | `consent_port.rs` | Consent store access: `check_consent`, `grant_consent`, `revoke_consent`. Enforces P1 sovereignty at the data-access boundary. |
