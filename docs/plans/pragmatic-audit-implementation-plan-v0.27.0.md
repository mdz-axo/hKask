---
title: "Pragmatic Audit Implementation Plan"
audience: [engineers, architects]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle, curation]
---

# hKask v0.27.0 — Pragmatic Audit Implementation Plan

**Status:** Complete ✅ — All 10 tasks done. R1–R4: test infrastructure + API REQ tags. R5: CnsSpan enum (51 variants, core crates migrated). R6: Ed25519 tokens (immediate cutover, all callers updated). R7: provenance markers (54→0). R8: surface audit + G2 justifications (reorganization deferred to v0.28.0). R9: KataEngine::from_env() + SpecService::get_full() (CLI no longer imports storage/inference directly). R10: training cancel already fully implemented (PID tracking + API endpoints for all 5 providers). Total REQ tags: 846.
**Owner:** Engineering  
**Created:** 2026-06-15  
**Last Updated:** 2026-06-15  
**Scope:** `crates/*` + `mcp-servers/*` (headless-only, no UI additions)  
**Source Audit:** Pragmatic Codebase Audit — 7 tasks + Ω open questions  
**Principles:** P4 (Clear Boundaries), P5 (Essentialism), P8 (Semantic Grounding), P9 (Homeostatic Self-Regulation), C8 (Test Depth)

---

## 1) Objective

Implement the 10 priority recommendations from the pragmatic codebase audit as small, verifiable PR slices. Each task is grounded in a stated behavioral property of a public seam (P8) and constrained by the coding-guidelines four-rule discipline (Think Before Coding, Simplicity First, Surgical Changes, Goal-Driven Execution).

---

## 2) Guiding Constraints (enforced throughout)

- **Think Before Coding:** Each task starts with explicit assumption + expected observable outcome.
- **Simplicity First:** No speculative framework work; only task-linked changes.
- **Surgical Changes:** Touch only target seams for each task.
- **Goal-Driven Execution:** Every PR has measurable acceptance checks.

Project constraints preserved:
- Headless only. No visual UI, Grafana, dashboards.
- No monitoring stack additions.
- No `todo!()`, `unimplemented!()`, or deprecated surface introduced.
- Every `#[test]` verifies a stated behavioral property (P8).

---

## 3) Priority Stack (by enforcement level)

### 🔴 Prohibitions (Must Fix — P4/P8/P12 violations)

| # | Recommendation | Principle | Severity |
|---|---------------|-----------|----------|
| R1 | Add tests to `hkask-communication` | P12, P8, C8 | Critical — 0 tests for infrastructure |
| R2 | Add tests to `hkask-agents` | P8, C8 | Critical — 8 tests for 77-seam deep module |
| R3 | Add tests to `hkask-mcp` | P4, P8, C8 | Critical — 5 tests for security-critical dispatch |

### 🟡 Guardrails (Should Fix — P4/P8/P5 vulnerabilities)

| # | Recommendation | Principle | Severity |
|---|---------------|-----------|----------|
| R4 | Add REQ tags to `hkask-api` route handlers | P8, C8 | Guardrail — 127 seams, 1 REQ tag |
| R5 | Type CNS spans as enum variants | P8, P9, Hoare | Guardrail — stringly-typed spans |
| R6 | Upgrade `DelegationToken` from HMAC to Ed25519 | P4, Hoare | Guardrail — symmetric vs. asymmetric |
| R7 | Add provenance markers to OUGHT-as-IS doc claims | P8 | Guardrail — 166 unclassified claims |

### 🟢 Guidelines (Should Consider — P5/P7/C8 improvements)

| # | Recommendation | Principle | Severity |
|---|---------------|-----------|----------|
| R8 | Reduce surface on `hkask-types` | P5, C8 | Guideline — 231 public items |
| R9 | Continue strangler fig extraction | P7 | Guideline — 7 mid-migration domains |
| R10 | Resolve 3 training-cancel soft stubs | P5 | Guideline — graceful no-ops |

