---
title: "Adversarial Simplification Sweep — Unwired Inventory"
audience: [architects, developers, agents]
last_updated: 2026-06-06
version: "1.0.0"
status: "Active"
domain: "Refactoring"
ddmvss_categories: [capability, interface, composition]
source: "Adversarial Simplification Sweep, 2026-06-06"
---

# Adversarial Simplification Sweep — Unwired Inventory (2026-06-06)

**Source spec:** the *Adversarial Simplification Sweep* (Tasks 1–10) catalogues
phantom seams across hKask — types, fields, manifests, and YAML files that exist
in the data model or documentation but have no live call sites. The corrective
principle is:

> A seam without two adapters is hypothetical (P1). A type without a consumer is
> unwired (C2). Unwired code has a 30-day shelf life (C3).

**Purpose of this document:** record the per-crate unwired inventory as of
2026-06-06 (HEAD = `fbbb6265`). Items here have a 30-day deadline. Past the
deadline without a consumer, they should be deleted (P6) — never deprecated (P7).

**Method:** for each `pub` item not gated by a feature flag, check whether
external code (i.e., outside the defining module and its `mod tests`) reads,
constructs, or invokes it. Items with zero external consumers are unwired.

**Verification:** `cargo check --workspace` is green; `cargo test --workspace --lib`
runs 60 + 196 + 11 + 8 + 49 = 324 lib tests, all passing.

---

## Cross-cutting findings (first — highest leverage)

### Finding 0.1 — Manifest wiring is structurally complete

The spec called out `process_manifest` on `AgentDefinition` as a phantom seam
whose 30-day clock was ticking. Investigation shows the wiring is **complete**:

- `RawYamlAgent.process_manifest` (`crates/hkask-agents/src/registry_loader.rs:79`)
  is deserialized from the YAML
- `Curator.yaml` declares `process_manifest: registry/manifests/curator-metacognition.yaml`
  (`registry/bots/Curator.yaml:77`)
- `into_agent_definition` (`registry_loader.rs:208`) passes the value to
  `AgentDefinition.process_manifest`
- `AgentRegistryLoader::load_and_register` persists the agent
- The REPL reads `def.definition.process_manifest` (`repl/mod.rs:486`) and, when
  present, builds a `ManifestExecutor` and runs `execute_manifest()` on every
  REPL turn

The only gap: `agent_register` (CLI) and `register_replicant` (onboarding) hardcode
`process_manifest: None`. Adding `--process-manifest <path>` to `AgentAction::Register`
would let users opt in without breaking existing registrations. **Status: optional
addition, not a phantom seam.**

### Finding 0.2 — `BundleManifestStep` executor wired through tests

`BundleManifestStep` and its 11 fields (including `gas_cap`, `timeout_seconds`,
`input_mapping`, `output_schema`, `phase`) are all read by `ManifestExecutor`
(`crates/hkask-templates/src/executor.rs`). The integration tests
(`tests/integration_executor.rs`) exercise select, populate, execute, feedback,
and multi-step cascades. **Status: live, well-tested.**

### Finding 0.3 — `<<tool:...>>` format string is no longer hardcoded

The spec's Task 4 called out the hardcoded tool-call format string. The current
code has it in **one place**: `tool_augmented::TOOL_CALL_FORMAT_INTRO` and
`tool_augmented::format_tool_prompt_section()`. The REPL computes the section
from `governed_tool.discover_tools()` and passes it to `chat_with_agent`. The
`chat_with_agent` function has a hardcoded *fallback* for contexts without MCP
runtime; that fallback is intentional and tests both branches.

