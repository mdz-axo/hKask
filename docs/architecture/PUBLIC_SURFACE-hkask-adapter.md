---
title: "Public Surface Justification — hkask-adapter"
audience: [architects, developers]
last_updated: 2026-06-17
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition, lifecycle, trust]
---

# Public Surface Justification — hkask-adapter

**Crate:** `hkask-adapter`  
**Public items in lib.rs:** 17  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-adapter` is the **trained adapter lifecycle & inference composition** crate — it manages the full lifecycle of LoRA adapters from training provenance through cloud deployment to cost-tracked inference. Its surface is large because it spans multiple concerns:

1. **Domain types** — `Expertise`, `MdsDomain`, `AdapterSource`, `AdapterConfig`, `TrainedLoRAAdapter` —
   the semantic grounding of what an adapter is, where it came from, and where it's hosted.
2. **Persistence** — `AdapterStore` (SQLite CRUD) following the `hkask-storage` pattern with `define_store!`.
3. **Lifecycle** — `EndpointLifecycle` state machine (5 phases: Provisioning → Ready → Active → Draining → Terminated)
   with validated transitions, cost accrual, budget enforcement.
4. **Provider abstraction** — `CostModel`, `ProviderCapability`, `ProviderInfo` for transparent pricing
   and compatibility checks across Together AI, Runpod, and Baseten.
5. **Composition trait** — `AdapterPort` (6 OCAP-gated methods) defines the contract boundary.
   `AdapterRouter` implements it with in-memory endpoint tracking and real HTTP upload/inference
   for Together AI.
6. **Safety** — `EndpointGuard` (RAII teardown), budget enforcement, `ProviderSelection` with
   P2 affirmative consent.

## Trait Architecture

```
AdapterPort (6 methods, OCAP-gated)
  └── AdapterRouter
        ├── AdapterStore (SQLite, Arc-shared)
        ├── HashMap<ProviderId, AdapterProviderBackend>
        │     ├── TogetherAdapterBackend  (real HTTP upload + inference)
        │     ├── RunpodAdapterBackend    (vLLM skeleton)
        │     └── BasetenAdapterBackend   (skeleton)
        └── Mutex<HashMap<Uuid, EndpointRecord>>  (active endpoints)
```

## Deletion Test

Delete `hkask-adapter` and the adapter lifecycle, provider routing, cost modeling,
and OCAP-gated composition reappear in every crate that needs to deploy trained adapters.
The crate earns its existence through the `AdapterPort` trait boundary.

## Essentialist Review Summary

| Gate | Finding | Action |
|------|---------|--------|
| G1 (Exist) | `base_model_family` duplicated in `Expertise.training_source` and `TrainedLoRAAdapter` | Kept for fast DB queries; derived from expertise at construction |
| G2 (Surface) | `list_compatible_providers` exposed publicly when `select_provider` supersedes it | Made `pub(crate)` — reduced public surface by 1 |
| G3 (Contract) | `EndpointLifecycle` encodes real behavior (5 phases, cost tracking) | Survived — deleting it causes behavior to vanish |

## CNS Span Coverage

```
cns.adapter.stored         — Adapter persisted to AdapterStore
cns.adapter.retrieved      — Adapter loaded from AdapterStore
cns.adapter.deleted        — Adapter removed from AdapterStore
cns.endpoint.create.started    — Endpoint provisioning initiated
cns.endpoint.create.confirmed  — Provider confirmed endpoint URL
cns.endpoint.inference         — Inference request served
cns.endpoint.draining          — Endpoint accepting no new requests
cns.endpoint.terminated        — Endpoint fully released
cns.endpoint.cost.accrued      — Cost update emitted
cns.endpoint.cost.budget_warning — Budget threshold exceeded
```
