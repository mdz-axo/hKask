---
title: "LoRA Adapter Store Guide — Storing, Routing, and Deploying Trained Adapters"
audience: [developers, operators, curators]
last_updated: 2026-06-17
version: "0.31.0"
status: "Active"
domain: "Technology"
mds_categories: [composition, lifecycle, trust]
---

# LoRA Adapter Store Guide — Storing, Routing, and Deploying Trained Adapters

This guide covers the full lifecycle of trained LoRA adapters in hKask: from training provenance through storage, composition with base models, cloud deployment, cost-tracked inference, and teardown. Every operation is OCAP-gated via `DelegationToken`. Every state transition emits a CNS span. Every adapter has an owner WebID (P12 — no anonymous artifacts).

**Key crates:** `hkask-adapter` (lifecycle + routing), `hkask-types` (CNS spans, WebID types), `hkask-storage` (define_store! pattern).

---

## 1. Adapter Lifecycle

An adapter moves through five macro-stages:

```mermaid
stateDiagram-v2
    [*] --> Trained: Training pipeline<br/>(hkask-mcp-training)
    Trained --> Stored: AdapterStore::store()
    Stored --> Loaded: AdapterStore::get_by_id()
    Loaded --> Composed: AdapterRouter::create_endpoint()
    Composed --> Deployed: EndpointLifecycle(Active)
    Deployed --> Terminated: AdapterRouter::teardown_endpoint()

    state Composed {
        Provisioning --> Ready: Provider confirms URL
        Ready --> Active: First inference
        Active --> Draining: Teardown requested
        Draining --> Terminated: In-flight requests complete
    }

    note right of Stored
        Content-addressed
        Checksum-verified
        Owner-scoped (P12 WebID)
    end note
```

| Stage | What Happens | Key Type | CNS Span |
|-------|-------------|----------|----------|
| Train | LoRA fine-tuning via `hkask-mcp-training` | `LoRAAdapter` (training MCP) | — |
| Store | Persist metadata + weights in `AdapterStore` | `TrainedLoRAAdapter` | `AdapterStored` |
| Load | Retrieve by ID, expertise, or owner filter | `TrainedLoRAAdapter` | `AdapterRetrieved` |
| Compose | Select provider, upload adapter, provision endpoint | `InferenceEndpointHandle` | `EndpointCreateStarted`, `EndpointCreateConfirmed` |
| Deploy | Run inference, accrue cost, monitor budget | `EndpointLifecycle` | `EndpointInference`, `EndpointCostAccrued` |
| Teardown | Drain in-flight requests, terminate endpoint | `EndpointGuard` (RAII) | `EndpointDraining`, `EndpointTerminated` |

---

## 2. AdapterStore CRUD

The `AdapterStore` (in `crates/hkask-adapter/src/adapter_store.rs`) is a SQLite-backed persistence layer for trained adapters. It follows the `hkask-storage` `define_store!` pattern — auto-migrated schema, content-addressed storage, owner-scoped access.

### 2.1 The `TrainedLoRAAdapter` Type

Every stored adapter is a `TrainedLoRAAdapter`:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Unique adapter identifier |
| `name` | `String` | Human-readable name (from expertise) |
| `owner` | `WebID` | Sovereign owner — no anonymous artifacts (P12) |
| `expertise` | `Expertise` | Named, domain-scoped capability descriptor |
| `base_model_family` | `String` | e.g. `"llama-3.3-70b"` — fast DB queries |
| `source` | `AdapterSource` | Distribution source (`HuggingFace { repo: String }`) |
| `checksum` | `Checksum` | SHA-256 of adapter weights |
| `storage_path` | `String` | Local path to `adapter_config.json` + `adapter_model.safetensors` |
| `version` | `Option<String>` | Caller-managed version tag (e.g. `"v3"`) |
| `size_bytes` | `Option<u64>` | Weight size, populated post-training |
| `created_at` | `String` | ISO 8601 timestamp |

### 2.2 Store an Adapter

```rust
use hkask_adapter::AdapterStore;
use hkask_adapter::{TrainedLoRAAdapter, AdapterSource, Expertise, MdsDomain};

let store = AdapterStore::open("hkask.db")?;
store.migrate()?;

let adapter = TrainedLoRAAdapter {
    id: Uuid::new_v4(),
    expertise: Expertise::new(
        "solidity-audit-v1".into(),
        MdsDomain::SolidityAudit,
        serde_json::json!({"capabilities": ["reentrancy-detection"]}),
        training_provenance,
    )?,
    checksum: Checksum::from_hex("abc123...")?,
    storage_path: "/data/adapters/solidity-audit-v1".into(),
    base_model_family: "llama-3.3-70b".into(),
    version: Some("v1".into()),
    source: AdapterSource::HuggingFace {
        repo: "mdz-axolotl/solidity-audit-v1".into(),
    },
    size_bytes: Some(45_000_000),
    owner: WebID::parse("https://my-domain.org/profile#me")?,
    created_at: Utc::now().to_rfc3339(),
};

store.store(&adapter)?;
// Emits: CNS span cns.adapter.stored
```

