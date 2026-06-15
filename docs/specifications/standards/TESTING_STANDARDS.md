---
title: "hKask Testing Standards"
audience: [architects, developers, agents]
last_updated: 2026-06-06
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Testing Standards

**Purpose:** Codify testing practices derived from Matt Pocock's TDD methodology, architecture deepening principles, and disciplined diagnosis — adapted to hKask's MDS categories and constraint-driven design (P1–P7, C1–C7).

**Related:** [`PRINCIPLES.md`](../architecture/core/PRINCIPLES.md), [`MDS.md`](../architecture/core/MDS.md), [`TRACEABILITY_MATRIX.md`](specs/TRACEABILITY_MATRIX.md), [`AGENTS.md`](../../AGENTS.md)

**Skills referenced:** `tdd`, `diagnose`, `improve-codebase-architecture`, `coding-guidelines`, `zoom-out`, `grill-me`, `skill-bundler`

**Verification:** `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`

---

## 1. Philosophy

### 1.1 Tests Verify Behavior Through Public Interfaces

Code can change entirely; tests shouldn't. A good test reads like a specification — it describes *what* the system does, not *how* it does it. Tests coupled to implementation details (mocking private collaborators, testing internal state, asserting on private methods) break when the implementation changes even if behavior is preserved.

**Good tests** are integration-style: they exercise real code paths through public APIs. **Bad tests** are coupled to implementation.

### 1.2 Vertical Slicing, Not Horizontal

**Do not write all tests first, then all implementation.** This is horizontal slicing — it produces tests that verify *imagined* behavior rather than *actual* behavior.

```
WRONG (horizontal):
  RED:   test1, test2, test3, test4, test5
  GREEN: impl1, impl2, impl3, impl4, impl5

RIGHT (vertical):
  RED→GREEN: test1→impl1
  RED→GREEN: test2→impl2
  RED→GREEN: test3→impl3
  ...
```

Each RED→GREEN cycle is a **tracer bullet**: one test confirming one behavior, then minimal code to pass.

### 1.3 The Interface Is the Test Surface

From `improve-codebase-architecture`: a module's **seam** is where its interface lives — the place behavior can be altered without editing in place. Tests should exercise modules through their seams.

- **One adapter = hypothetical seam** (only the production implementation exists)
- **Two adapters = real seam** (a test adapter and a production adapter both exist)

When a module is hard to test, that signals a **shallow module** — its interface is nearly as complex as its implementation. The fix is to deepen the module (move complexity behind a simpler interface), not to add more mocks.

### 1.4 Diagnosis Before Fix

From `diagnose`: build a feedback loop before hypothesizing. A 2-second deterministic test loop is a debugging superpower. A 30-second flaky loop is barely better than no loop.

Write the regression test **before the fix** — but only if there is a correct seam for it. If no correct seam exists, that itself is an architecture finding.

[^beck-tdd]: Beck, Kent. *Test-Driven Development: By Example.* Addison-Wesley, 2003. — The red-green-refactor cycle and the principle of writing tests before implementation.
[^feathers-seam]: Feathers, Michael. *Working Effectively with Legacy Code.* Prentice Hall, 2004. Chapter 4: "The Seam Model" — a seam is a place where behavior can be altered without editing in place.

---

## 2. Test Classification

Every test in the workspace falls into one of three categories:

| Category | Definition | Survives Refactor? | Priority |
|----------|-----------|-------------------|----------|
| **Public Interface** | Tests behavior through a module's public API or trait seam | ✅ Yes | **Required** |
| **Seam Integration** | Tests interaction between two modules through a shared trait | ✅ Yes | **Required** |
| **Implementation-Coupled** | Tests private methods, internal state, or mocked collaborators | ❌ No | **Flag for rewrite** |

### 2.1 Classifying an Existing Test

Ask: *"If I rewrote the entire internals of this module, would this test still pass?"*

- **Yes** → Public Interface test. Keep.
- **Only if the new internals use the same trait** → Seam integration test. Keep.
- **No** → Implementation-coupled test. Flag for rewrite or removal.

### 2.2 Implementation-Coupled Tests Are Technical Debt

Implementation-coupled tests are not forbidden — they exist because some code currently lacks a clean seam. But they must be tracked:

- Add a `// TEST-DEBT: tests private <detail>` comment above the test
- Link the debt in the test traceability section of the relevant MDS category
- The debt is resolved when a deeper interface makes the test unnecessary

[^feathers-test-categories]: Feathers, Michael. *Working Effectively with Legacy Code.* Prentice Hall, 2004. Chapter 13: "I Need to Make a Change but I Don't Know What Tests to Write" — characterization tests vs. regression tests, and the concept of test debt.

