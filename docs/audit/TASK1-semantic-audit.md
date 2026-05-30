# Task 1: Semantic Audit — Abstraction-to-Purpose Mapping

## Method

Every abstraction in the hKask codebase was traced to one of four root purposes:

| Root Purpose | Definition | Example |
|---|---|---|
| **compute** | Transforms data, produces outputs | `blake3_hash`, `combine_confidences`, `infer` |
| **remember** | Persists, stores, recalls, retrieves | `TripleStore::insert`, `NuEventStore`, `Keychain::store` |
| **communicate** | Sends, receives, routes, formats messages/signals | `AcpPort`, `InferencePort`, `SpanEmitter::emit` |
| **govern** | Enforces rules, controls access, manages lifecycles, makes policy | `verify_capability`, `check_visibility`, `EnergyBudget::try_consume` |

Abstractions that serve no root purpose beyond hosting other abstractions are flagged as **structural overhead** — candidates for collapse/fold/delete.

## Statistics

| Root Purpose | Abstractions | Percentage |
|---|---|---|
| govern | ~180 | 37% |
| remember | ~120 | 25% |
| structural-overhead | ~115 | 24% |
| communicate | ~45 | 9% |
| compute | ~22 | 5% |

hKask is governance-heavy by design — it's an OCAP-first agent container where every operation is capability-gated, visibility-controlled, and sovereignty-checked. The `remember` layer (25%) is SQLite+SQLCipher persistence. `compute` (5%) is minimal because hKask orchestrates rather than computes — the LLM does the heavy lifting.

## Structural Overhead Breakdown

| Category | Count | Description |
|---|---|---|
| Error enums | ~35 | Nearly every module wraps `InfrastructureError`. Could consolidate to crate-level errors. |
| Builder methods | ~25 | `with_*` constructors and `Default` impls that add zero logic. |
| YAML deserialization DTOs | ~20 | `YamlAgentHeader`, `YamlCharter`, `OkapiConfig`, etc. — exist solely for serde. |
| Re-exports | ~15 | Types re-exported across crate boundaries without transformation. |
| Module namespace markers | ~10 | Empty `mod` blocks that exist only to organize. |
| Single-variant enums | ~5 | `EpisodicMemoryError`, etc. — wrap another error with no added dispatch. |
| Dead code | ~5 | `#[allow(dead_code)]` fields never wired to runtime. |

## Collapse Candidates (7 high-priority)

Types that differ only by a non-behavioral discriminator — they should be unified with the minimal discriminator.

| # | Abstractions | Why Collapse | Unified Form |
|---|---|---|---|
| C1 | `CnsSpan` (14 variants) + `Span` (14 variants) | Identical structure — each variant wraps a string with `cns.*` prefix. `Span` already has `category` + `path` via serde tag. | `Span { category: SpanCategory, path: String }`. Keep `SpanCategory` as the exhaustive enum. Delete `CnsSpan`. |
| C2 | `BotCapabilities` + `ReplicantCapabilities` | Nearly identical: both carry `can_invoke_tools`, `can_dispatch_templates`, `can_escalate`. Replicant adds memory access; Bot has `can_access_memory`. | `AgentCapabilities` with `MemoryAccess` enum (`None` / `Semantic` / `SemanticAndEpisodic`). |
| C3 | `ContractValidator` + `CapabilityAwareValidator` | Both validate template frontmatter against Okapi capabilities. The latter wraps the former. | `ContractValidator` with `with_capability_checking()` builder. |
| C4 | `OkapiHttpClient` + `OkapiImprovClient` | Both implement `InferenceClient`; differ only in response parsing. | `OkapiInference` with `InferenceMode` enum. |
| C5 | `SystemHealthSnapshot` + `StoredHealthSnapshot` | Overlapping fields (cns_health, critical_alerts, total_alerts, variety counters). | `SystemHealthSnapshot` with `to_stored()` conversion. |
| C6 | `EnsembleChatManager` + `DeliberationCoordinator` | Structurally identical `HashMap<String, Arc<RwLock<Session>>>` pattern. | Generic `SessionMap<K, V>` with type aliases. |
| C7 | `AgentPersonaInput` + `AgentPersona` | Both carry name, type, capabilities. The input DTO shadows the domain type. | Validate `AgentPersona` directly; remove the separate input DTO. |

## Fold Candidates (15 high-priority)

Functions that merely delegate to another function with no added logic — inline the delegation and remove the wrapper.

