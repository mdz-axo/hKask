---
title: "hKask Testing Discipline"
audience: [engineers, agents, replicants]
last_updated: 2026-06-18
version: "0.29.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Testing Discipline

**Method:** Property-Based Testing (QuickCheck, Claessen & Hughes, 2000) verified through CNS observability.  
**Internal bridge:** TDD skill (`.agents/skills/tdd/SKILL.md`) — the process for writing verified tests.  
**Governing principles:** P4 (Clear Boundaries), P8 (Semantic Grounding), P9 (Homeostatic Self-Regulation).

**Supersedes:** `docs/specifications/specs/test-program.md` (archived 2026-06-15), `docs/specifications/standards/TESTING_STANDARDS.md` (archived 2026-06-15). This document is the single authoritative reference for all hKask testing practices, standards, and philosophy.

---

## 1. The Verification Method: Property-Based Testing

### 1.1 The Principle

A property-based test does not test a single example. It tests an invariant across randomly generated inputs:

```rust
proptest! {
    #[test]
    fn compression_is_idempotent(input in any::<String>()) {
        prop_assume!(!input.is_empty());
        let once = compress(&input);
        let twice = compress(&once);
        prop_assert_eq!(once, twice);
    }
}
```

`prop_assume!` enforces preconditions — inputs that violate them are skipped. `prop_assert!` verifies postconditions. Proptest generates 10,000+ random inputs and shrinks failures to minimal counterexamples.

### 1.2 When to Use Property-Based Tests

| Situation | Use PBT? | Reason |
|-----------|----------|--------|
| Function has mathematical invariants | **Yes** | One proptest replaces dozens of examples |
| Function is pure (no side effects) | **Yes** | Deterministic, easy to verify |
| Function is stateful but has invariants across operation sequences | **Yes** | State machine PBT (generate sequences, verify invariant holds throughout) |
| Function is I/O-bound (network, filesystem) | **No** | Use integration tracer bullet instead |
| Function has no meaningful invariant beyond "doesn't panic" | **Fuzz only** | `catch_unwind` + arbitrary input |

### 1.3 The Test Pyramid

| Layer | What | Verification |
|-------|------|-------------|
| **Unit** | Single function's behavior | Proptest on the function directly |
| **Integration** | Cross-function chains | Proptest on the entry point; CNS spans verify called functions' behavior |
| **State machine** | Invariants across operation sequences | Proptest on operation sequences; CNS `cns.gas` spans track budget invariants |
| **Fuzz** | Input surface robustness | `catch_unwind` + arbitrary input; verifies no panic |
| **System** | End-to-end workflows | Integration tracer bullet (TDD skill); verifies full vertical slice |

### 1.4 Deployment Testing

Deployment testing covers the provisioning surface — the operations that initialize and configure a running hKask server:

| Domain | Test Type | Example |
|--------|-----------|---------|
| Server init | Integration | `init_server_creates_config_and_keychain_entries` |
| Sidecar generation | Integration | `deploy_sidecar_generates_valid_docker_compose` |
| OAuth callback | Integration | `oauth_callback_provisions_human_user_and_session` |
| Health endpoint | Unit | `health_endpoint_returns_cns_status` |
| Single binary | Smoke | `single_binary_contains_all_components` |
| Docker build | CI | `docker_build_produces_working_image` |

---

## 2. Ontology — Testing Vocabulary