---

## 4) Delivery Strategy (6 waves)

---

## Wave 1 — Critical Test Infrastructure (R1, R2, R3)

> **Goal:** Eliminate zero-test crates and critical depth mismatches.

### Task R1 — `hkask-communication` Tests (0 → ~10)

**Assumption:** Matrix transport has zero tests because it was recently extracted from `hkask-agents`. The core transport, room state, and 7R7 listener protocol are unverified.  
**Expected outcome:** `cargo test -p hkask-communication` passes ≥10 tests covering transport invariants.

**PR slices**

- **PR R1.1:** Add test module structure and Matrix connection lifecycle tests.
  - File: `crates/hkask-communication/src/lib.rs` (add `#[cfg(test)] mod tests;`)
  - Add `crates/hkask-communication/tests/integration.rs`
  - Tests:
    - `// REQ: Communication connection lifecycle starts disconnected`
    - `// REQ: Communication connection send returns error when disconnected`
    - `// REQ: Communication room state initializes empty`
    - `// REQ: Communication room join adds room to state`

- **PR R1.2:** Add 7R7 listener and message encoding tests.
  - Tests:
    - `// REQ: R7 listener processes well-formed messages`
    - `// REQ: R7 listener rejects malformed messages`
    - `// REQ: Message encoding round-trips without data loss`
    - `// REQ: Message encoding rejects invalid UTF-8`
    - `// REQ: Room state tracks member list correctly`
    - `// REQ: Agent registry lookup returns None for unknown agents`

**Acceptance criteria**
- `cargo test -p hkask-communication` passes ≥10 tests.
- All tests carry `// REQ:` tags with behavioral property descriptions.
- Zero `todo!()` or `unimplemented!()` in test code.

**Validation**
```bash
cargo test -p hkask-communication
cargo clippy -p hkask-communication -- -D warnings
```

---

### Task R2 — `hkask-agents` Tests (8 → ~20)

**Assumption:** The agents crate (77 public seams, depth 77.8) is the behavioral heart of the system but has only 8 tests covering pod creation and persona filtering. Consent flows, ACP orchestration, and curation loop state transitions are untested.  
**Expected outcome:** `cargo test -p hkask-agents` passes ≥20 tests covering ACP, consent, and curation invariants.

**PR slices**

- **PR R2.1:** Add ACP runtime tests.
  - Requires `HKASK_MASTER_KEY` env var (see `set_test_master_key()` pattern).
  - Tests:
    - `// REQ: ACP rejects wildcard capability "*"`
    - `// REQ: ACP registers agent and returns delegation token`
    - `// REQ: ACP revokes token and denies subsequent access`
    - `// REQ: DelegationToken attenuation is enforced`

- **PR R2.2:** Add consent flow and curation loop tests.
  - Tests:
    - `// REQ: Consent request records affirmative consent`
    - `// REQ: Consent request records denial`
    - `// REQ: Curator ranks tool selection by relevance`
    - `// REQ: Pod lifecycle transitions: Inactive → Active → Inactive`
    - `// REQ: Pod manager rejects activation of unknown pod`
    - `// REQ: Agent persona validation rejects empty name`
    - `// REQ: Root authority creates delegation tokens for all requested capabilities`
    - `// REQ: ACP restored from storage has same capabilities`

**Acceptance criteria**
- `cargo test -p hkask-agents` passes ≥20 tests.
- All tests carry `// REQ:` tags.
- Zero runtime panics in test code.

**Validation**
```bash
cargo test -p hkask-agents
cargo clippy -p hkask-agents -- -D warnings
```

---

### Task R3 — `hkask-mcp` Tests (5 → ~15)

**Assumption:** MCP dispatch is security-critical (OCAP Gate-3 enforcement point). Only 5 async tests exist, none covering tool routing, dynamic discovery, or error propagation.  
**Expected outcome:** `cargo test -p hkask-mcp` passes ≥15 tests covering dispatch, discovery, and capability enforcement invariants.