---

## 3. Rust Conventions

### 3.1 Test Location

| Test Type | Location | Convention |
|-----------|----------|------------|
| Unit (same-module) | `#[cfg(test)] mod tests` inside source file | For testing public interface of a single module |
| Integration | `tests/` directory at crate root | For testing cross-module behavior through crate public API |
| MCP server | `#[cfg(test)] mod tests` in `main.rs` | For testing tool handlers and response types |

### 3.2 Test Rules

1. Use `#[cfg(test)]` module for unit tests alongside the code they test.
2. Use `tests/` directory for integration tests that exercise crate public APIs.
3. Use `#[tokio::test]` for async tests.
4. Use `tempfile` for tests needing filesystem — never write to the project tree.
5. Prefer `assert!` with meaningful messages over `assert_eq!` when the message adds diagnostic value.
6. Test error paths — verify error variants, not just happy paths.
7. **No `todo!()` or `unimplemented!()`** — write minimal stubs that return sensible defaults or errors, not panics.
8. **No `#[allow(dead_code)]` on test code** — if test code is dead, remove the test.

### 3.3 Async Test Helpers

For tests that need a temporary database or runtime:

```rust
#[cfg(test)]
mod tests {
    // Prefer: test through the public trait seam
    // Avoid: constructing full runtime with all real dependencies
    // When a seam doesn't exist, that's a finding (see §2.2)
}
```

[^rust-book]: Klabnik, Steve, and Carol Nichols. *The Rust Programming Language.* No Starch Press, 2019. Chapter 11: "Writing Automated Tests" — `#[cfg(test)]` modules, `#[test]` attribute, and test organization conventions.

---

## 4. MDS Category → Test Strategy

Each MDS category has a distinct testing emphasis:

### 4.1 Domain (REQ-DOM-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `WebID`, `NuEvent`, `HLexicon` public APIs |
| **Test type** | Unit: type construction, parsing, validation |
| **Key invariant** | hLexicon round-trips (markdown → YAML → loaded vocabulary) |
| **Anti-pattern** | Testing internal hashmap structure of `HLexicon` |

### 4.2 Capability (REQ-CAP-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `Capability`, `Delegation`, `AcpRuntime` traits |
| **Test type** | Integration: capability attenuation chains, per-replicant key derivation |
| **Key invariant** | Fail-closed: no checker = `CapabilityDenied`, not open |
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
| **Primary seam** | `SqliteRegistry`, `TemplateResolver`, `ContractValidator` |
| **Test type** | Integration: register → resolve → render round-trips; cascade depth enforcement |
| **Key invariant** | Template cascade terminates within depth limit |
| **Anti-pattern** | Testing Jinja2 string manipulation in isolation |

### 4.5 Trust & Security (REQ-TRU-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `SecurityGateway`, `AcpRuntime`, key derivation |
| **Test type** | Unit + integration: deterministic key derivation, attenuation depth limits, fail-closed |
| **Key invariant** | Security boundaries are never relaxed by default |
| **Anti-pattern** | Only testing the happy path; not testing what happens when tokens are invalid, expired, or wrong |

### 4.6 Observability (REQ-OBS-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `CnsObserver`, `SseObserver`, `AlgedonicManager` |
| **Test type** | Unit: span emission, variety counter thresholds; Integration: SSE event serialization |
| **Key invariant** | Algedonic alerts fire at threshold/2 (warning) and threshold (critical) |
| **Anti-pattern** | Testing `tracing::info!` output format rather than the observer's behavior |

### 4.7 Persistence (REQ-PER-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | Repository traits (`GoalRepository`, `TripleStore`, `SpecStore`) |
| **Test type** | Integration: round-trip through SQLite with temp databases |
| **Key invariant** | Bitemporal queries return correct results; encrypted storage fails without key |
| **Anti-pattern** | Testing SQL query strings rather than repository behavior |

### 4.8 Lifecycle (REQ-LIF-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `main()` entry point, migration functions |
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

[^mds-strategy]: hKask Team. (2026). *MDS — Minimal Domain Specification.* `docs/architecture/MDS.md` — the 5-category, 5-tool framework that defines the test strategy categories.

---

## 5. Workflow (Per Skill)

### 5.1 TDD: Red-Green-Refactor (Vertical Slice)

For each behavior to test:

```
1. RED:    Write one test that confirms one behavior through a public seam
2. GREEN:  Write minimal code to make the test pass
3. REFACTOR: Deepen modules, extract duplication — only while GREEN
```

Rules:
- One test at a time
- Only enough code to pass the current test
- Don't anticipate future tests
- Keep tests focused on observable behavior

