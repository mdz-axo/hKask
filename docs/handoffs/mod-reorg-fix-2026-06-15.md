# Handoff — Module Reorganization Import Fix

**Date:** 2026-06-15  
**Scope:** Fix workspace-wide import breakage from `hkask-types` module re-organization  
**Completion:** All imports fixed. `cargo check --workspace` and `cargo check --workspace --tests` pass cleanly. Test suite NOT yet verified (run timed out).

---

## 1. Session Context

This session began by receiving the prior handoff (`test-contract-db-2026-06-15.md`) which claimed: 83 test suites, 0 failures, workspace clean. On verification, `cargo check --workspace` failed with ~40 unresolved import errors across 20+ files and 10+ crates. A `hkask-types` module re-organization had moved many types into sub-modules (`template::`, `secret::`, `lexicon::`, `bundle::`, `r7::`, `identity::`, `sovereignty::`, `visibility::`, `time::`, `text::`) with only a selective subset re-exported at root level (≥3 downstream crates rule). The prior session's work was done before or during this re-org, leaving ~20 files with broken import paths.

The entire session was spent fixing these import errors. All compilation errors are resolved. The test suite was not fully verified — `cargo test --workspace` was started but timed out before completion.

---

## 2. What Was Done

### 2.1 Import Path Fixes (20+ files, 10+ crates)

Types were moved from `hkask_types::TypeName` into sub-modules. The following patterns were fixed:

| Old Path | New Path | Files Affected |
|----------|----------|----------------|
| `hkask_types::LLMParameters` | `hkask_types::template::LLMParameters` | chat.rs, inference_router.rs, ollama_backend.rs, deepinfra_backend.rs, fal_backend.rs, together_backend.rs, chat_protocol.rs, executor.rs, mcp-servers/condenser, mcp-servers/docproc, mcp-servers/training |
| `hkask_types::SecretRef` | `hkask_types::secret::SecretRef` | keychain.rs, config.rs |
| `hkask_types::derivation_contexts` | `hkask_types::secret::derivation_contexts` | keychain.rs, master_key.rs |
| `hkask_types::now_rfc3339` | `hkask_types::time::now_rfc3339` | onboarding.rs, mcp-servers: companies, condenser, docproc, media, memory, research, replica, spec, training |
| `hkask_types::DataCategory` | `hkask_types::sovereignty::DataCategory` | context.rs (agents), mod.rs (pod), sovereignty.rs |
| `hkask_types::UserSovereigntyState` | `hkask_types::sovereignty::UserSovereigntyState` | sovereignty.rs |
| `hkask_types::AccessControl` | `hkask_types::visibility::AccessControl` | memory_storage.rs |
| `hkask_types::HLexicon` | `hkask_types::lexicon::HLexicon` | contract_validator.rs, lexicon.rs, registry.rs, registry_sqlite.rs |
| `hkask_types::LexiconTerm` | `hkask_types::lexicon::LexiconTerm` | lexicon.rs, contract_validator.rs (test) |
| `hkask_types::TemplateType` | `hkask_types::lexicon::TemplateType` | lexicon.rs, registry.rs, registry_sqlite.rs, skill_loader.rs, contract_validator.rs (test) |
| `hkask_types::SkillPolarity` | `hkask_types::bundle::SkillPolarity` | registry_sqlite.rs, lib.rs (templates) |
| `hkask_types::BundleManifestStep` | `hkask_types::bundle::BundleManifestStep` | executor.rs |
| `hkask_types::TemplateCrate` | `hkask_types::template::TemplateCrate` | mod.rs (pod), mod.rs (git_cas) |
| `hkask_types::TemplateFile` | `hkask_types::template::TemplateFile` | mod.rs (pod), mod.rs (git_cas) |
| `hkask_types::R7BotIdentity` | `hkask_types::r7::R7BotIdentity` | bootstrap.rs |
| `hkask_types::default_r7_bots` | `hkask_types::r7::default_r7_bots` | bootstrap.rs |
| `hkask_types::RegistrationRequest` | `hkask_types::identity::RegistrationRequest` | user.rs |
| `hkask_types::ReplicantIdentity` | `hkask_types::identity::ReplicantIdentity` | user.rs |
| `hkask_types::UserSession` | `hkask_types::identity::UserSession` | user.rs |
| `hkask_types::ZeroizingSecret` | `hkask_types::secret::ZeroizingSecret` | (checked, no usage in workspace) |
| `hkask_types::blake3_hash` | `hkask_types::text::blake3_hash` | recall_dedup.rs |

### 2.2 Test-Specific Fixes