### 2.3 Retrieve an Adapter

```rust
// By ID
let adapter = store.get_by_id(&adapter_id)?;
// Emits: cns.adapter.retrieved

// By expertise name
let adapters = store.get_by_expertise("solidity-audit-v1")?;

// List all adapters owned by a WebID
let my_adapters = store.list_owner(&my_webid)?;
```

### 2.4 Delete an Adapter

```rust
store.delete(&adapter_id)?;
// Emits: cns.adapter.deleted
```

Deletion is permanent. The store errors with `AdapterStoreError::NotFound` if the adapter does not exist.

### 2.5 The `AdapterSource` Enum

Adapters are distributed from a known source — currently HuggingFace, with the enum designed for extension:

```rust
pub enum AdapterSource {
    HuggingFace { repo: String },
}
```

The `repo` field holds the full HF repository path (e.g. `"mdz-axolotl/solidity-audit-v1"`). Providers pull adapter weights from this source during endpoint provisioning.

### 2.6 Error Types

| Variant | Meaning |
|---------|---------|
| `NotFound` | Adapter ID not in store |
| `ExpertiseNotFound` | No adapters match the expertise query |
| `ChecksumMismatch` | Weight checksum validation failed |
| `Database` | SQLite I/O error |
| `Infra` | Storage infrastructure error |
| `Serialization` | Failed to serialize/deserialize adapter metadata |

---

## 3. AdapterRouter — Composition and Routing

The `AdapterRouter` (in `crates/hkask-adapter/src/adapter_router.rs`) composes adapters with base models via cloud inference providers. It implements the `AdapterPort` trait — the 6-method, OCAP-gated boundary for all adapter lifecycle operations.

### 3.1 The `AdapterPort` Trait (6 Methods)

Every method requires a `DelegationToken` with the appropriate capability:

| Method | Capability Required | Purpose |
|--------|-------------------|---------|
| `list_adapters(expertise?, token)` | `adapter:read` | List adapters owned by the caller |
| `estimate_composition(adapter_id, provider, token)` | `adapter:deploy` | Estimate cost + setup time for a provider |
| `create_endpoint(adapter_id, provider, token)` | `adapter:deploy` | Provision an inference endpoint |
| `endpoint_status(endpoint_id, token)` | `adapter:read` | Query endpoint phase + cost |
| `infer(endpoint_id, prompt, params, token)` | `adapter:infer` | Run inference against a composed endpoint |
| `teardown_endpoint(endpoint_id, token)` | `adapter:teardown` | Transition to Draining → Terminated |

No ambient access. No silent operations. Every call carries a provable capability.

### 3.2 Router Architecture

```
AdapterRouter
  ├── AdapterStore (Arc-shared, SQLite)
  ├── HashMap<ProviderId, AdapterProviderBackend>
  │     ├── TogetherAdapterBackend  (real HTTP upload + inference)
  │     ├── RunpodAdapterBackend    (vLLM skeleton)
  │     └── BasetenAdapterBackend   (skeleton)
  └── Mutex<HashMap<Uuid, EndpointRecord>>  (active endpoints)
```

### 3.3 Provider Selection (P2 Affirmative Consent)

Before creating an endpoint, select a provider:

```rust
let selection = router.select_provider(
    adapter_id,
    budget_limit, // optional: max hourly rate
    &token,
)?;

// selection.providers — all compatible providers, sorted cheapest first
// selection.within_budget_count — how many fall within budget
// selection.single_candidate — if exactly one provider is compatible
//   (but requires_confirmation is ALWAYS true — P2)

// Present to user, get explicit consent, then:
let handle = router.create_endpoint(
    adapter_id,
    selected_provider,
    &token,
)?;
```

The `ProviderSelection` struct always requires user confirmation — even when only one provider is compatible. This is P2 (Affirmative Consent): the system never silently selects a provider.

### 3.4 EndpointGuard — RAII Teardown (P5)

Every created endpoint is wrapped in an `EndpointGuard`:

```rust
pub struct EndpointGuard {
    endpoint_id: Uuid,
    router: Arc<AdapterRouter>,
    consumed: bool,  // prevents double-teardown
}
```

The guard's `Drop` implementation calls `teardown_endpoint()` automatically — ensuring resources are released even on panic, session exit, or budget exhaustion. You can also call `guard.teardown()` explicitly.

No dangling endpoints. No leaked GPU billing. P5 (Generative Space) — the system cleans up after itself.

---

## 4. EndpointLifecycle — 5-Phase State Machine