| # | Function | Delegates To | Why Fold |
|---|---|---|---|
| F1 | `SemanticMemory::recall()` | `self.query_deduped()` | Zero logic — entire body is `self.query_deduped(entity)`. |
| F2 | `GoalMemory::list_goals_result()` | `self.list_goals()` | Zero logic — entire body is `self.list_goals(webid)`. |
| F3 | `GoalMemory::recall_semantic()` | `self.recall_goal_semantic()?.ok_or_else(...)` | Adds only `None → NotFound` mapping — weak fold. |
| F4 | `SpanEmitter::emit()` | `self.emit_with_phase(span, Phase::Observe, ...)` | Zero logic — convenience for default phase. |
| F5 | `SpanEmitter::emit_*()` (9 methods) | `self.emit()` then `self.emit_with_phase()` | 9 convenience methods that just call `emit_with_phase` with a category. |
| F6 | `CnsEmit::emit()` (default method) | `self.emit_event(span, "observe", ...)` | Default that just forwards to `emit_event`. |
| F7 | `CnsRuntime::new()` | `Self::with_threshold(DEFAULT_THRESHOLD)` | Standard delegation constructor. |
| F8 | `BayesianOps::new()` | Returns `Self` (unit struct) | Remove unit struct; keep `combine`, `retract`, `decay` as free functions. The subloop they serve (confidence resolution) is essential — only the wrapper is folded. |
| F9 | `GoalVarietyCounter::variety_counter()` | Wraps `self.active_goal_count` in `VarietyCounter(usize)` | Pure wrapping, no logic. |
| F10 | `derive_sub_key_hex()` | `derive_sub_key()` + `hex::encode()` | Zero logic — just format conversion. |
| F11 | `Keychain::store/retrieve/delete` (WebID variants) | `store_by_key(webid.0.to_string(), ...)` etc. | WebID methods duplicate the `*_by_key` methods. |
| F12 | `config::load_yaml_config()` | Single function in its own module | Only consumer is `cascade::load_cascade_config`. Fold into cascade. |
| F13 | `emit_tool_span()` | `emit_tool_span_with_caller(None)` | Zero logic — just passes `None` for caller. |
| F14 | `PermissiveSovereigntyChecker` | Returns `true` for all checks | Replace with `Option<Box<dyn SovereigntyPort>>` where `None` means allow-all. |
| F15 | `CnsIntegrationBuilder` | `build()` ignores the one option | Builder has one option and ignores it. Fold into `CnsIntegration::with_variety_threshold()`. |

## Delete Candidates (6 dead code items)

| # | What | Why Delete |
|---|---|---|
| D1 | `BotHealthStatus::Unresponsive` | Never constructed anywhere in the codebase. |
| D2 | `EnergyError::InvalidCost` + `EnergyError::Deficit` | Never constructed anywhere in the codebase. |
| D3 | `KeychainError::Encryption` | Vestigial — no code in the keychain module produces this variant. |
| D4 | `StandingSessionConfig.consensus_required`, `.orchestration_model`, `BootstrapConfig.auto_start`, `ParticipantEntry.voting` | `#[allow(dead_code)]` fields never wired to runtime. |
| D5 | `services/sovereignty.rs` (hkask-api) | 17-line stub not used by `routes/sovereignty.rs`. |
| D6 | `McpMcpRetryConfig` + unused `_retry_config` field on `McpDispatcher` | Dead code. |

## Key Insight

The 24% structural overhead is not wasted — it's Rust-idiomatic scaffolding (error types, builders, serde DTOs). The collapse/fold/delete targets are specific: duplicate types, delegation wrappers, and dead code. **No essential subloop is targeted for removal.** The Bayesian combination functions (fold candidate F8) remain — only the `BayesianOps` unit struct wrapper is removed, keeping `combine()`, `retract()`, and `decay()` as free functions.

## Irreducible Entity Model

Complementary analysis (system-simplification-core-loops.md Task 0) reduced hKask's nominal concepts to 7 irreducible entities. Every concept not in this list collapses into an attribute, constraint, or derivation of one of these 7.

| # | Entity | Location | Justification |
|---|--------|----------|---------------|
| 1 | **AgentPod** | `hkask-agents/src/pod/mod.rs` | The container. No container, no agent. |
| 2 | **WebID** | `hkask-types/src/id.rs` | Deterministic identity via UUID v5. No identity, no delegation, no ownership. |
| 3 | **CapabilityToken** | `hkask-types/src/capability/mod.rs` | OCAP primitive — HMAC-SHA256 signed, caveated, attenuable. No capability, no least-authority. |
| 4 | **NuEvent** | `hkask-types/src/event.rs` | Cybernetic observability atom (observer → span → phase). No event, no CNS. |
| 5 | **TemplateEntry** | `hkask-templates/src/registry.rs` | The "what" an agent does — Prompt, Process, Cognition, or Specification. No template, no behavior. |
| 6 | **Triple** | `hkask-storage/src/triples.rs` | Knowledge atom (entity, attribute, value) with bitemporal dimensions. No triple, no memory. |
| 7 | **Goal** | `hkask-types/src/goal.rs` | User intent atom — decomposable, with criteria and artifacts. No goal, no directed autonomy. |

### Collapsed Into Attributes / Constraints

| Nominal Concept | Collapses Into | Reason |
|----------------|----------------|--------|
| **Span** (14 variants) | NuEvent's `span` field | Span categorizes NuEvent — it's a field, not an independent entity. |
| **Spec** | Subtype of Goal | Spec is a *structured* goal with DDMVSS scaffolding. Same `id`, `state`, `visibility` fields. |
| **SecretRef** | Derivation recipe | `Env | Keychain | Derived | Generated` are *how* secrets resolve, not an entity with independent lifecycle. |
| **HLexicon** | Constraint vocabulary on TemplateEntry | 87 term-slots anchor templates to domains. A lexicon term has no independent existence. |
| **DataCategory** | Visibility attribute on Triple + Goal | `Private | Shared | Public` is a field, not an entity. |
| **AgentDefinition** | TemplateEntry (Specification type) | Declarative agent config *is* a specification template. |
| **TemplateInvocation** | Audit artifact of Rendering sub-loop | Rendering record — essential for traceability, but not a structural entity. |
| **Embedding** | Derived from Triple | Vectorized representation for KNN search. Exists only as a function of a Triple. |

This 7-entity model anchors the capability boundary design in TASK5: each handle grants access to exactly the subset of these entities that its loop needs.