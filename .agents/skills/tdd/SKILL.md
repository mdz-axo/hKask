---
name: tdd
visibility: public
description: "Test-driven development with red-green-refactor loop. Use when building features or fixing bugs with TDD, mentions 'red-green-refactor', wants integration tests, or asks for test-first development."
---

# Test-Driven Development

Adapted from Matt Pocock's TDD skill.

**Anchoring discipline:** [`docs/architecture/core/TESTING_DISCIPLINE.md`](../../docs/architecture/core/TESTING_DISCIPLINE.md) — Design by Contract (Meyer, 1986), verified through Property-Based Testing (QuickCheck, Claessen & Hughes, 2000). This skill defines the *process* for writing tests. The Testing Discipline defines *what* the tests must verify: contracts (preconditions, postconditions, invariants).

## Registry Templates

This skill's runtime templates live in `registry/templates/tdd/`:

| Template | Type | Purpose |
|----------|------|--------|
| `tdd-plan.j2` | KnowAct | Plan TDD cycle: extract requirements from specs, prioritize by risk |
| `tdd-tracer.j2` | KnowAct | Execute tracer bullet: write one failing test, then minimal code to pass |
| `tdd-refactor.j2` | KnowAct | Refactor while GREEN: extract duplication, deepen modules |
| `tdd-verify.j2` | KnowAct | Verify TDD cycle completion: tests pass, clippy clean, spec traceability |
| `tdd-gap-check.j2` | KnowAct | Functional gap analysis: compare spec requirements against tested behaviors |

The SKILL.md (this file) teaches the Zed coding agent the TDD methodology. The .j2 templates are executable process steps the hKask runtime invokes during `kask chat` sessions.

## Philosophy

**Core principle**: Tests verify behavioral contracts through public interfaces, not implementation details. Code can change entirely; contracts shouldn't.

**Anchoring discipline:** [`docs/architecture/core/TESTING_DISCIPLINE.md`](../../docs/architecture/core/TESTING_DISCIPLINE.md) — Design by Contract (Meyer, 1986), verified through Property-Based Testing (QuickCheck, Claessen & Hughes, 2000). This skill defines the *process* for writing tests. The Testing Discipline defines *what* the tests must verify: contracts (preconditions, postconditions, invariants).

**Contract-first ordering:** Write the contract before the test. The contract is the specification of the test. Without a contract, the test verifies *something*, but it's not clear what. Order: (1) Contract → (2) Property-Based Test → (3) Implementation.

**Good tests** are property-based: they verify that a contract holds for all valid inputs, not just hand-picked examples. They describe *what* the system guarantees, not *how* it achieves it. A good test reads like an executable contract. These tests survive refactors because they don't care about internal structure.

**Bad tests** are example-based and coupled to implementation. They test specific input-output pairs rather than invariants. They mock internal collaborators, test private methods, or verify through external means. If you rename an internal function and tests fail, those tests were testing implementation, not behavior.

**Probabilistic contracts** extend Design by Contract for non-deterministic functions — e.g., LLM agent behaviors, inference output validation, improv mode compliance. Per Testing Discipline §7.6, these use `(p, δ, k)`-satisfaction:
- `p`: Probability threshold (e.g., 0.95 = contract must hold in 95% of executions)
- `δ`: Tolerance bound (how far from the postcondition is acceptable)
- `k`: Recovery window (how many steps the agent has to self-correct before violation is reported)

Probabilistic contracts use the same `/// REQ:` doc-comment format with an additional `prob:` field:
```rust
/// REQ: improv-plussing-001
/// pre:  mode is Plussing, context is non-empty
/// post: response builds on input (yes-and pattern) with prob ≥ 0.90
/// prob: p=0.90, δ=semantic_similarity≥0.7, k=3
pub async fn plussing_respond(&self, input: &str) -> String { ... }
```

Only apply probabilistic contracts to LLM agent behaviors or other non-deterministic functions. Deterministic functions use standard contracts without `prob:`.

### Spec-Anchored Testing

Every tracer bullet starts from a specification requirement, not from intuition. The specification is the source of truth for *what* to test. Without spec anchoring, tests validate behavior that may not matter and miss behavior that does.

**Traceability chain**: `spec/goal/capture` → `Spec` objects → `GoalSpec.criteria` → `// REQ:` comment → test → implementation.

The TDD process queries the specification infrastructure before planning. Requirements come from structured `Spec` and `GoalSpec` objects — not from LLM interpretation of markdown. All five MDS §3 tools are available via the `hkask-mcp-spec` MCP server:

- `spec/goal/capture` — creates a new specification with auto-inferred MDS category and criteria seeding
- `spec/goal/decompose` — breaks a goal into ordered sub-goals with dependencies
- `spec/graph/query` — queries specs by text match across name, goals, and category, returning graph nodes/edges/paths
- `spec/graph/coherence` — computes collection coherence, identifies missing categories and incomplete specs
- `spec/require/writing-quality` — gates spec readability before testing