Every inference endpoint is governed by a validated state machine (`crates/hkask-adapter/src/endpoint_lifecycle.rs`) that enforces legal phase transitions and tracks cost accrual.

### 4.1 Phase Diagram

```mermaid
stateDiagram-v2
    [*] --> Provisioning: EndpointLifecycle::new()
    Provisioning --> Ready: Provider confirms endpoint URL
    Ready --> Active: First inference request
    Ready --> Draining: Direct teardown (never Active)
    Active --> Active: Subsequent inference
    Active --> Draining: Teardown requested
    Draining --> Terminated: In-flight requests complete
    Terminated --> [*]
```

### 4.2 Validated Transitions

| From | To | Allowed? | CNS Span |
|------|----|----------|----------|
| `Provisioning` | `Ready` | ✅ | `EndpointCreateConfirmed` |
| `Ready` | `Active` | ✅ | `EndpointInference` |
| `Ready` | `Draining` | ✅ (direct teardown) | `EndpointDraining` |
| `Active` | `Active` | ✅ (self-loop) | `EndpointInference` |
| `Active` | `Draining` | ✅ | `EndpointDraining` |
| `Draining` | `Terminated` | ✅ | `EndpointTerminated` |
| `Provisioning` | `Active` | ❌ | — |
| `Terminated` | *anything* | ❌ | — |

Any invalid transition returns `EndpointPhaseError::InvalidTransition` — the phase does not change.

### 4.3 Cost Accrual (P9 Homeostasis)

Three phases are billable: `Provisioning`, `Ready`, and `Active`. Cost accrues automatically on phase transitions:

```rust
let mut lc = EndpointLifecycle::new(1.10)?; // $1.10/hr (Together AI)
// Phase: Provisioning, cost_accrued: $0.00

lc.transition(EndpointPhase::Ready)?;
// Cost accrued for time spent in Provisioning
// Phase: Ready

lc.accrue_cost(3600.0); // 1 hour of Active time → $1.10
lc.transition(EndpointPhase::Draining)?;
// Draining and Terminated do NOT accrue cost
```

Budget enforcement is built-in:

```rust
if lc.is_over_budget(50.0) {
    // Emits: cns.endpoint.cost.budget_warning
    // Trigger teardown
}

let remaining = lc.time_until_budget_exceeded(50.0);
// Returns seconds until budget cap is hit
```

### 4.4 EndpointLifecycle Fields

| Field | Type | Description |
|-------|------|-------------|
| `phase` | `EndpointPhase` | Current phase |
| `phase_changed_at` | `DateTime<Utc>` | When current phase was entered |
| `cost_accrued` | `f64` | Total cost in configured currency |
| `hourly_rate` | `f64` | Provider GPU hourly rate |
| `created_at` | `DateTime<Utc>` | When the endpoint was created |

---

## 5. Ownership Model (P12)

**Every adapter has an owner.** The `TrainedLoRAAdapter.owner` field is a `WebID` — a sovereign identity URI. This is P12 (Replicant Host Mandate): no anonymous artifacts, no anonymous agency.

- **Store:** `owner` is required at insertion time
- **List:** `list_owner(webid)` returns only adapters owned by that WebID
- **Delete:** Only the owning agent should delete — enforced at the application layer
- **Inference:** Every `EndpointInference` CNS span carries the owning WebID

No root. No `sudo`. No shared "admin" adapter pool. Every adapter is sovereign-scoped.

---

## 6. CNS Observability

All adapter and endpoint operations emit CNS spans for programmatic observability. Query via `kask cns health`.

### 6.1 Adapter Lifecycle Spans

| Span | Trigger |
|------|---------|
| `cns.adapter.stored` | `AdapterStore::store()` succeeds |
| `cns.adapter.retrieved` | `AdapterStore::get_by_id()` succeeds |
| `cns.adapter.deleted` | `AdapterStore::delete()` succeeds |

Defined in `crates/hkask-types/src/cns.rs` as `CnsSpan::AdapterStored`, `AdapterRetrieved`, `AdapterDeleted`.

### 6.2 Endpoint Lifecycle Spans

| Span | Trigger |
|------|---------|
| `cns.endpoint.create.started` | `create_endpoint()` called — provisioning begins |
| `cns.endpoint.create.confirmed` | Provider returns endpoint URL (Provisioning → Ready) |
| `cns.endpoint.inference` | Each inference request served |
| `cns.endpoint.draining` | Teardown initiated (→ Draining phase) |
| `cns.endpoint.terminated` | Endpoint fully released (→ Terminated) |
| `cns.endpoint.cost.accrued` | Cost updated on phase transition or explicit `accrue_cost()` |
| `cns.endpoint.cost.budget_warning` | `is_over_budget()` returns `true` |

Defined in `crates/hkask-types/src/cns.rs` as `CnsSpan::EndpointCreateStarted` through `EndpointCostBudgetWarning`.