- **`crates/hkask-types/src/capability/mod.rs`**: Added `use crate::capability::token_types::AttenuationLevel;` in the `proptest_tests` module (lines 254, 266). `AttenuationLevel` is `pub(crate)` in `token_types` and was not reachable via `use super::*;`.
- **`crates/hkask-templates/src/contract_validator.rs`**: Test module was importing `LexiconTerm` and `TemplateType` from root — changed to `hkask_types::lexicon::{...}`.

### 2.3 Build Verification

| Command | Result |
|---------|--------|
| `cargo check --workspace` | ✅ Pass (16 pre-existing warnings in hkask-types) |
| `cargo check --workspace --tests` | ✅ Pass (zero errors) |
| `cargo test --workspace` | ⚠️ Not verified (timed out; test compilation clean) |

---

## 3. What Remains

### HIGH — Verify Full Test Suite

- **What**: Run `cargo test --workspace` and confirm all 83 test suites pass with 0 failures
- **Command**: `cargo test --workspace` (expect 15+ minute runtime for full suite including proptests)
- **If failures**: Investigate and fix. The test code compiles cleanly, so failures would be runtime-only.
- **Quick smoke test first**: `cargo test -p hkask-wallet -- balance_conservation && cargo test -p hkask-agents --test agent_pod_integration && cargo test -p hkask-services --test cli_to_storage_integration`

### MEDIUM — Audit Warnings

- **What**: 16 pre-existing warnings in `hkask-types` (private interfaces, dead code in bundle/cascade, bundle/composition, bundle/manifest, capability/token_types, ocr/cns)
- **Files**: `crates/hkask-types/src/bundle/{cascade,composition,manifest}.rs`, `crates/hkask-types/src/capability/token_types.rs`, `crates/hkask-types/src/ocr/cns.rs`
- **These are NOT blocking** — they're architectural decisions about pub(crate) types in public structs. Not this session's scope.

### LOW — Prior Handoff Tasks (from test-contract-db-2026-06-15.md)

All LOW priority tasks from the prior handoff remain deferred:
- L2: MCP tool invocation (call_tool path) — blocked on `Peer<RoleClient>` trait
- L3: MCP tool schema contract (detailed error messages) — jsonschema 0.28 API adaptation
- L4: Agents↔Inference improv interaction — MockInferencePort exists, wiring needed

### Contract Migration Continuation

From prior handoff, unchanged:
- Phase A1 Expand: Add contracts to remaining 14 crates (613 contracted / 1,551 pub fns = 39.5%)
- Phase A2 Complete: Target 100% contract coverage
- Phase B2 Proposal: Agent contract generation workflow

---

## 4. Recommended Skills and Tools

### Skills to Load (in order)

1. **`condenser-continuation`** — Restores session state from both this handoff and `test-contract-db-2026-06-15.md`
2. **`coding-guidelines`** — For any new code changes
3. **`tdd`** — If test failures need fixing or new tests needed
4. **`pragmatics`** — If architectural questions arise about the module re-organization

### Key Commands

```bash
# Verify workspace health (should pass cleanly)
cargo check --workspace
cargo check --workspace --tests

# Full test suite (long — use generous timeout)
cargo test --workspace

# Quick smoke test of key integration suites
cargo test -p hkask-wallet -- balance_conservation
cargo test -p hkask-agents --test agent_pod_integration
cargo test -p hkask-services --test cli_to_storage_integration
cargo test -p hkask-inference --test inference_routing_integration
cargo test -p hkask-mcp --test mcp_lifecycle_integration
cargo test -p hkask-cns -- contract_discipline
cargo test -p hkask-test-harness -- mocks

# Contract coverage audit
scripts/contract-audit.sh --summary

# Prohibition sweep
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/ --include="*.rs"
```

---

## 5. Key Decisions to Preserve

1. **Root re-exports follow ≥3 downstream crates rule.** `hkask-types/src/lib.rs` re-exports only types used by ≥3 downstream crates (G2 justification). Less-commonly-used types must be imported via sub-module paths (e.g., `hkask_types::sovereignty::DataCategory`). Do NOT add bulk re-exports to the root without verifying the ≥3 rule.

2. **All prior handoff decisions (from test-contract-db-2026-06-15.md) remain valid.** The 10 key decisions listed there (shared DB connection, daemon socket guard, MockInferencePort prefix matching, proptest shrinking disabled, contract format, jsonschema v0.28 API, DelegationToken API, LLMParameters adapter field, etc.) are unchanged — this session only fixed import paths, not behavior.

3. **`AttenuationLevel` is `pub(crate)` in `token_types`.** It cannot be imported from outside the `hkask-types` crate. Tests within the crate must use `use crate::capability::token_types::AttenuationLevel;`. Do not make it `pub` without a deliberate visibility review.

4. **Test compilation is verified clean, NOT test execution.** The session ran out of time before the full test suite could execute. The next session's first task should be verifying test execution.

---

*ℏKask — A Minimal Viable Container for Agents — v0.27.0*