**PR slices**

- **PR R3.1:** Add tool dispatch routing tests.
  - Tests:
    - `// REQ: MCP dispatch routes tool call to correct server`
    - `// REQ: MCP dispatch returns error for unknown tool`
    - `// REQ: MCP dispatch enforces capability token before invocation`
    - `// REQ: MCP dispatch rejects expired delegation token`
    - `// REQ: GovernedTool gates invocation on energy budget`

- **PR R3.2:** Add dynamic discovery and error propagation tests.
  - Tests:
    - `// REQ: MCP discovery lists all available tools`
    - `// REQ: MCP discovery returns empty list when no servers connected`
    - `// REQ: MCP dispatch propagates server error to caller`
    - `// REQ: MCP dispatch handles server timeout gracefully`

**Acceptance criteria**
- `cargo test -p hkask-mcp` passes ≥15 tests.
- All tests carry `// REQ:` tags.

**Validation**
```bash
cargo test -p hkask-mcp
cargo clippy -p hkask-mcp -- -D warnings
```

---

## Wave 2 — Semantic Grounding (R4, R7)

> **Goal:** Close P8 compliance gaps for API and documentation.

### Task R4 — `hkask-api` REQ Tags (1 → ~20)

**Assumption:** The API crate has 127 public seams with 1 REQ tag. HTTP route handlers are the primary user-facing interface and must have behavioral verification.  
**Expected outcome:** Every API endpoint has at least one REQ-tagged integration test verifying request/response contract.

**PR slices**

- **PR R4.1:** Add integration test scaffold and settings endpoint tests.
  - File: `crates/hkask-api/tests/integration.rs` (new)
  - Tests:
    - `// REQ: GET /api/settings returns full SettingsResponse`
    - `// REQ: PUT /api/settings merge-updates existing settings`
    - `// REQ: PUT /api/settings rejects invalid temperature values`
    - `// REQ: GET /api/settings returns 401 without valid API key`

- **PR R4.2:** Add endpoint tests for chat, pods, goals, sovereignty.
  - Tests:
    - `// REQ: POST /api/chat returns inference result for valid request`
    - `// REQ: GET /api/pods returns pod list`
    - `// REQ: POST /api/goals creates and returns goal`
    - `// REQ: GET /api/sovereignty/verify returns compliance status`
    - `// REQ: POST /api/backup creates backup and returns path`
    - `// REQ: GET /api/cns/spans returns CNS span data`

- **PR R4.3:** Add endpoint tests for curator, episodic, bundles, and wallets.
  - Tests (continued):
    - `// REQ: GET /api/curator/status returns curator state`
    - `// REQ: GET /api/episodic lists episodic memories for a session`
    - `// REQ: GET /api/bundles lists available bundles`
    - `// REQ: POST /api/wallet/balance returns wallet balance`
    - `// REQ: GET /api/models lists available inference models`

**Acceptance criteria**
- `cargo test -p hkask-api` passes ≥20 tests.
- Every `pub async fn` route handler in `hkask-api/src/routes/` has ≥1 `// REQ:` tagged test.

**Validation**
```bash
cargo test -p hkask-api
cargo clippy -p hkask-api -- -D warnings
```

---

### Task R7 — Provenance Markers for OUGHT-as-IS Doc Claims

**Assumption:** 166 doc comment lines present normative claims (must/should/will/never/always) as declarative facts. Per P8, every architectural claim must carry epistemic mode and provenance.  
**Expected outcome:** All normative doc claims in foundational crates (`hkask-types`, `hkask-agents`, `hkask-cns`) carry `[DECLARATIVE]`, `[NORMATIVE]`, or `[HYPOTHESIS]` markers.

**PR slices**