| Term | Definition | Domain |
|------|-----------|--------|
| **Seam** | A public interface (`pub` trait, `pub` fn, `pub` struct with `pub` methods) that is the test surface | FlowDef |
| **Invariant** | A behavioral property that must hold for all valid inputs or across all operations on a type | KnowAct |
| **Tracer-bullet** | A vertical RED→GREEN cycle: one invariant, one test, one implementation. Never horizontal slices | FlowDef |
| **Behavioral test** | A test that exercises the public seam and verifies *what* the system does, not *how*. Survives refactors | KnowAct |
| **Structural test** | A test coupled to implementation detail. Must be rewritten or documented as debt | KnowAct |
| **Deep seam** | Small interface, high leverage (few methods, many behaviors). Prefer testing at deep seams | WordAct |
| **Shallow seam** | Interface as complex as implementation. Deepening candidate, not a testing target | WordAct |
| **Test cycle** | `tracer-bullet` (first test for new behavior), `regression` (test after fix), `property` (proptest/fuzz) | FlowDef |
| **Test debt** | Implementation-coupled tests that exist because a clean seam hasn't been extracted yet | KnowAct |

---

## 3. Test Classification

Every test falls into one of three categories:

| Category | Definition | Survives Refactor? | Priority |
|----------|-----------|-------------------|----------|
| **Public Interface** | Tests behavior through a module's public API or trait seam | ✅ Yes | **Required** |
| **Seam Integration** | Tests interaction between two modules through a shared trait | ✅ Yes | **Required** |
| **Implementation-Coupled** | Tests private methods, internal state, or mocked collaborators | ❌ No | **Flag for rewrite** |

### 3.1 Classifying an Existing Test

Ask: *"If I rewrote the entire internals of this module, would this test still pass?"*

- **Yes** → Public Interface test. Keep.
- **Only if the new internals use the same trait** → Seam integration test. Keep.
- **No** → Implementation-coupled test. Flag for rewrite or removal.

### 3.2 Implementation-Coupled Tests Are Technical Debt

Implementation-coupled tests are not forbidden — they exist because some code currently lacks a clean seam. But they must be tracked:

- Add a `// TEST-DEBT: tests private <detail>` comment above the test
- The debt is resolved when a deeper interface makes the test unnecessary

---

## 4. MDS Category → Test Strategy

### 4.1 Domain (REQ-DOM-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `WebID`, `NuEvent` public APIs |
| **Test type** | Unit: type construction, parsing, validation. Serialization round-trips |
| **Key invariant** | Lexicon round-trips (markdown → YAML → loaded vocabulary) |
| **Anti-pattern** | Testing internal hashmap structure of lexicon types |

### 4.2 Capability (REQ-CAP-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `CapabilitySpec`, `DelegationToken`, capability verification traits |
| **Test type** | Integration: capability attenuation chains, per-replicant key derivation |
| **Key invariant** | Fail-closed: no checker = denied, not open |
| **Anti-pattern** | Testing HMAC internals rather than attenuation behavior |

### 4.3 Interface (REQ-IFC-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | CLI ↔ API ↔ MCP equivalence |
| **Test type** | Integration: cross-surface parity (same operation, same result, via all three surfaces) |
| **Key invariant** | `MCP ≡ CLI ≡ API` for every operation |
| **Anti-pattern** | Testing only one surface and assuming the others work |

### 4.4 Composition (REQ-COM-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `SqliteRegistry`, `TemplateResolver` |
| **Test type** | Integration: register → resolve → render round-trips; cascade depth enforcement |
| **Key invariant** | Template cascade terminates within depth limit |
| **Anti-pattern** | Testing Jinja2 string manipulation in isolation |

### 4.5 Trust & Security (REQ-TRU-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `GovernedTool`, key derivation, OCAP verification |
| **Test type** | Unit + integration: deterministic key derivation, attenuation depth limits, fail-closed |
| **Key invariant** | Security boundaries are never relaxed by default |
| **Anti-pattern** | Only testing the happy path; not testing invalid, expired, or wrong tokens |

### 4.6 Observability (REQ-OBS-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `CnsRuntime`, `AlgedonicManager`, `NuEventSink` |
| **Test type** | Unit: span emission, variety counter thresholds; Integration: CNS feedback loop closure |
| **Key invariant** | Algedonic alerts fire at threshold; homeostasis restores after perturbation |
| **Anti-pattern** | Testing `tracing::info!` output format rather than the observer's behavior |

