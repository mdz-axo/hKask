# Task 4: Simplify — Collapse, Fold, Delete Changelog

**Principle:** These changes simplify *implementation packaging*, not *essential subloops*. Every subloop identified in Task 2 is essential — the question is whether the current packaging is the minimal way to close it. See Task 2 for the subloop inventory and the corrected "essential subloops" analysis.

---

## COLLAPSE — Types that differ only by a non-behavioral discriminator

| # | What | Action | Core Loop Preserved |
|---|------|--------|---------------------|
| C1 | `CnsSpan` (14 variants) + `Span` (14 variants) | **Collapse** into `Span { category: SpanCategory, path: String }`. Keep `SpanCategory` as the exhaustive enum. Delete `CnsSpan`. | ✓ Inference, Observability — `emit_event()` takes `Span`, which uses `SpanCategory` |
| C2 | `BotCapabilities` + `ReplicantCapabilities` | **Collapse** into `AgentCapabilities` with `MemoryAccess` enum (`None` / `Semantic` / `SemanticAndEpisodic`) | ✓ Governance — capability checks use `AgentCapabilities` uniformly |
| C3 | `ContractValidator` + `CapabilityAwareValidator` | **Collapse** into `ContractValidator` with `with_capability_checking()` builder | ✓ Inference — template validation still validates |
| C4 | `OkapiHttpClient` + `OkapiImprovClient` | **Collapse** into `OkapiInference` with `InferenceMode` enum (Standard / Improv) | ✓ Inference — same interface, same behavior |
| C5 | `SystemHealthSnapshot` + `StoredHealthSnapshot` | **Collapse** into `SystemHealthSnapshot` with `to_stored()` conversion | ✓ Observability — CNS health still captured |
| C6 | `EnsembleChatManager` + `DeliberationCoordinator` | **Collapse** into generic `SessionMap<K, V>` with type aliases | ✓ Communicate — same session management |
| C7 | `AgentPersonaInput` + `AgentPersona` | **Collapse** — validate `AgentPersona` directly, remove the separate input DTO | ✓ Governance — same validation |

## FOLD — Functions that merely delegate with no added logic

| # | What | Action | Core Loop Preserved |
|---|------|--------|---------------------|
| F1 | `SemanticMemory::recall()` | **Fold** — inline at call sites to `query_deduped()` | ✓ Memory |
| F2 | `GoalMemory::list_goals_result()` | **Fold** — inline at call sites to `list_goals()` | ✓ Memory |
| F3 | `GoalMemory::recall_semantic()` | **Fold** — inline at call sites to `recall_goal_semantic()?.ok_or_else(...)` | ✓ Memory |
| F4 | `SpanEmitter::emit()` | **Fold** — callers use `emit_with_phase(span, Phase::Observe, ...)` | ✓ Observability |
| F5 | `SpanEmitter::emit_*()` (9 convenience methods) | **Fold** — callers use `emit_with_phase()` directly | ✓ Observability |
| F6 | `CnsEmit::emit()` (default method) | **Fold** — callers use `emit_event()` directly | ✓ Observability |
| F7 | `CnsRuntime::new()` | **Fold** — callers use `CnsRuntime::with_threshold(DEFAULT_THRESHOLD)` | ✓ Observability |
| F8 | `BayesianOps::new()` | **Fold** — remove unit struct; keep `combine`, `retract`, `decay` as free functions. The subloop (confidence resolution) is essential — only the struct wrapper is folded. | ✓ Memory |
| F9 | `GoalVarietyCounter::variety_counter()` | **Fold** — inline at call sites | ✓ Observability |
| F10 | `derive_sub_key_hex()` | **Fold** — inline at call sites as `hex::encode(derive_sub_key(...))` | ✓ Governance |
| F11 | `Keychain::store/retrieve/delete` (WebID variants) | **Fold** — convert WebID to string inline, call `*_by_key` | ✓ Remember |
| F12 | `config::load_yaml_config()` | **Fold** into `cascade::load_cascade_config()` — only consumer | ✓ Inference |
| F13 | `emit_tool_span()` | **Fold** — inline at call sites to `emit_tool_span_with_caller(None)` | ✓ Observability |
| F14 | `PermissiveSovereigntyChecker` | **Fold** — replace with `Option<Box<dyn SovereigntyPort>>` where `None` means allow-all | ✓ Governance |
| F15 | `CnsIntegrationBuilder` | **Fold** into `CnsIntegration::with_variety_threshold()` — builder has one option and ignores it | ✓ Observability |

## DELETE — Abstractions that trace to no root purpose

| # | What | Action | Core Loop Preserved |
|---|------|--------|---------------------|
| D1 | `BotHealthStatus::Unresponsive` | **Delete** — never constructed anywhere in the codebase | ✓ Governance |
| D2 | `EnergyError::InvalidCost` + `EnergyError::Deficit` | **Delete** — never constructed anywhere in the codebase | ✓ Governance |
| D3 | `KeychainError::Encryption` | **Delete** — vestigial, never produced by any code in keychain module | ✓ Governance |
| D4 | `StandingSessionConfig.consensus_required`, `.orchestration_model`, `BootstrapConfig.auto_start`, `ParticipantEntry.voting` | **Delete** — `#[allow(dead_code)]` fields never wired to runtime | ✓ Communicate |
| D5 | `services/sovereignty.rs` (hkask-api) | **Delete** — 17-line stub, not used by `routes/sovereignty.rs` | ✓ Governance |
| D6 | `McpMcpRetryConfig` + unused `_retry_config` field on `McpDispatcher` | **Delete** — dead code | ✓ Governance |

## Verification Matrix

Every change is verified against all four core loops and their essential subloops:

| Core Loop | Essential Functions + Subloops | Verified After Changes |
|---|---|---|
| **Inference** | render_template, assemble_context, get_or_infer, infer, try_consume, with_circuit_breaker, parse_response, dispatch_action, emit_span | ✓ C4/F8/F12 don't affect essential path |
| **Memory** | encode_triple, store_triple, query_triples, dedup_triples, consolidate, combine_confidences, retract_confidence, decay_confidence, assemble_context | ✓ C1/F1/F2/F3 don't affect essential path |
| **Governance** | verify_capability, is_revoked, attenuate_token, revoke_capability, check_visibility, can_transition_to, try_consume, process_alert, calibrate_threshold | ✓ C2/C7/F14 don't affect essential path |
| **Observability** | emit_event, increment_variety, check_variety, determine_severity, process_alert, record_calibration, evaluate_bot, process_sovereignty_event | ✓ C1/F4/F5/F6 don't affect essential path |