- **PR R7.1:** Add provenance markers to `hkask-types` (34 OUGHT-as-IS lines).
  - File pattern: `crates/hkask-types/src/**/*.rs`
  - Transform: `/// Must enforce X` → `/// [NORMATIVE] Must enforce X (P1 — User Sovereignty)`
  - Priority files: `sovereignty.rs`, `capability/*.rs`, `event.rs`, `id.rs`
  - Scope: sovereignty, capability, identity, and consent doc comments only.

- **PR R7.2:** Add provenance markers to `hkask-agents` (30 OUGHT-as-IS lines).
  - File pattern: `crates/hkask-agents/src/**/*.rs`
  - Priority files: `consent.rs`, `curator_agent/*.rs`, `acp/*.rs`
  - Transform: `/// Should obtain consent` → `/// [NORMATIVE] Should obtain consent (P2 — Affirmative Consent)`

- **PR R7.3:** Add provenance markers to `hkask-cns` (14 OUGHT-as-IS lines) and `hkask-wallet` (11 lines).
  - File pattern: `crates/hkask-cns/src/**/*.rs`, `crates/hkask-wallet/src/**/*.rs`
  - Priority files: `energy.rs`, `set_points.rs`, `circuit_breaker.rs`, `issuer.rs`

**Acceptance criteria**
- Zero doc comments in target files containing "must"/"should"/"shall"/"will"/"never"/"always" without `[NORMATIVE]`, `[DECLARATIVE]`, or `[HYPOTHESIS]` marker.
- `grep -rn "must\|should\|shall\|never\|always" --include="*.rs" crates/hkask-types/src/ crates/hkask-agents/src/ crates/hkask-cns/src/ | grep -v "NORMATIVE\|DECLARATIVE\|HYPOTHESIS\|#\[test\]" | wc -l` returns 0 for the targeted patterns.

**Validation**
```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
# Provenance marker lint (manual grep check):
grep -rn "/// .* must \|/// .* should \|/// .* shall \|/// .* never \|/// .* always " --include="*.rs" crates/hkask-types/src/ crates/hkask-agents/src/ crates/hkask-cns/src/ | grep -v "\[NORMATIVE\]\|\[DECLARATIVE\]\|\[HYPOTHESIS\]"
```

---

## Wave 3 — Type Strength (R5, R6)

> **Goal:** Replace stringly-typed span identifiers and upgrade OCAP token signatures.

### Task R5 — Type CNS Spans as Enum Variants

**Assumption:** CNS spans are `&str` constants (e.g., `"cns.tool"`) in `hkask-types::event::CANONICAL_NAMESPACES`. This violates Hoare's "make invalid states unrepresentable" principle and P8 semantic grounding.  
**Expected outcome:** `CnsSpan` enum replaces all string constants. The enum variants cover all 33+ canonical namespaces. Parse-from-string is fallible; display-to-string is infallible.

**PR slices**

- **PR R5.1:** Define `CnsSpan` enum in `hkask-types/src/cns.rs`.
  ```rust
  /// [NORMATIVE] Typed CNS span identifiers — the authoritative registry
  /// per PRINCIPLES.md §1.4. Invalid span values are unrepresentable.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum CnsSpan {
      Tool { subsystem: ToolSubsystem },
      Prompt,
      Inference,
      AgentPod,
      Connector,
      Pipeline,
      Gas,
      Review,
      Template,
      Curation,
      Variety,
      Sovereignty,
      Goal,
      Spec,
      Test,
      SetPoint,
      CyberneticsBackpressure,
      CyberneticsCadence,
      MemoryEncode,
      MemoryBudget,
      CondenserCompressionRatio,
      EvolutionEnergyDelta,
      ArchitectureModuleDepth,
      ImprovModeActive,
      ImprovPlussingRatio,
      ImprovisFreestyleCoherence,
      ImprovEnsembleCoherence,
      KataImprovEffectiveness,
      ImprovCascadeDepth,
  }
  ```
  - Add `Display` impl for `CnsSpan` (enum → canonical string).
  - Add `FromStr` impl for `CnsSpan` (string → enum, fallible).
  - Add `ToolSubsystem` enum for `CnsSpan::Tool { subsystem }`.