**Checklist per cycle:**
```
[ ] Test describes behavior, not implementation
[ ] Test uses public interface only (seam, not internals)
[ ] Test would survive internal refactor
[ ] Code is minimal for this test
[ ] No speculative features added
[ ] No todo!() or unimplemented!() stubs
[ ] cargo test -p <crate> passes
[ ] cargo clippy -p <crate> -- -D warnings passes
```

### 5.2 Diagnose: Feedback Loop First

When a bug or regression is reported:

```
1. BUILD LOOP:    Create a failing test at the nearest seam
2. REPRODUCE:     Run the loop, confirm the failure
3. HYPOTHESISE:   Generate 3-5 ranked falsifiable hypotheses
4. INSTRUMENT:    One variable at a time, mapped to specific hypotheses
5. FIX + REGRESS: Write regression test BEFORE the fix
6. CLEANUP:       Remove all [DIAG-...] instrumentation
```

### 5.3 Improve Architecture: Deepen Before Testing

When a module is hard to test:

```
1. EXPLORE:   Find where understanding requires bouncing between many small modules
2. CANDIDATES: Present deepening opportunities with deletion-test reasoning
3. GRILL:      Walk the design tree with the user
4. DEEPEN:     Move complexity behind a simpler interface
5. TEST:       Write tests through the new seam
```

### 5.4 Coding Guidelines: Surgical Changes

When making changes:

```
1. THINK:  State assumptions, present alternatives, surface tradeoffs
2. SIMPLE:  Minimum code that solves the problem, nothing speculative
3. SURGICAL: Touch only what you must, match existing style
4. VERIFY:  Define success criteria, loop until verified
```

### 5.5 Skill Bundles for Common Workflows

| Bundle | Skills | Use Case |
|--------|--------|----------|
| **tdd-session** | `tdd` + `coding-guidelines` + `diagnose` | Feature development with TDD, guardrails, and debugging fallback |
| **debug-session** | `diagnose` + `coding-guidelines` | Finding and fixing bugs systematically |
| **architecture-review** | `improve-codebase-architecture` + `zoom-out` + `grill-me` | Refactoring with big-picture context and stress-testing |
| **spec-session** | `tdd` + `skill-bundler` | Writing MDS specs with test traceability |

[^beck-tdd-workflow]: Beck, Kent. *Test-Driven Development: By Example.* Addison-Wesley, 2003. — The red-green-refactor cycle as a workflow discipline.
[^pocock-tdd-skill]: Pocock, M. (2025). *TDD Skill.* Project-local skill: `.agents/skills/tdd/SKILL.md`
[^diagnose-skill]: hKask Team. (2026). *Diagnose Skill.* Project-local skill: `.agents/skills/diagnose/SKILL.md`

---

## 6. Test Traceability

### 6.1 Requirement → Test Mapping

Every MDS requirement in `TRACEABILITY_MATRIX.md` must map to at least one test. If no test exists, the "Tests" column must say `— GAP` (not `—`).

Current state (as of v0.23.0):

| Category | Tested | Gaps | Total |
|----------|--------|------|-------|
| Domain | 1 | 2 | 3 |
| Capability | 1 | 3 | 4 |
| Interface | 1 | 2 | 3 |
| Composition | 0 | 3 | 3 |
| Trust & Security | 0 | 3 | 3 |
| Observability | 1 | 2 | 3 |
| Persistence | 0 | 2 | 2 |
| Lifecycle | 0 | 2 | 2 |
| Curation | 0 | 2 | 2 |
| **Total** | **4** | **21** | **25** |

### 6.2 Closing the Gaps — Priority Order

Priority is determined by risk: security and correctness-critical paths first.

| Priority | Category | Gap | Target Seam | Test Type |
|----------|----------|-----|-------------|-----------|
| P0 | Trust & Security | Fail-closed capability checker | `CapabilityChecker` trait | Integration: `no checker → denied` |
| P0 | Trust & Security | Per-replicant key derivation | `AcpRuntime::derive_agent_secret` | Integration: deterministic, different agents → different keys |
| P0 | Trust & Security | Encrypted storage at rest | `Database` (SQLCipher) | Integration: write → read with key, write → read without key → error |
| P1 | Capability | OCAP attenuation depth | `Delegation` trait | Unit: attenuation chain terminates at depth limit |
| P1 | Interface | MCP ≡ CLI ≡ API parity | `GoalServer`, `goal_router`, `kask goal` | Integration: same operation via all three surfaces |
| P1 | Interface | CNS SSE endpoint | `SseObserver` | Integration: subscribe → receive events |
| P2 | Observability | Algedonic alert thresholds | `AlgedonicManager` | Unit: deficit > 50 → warning, > 100 → critical |
| P2 | Composition | Template cascade depth | `TemplateResolver` | Integration: deep nesting terminates within limit |
| P2 | Persistence | Bitemporal triple storage | `TripleStore` | Integration: write → query with temporal context |
| P3 | Domain | hLexicon drift detection | `ContractValidator` | Integration: template with invalid terms → rejection |
| P3 | Lifecycle | Bootstrap sequence | `main()` | Integration: clean start → operational |
| P3 | Curation | Spec curation pipeline | `DefaultSpecCurator` | Integration: capture → evaluate → cultivate |