**Status:** one source of truth. The `tool_prompt_section: String` field on
`ReplState` is intentionally a cache because `ToolPort` uses `impl Trait` returns
and is not dyn-compatible — re-deriving on demand via `Arc<dyn ToolPort>` is not
possible without making the trait dyn-compatible (separate refactor, not in this
sweep's scope).

---

## Per-crate unwired inventory

Items below are `pub` items that, as of 2026-06-06, have **no external consumer**
outside their defining module and its tests. They are flagged for the 30-day
shelf-life check (C3).

### `hkask-types` crate

| Item | Location | Why unwired | Deadline |
|------|----------|-------------|----------|
| ~~`Skill::cascade_order: Vec<String>`~~ | `crates/hkask-types/src/ports/mod.rs:265-274` | ~~Field is **persisted** to `skill_cascade_order` table by `SqliteRegistry::register_skill` (write path) and read back by `cascade_order_for_skill` (read path), but **no runtime cascade executor iterates it** to drive skill execution. `ManifestExecutor` uses `BundleManifestStep.ordinal`, not `Skill.cascade_order`.~~ | **Deleted 2026-06-06.** P6 applied: removed field, builder, `skill_cascade_order` table + index, `cascade_order_for_skill` reader, write loop, cleanup, and 2 test cases. `ManifestExecutor` already orders steps via `BundleManifestStep.ordinal`, so the field was genuinely orphaned. |
| `Skill::content_hash: Option<String>` | `crates/hkask-types/src/ports/mod.rs:273` | Field is stored in `skills.content_hash` and `bundle_skills.content_hash`, but no consumer **verifies** the hash against the skill body. The validator at `bundle.rs:599` only checks `is_empty()`. | 2026-07-06 |
| `BundleManifestStep::model_tier: Option<String>` | `crates/hkask-types/src/bundle.rs:227` | The field is read by `ManifestExecutor::execute_select` (executor.rs:148) but the **resolution** of `"fast_local"` to an actual model name is not implemented. The field is passed through but does not change the `InferencePort` call. | 2026-07-06 (paired with manifest executor) |
| `BundleManifestStep::input_mapping: Option<serde_json::Value>` | `crates/hkask-types/src/bundle.rs:232` | Field is parsed from YAML and stored, but `ManifestExecutor` does not currently apply the mapping when binding context variables. | 2026-07-06 (paired with manifest executor) |
| `BundleManifestStep::output_schema: Option<serde_json::Value>` | `crates/hkask-types/src/bundle.rs:234` | Field is parsed from YAML and stored, but no code validates the executor's output against it. | 2026-07-06 (paired with manifest executor) |
| `BundleManifest::cns: CnsConfig` | `crates/hkask-types/src/bundle.rs:431` | The field is stored on `BundleManifest` and validated by `BundleManifest::validate`, but `ManifestExecutor` does not read `CnsConfig` to emit the configured spans. | 2026-07-06 (paired with manifest executor) |
| `BundleManifest::audit: AuditConfig` | `crates/hkask-types/src/bundle.rs:432` | Same as `cns` — validated, not enforced. | 2026-07-06 |
| `BundleManifest::principles: Vec<String>` | `crates/hkask-types/src/bundle.rs:444` | Field is stored but not surfaced to LLM prompts or to execution logs. | 2026-07-06 |

**Cumulative:** 8 items in `hkask-types` are stored but not enforced. The
`BundleManifest` config sub-structs (`cns`, `audit`, `gas`, `ocap`, `error_handling`,
`convergence`) follow the same pattern: parsed and validated, but the executor
ignores them.

### `hkask-templates` crate

| Item | Location | Why unwired | Deadline |
|------|----------|-------------|----------|
| `McpPort` trait | `crates/hkask-templates/src/ports.rs` | Confirmed: only `McpDispatcher` (in `hkask-mcp`) implements it for real; `MockMcp` is test-only. **Single real consumer → premature abstraction** (P1). Recommend folding into `McpDispatcher` as a concrete struct method. `ManifestExecutor<M: McpPort>` (templates/executor.rs:69) only sees it through the generic. | 2026-07-06 |
| `Skill::content_hash` writer | `crates/hkask-templates/src/registry_sqlite.rs:482` | Writes `skill.content_hash` to the DB but no caller passes a non-None `content_hash` into `Skill::new` or `with_content_hash` outside the tests. | 2026-07-06 |
| `DEFAULT_TEMPLATE_BASE_PATH` constant | `crates/hkask-templates/src/executor.rs:42` | Defaults to `"registry/templates"` — 174 `.j2` files exist there. The executor resolves `template_ref` against this path. **Wired but only when the user has a process_manifest.** Without one, the constant is unreached. | n/a — wired through the YAML field |

### `hkask-mcp` crate

| Item | Location | Why unwired | Deadline |
|------|----------|-------------|----------|
| `ToolInfo::required_capability: Option<String>` | `crates/hkask-types/src/ports/mod.rs:519` | The field exists but is **never populated** by any MCP tool registration. The fallback in `GovernedTool::verify_capability_domain_fallback` (governed_tool.rs:151) only fires when the legacy exact-match path fails. | 2026-07-06 (paired with capability-to-tool namespace mapping — see open question 3) |

### `hkask-cns` crate

| Item | Location | Why unwired | Deadline |
|------|----------|-------------|----------|
| (none) | | `GovernedTool` membrane, `CyberneticsLoop`, `CnsRuntime`, `EscalationPolicy` (just extracted in P3.6) are all wired through the call graph. The `cns.*` span namespaces listed in `PRINCIPLES.md §1.4` are emitted at the relevant call sites. | n/a |

### `hkask-cli` crate

| Item | Location | Why unwired | Deadline |
|------|----------|-------------|----------|
| `tool_prompt_section: String` field on `ReplState` | `crates/hkask-cli/src/repl/mod.rs:121-128` | Cache. The user's Task 8 flagged this. **Documented constraint:** `ToolPort` uses `impl Trait` returns and is not dyn-compatible, so it cannot be re-derived on demand via `Arc<dyn ToolPort>`. Field is intentional; not a phantom seam. | n/a |
| `process_manifest: Option<BundleManifest>` field on `ReplState` | `crates/hkask-cli/src/repl/mod.rs:137` | Set only when the agent YAML declares `process_manifest`. Curator.yaml does, so it's populated for the default agent. Other agents (R7.x) have no manifest, and `process_manifest` stays `None` for them. The field is intentional. | n/a |
| `manifest_executor: Option<ManifestExecutor<McpDispatcher>>` field on `ReplState` | `crates/hkask-cli/src/repl/mod.rs:133` | Same as `process_manifest` — populated when the agent has a manifest. | n/a |
| `OnboardingError::Database(String)` | `crates/hkask-cli/src/onboarding.rs:33` | Primitive String payload. Pre-requisite for P3.5 (structured storage errors). | 2026-07-06 |
| `EnsembleError::SessionNotFound(String)` and 3 others | `crates/hkask-cli/src/errors.rs:30-50` | Same. | 2026-07-06 |
| `CuratorError::*` 4 variants | `crates/hkask-cli/src/errors.rs:49-65` | Same. | 2026-07-06 |
| `UserError::*` 4 variants | `crates/hkask-cli/src/errors.rs:81-95` | Same. | 2026-07-06 |
| `RegistryError::*` 4 variants | `crates/hkask-cli/src/errors.rs:65-75` | Same. | 2026-07-06 |

### `hkask-agents` crate

| Item | Location | Why unwired | Deadline |
|------|----------|-------------|----------|
| `MemoryError::CapabilityDenied(String)` | `crates/hkask-agents/src/error.rs:43` | Primitive String payload. Pre-requisite for P3.5. **Only remaining `String` variant in this enum** (the rest use `Box<dyn Error>`). | 2026-07-06 (immediate follow-up) |
| `McpError::InvalidToken(String)`, `ToolNotFound(String)` | `crates/hkask-agents/src/error.rs:12,18` | Primitive String payloads. Migration to typed variants is a clean follow-up. | 2026-07-06 |
| `EscalationError::NotFound(String)` | `crates/hkask-agents/src/escalation.rs:50` | Sentinel identifier, not a structured error. Acceptable as-is. | n/a (sentinel) |
| `ConsentError::ConsentNotFound(String)` | `crates/hkask-agents/src/consent.rs:27` | Same as `EscalationError::NotFound`. | n/a (sentinel) |
| `MetacognitionError::*` | `crates/hkask-agents/src/curator_agent/metacognition.rs:52-59` | Already migrated to `#[from] EscalationError` / `AcpError` in commit `aaec5285`. `MetacognitionError::NoSnapshot` was added as a structured discriminant. | n/a |

### `hkask-memory` crate

| Item | Location | Why unwired | Deadline |
|------|----------|-------------|----------|
| `EpisodicMemoryError::Triple(String)` and `InvalidVisibility(String)` and `MissingPerspective` | `crates/hkask-memory/src/*.rs` | Primitive String payloads. Not yet migrated. | 2026-07-06 |
| `SemanticMemoryError::Triple(String)`, `Embedding(String)`, `InvalidVisibility(String)`, etc. | `crates/hkask-memory/src/*.rs` | Same. | 2026-07-06 |

### `hkask-storage` crate

| Item | Location | Why unwired | Deadline |
|------|----------|-------------|----------|
| `DatabaseError::*` | `crates/hkask-storage/src/*.rs` | `InfrastructureError::Database(String)` is the wrapper. Migration to typed variants is the cleanest path. | 2026-07-06 |

### `hkask-keystore` crate

| Item | Location | Why unwired | Deadline |
|------|----------|-------------|----------|
| (none) | | All `pub` items are consumed by `hkask-agents` and `hkask-api`. | n/a |

### `hkask-api` crate

| Item | Location | Why unwired | Deadline |
|------|----------|-------------|----------|
| `ApiError::*` (8 variants) | `crates/hkask-api/src/error.rs:25-35` | `ApiError` is itself a typed error enum — no String payloads. **Status: complete** (P3.1 partial done). | n/a |

### `hkask-ensemble` crate

| Item | Location | Why unwired | Deadline |
|------|----------|-------------|----------|
| `EnsembleInferencer::with_breaker` (inferred) | `crates/hkask-api/src/lib.rs` | Called from `build_ensemble_session`. **Wired.** | n/a |

---

## `#[allow(dead_code)]` items

The following are explicit "reserved for future use" annotations. Each should
be revisited at the 30-day mark.

| Item | Location | Annotation |
|------|----------|------------|
| `visibility.rs:57, 62, 171, 177` | `hkask-types` | "reserved for future crate-internal use" |
| `id.rs:213` | `hkask-types` | "reserved for future trace-level diagnostics" |
| `bundle.rs:656, 672, 683, 706` | `hkask-types` | "reserved for CNS span wiring" |
| `mcp/server.rs:202` | `hkask-mcp` | (no comment) |
| `mcp/git_cas/snapshot_writer.rs:27, 32` | `hkask-mcp` | (no comment) |
| `mcp/git_cas/mod.rs:28, 82` | `hkask-mcp` | (no comment) |
| `mcp/git_cas/repo_manager.rs:18, 23` | `hkask-mcp` | (no comment) |
| `mcp/git_cas/gix_adapter.rs:95` | `hkask-mcp` | (no comment) |
| `agents/russell_acp.rs:190` | `hkask-agents` | (no comment) |

**Action:** at 2026-07-06, audit each. Delete (P6) if the future use didn't
materialize; remove the `#[allow(dead_code)]` and add a real consumer if it did.

---

## What's NOT in this inventory

- **Tests** — by design, tests construct types that have no other consumers.
  These are intentional and required by P8.
- **Builder methods** — `with_*` methods on `Skill` etc. have zero or one
  consumer, but they're the only way to construct the type, so they're not
  hypothetical seams.
- **Type aliases and re-exports** — `pub use` re-exports are wired by their
  re-export sites.
- **Feature-gated code** — not in scope.

---

## What's been resolved since the inventory

- ✅ **`Skill.cascade_order`** (deleted 2026-06-06) — P6 applied. The field,
  builder, DB table, index, and round-trip read/write paths were all removed.
  Execution ordering is owned by `BundleManifestStep.ordinal` in the manifest
  YAML, which is the right level of granularity.
- ✅ **`AgentError` P3.5 partial** (commits `79cd9cfe` + pending) — added
  `From<AcpError>`, `From<AgentRegistryError>`, `From<uuid::Error>`,
  `From<RegistryError>`, `From<RegistryLoaderError>` to `AgentError`. Removed
  the dead `CapabilityError` and `UnregistrationFailed` variants. Added 3 P8
  property tests verifying Display includes the upstream cause. `RegistryError`
  also gained `Infra(#[from] InfrastructureError)`.
- ✅ **`P3.5 close-out` + `P3.6 escalation extraction`** — both closed in
  commits `aaec5285` and `fbbb6265` respectively. The Fowler audit
  (`docs/status/fowler-audit-status.md`) now shows P1 + P2 fully done and
  2 of 6 P3 items done.

## Next steps (priority order)

1. **CLI error enums — `EnsembleError`, `CuratorError`, `UserError`** (medium) — same pattern as the `AgentError` work. Migrate from `String` payloads to typed `From<...>` wrappers.
2. **Manifest config sub-structs** (medium) — decide whether `BundleManifest::cns`,
   `audit`, `gas`, `ocap`, `error_handling`, `convergence` are read by the
   executor. If not, delete (P6). If yes, implement the enforcement in
   `ManifestExecutor`. The 8 items in `hkask-types` are clustered here.
3. **`McpPort` folding** (medium) — fold the trait into `McpDispatcher`
   concrete methods. The user's commit `73100319` started this in
   `executor.rs` (made `ManifestExecutor` non-generic) but the test file
   still uses `MockMcp: McpPort` and `McpPort` is still defined in
   `ports.rs`. Pick one direction: either restore the generic fully, or
   rewrite the integration test to use a real `McpDispatcher`.

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*