---

## 7. Provider Support

Three cloud inference providers are supported, each with a `CostModel` and `ProviderCapability` for transparent pricing (P2 affirmative consent, P9 homeostasis).

### 7.1 Provider Comparison

| Provider | LoRA Compose | Hourly Rate (USD) | Setup Time | Max Adapter | Base Models |
|----------|-------------|-------------------|------------|-------------|-------------|
| **Together AI** | ✅ Real HTTP | $1.10/hr | ~3 min | 500 MB | llama-3.3-70b, llama-3.1-70b, qwen2.5-72b |
| **Runpod** | ✅ vLLM skeleton | $0.79/hr | ~5 min | 500 MB | llama-3.3-70b, llama-3.1-70b, qwen2.5-72b, mixtral-8x7b |
| **Baseten** | ✅ vLLM skeleton | $0.85/hr | ~4 min | 256 MB | llama-3.3-70b, qwen2.5-72b |

### 7.2 CostModel Structure

Every provider exposes:

```rust
pub struct CostModel {
    pub provider: ProviderId,             // Together | Runpod | Baseten
    pub gpu_hourly_rate: f64,             // e.g. 1.10
    pub estimated_setup_minutes: u32,      // e.g. 3
    pub estimated_teardown_grace_seconds: u32,  // e.g. 30
    pub currency: String,                 // e.g. "USD"
}
```

Cost estimates are transparent and user-visible before provisioning. The system never silently selects a provider or hides a cost (P2).

### 7.3 Together AI — Full Implementation

Together AI has the most complete backend — real HTTP upload of adapter weights (`adapter_model.safetensors` + `adapter_config.json`) and live inference via their API. It polls until the fine-tuning job completes, then returns the endpoint URL.

### 7.4 Runpod and Baseten — vLLM Skeletons

Runpod and Baseten backends follow the `AdapterProviderBackend` trait interface with vLLM-based provisioning, inference, and teardown. They operate as skeletons — the trait seam allows adding full implementations without changing the router (P7 Evolutionary Architecture).

### 7.5 Non-LoRA Providers

Some providers (`DeepInfra`) do not support LoRA composition. Their `ProviderCapability::supports_lora_composition` is `false`, and `can_compose()` returns `false` for all base models. The `select_provider()` method filters them out automatically.

---

## Quick Reference

| Operation | Method | Capability | CNS Span |
|-----------|--------|-----------|----------|
| Store adapter | `AdapterStore::store()` | — (local) | `AdapterStored` |
| Get adapter | `AdapterStore::get_by_id()` | — (local) | `AdapterRetrieved` |
| List owner's | `AdapterStore::list_owner()` | — (local) | — |
| Delete adapter | `AdapterStore::delete()` | — (local) | `AdapterDeleted` |
| Estimate cost | `AdapterPort::estimate_composition()` | `adapter:deploy` | — |
| Create endpoint | `AdapterPort::create_endpoint()` | `adapter:deploy` | `EndpointCreateStarted`, `EndpointCreateConfirmed` |
| Check status | `AdapterPort::endpoint_status()` | `adapter:read` | — |
| Run inference | `AdapterPort::infer()` | `adapter:infer` | `EndpointInference` |
| Tear down | `AdapterPort::teardown_endpoint()` | `adapter:teardown` | `EndpointDraining`, `EndpointTerminated` |

---

## References

- [`crates/hkask-adapter/src/adapter_store.rs`](../../crates/hkask-adapter/src/adapter_store.rs) — AdapterStore + TrainedLoRAAdapter
- [`crates/hkask-adapter/src/adapter_router.rs`](../../crates/hkask-adapter/src/adapter_router.rs) — AdapterRouter + EndpointGuard
- [`crates/hkask-adapter/src/adapter_port.rs`](../../crates/hkask-adapter/src/adapter_port.rs) — AdapterPort trait (6 OCAP-gated methods)
- [`crates/hkask-adapter/src/endpoint_lifecycle.rs`](../../crates/hkask-adapter/src/endpoint_lifecycle.rs) — 5-phase state machine
- [`crates/hkask-adapter/src/expertise.rs`](../../crates/hkask-adapter/src/expertise.rs) — Expertise + MdsDomain
- [`crates/hkask-adapter/src/provider_cost.rs`](../../crates/hkask-adapter/src/provider_cost.rs) — CostModel + ProviderCapability
- [`crates/hkask-types/src/cns.rs`](../../crates/hkask-types/src/cns.rs) — CNS span registry
- [`docs/architecture/hKask-architecture-master.md`](../architecture/hKask-architecture-master.md) — includes deep-module public surface audit
- [`docs/architecture/core/PRINCIPLES.md`](../architecture/core/PRINCIPLES.md) — P1-P12 principle definitions