### 4.7 Persistence (REQ-PER-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | Repository traits (`TripleStore`, `SpecStore`, `WalletStore`) |
| **Test type** | Integration: round-trip through SQLite with `TestDb` from harness crate |
| **Key invariant** | Bitemporal queries return correct results; encrypted storage fails without key |
| **Anti-pattern** | Testing SQL query strings rather than repository behavior |

### 4.8 Lifecycle (REQ-LIF-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `main()` entry point, migration functions, bootstrap sequence |
| **Test type** | Integration: bootstrap sequence, schema migration |
| **Key invariant** | Forward-only evolution — no rollback paths |
| **Anti-pattern** | Testing CLI argument parsing in isolation when the real risk is bootstrap ordering |

### 4.9 Curation (REQ-CUR-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `SpecCurator`, `SpecStore`, MCP spec tool handlers |
| **Test type** | Integration: spec capture → evaluate → cultivate round-trip |
| **Key invariant** | Coherence threshold gates curation decisions |
| **Anti-pattern** | Testing Jaccard similarity in isolation without testing the full curation pipeline |

---

## 5. Principle Alignment

### 5.1 P4 — Clear Boundaries (OCAP)

Invariants at crate boundaries detect **semantic drift** — when a type changes in a way that's type-compatible but behaviorally different. The compiler can't catch this. Property-based tests can. CNS spans (`cns.gas`, `cns.tool.*`) provide runtime verification at every boundary.

### 5.2 P8 — Semantic Grounding

Every test verifies an IS claim about system behavior. The CNS span registry (`CnsSpan` in `crates/hkask-types/src/cns.rs`) defines the canonical observability namespace. Test output is traceable to span types.

### 5.3 P9 — Homeostatic Self-Regulation

**The test suite is a feedback loop.** Under the Good Regulator Theorem (Conant & Ashby, 1970), every good regulator must be a model of the system it regulates. The test suite IS that model.