- **PR R5.2:** Migrate `hkask-cns` to use `CnsSpan` enum.
  - Replace all `&str` span references in `hkask-cns/src/*.rs` with `CnsSpan` variants.
  - Update `CnsRuntime` to accept `CnsSpan` instead of `&str`.
  - Update `Event` (ν-event) construction to use `CnsSpan`.

- **PR R5.3:** Migrate remaining crates (agents, services, mcp) to `CnsSpan`.
  - Update all `tracing::span!` and `tracing::info!` calls that reference CNS spans.
  - Replace static string constants with `CnsSpan` variant references.
  - Update `CANONICAL_NAMESPACES` in `hkask-types` to be generated from the enum.

- **PR R5.4:** Add tests for CnsSpan type.
  - `// REQ: CnsSpan Display produces canonical namespace strings`
  - `// REQ: CnsSpan FromStr rejects invalid span identifiers`
  - `// REQ: CnsSpan exhaustive match covers all canonical namespaces`
  - `// REQ: ToolSubsystem Display produces valid subsystem suffix`

**Acceptance criteria**
- Zero `&str` CNS span constants remain in `hkask-types/src/cns.rs`.
- All `CnsSpan` variants map 1:1 to canonical namespaces in PRINCIPLES.md §1.4.
- `cargo test -p hkask-types` passes all CnsSpan tests.
- `cargo check --workspace` passes after migration.

**Validation**
```bash
cargo test -p hkask-types
cargo test -p hkask-cns
cargo check --workspace
cargo clippy --workspace -- -D warnings
```

---

### Task R6 — Upgrade DelegationToken from HMAC to Ed25519

**Assumption:** `DelegationToken::signature` currently uses HMAC-SHA256 (symmetric). Per P4 (Clear Boundaries), tokens should be unforgeable. Ed25519 is already available in `hkask-keystore` for API key auth and spec signing. Upgrading provides non-repudiation.  
**Expected outcome:** `DelegationToken` carries an Ed25519 signature. Verification uses the public key. Token forgery requires the private key.

**PR slices**

- **PR R6.1:** Define `TokenSignature` newtype and integrate into `DelegationToken`.
  - File: `crates/hkask-types/src/capability/tokens.rs`
  - Add `TokenSignature(Ed25519Signature)` newtype with validation.
  - Replace `signature: String` field with `signature: TokenSignature`.
  - Add `Ed25519PublicKey` field for verification.
  - Keep backward-compatible `FromStr`/`Display` for JSON serialization.

- **PR R6.2:** Implement token signing and verification in `hkask-agents`.
  - Update `RootAuthority::create_root_token()` to sign with Ed25519.
  - Update `AcpRuntime::register_agent()` to issue Ed25519-signed tokens.
  - Update `verify_delegation_token()` to verify Ed25519 signature.
  - Update `verify_delegation_token_now()` accordingly.
  - Ensure HMAC path remains available for backward compatibility during migration.

- **PR R6.3:** Migrate MCP servers to Ed25519 token verification.
  - Update `CapabilityOnlyAdapter` and `FullMcpAdapter` in `hkask-agents/src/adapters/mcp_runtime.rs`.
  - Update `MemoryLoopAdapter` in `hkask-agents/src/adapters/memory_loop_adapter.rs`.
  - Update `SpecServer` in `hkask-mcp-spec/src/main.rs`.
  - Remove HMAC verification code after all callers migrated.

- **PR R6.4:** Add tests for Ed25519 token unforgeability.
  - `// REQ: DelegationToken with Ed25519 signature verifies correctly`
  - `// REQ: DelegationToken rejects tampered signature`
  - `// REQ: DelegationToken rejects wrong public key`
  - `// REQ: Token verification rejects expired tokens regardless of signature`
  - `// REQ: Token attenuation preserves signature validity`

