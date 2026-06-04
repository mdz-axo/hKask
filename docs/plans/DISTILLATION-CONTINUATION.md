# hKask Loop Distillation — Continuation Prompt

### What Was Done

**P0 (Architecture Integrity) — COMPLETE**

- **Task 1:** Excised 15 dead code items across 20+ files (ThrottleBucket, CuratorId, BotCapabilities, AUTHORITY_EDGES, McpGovernor dead methods, McpDispatcher dead methods, McpToolOutput/McpToolError dead methods, AdapterContainer::clear, GoalRepositoryError::CapabilityDenied, SynthesisMode→pub(crate), consolidation_count, SemanticLoop::with_budget, default_gas_table→pub(crate), CircuitBreaker/KillZoneDetector→pub(crate)). Corrected 4 claims: 1h (only `clear` dead, `has_git_cas`/`get_base_path` live), 1j (live stubs not dead), 1l (EncryptionService has external consumers), 1r (AllostericError in return type, can't be pub(crate)).
- **Task 4:** Fixed DIAG-LOOP-002 ERD — removed 3 wrong `CommunicationLoop → "regulates"` edges, changed `CurationLoop → CommunicationLoop` from "regulates" to "signals", added `CyberneticsLoop → CommunicationLoop : "regulates"`. Updated in authoritative `loop-architecture.md`. Reference copy marked superseded.
- **Task 5:** Elevated loop-architecture.md to framework doc. Added Loop Assignment sections to all 4 authoritative specs. Added §1.7 Loop Mapping to PRINCIPLES.md mapping Five Anchors to 6 loops. Updated hKask-architecture-master.md.

**P1 (Authority & Sovereignty) — COMPLETE**

- **Task 2 (Approach B):** Added §1.6 "External-Boundary Rate Limiting — Security Exception" to authoritative `loop-architecture.md`. Classified per-tool RateLimiter in `hkask-mcp-web` as Communication Loop (Loop 4) security membrane, distinct from Cybernetics (Loop 6) energy budget regulation. Documented CNS override authority.
- **Task 3:** Hardened privacy membrane in code:
  - 3a: `SemanticMemory::store()` now returns `Err(SemanticMemoryError::InvalidVisibility)` when `visibility != Shared` or `perspective.is_some()`. No more silent override.
  - 3b: `EpisodicMemory::store()` now rejects `Visibility::Shared` triples and triples without perspective.
  - 3c: `SemanticMemory::query_deduped()` filters by `visibility == Shared` as defense-in-depth.
  - Bonus: Fixed `ConsolidationBridge` to set `visibility: Visibility::Shared` on consolidated triples (was preserving `triple.visibility` which would be `Private` and invisible under the new filter).
- **Task 10:** Added §10 "Consolidation Protocol" to `domain-and-capability.md` specifying: one-way invariant, trigger chain, implicit consent, OCAP authority chain (CuratorHandle→ConsolidationToken→ConsolidationBridge), 4-step algorithm, privacy boundary crossing (perspective stripping), retraction policy, failure semantics, CNS observability.
- **Task 11:** Fixed Curation→Cybernetics compliance gap:
  - Fix 1 (CRITICAL): Added `.with_target(LoopId::Cybernetics)` to `send_curator_directive()` — all CuratorDirectives were being broadcast (no target) and silently dropped by CommunicationLoop.
  - Fix 2: Fixed string mismatch `"calibrate"` → `"calibrate_threshold"` in `CyberneticsLoop::process_inbox()`.
  - Fix 3: Added handlers for `"update_capabilities"` and `"seek_more_evidence"` directive types.
  - Fix 4: Added `tracing::info!` acknowledgment after directive processing with TODO for full NuEvent emission.
  - Removed dead `"adjust_gas_budget"` match arm (never produced by Curation dispatch).
  - Updated 4 test cases to match corrected directive type strings.
  - All 91 CNS tests pass, 13 agent tests pass.

**P2 (Surface Cleanup) — COMPLETE**

- **Task 6 (Spec-Code Divergences):**
  - 6a: Updated PRINCIPLES.md §1.4 span namespaces from 4 listed → 15 canonical namespaces matching `CANONICAL_NAMESPACES` in `hkask-types/src/event.rs`.
  - 6b: Added `Specification` variant to `TemplateType` enum (code + ERDs), matching ADR-024.
  - 6c: Updated subsystem-erds.md Span entity from 10→15 variants; fixed "Energy"→"Gas"; added Inference, Template, Curation, Variety, KillZone.
  - 6d: Reconciled MCP server count 14→15 across 10+ docs (PRINCIPLES, domain-and-capability, AGENTS.md, README, OPEN_QUESTIONS, DDMVSS, REQUIREMENTS, mcp-server-audit, persistence-and-lifecycle, subsystem-erds). Added `hkask-mcp-goal` to all server lists.
  - 6e: Replaced "ARL" references in domain-and-capability.md with concrete `AllostericGate`/`hkask-cns::allosteric`. Updated cns.rs comment 14→15 canonical namespaces.
  - 6f: magna-carta.md version v0.21.2 → v0.22.0.
  - 6g: hKask-erd.md version v0.21.0 → v0.22.0. Updated CNS span hierarchy mermaid diagram to show all 15 namespaces with code-accurate verb names (invoked/completed, not invocation/result).
  - 6h: Removed false circuit-breaker claim from interface-and-composition.md `InferenceClient` table entry. Removed `CircuitBreaker` entity and `ResilientOkapiClient` edge from subsystem-erds.md ensemble ERD.

- **Task 7 (Wire Unconnected Components):**
  - 7c: Wired `cns_calibrate` to actually call `CnsRuntime::calibrate_threshold()`. Changed `threshold` field to `AtomicU64` for interior mutability in `&self` MCP tool methods.
  - 7d: Added `consolidation_candidate_count()` to `ConsolidationPort` trait + `ConsolidationBridge` impl. Added `consolidation_candidates` signal in `CurationLoop::sense()`.
  - 7e: Wired `recall_pod_events` to call `self.episodic_storage.recall_episodic("lifecycle", &pod.webid, &pod.capability_token)` instead of returning empty vec.
  - 7f: Wired `MetacognitionLoop::direct_bot` to `AcpPort::send_message()` when available. Added `acp: Option<Arc<dyn AcpPort>>` to `CuratorContext` with `with_acp()` builder. Graceful degradation (warn + Ok) when ACP not configured.
  - 7g: Verified already complete — standing session routes were already wired to `ApiState.standing_sessions`.

- **Task 8 (MCP Dead Weight):**
  - 8a: Removed `keystore:prompt` — no-op stub that couldn't actually prompt (no MCP client-side mechanism).
  - 8b: Removed `git:fork` — validation-only stub that claimed `"forked": true` without performing any actual fork operation (dangerous false-positive).
  - 8d: Fixed `telnyx_list_voices` — added `language` filter parameter, documented as static catalog sourced from Telnyx Call Control API docs (no live endpoint exists).
  - 8e: Removed `telnyx_tts` — dangerous stub with hardcoded fake phone numbers (+18001234567, +18007654321) and `https://example.com/tts-webhook` that actually made real API calls with bogus parameters.
  - 8f: Removed `fal:generate_image_fast` — functionally identical to `generate_image` (same `fal-ai/flux/schnell` model, same endpoint, same default behavior).

- **Task 9 (Rust Idioms):**
  - 9b: Wired `CyberneticsToken::new` into `CyberneticsLoop` via new `CyberneticsHandle::issue_cybernetics_token()` bridge (follows `CuratorHandle` pattern). Removed `#[allow(dead_code)]`.
  - 9c: Wired `CurationToken::new` into `CurationLoop` via new `CuratorHandle::issue_curation_token()` bridge. Removed `#[allow(dead_code)]`.

- **Task 12 (API/CLI Shell Cleanup):**
  - 12b: Injected `InferencePort` via `ApiState.inference_port` (extracted from `ensemble_inferencer`). Chat route now uses shared inference port when available (avoids per-request `OkapiInference::new` construction), falls back to per-request construction when not configured. Uses `generate_with_model()` for model override.

### Validation Status

- `cargo check --workspace` passes with 0 errors
- `cargo test --workspace` passes: 129 tests, 0 failures (91 CNS + 13 agents + 10 types + 9 storage + 3 ensemble + 1 CLI + 2 doc-tests)
- All modified crates compile and pass tests

### Git State

- Commits `34a8534d` through `6b0b25d6` ("distill"/"distilling") contain all P0 + P1 code changes
- Two uncommitted files: `crates/hkask-api/src/lib.rs` and `crates/hkask-api/src/routes/chat.rs` (Task 12b — InferencePort injection)
- Total files changed across P0+P1+P2: ~56 files, +1793 / -391 lines

---

### What Remains — P2 (Deferred Items)

| Task | Subtask | What | Defer Reason |
|------|---------|------|---------------|
| **7** | 7a | Wire `hkask-mcp-condenser` to `EpisodicMemory` for persistence | Cross-process challenge — MCP servers run as separate stdio processes; no shared memory/adapter. Options: (a) embed condenser in-process, (b) condenser calls back to persistence MCP tool, (c) give condenser its own EpisodicMemory via SQLite path in ServerContext |
| **7** | 7b | Replace `hkask-mcp-rss-reader` duplicate SQLite with `hkask-storage::Database` | No schema extension mechanism in `Database`. RSS reader has 4 tables + FTS5 virtual table + triggers. `Database::initialize_schema()` hardcodes DDL with no hook for custom tables. Needs `Database::open_with_extensions()` or similar |
| **8** | 8c | Route `git_commit()` through `GitCASPort` instead of shelling out | `GitCASPort` trait has no `commit()` method — only `load_template_crate()` and `resolve_sha()`. Need to add `fn commit(&self, message: &str) -> Result<String, GitError>` to the port, implement in `GitCasAdapter`. Long-term: replace `std::process::Command` with `gix` crate API |
| **8** | 8g | Extract web search orchestration from `hkask-mcp-web` into `hkask-memory` or new crate | Significant refactoring with no current consumer. `WebSearchPort` + `ProviderPool` is tightly coupled to HTTP client construction. If extraction is desired: extract trait to `hkask-types`, move pool to new `hkask-web` crate, MCP server becomes thin shim |
| **9** | 9d | Consider `AgentKind` behavioral dispatch via associated types | Architectural decision — currently `AgentKind` is cosmetic (Bot/Replicant string tag). Adding behavioral dispatch would change it from an enum to a trait. Needs design discussion |
| **12** | 12a | Extract prompt template heuristics from `hkask-api/routes/chat.rs` to `hkask-templates` | 12 lines of keyword-matching heuristic (`?`/`what`/`how` → "Answer concisely", `create`/`make`/`build` → step-by-step). The bigger gap: `template_id` doesn't actually look up a template from the registry — it just tags the prompt. Extracting to `PromptStrategy` enum would make heuristic testable/reusable |
| **12** | 12c | Inject `InferencePort` via configured port in CLI | Same pattern as 12b but 3+ call sites: `chat_with_agent()`, `ensemble_improv_turn()`, ensemble commands. Each reconstructs `OkapiInference::new()` per turn. Need to construct once in `run_chat()` or REPL's `run()` and pass through |

### What Remains — P3 (Open Questions)

| Task | Question | Decision Needed | Notes |
|------|----------|-----------------|-------|
| 13a | ACP Transport — what does it look like when not a child process? | Architecture | Current ACP is JSON-RPC 2.0 over stdio (child process). For networked agents or in-process, need transport abstraction |
| 13b | `CyberneticsToken`/`CurationToken` never consumed at runtime — speculative or future? | Design | Tokens are now minted at loop construction (9b/9c) but not yet presented to capability gates. The OCAP authority chain exists structurally but is not yet enforced at runtime |
| 13c | `AgentKind` behavioral dispatch — type-level or cosmetic? | Architecture | If type-level: trait with associated types. If cosmetic: keep as-is. Affects pod composition and template selection |
| 13d | Episodic encryption boundary — separate keys for episodic vs semantic? | Security | Currently same master key. Episodic (private) and semantic (shared) have different threat models. Separate keys would add defense-in-depth |
| 13e | Loop membrane persistence — state lost on crash, acceptable? | Infrastructure | Loop inboxes and variety counters are in-memory. On crash, all pending directives are lost. Need WAL or checkpoint mechanism? |
| 13f | Semantic Loop has no MCP server — gap or intentional? | Architecture | 14 of 15 MCP servers exist; Semantic Memory (Loop 2b) has no direct tool surface. Episodic and semantic queries go through `hkask-mcp-cns` or `hkask-mcp-registry`. Is this sufficient? |
| 13g | `hkask-mcp-web` search orchestration — extract to `hkask-memory` or new crate? | Architecture | Deferred from 8g. No current consumer. Build when the need arises |
| 13h | Set-point YAML configuration — how to configure at deploy time? | Infrastructure | CNS thresholds, gas budgets, variety set-points are currently hardcoded. Need YAML/env configuration for deploy-time tuning |

### Important Notes for Continuation

1. **Two uncommitted files** from Task 12b: `crates/hkask-api/src/lib.rs` (added `inference_port` field + extraction) and `crates/hkask-api/src/routes/chat.rs` (uses `state.inference_port` with `generate_with_model()`). These should be committed before continuing.

2. **Pre-existing dead code warnings** in `hkask-ensemble`: `MetacognitionConfig::config()` (unused), `DeliberationParticipant::{add_participant, status, complete, cancel}` (unused), `DeliberationSession` fields `webid`/`name` (unread). These are not caused by our changes.

3. **Pre-existing dead code warnings** in `hkask-cns`: `VarietyTracker::{total, entropy}` (unused `pub(crate)` methods), `CircuitBreakerPort` methods (`new`, `allow_request`, `record_success`, `record_failure`, `state`, `set_state` — trait defined in `hkask-types` but only `OkapiInference` implements it conditionally).

4. **`hkask-mcp-ensemble`** was created in a prior distillation commit (34a8534d..e3e6c144). It exists as a Cargo.toml + main.rs but is NOT a workspace member — it's not listed in the root `Cargo.toml`. This server needs to be either added to workspace members or removed.

5. **`TemplateType::Specification`** was added to the code (6b), but no templates of type `Specification` are bootstrapped in `Registry::bootstrap()`. This means the type exists in the enum but has no entries. To make it functional, add `TemplateEntry::new(...)` calls with `TemplateType::Specification` for the DDMVSS spec templates (spec/goal-capture, spec/curate/evaluate, etc.).

6. **The `ConsolidationPort::consolidation_candidate_count()` method** (7d) requires a `&WebID` parameter for perspective, but `CurationLoop::sense()` currently passes the curator's ID. Verify this is the correct perspective for counting consolidation candidates — the consolidation protocol specifies the Curator's perspective.

7. **`CuratorContext::with_acp()` builder** (7f) exists but is never called at any construction site. The ACP port is currently always `None`, meaning `direct_bot` will always log the "ACP port not configured" warning. To make this functional, thread an `AcpPort` implementation through `CuratorContext` construction at bootstrap time (in `hkask-cli` and `hkask-api`).

8. **`ApiState.inference_port`** (12b) is only populated when `ensemble_inferencer` is provided. In the default `ApiState::new()` path (e.g., when no ensemble is configured), `inference_port` is `None` and the chat route falls back to per-request `OkapiInference::new`. This is correct backward-compatible behavior but means the shared-port optimization only activates when the ensemble is configured.

---

*ℏKask Loop Distillation — P0+P1+P2 Complete — v0.22.0*