These tools are also exposed via the HTTP API (`/api/specs` routes). Curation decisions (Accept/Revise/Reject) are external to the spec server — the Curator or human makes them (MDS §2).

If no specification exists for the feature, use `spec/goal/capture` to create one before planning tests. A feature without a spec cannot be spec-anchored.

## Anti-Pattern: Horizontal Slices

**DO NOT write all tests first, then all implementation.** This is "horizontal slicing" — treating RED as "write all tests" and GREEN as "write all code."

This produces **crap tests**:
- Tests written in bulk test *imagined* behavior, not *actual* behavior
- You end up testing the *shape* of things rather than user-facing behavior
- Tests become insensitive to real changes — they pass when behavior breaks, fail when behavior is fine
- You outrun your headlights, committing to test structure before understanding the implementation

**Correct approach**: Vertical slices via tracer bullets. One test → one implementation → repeat.

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

## Rust Conventions

1. Use `#[cfg(test)]` module for unit tests alongside the code they test.
2. Use `tests/` directory for integration tests that exercise crate public APIs.
3. Use `#[tokio::test]` for async tests.
4. Use `tempfile` for tests needing filesystem — never write to the project tree. Use `hkask-test-harness` (provides `TestDb` and other test fixtures) for database and persistence tests per Testing Discipline §8.3 T9.
5. Prefer `assert!` with meaningful messages over `assert_eq!` when the message adds diagnostic value.
6. Test error paths — verify error variants, not just happy paths.
7. **No `todo!()` or `unimplemented!()`** — write minimal stubs that return sensible defaults or errors, not panics.
8. **Every test carries a `// REQ:` comment** naming the spec requirement it validates. Format: `// REQ: <spec_id> — requirement summary`. The `spec_id` comes from `spec/goal/capture`. If a test has no spec_id, it tests implementation detail, not a functional requirement.

## Workflow

### 1. Spec-Anchored Planning

Before writing any code:

**Step 1 — Extract requirements from specifications:**
- Identify the relevant specification document(s) for the change
- Extract functional requirements with their MDS category and spec_id (e.g., from `spec/goal/capture` output)
- If no spec exists, create a minimal one before proceeding

**Step 2 — Map requirements to testable behaviors:**
- Each functional requirement maps to one or more observable behaviors on a public seam
- If a requirement has no testable behavior, deepen the module first
- Confirm with user what interface changes are needed

**Step 3 — Prioritize by risk:**
- P0 (Security/correctness-critical): Trust & Security, fail-closed behavior
- P1 (Correctness): Interface parity, core algorithms
- P2+ (Ergonomics): Convenience, polish

**Step 4 — List behaviors with traceability:**
- Each behavior must reference a spec_id from the specification
- List behaviors to test (not implementation steps)
- Get user approval on the plan

Ask: "Which MDS categories does this change touch? What should the public interface look like? Which requirements are most critical to test?"

**Spec resolution:** Before writing any test plan, query the spec infrastructure for requirements in the relevant MDS category. Use `spec/graph/query` (via the `hkask-mcp-spec` MCP server) to retrieve structured requirements. Use `spec/graph/coherence` to verify collection health. Curation decisions (Accept/Revise/Reject) are made externally by the Curator or human per MDS §2. Only plan tracer bullets for specs with `Accept` curation decisions.

**You can't test everything.** Focus on requirements in the change scope, prioritized by risk.

### 2. Tracer Bullet

Write ONE contract and ONE test that confirm ONE thing about the system:

```
CONTRACT: Write the contract (// REQ: pre: ... post: ...) on the function signature
RED:     Write property-based test verifying the contract → test fails
GREEN:   Write minimal code to satisfy the contract → test passes
```

Each contract must include a `// REQ:` tag that references the spec's `id` field from `Spec`. For deterministic functions, the contract has `pre:` and `post:` fields. For non-deterministic functions (LLM agent behaviors, inference output), add a `prob:` field with `p`, `δ`, and `k` parameters per Testing Discipline §7.6.
```rust
/// REQ: <spec_id>
/// pre:  webid is a valid, non-nil WebID
/// post: returns Ok(state) where state.webid == webid
pub fn verify_sovereignty(webid: &WebID) -> Result<SovereigntyState, SovereigntyError> {
    // ...
}
```

Each test must include a `// REQ:` comment matching the contract's spec_id:
```rust
// REQ: <spec_id> — sovereignty verification returns correct state
#[test]
fn sovereignty_verify_returns_correct_state() { ... }
```
The `spec_id` is the UUID returned by `spec/goal/capture`. For human readability, include the MDS category and a brief summary after the em-dash.

### 3. Incremental Loop

For each remaining behavior:

```
CONTRACT: Write next contract →
RED:     Write next property-based test → fails
GREEN:   Minimal code to pass → passes
```

Rules:
- One contract + one test at a time
- Only enough code to satisfy the current contract
- Don't anticipate future contracts
- Keep contracts focused on observable behavior
- Each contract and test carries its `// REQ:` tag

**Fuzz and system layers** follow the same tracer-bullet pattern when applicable:
- **Fuzz tracer bullet** (Testing Discipline §2.4, T6): For `pub fn` input surfaces that accept arbitrary data from external sources. Contract: precondition = "any input", postcondition = "does not panic". Use `catch_unwind` + proptest with unlimited input generation. Fuzz tests live in `tests/` at crate root.
- **System tracer bullet** (Testing Discipline §2.4): For cross-crate end-to-end workflows that span multiple `pub fn` boundaries. Write a single integration test in `tests/` that exercises the full vertical slice. Contracts chain through the call stack — `f`'s system test transitively verifies `g`'s contracts if `f` calls `g`.

### 4. Refactor

After all tests pass, look for refactor candidates:
- Extract duplication
- Deepen modules (move complexity behind simple interfaces)
- Apply SOLID principles where natural
- Consider what new code reveals about existing code
- Run tests after each refactor step
- **Preserve `// REQ:` tags** — refactoring changes structure, not functional alignment

**Never refactor while RED.** Get to GREEN first.

### 5. Verify

```bash
cargo test -p <crate>           # Run the specific crate's tests
cargo clippy -p <crate> -- -D warnings  # Lint
cargo check -p <crate>          # Type-check
```

### 6. Functional Gap Check

After verification, compare tested behaviors against specification requirements:

1. **Call `spec/graph/query`** via the `hkask-mcp-spec` MCP server to retrieve all specs in scope
2. **For each spec**, check `is_complete()` — if false, the spec has unsatisfied criteria that may need tracer bullets
3. **Gaps** — spec requirements with no matching `// REQ:` tag — must be addressed:
   - Write a tracer bullet for the gap, OR
   - Document the gap in `OPEN_QUESTIONS.md` with a deferral rationale
4. **Call `spec/graph/coherence`** to check overall collection coherence and identify missing MDS categories

This step catches the "tested but wrong" problem (tests that don't validate real requirements) and the "untested requirement" problem (spec requirements with no coverage).

### 7. CNS Feedback Integration

The TDD cycle is a pre-commit development activity. Post-deployment, the CNS provides runtime contract monitoring per Testing Discipline §7.3. CNS violations feed back into the TDD cycle as triggers for new tracer bullets:

1. **Contract violations** (`cns.contract.violated`) — A runtime contract assertion failed in production. This is a bug where the implementation violated a correct contract, or the contract was too weak to catch the violation. Open a tracer bullet to strengthen the contract to exclude the violation scenario, then fix the implementation. Per Testing Discipline §7.5, the contract now permanently guards against that class of bug.

2. **Coverage drops** (`cns.contract.coverage`) — Variety per domain fell below threshold. The seam watcher (`SeamWatcher::check_drift`) detected that tested behaviors no longer cover what the system actually does. Open a tracer bullet to restore coverage and ensure the contract is still the behavioral specification.

3. **Mutation escapes** — `cargo-mutants` (Testing Discipline §8.5 Q1) detected mutants the test suite didn't catch. Open a tracer bullet with a strengthened contract that excludes the mutation path.

**Principle:** Every CNS contract alert is a candidate tracer bullet. The loop: CNS detects a violation → TDD writes a contract + test that excludes it → implementation fixes it → CNS monitors the new contract.

**Check before closing a CNS alert:** Does the fix have a contract? Does the contract have a test? Is the test traceable to a spec requirement? If any answer is no, the fix is incomplete — the bug will recur.

## Checklist Per Cycle

```
[ ] Contract written before test (// REQ: pre: ... post: ...)
[ ] Contract references a specification requirement (spec_id)
[ ] Test is property-based (proptest) where applicable, verifying the contract
[ ] Test uses public interface only (seam, not internals)
[ ] Test would survive internal refactor
[ ] Test carries a // REQ: tag matching the contract's spec_id
[ ] Code is minimal to satisfy the contract
[ ] No speculative features added
[ ] No todo!() or unimplemented!() stubs
[ ] cargo test -p <crate> passes
[ ] cargo clippy -p <crate> -- -D warnings passes
```

## End-of-Session Checklist

```
[ ] Every spec requirement in scope has a contract + tracer bullet OR a documented deferral
[ ] No // REQ: tag references a non-existent spec_id
[ ] Each MDS category in scope has coverage (Domain, Composition, Trust, Lifecycle, Curation)
[ ] Contract completeness audit shows no regression (Testing Discipline §9.2)
[ ] Gaps are recorded in OPEN_QUESTIONS.md with deferral rationale
```