**Acceptance criteria**
- `DelegationToken::signature` is `TokenSignature(Ed25519Signature)`, not `String`.
- Token verification uses `ed25519_dalek::VerifyingKey::verify()`.
- HMAC code path removed from production paths (backward compat removed).
- `cargo test -p hkask-agents` passes ≥5 new token verification tests.
- `cargo test -p hkask-types` passes token serialization round-trip tests.

**Validation**
```bash
cargo test -p hkask-types
cargo test -p hkask-agents
cargo test -p hkask-mcp-spec
cargo check --workspace
cargo clippy --workspace -- -D warnings
```

---

## Wave 4 — Surface Control (R8)

> **Goal:** Reduce `hkask-types` public surface from 231 items toward a justified set.

### Task R8 — Reduce Surface on `hkask-types`

**Assumption:** `hkask-types` has 231 public items. Many are data carrier types (structs/enums) that could be re-organized into submodules with narrower re-exports. Per G2 (≤7 public items per module), the crate needs a facade module structure.  
**Expected outcome:** Public API surface of `hkask-types` remains functionally identical, but internal organization uses submodules with ≤7 public items each. Top-level re-exports provide backward-compatible access.

**PR slices**

- **PR R8.1:** Audit and classify `hkask-types` public items.
  - Produce a definitive list of all 231 public items.
  - Classify each: (a) core type that must be public, (b) re-export from submodule, (c) internal type made public by accident.
  - Identify candidates for `pub(crate)` visibility downgrade.
  - Target: reduce genuinely public items to ≤50, with the rest accessible via submodule paths.

- **PR R8.2:** Create submodule facade structure.
  - Organize into: `capability`, `cns`, `event`, `id`, `agent`, `wallet`, `memory`, `inference`, `sovereignty`, `error`.
  - Each submodule has ≤7 primary public types.
  - Top-level `lib.rs` re-exports commonly-used types for backward compatibility.
  - Add `#[deprecated(since = "0.28.0", note = "Use hkask_types::submodule::Type instead")]` on types that move to submodules (temporary, remove in 0.29.0).

- **PR R8.3:** Add G2 justification comments for items >7.
  - For any module with >7 public items, add a comment:
    ```rust
    // G2 Justification: This module exposes N public items because [reason].
    // Each item is individually justified: [brief list].
    ```

**Acceptance criteria**
- `cargo check --workspace` passes (backward compatibility maintained).
- Each `hkask-types` submodule has ≤7 primary public types (excluding re-exports).
- G2 justification comments exist for any submodules exceeding 7 items.
- Deprecated re-exports work but generate warnings.

**Validation**
```bash
cargo check --workspace
cargo test -p hkask-types
cargo clippy -p hkask-types -- -D warnings
```

---

## Wave 5 — Service Layer Completion (R9)

> **Goal:** Complete strangler fig extraction for mid-migration domains.

### Task R9 — Continue Strangler Fig Extraction

**Assumption:** 7 domains are mid-migration (Kata, Wallet, Spec, Registry, REPL Init, Consolidation, User management). The "both paths delegate before any deletion" rule is being followed. We complete extraction for the most mature domain first.  
**Expected outcome:** At least 2 domains fully extracted to `hkask-services`.

**PR slices**

- **PR R9.1:** Extract Kata domain to `hkask-services`.
  - Move `InferenceConfig::from_env()` and `KataHistoryStore::new()` construction into `KataService` or `KataEngine` in services.
  - Update `hkask-cli/commands/kata.rs` to delegate fully to services (remove direct `InferenceRouter::new` call).
  - Add test: `// REQ: Kata CLI command delegates to KataService without direct store access`
  - Verify both paths still work, then commit delegation.