### 6.3 Verification Commands

| Gate | Command | Expected |
|------|---------|----------|
| Build | `cargo check --workspace` | Pass |
| Tests | `cargo test --workspace` | All pass |
| Lint | `cargo clippy --workspace -- -D warnings` | No warnings |
| Format | `cargo fmt --check` | No diffs |
| Test Debt | `grep -r "TEST-DEBT" crates/ --include="*.rs" \| wc -l` | Decreasing over time |
| P6/P7 | `grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/ --include="*.rs"` | Zero |

[^traceability-matrix]: hKask Team. (2026). *Traceability Matrix.* `do../specifications/specs/TRACEABILITY_MATRIX.md` — bidirectional code→test traceability with requirement tags.

---

## 7. Integration Test Infrastructure

### 7.1 Shared Test Utilities

Create `crates/hkask-types/src/test_fixtures.rs` with:

- `test_webid()` — deterministic WebID for tests
- `test_nu_event()` — deterministic NuEvent for tests
- `test_db()` — temporary SQLite database with schema

### 7.2 MCP Server Testing Pattern

Each MCP server should have a `#[cfg(test)] mod tests` block that:

1. Constructs the server with a `test_webid()` and `test_db()`
2. Calls tool handlers through the public `call_tool` interface
3. Asserts on the response shape and content

### 7.3 Cross-Surface Parity Testing

For the `MCP ≡ CLI ≡ API` axiom (REQ-IFC-001), create integration tests that:

1. Perform the same operation through each surface
2. Assert that the result is structurally identical
3. Run as `tests/surface_parity.rs` in `hkask-api`

[^meszaros-xunit]: Meszaros, Gerard. *xUnit Test Patterns: Refactoring Test Code.* Addison-Wesley, 2007. — Shared fixtures, test doubles, and the test infrastructure patterns referenced in this section.

---

## 8. Skill Integration

The following skills are project-local in `hKask/.agents/skills/` and govern testing practices:

| Skill | Testing Role | When to Use |
|-------|-------------|-------------|
| `tdd` | Red-green-refactor with vertical slicing | Building features, fixing bugs |
| `diagnose` | Build feedback loop before hypothesizing | Bug reports, performance regressions |
| `improve-codebase-architecture` | Identify shallow modules and deepen seams | When code is hard to test through its interface |
| `coding-guidelines` | Surgical changes, simplicity first, goal-driven | All code changes |
| `zoom-out` | Module map, caller graph, data flow, boundary summary | Unfamiliar code, lost in the weeds |
| `grill-me` | Socratic interrogation of design decisions | Before implementing, after reviewing, during architecture review |
| `skill-bundler` | Compose multiple skills into a coordinated session | When multiple skills need to coordinate |

[^karpathy-guidelines-skill]: Karpathy, A. (2025). *Coding Guidelines Skill.* Project-local skill: `.agents/skills/coding-guidelines/SKILL.md` — the four behavioral principles governing all code changes.

---

## References

[^pocock-tdd]: Pocock, M. (2025). *TDD Skill — Red-green-refactor with vertical slicing.* Project-local skill: `.agents/skills/tdd/SKILL.md`
[^pocock-architecture]: Pocock, M. (2025). *Improve Codebase Architecture — Deepening opportunities.* Project-local skill: `.agents/skills/improve-codebase-architecture/SKILL.md`
[^karpathy-guidelines]: Karpathy, A. (2025). *Coding Guidelines — Think, simplify, surgical, verify.* Project-local skill: `.agents/skills/coding-guidelines/SKILL.md`
[^diagnose]: hKask Team. (2026). *Diagnose — Build a feedback loop first.* Project-local skill: `.agents/skills/diagnose/SKILL.md`
[^principles]: hKask Team. (2026). *Architecture Principles.* `docs/architecture/PRINCIPLES.md`
[^mds]: hKask Team. (2026). *MDS — Minimal Domain Specification.* `docs/architecture/MDS.md`

---

*hKask Testing Standards v1.0.0 — Tests verify behavior through public interfaces, not implementation details.*