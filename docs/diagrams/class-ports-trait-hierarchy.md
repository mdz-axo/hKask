---
title: "Hexagonal Ports Trait Hierarchy — Class Diagram"
diataxis: reference
---

# Hexagonal Ports Trait Hierarchy — Class Diagram

**Diataxis quadrant:** Reference  
**Domain ontology tier:** Core  
**Purpose:** Show the hexagonal ports/adapter interface hierarchy — the trait contracts that define hKask's dependency inversion boundary.  
**Verified against:** `crates/hkask-ports/src/lib.rs`, `crates/hkask-ports/src/federation.rs`  
last-verified-against: "3d1a876f45e3ce64864c3453f1e71d75b2f14376"

```mermaid
classDiagram
    class InferencePort {
        <<trait>>
        +infer(request: InferenceRequest) Result~InferenceResult, InferenceError~
        +infer_stream(request: InferenceRequest) Stream~InferenceStreamChunk~
        +list_models() Vec~ModelInfo~
    }

    class ToolPort {
        <<trait>>
        +invoke(name: String, params: Value) Result~Value, ToolPortError~
        +list_tools() Vec~ToolInfo~
        +describe_tool(name: String) Option~ToolInfo~
    }

    class CircuitBreakerPort {
        <<trait>>
        +check() CircuitState
        +record_success()
        +record_failure()
        +reset()
    }

    class CnsObserver {
        <<trait>>
        +emit(event: NuEvent)
        +observe(span: ObservableSpan)
    }

    class CnsStoragePort {
        <<trait>>
        +store_event(event: WeightedEvent) Result~(), Error~
        +query_events(filter: EventFilter) Vec~WeightedEvent~
        +apply_decay(config: DecayConfig)
    }

    class ConsentPort {
        <<trait>>
        +has_consent(resource: String, action: String) bool
        +grant(resource: String, action: String, scope: ConsentScope)
        +revoke(resource: String, action: String)
        +list_grants() Vec~ConsentGrant~
    }

    class FederationDispatch {
        <<trait>>
        +dispatch(message: FederationMessage) Result~FederationResponse, FederationDispatchError~
        +sync_crdt(peer: ReplicaId) Result~CrdtSyncResult, FederationDispatchError~
        +resolve_link(link: FederationLink) Result~LinkResolution, FederationDispatchError~
    }

    class EmbeddingPort {
        <<trait>>
        +embed(text: String) Result~Vec~f32~, EmbeddingError~
        +embed_batch(texts: Vec~String~) Result~Vec~Vec~f32~~, EmbeddingError~
        +similarity(a: Vec~f32~, b: Vec~f32~) f32
    }

    class WalletBudgetPort {
        <<trait>>
        +gas_to_rjoules(gas: u64) RJoule
        +get_encumbrance(key_id) Option~Encumbrance~
        +can_afford(wallet_id, cost_rj) bool
        +get_balance(wallet_id) Result
        +consume(key_id, gas_rj) Result
        +settle_rjoules(wallet_id, reserved, actual) Result
        +gas_per_rjoule() u64
        +set_gas_per_rjoule(rate)
        +emit_key_alert(key_id, exhausted, expired)
        +get_api_key(key_id) Option~ApiKeyCapability~
    }

    class InferencePort <<trait>> InferencePort
    class ToolPort <<trait>> ToolPort
    class CircuitBreakerPort <<trait>> CircuitBreakerPort
    class CnsObserver <<trait>> CnsObserver

    InferencePort <|.. InferenceRouter : implements
    ToolPort <|.. McpDispatcher : implements
    ToolPort <|.. GovernedTool : decorates (OCAP)
    CircuitBreakerPort <|.. CircuitBreaker : implements
    CnsObserver <|.. CnsRuntime : implements
    WalletBudgetPort <|.. WalletManager : implements (in hkask-wallet)

    GovernedTool --> ToolPort : delegates to
    GovernedTool --> CircuitBreakerPort : checks before dispatch
    InferenceRouter --> CircuitBreakerPort : checks before inference
    CnsRuntime --> CnsStoragePort : persists events
    ConsentPort --> CnsObserver : emits on denial
    FederationDispatch --> CnsObserver : emits sync events
    CyberneticsLoop --> WalletBudgetPort : regulates via port (not concrete)

    note for GovernedTool "OCAP membrane:\n1. Check capability\n2. Reserve energy\n3. Emit ν-event\n4. Delegate\n5. Settle energy\n6. Emit ν-event"

    note for InferenceRouter "Multi-provider:\n- DeepInfra\n- Together AI\n- fal.ai\n- OpenRouter\n- KiloCode"
```

**Trait-to-file mapping:**

| Trait | Source File |
|-------|------------|
| `InferencePort` | `crates/hkask-ports/src/inference_port.rs` |
| `ToolPort` | `crates/hkask-ports/src/tool.rs` |
| `CircuitBreakerPort` | `crates/hkask-ports/src/lib.rs` |
| `CnsObserver` | `crates/hkask-ports/src/cns.rs` |
| `CnsStoragePort` | `crates/hkask-ports/src/cns.rs` |
| `ConsentPort` | `crates/hkask-ports/src/consent_port.rs` |
| `FederationDispatch` | `crates/hkask-ports/src/federation.rs` |
| `EmbeddingPort` | `crates/hkask-ports/src/embedding_port.rs` |
| `WalletBudgetPort` | `crates/hkask-ports/src/wallet_budget_port.rs` |

**Cardinality:** 9 port traits defined in `hkask-ports`. `InferenceRouter` (in `hkask-inference`) implements `InferencePort`. `McpDispatcher` (in `hkask-mcp`) implements `ToolPort`. `GovernedTool` (in `hkask-cns`) decorates `ToolPort` with OCAP membrane. `CircuitBreaker` (in `hkask-cns`) implements `CircuitBreakerPort`. `CnsRuntime` (in `hkask-cns`) implements `CnsObserver`. `WalletManager` (in `hkask-wallet`) implements `WalletBudgetPort` — CNS consumes the port, not the concrete type.