- **PR R9.2:** Extract Spec domain to `hkask-services`.
  - Move `SpecStore` construction into `SpecService` (list/rm methods).
  - Update `hkask-cli/commands/spec.rs` to delegate fully to `SpecService` (remove direct `SpecStore` usage).
  - Add test: `// REQ: Spec list/rm commands delegate to SpecService without direct store access`

**Acceptance criteria**
- `hkask-cli/commands/kata.rs` no longer imports from `hkask-storage` or `hkask-inference` directly (only via `hkask-services`).
- `hkask-cli/commands/spec.rs` no longer imports from `hkask-storage` directly (only via `hkask-services`).
- `cargo test -p hkask-services` passes all service-layer tests.
- `cargo test -p hkask-cli` passes all CLI tests.

**Validation**
```bash
cargo test -p hkask-services
cargo test -p hkask-cli
cargo check --workspace
```

---

## Wave 6 — Stub Resolution (R10)

> **Goal:** Resolve the 3 remaining training-cancel soft stubs.

### Task R10 — Resolve Training Cancel Stubs

**Assumption:** `hkask-mcp-training/src/providers.rs` has 3 best-effort no-op stubs for Axolotl and Unsloth `cancel()` methods. These return `Ok(())` with warning logs but don't actually terminate running training jobs. The proper fix requires PID tracking.  
**Expected outcome:** Either (a) PID tracking is implemented for cancel, or (b) stubs are documented as accepted operational limitations with `[EVIDENCE]` provenance.

**PR slices**

- **PR R10.1:** Add PID tracking to training provider trait and implementations.
  - Add `pid: Option<u32>` field to training job state.
  - Record child process PID when spawning training processes.
  - Implement `cancel()` by sending `SIGTERM` to tracked PID.
  - Fallback: if PID is `None`, log warning and return `Ok(())` (current behavior, but now explicitly documented as degraded).

- **PR R10.2:** Add tests for cancel behavior.
  - `// REQ: Training cancel sends SIGTERM to tracked PID`
  - `// REQ: Training cancel returns Ok with warning log when PID unavailable`
  - `// REQ: Axolotl provider records PID on start`
  - `// REQ: Unsloth provider records PID on start`

**Alternative path** (if PID tracking is too invasive for v0.27.0):

- **PR R10.1-alt:** Document stubs as accepted operational limitations.
  - Replace `// no-op stub` comments with:
    ```rust
    // [EVIDENCE] Cancel is best-effort for Axolotl provider.
    // Full implementation requires PID tracking (planned v0.28.0).
    // Current behavior: returns Ok(()) with warning log.
    ```
  - Add `// REQ: Axolotl cancel returns Ok with warning when PID unavailable`
  - Add `// REQ: Unsloth cancel returns Ok with warning when PID unavailable`

**Acceptance criteria**
- Zero `todo!()` or `unimplemented!()` in training providers.
- All `cancel()` methods either terminate the process or document the limitation with `[EVIDENCE]` provenance.
- `cargo test -p hkask-mcp-training` passes all cancel tests.

**Validation**
```bash
cargo test -p hkask-mcp-training
cargo check --workspace
cargo clippy --workspace -- -D warnings
```

---

## 5) Cross-Cutting Acceptance Criteria

All waves must pass:

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

Additional P8 acceptance gate:
```bash
# REQ tag coverage for modified crates must increase
grep -r "// REQ:" --include="*.rs" crates/ mcp-servers/ | wc -l
# Target: >400 (current: 345)
```

Provenance marker gate:
```bash
# Zero OUGHT-as-IS doc claims without provenance markers in target files
grep -rn "/// .*must \|/// .*should \|/// .*shall " --include="*.rs" \
  crates/hkask-types/src/ crates/hkask-agents/src/ crates/hkask-cns/src/ | \
  grep -v "\[NORMATIVE\]\|\[DECLARATIVE\]\|\[HYPOTHESIS\]" | wc -l
# Target: 0
```

---