- **CNS spans provide runtime observability.** `cns.gas` spans on `reserve`/`settle`/`consume`/`reset_to` track budget invariants in production. Type-enforced invariants (private fields on `EnergyBudget`) prevent violations structurally.
- **Test coverage is variety.** The CNS tracks test coverage per domain as variety (Ashby's Law). A drop in variety triggers an alert.
- **Mutation testing measures regulator quality.** `cargo-mutants` injects bugs; the percentage caught measures how well the test suite models the system.

### 5.4 P6 — Space for Replicants

Replicants propose tests for their own behavior. A replicant can open a PR containing a property-based test verifying its intended behavior, with the replicant's WebID as the authenticated author (P12). A human operator provides affirmative consent (P2) to merge.

### 5.5 P7 — Evolutionary Architecture

Tests evolve from actual failures, not speculation. When a bug escapes to production:
1. Write a proptest that captures the failure mode
2. Verify it fails (reproduces the bug)
3. Fix the implementation
4. The proptest now permanently guards against that class of bug

Tests accumulate the scar tissue of every production incident. They become the real engineering artifact — the implementation is replaceable; the invariants are not.

---

## 6. Rules for the Testing Program

### 6.1 Test Location

| Test Type | Location | Convention |
|-----------|----------|------------|
| Unit (same-module) | `#[cfg(test)] mod tests` inside source file | For testing public interface of a single module |
| Integration | `tests/` directory at crate root | For testing cross-module behavior through crate public API |
| Fuzz | `tests/` directory at crate root | For panic-free verification on arbitrary input |
| MCP server | `#[cfg(test)] mod tests` in `main.rs` | For testing tool handlers and response types |

### 6.2 Testing Rules

| Rule | Description |
|------|-------------|
| **T1** | Prefer property-based tests for functions with mathematical or state-machine invariants |
| **T2** | Property-based tests use `prop_assume!` to enforce preconditions |
| **T3** | Property-based tests use `prop_assert!` to verify postconditions and invariants |
| **T4** | Integration tracer bullets follow the TDD skill's vertical slice pattern |
| **T5** | Fuzz tests verify that all input surfaces handle arbitrary input without panicking |
| **T6** | No `todo!()`, `unimplemented!()`, or `#[deprecated]` in test code (P5) |
| **T7** | Use `#[cfg(test)]` module for unit tests; `tests/` for integration; `#[tokio::test]` for async |
| **T8** | Use `tempfile` or `hkask-test-harness` for filesystem/database — never write to the project tree |
| **T9** | Prefer `assert!` with meaningful messages; test error paths, not just happy paths |

### 6.3 Process Rules

| Rule | Description |
|------|-------------|
| **P1** | Test first, implementation second (TDD) |
| **P2** | One test per TDD cycle (vertical slice, not horizontal) |
| **P3** | Refactor only when GREEN — never while RED |
| **P4** | After every bug fix, add a regression test that captures that class of bug |
| **P5** | Replicants may propose tests; humans provide consent to merge (P2, P6) |
| **P6** | Every test action carries an authenticated author (TestWebId or replicant WebID) (P12) |

### 6.4 Quality Rules

| Rule | Description |
|------|-------------|
| **Q1** | Mutation testing runs periodically; target ≥70% mutant detection |
| **Q2** | CNS monitors test coverage as variety per domain; drops trigger algedonic alerts |
| **Q3** | Type-enforced invariants (private fields, constructor validation) preferred over runtime assertions |

---

## 7. Verification & Audit

### 7.1 Verification Gates

| Gate | Command | Expected |
|------|---------|----------|
| Build | `cargo check --workspace` | Pass |
| Tests | `cargo test --workspace` | All pass |
| Lint | `cargo clippy --workspace -- -D warnings` | No warnings |
| Format | `cargo fmt --check` | No diffs |
| Prohibitions | `grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/ --include="*.rs"` | Zero |
| Headless | `grep -r "grafana\|prometheus\|dashboard\|visual.*ui" crates/ --include="*.rs"` | Zero |
| CNS daemon | `kask daemon start` (smoke test) | Binds socket, loops active |
| Deployment smoke | `kask init --profile server && kask daemon` | Server starts, health endpoint responds |
| Deployment sidecar | `kask matrix deploy-sidecar --domain localhost` | Valid docker-compose.yml generated |

---

## 8. References

### External Discipline

- Claessen, K. & Hughes, J. (2000). "QuickCheck: A Lightweight Tool for Random Testing of Haskell Programs." *ICFP*. The foundational PBT text.
- Meyer, B. (1986). "Design by Contract." *IEEE Computer*.

### Cybernetic Foundation

- Conant, R.C. & Ashby, W.R. (1970). "Every Good Regulator of a System Must Be a Model of That System." *Int. J. Systems Sci.* The theoretical basis for tests as regulator model.
- Ashby, W.R. (1956). *An Introduction to Cybernetics*. The theoretical basis for variety-based coverage monitoring.

### Software Engineering Foundations

- Beck, K. (2003). *Test-Driven Development: By Example.* Addison-Wesley. The red-green-refactor cycle.
- Feathers, M. (2004). *Working Effectively with Legacy Code.* Prentice Hall. The seam model and test classification.
- Meszaros, G. (2007). *xUnit Test Patterns: Refactoring Test Code.* Addison-Wesley. Shared fixtures and test infrastructure patterns.

### hKask Internal

- `docs/architecture/core/PRINCIPLES.md` — P1–P12 governing principles
- `.agents/skills/tdd/SKILL.md` — TDD process (RED→GREEN→REFACTOR with spec anchoring)
- `docs/architecture/core/MDS.md` — Minimal Domain Specification

---

*ℏKask - A Minimal Viable Container for Agents — v0.29.0*