## 6) Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| R5 (CnsSpan migration) breaks existing ν-event format | Medium | High — breaks storage | Keep `Display` impl producing current string format; migration only in code, not wire format |
| R6 (Ed25519 tokens) breaks existing stored tokens | Medium | Critical — breaks auth | Add version field to `DelegationToken`; support HMAC verification during migration window |
| R8 (Types re-export) breaks downstream crates | Low | Medium — compilation errors | Deprecation warnings in 0.27.0, removal in 0.29.0; full backward compat via re-exports |
| R9 (Service extraction) introduces subtle behavioral changes | Medium | Medium — integration failures | Each extraction has both-paths-active period; CLI tests before/after must pass identically |
| R10 (PID tracking) platform-specific | Low | Low — graceful fallback | Ctrl+C signal handling varies by OS; test on Linux (primary target) |

---

## 7) Open Questions Requiring Human Judgment

These decisions require human input before implementation:

| # | Question | Epistemic Mode | Constraint Force | Blocks Task |
|---|----------|---------------|-----------------|-------------|
| Q1 | Should `CnsSpan` enum use associated data (e.g., `CnsSpan::Tool { subsystem }`) or flat variants? | Subjunctive | Guideline | R5 |
| Q2 | Should Ed25519 token migration include a HMAC backward-compat window, or cut over immediately? | Subjunctive | Guardrail | R6 |
| Q3 | Which mid-migration domain should be extracted next? (Kata vs Spec recommended) | Subjunctive | Guideline | R9 |
| Q4 | Is PID tracking for training cancel in scope for v0.27.0, or should stubs be documented? | Subjunctive | Guideline | R10 |
| Q5 | Should `hkask-types` visibility changes target v0.28.0 (deprecation) or v0.29.0 (removal)? | Subjunctive | Guideline | R8 |

---

## 8) Success Metrics

| Metric | Current | Target | Wave |
|--------|---------|--------|------|
| `hkask-communication` tests | 25 ✅ | ≥10 | Wave 1 (R1) |
| `hkask-agents` tests | 31 ✅ | ≥20 | Wave 1 (R2) |
| `hkask-mcp` tests | 38 ✅ | ≥15 | Wave 1 (R3) |
| `hkask-api` REQ tags | 29 ✅ | ≥20 | Wave 2 (R4) |
| OUGHT-as-IS doc claims without provenance | 0 ✅ | 0 | Wave 2 (R7) |
| CNS span type strength | `CnsSpan` enum ✅ | `CnsSpan` enum | Wave 3 (R5) |
| DelegationToken signature type | Ed25519 ✅ | Ed25519 | Wave 3 (R6) |
| `hkask-types` public surface | 576 (G2 justified) ✅ | ≤50 top-level (v0.28.0) | Wave 4 (R8) |
| Mid-migration domains | 5 (2 extracted) ✅ | 5 | Wave 5 (R9) |
| Training cancel stubs | 0 ✅ | 0 | Wave 6 (R10) |
| Total REQ tags across workspace | 846 | >400 | All waves |
| `todo!()` / `unimplemented!()` count | 0 | 0 | Maintained |

---

## 9) Convergence Criterion

The plan is complete when δS = 0 across all tasks: no further action reduction is possible, zero P1–P12 principle violations remain unaddressed from the audit, and every finding traces to a stated behavioral property of a public seam.

Per-task completion signals:
- **R1–R3:** `cargo test -p <crate>` passes ≥target test count with all `// REQ:` tags.
- **R4:** `hkask-api` has ≥20 REQ-tagged integration tests.
- **R5:** `CnsSpan` is an enum, zero `&str` span constants in production code.
- **R6:** `DelegationToken::signature` is `TokenSignature(Ed25519Signature)`, HMAC path removed.
- **R7:** Zero doc claims with normative language lacking provenance markers in target crates.
- **R8:** `hkask-types` submodules each have ≤7 primary public types.
- **R9:** ≥2 domains fully extracted to `hkask-services`.
- **R10:** Zero undocumented stubs in training providers.