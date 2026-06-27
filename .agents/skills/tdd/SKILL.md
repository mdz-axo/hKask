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
| `tdd-plan.j2` | KnowAct | Plan TDD cycle: identify requirements, prioritize by risk |
| `tdd-tracer.j2` | KnowAct | Execute tracer bullet: write one contract, then one failing test, then minimal code to pass |
| `tdd-refactor.j2` | KnowAct | Refactor while GREEN: extract duplication, deepen modules |
| `tdd-verify.j2` | KnowAct | Verify TDD cycle completion: tests pass, clippy clean |
| `tdd-gap-check.j2` | KnowAct | Functional gap analysis: compare requirements against tested behaviors |

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

Probabilistic contracts use the standard doc-comment format with an additional `prob:` field:
```rust
/// pre:  mode is Plussing, context is non-empty
/// post: response builds on input (yes-and pattern) with prob ≥ 0.90
/// prob: p=0.90, δ=semantic_similarity≥0.7, k=3
pub async fn plussing_respond(&self, input: &str) -> String { ... }
```

Only apply probabilistic contracts to LLM agent behaviors or other non-deterministic functions. Deterministic functions use standard contracts without `prob:`.

### Contract Architecture

Every contract is documented directly on the function signature using `expect:` + `[P{N}]` annotations. A contract has four layers, each answering a distinct question:

| Layer | Field | Question Answered |
|-------|-------|-------------------|
| **Verbal expectation** | `expect:` | What did the user say they want? (in the user's voice) |
| **Goal principle** | `[P{N}] Motivating:` | Which principle justifies this as a user-visible guarantee? |
| **Constraining principles** | `[P{N}] Constraining:` | What can the code *not* do and stay principle-aligned? |
| **Behavioral specification** | `pre:` / `post:` / `inv:` | What are the caller's obligations and the function's guarantees? |

A complete contract:

```rust
/// expect: "I can check whether an agent has enough gas before executing"
/// [P9] Motivating: Homeostatic Self-Regulation — prevents runaway agent execution
/// pre:  gas is a valid EnergyCost
/// post: returns true iff budget has >= gas remaining and circuit breaker allows
/// inv:  does not consume gas (read-only check)
/// [P4] Constraining: Clear Boundaries — cap enforces resource boundary
/// [P3] Constraining: Generative Space — cap is user-visible, not hidden
pub fn can_proceed(&self, gas: EnergyCost) -> bool { ... }
```

The TDD's role is to ensure every tracer bullet produces a contract with all four layers and validates each:
- Does `expect:` faithfully capture the requirement in the user's voice?
- Does the `[P{N}] Motivating:` annotation correctly identify the goal principle?
- Do `[P{N}] Constraining:` annotations correctly express what principles forbid?
- Do `pre:` / `post:` / `inv:` form a machine-checkable behavioral specification?
- Does the implementation satisfy the behavioral specification without violating constraining principles?

### Requirement-Anchored Testing

Every tracer bullet starts from a requirement, not from intuition. The requirement is the source of truth for *what* to test. Without anchoring, tests validate behavior that may not matter and miss behavior that does.

Identify the functional requirement before planning tests. If no requirement is documented for the feature, state it explicitly before proceeding. A feature without a documented requirement cannot be properly anchored.

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

## Workflow

### 1. Planning

Contracts are **generated, not authored**. The contract-generator runs a classifier-aware Jinja2 prompt template that reads the functional specification, architectural principles, and function source code, then produces `expect:` + `[P{N}]` annotations with quality self-scoring. Human curation (Accept/Revise/Reject) gates the output before any test is written.

**Step 1 — Anchor on the functional specification:**
- Read `docs/architecture/core/FUNCTIONAL_SPECIFICATION.md` for the domain you're working in
- Identify the functional requirement (FR#) that the behavior addresses
- The FR description provides the user expectation and the spec-assigned goal principle

**Step 2 — Run the contract-generator:**
- Feed the function source, spec context, and principle context into `contract-generator/contract-generator.j2`
- The generator classifies the function against the MDS domain → goal principle mapping
- It produces a complete contract annotation block with quality self-scoring (0-3)
- Contracts scoring 0-1 must be regenerated; 2 is acceptable; 3 is complete

**Step 3 — Curate the contract:**
- Review the generated `expect:` — does it faithfully capture what the user wants?
- Verify the `[P{N}] Motivating:` matches the spec-assigned goal principle
- Check `[P{N}] Constraining:` annotations for completeness (minimum P1-P4 Magna Carta)
- Accept, revise, or reject. The contract is the ground truth for the test.

**Step 4 — Map to testable behaviors:**
- Each functional requirement maps to one or more observable behaviors on a public seam
- If a requirement has no testable behavior, deepen the module first
- Confirm with user what interface changes are needed

**Step 5 — Prioritize by risk:**
- P0 (Security/correctness-critical): Trust & Security, fail-closed behavior
- P1 (Correctness): Interface parity, core algorithms
- P2+ (Ergonomics): Convenience, polish

Ask: "What functional requirement does this change address? Has the contract-generator produced an acceptable contract? Which requirements are most critical to test?"

**You can't test everything.** Focus on requirements in the change scope, prioritized by risk.

### 2. Tracer Bullet

Write ONE contract and ONE test that confirm ONE thing about the system:

```
CONTRACT: Write the full contract on the function signature — expect:, pre/post/inv, [P{N}] Motivating/Constraining
RED:     Write property-based test verifying the behavioral specification → test fails
GREEN:   Write minimal code to satisfy the contract without violating constraining principles → test passes
```

Each contract must include:
- `/// expect:` — the user's functional expectation in their own voice
- `/// [P{N}] Motivating:` — the goal principle that drives this contract (exactly one)
- `/// pre:` and `/// post:` — the behavioral specification (Testing Discipline §1.2)
- `/// inv:` — type invariants where applicable
- `/// [P{N}] Constraining:` — principles that constrain how the goal is delivered (zero to many)

For non-deterministic functions (LLM agent behaviors, inference output), add `/// prob: p=X, δ=Y, k=Z` per Testing Discipline §7.6.

```rust
/// expect: "I can check whether an agent has enough gas before executing"
/// [P9] Motivating: Homeostatic Self-Regulation — prevents runaway agent execution
/// pre:  gas is a valid EnergyCost
/// post: returns true iff budget has >= gas remaining and circuit breaker allows
/// inv:  does not consume gas (read-only check)
/// [P4] Constraining: Clear Boundaries — cap enforces resource boundary
pub fn can_proceed(&self, gas: EnergyCost) -> bool { ... }
```

### 3. Incremental Loop

For each remaining behavior:

```
CONTRACT: Write next contract →
RED:     Write next property-based test → fails
GREEN:   Minimal code to pass → passes
```

Rules:
- One contract + one test at a time
- Only enough code to satisfy the current contract without violating constraining principles
- Don't anticipate future contracts
- Keep contracts focused on observable behavior
- Each contract includes all 4 layers: expect:, goal principle, constraining principles, behavioral specification

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

**Rule 6bis — Contract metadata must travel with the function.** When moving or renaming a function, the `expect:` field, `[P{N}]` goal-principle annotation, and `[P{N}] Constraining:` annotations must travel with the contract. Loss of any of these fields is a REFACTOR violation — it severs the traceability chain.

**Rule 8bis — Verify contract metadata after each step.** Run:
```bash
grep -rn "/// \[P[0-9]*\]" crates/ --include="*.rs" | wc -l
```
Compare counts against pre-refactor counts. Any decrease means contract metadata was lost — revert.

**Contract evolution requiring user consent (P2):** If a contract's `expect:` field or goal principle changed during refactoring, this is NOT a pure refactor — it's a contract evolution requiring P2 affirmative consent. Flag such changes for human review.

**Refactor-safe contract evolution rule:** The `expect:` field is the ground truth. If the formal contract (`pre:`/`post:`) drifts from the user expectation, the contract is wrong, not the expectation. Weakening a precondition or strengthening a postcondition that causes `expect:` to no longer semantically match is a critical violation.

**Never refactor while RED.** Get to GREEN first.

### 5. Verify

```bash
cargo test -p <crate>           # Run the specific crate's tests
cargo clippy -p <crate> -- -D warnings  # Lint
cargo check -p <crate>          # Type-check
```

**Contract metadata audit:**
```bash
# Count public functions with principle grounding
grep -rn "/// \[P[0-9]*\]" crates/ --include="*.rs" | wc -l

# Check expect: field presence
grep -rn "/// expect:" crates/ --include="*.rs" | wc -l
```

### 6. Contract Quality Check

For each contract in scope, verify all layers with scoring:

1. **`expect:` quality** — Scored 0-3: 0 (missing), 1 (vacuous — restates function name), 2 (functional — describes user need), 3 (anchored — names motivating principle with rationale). Contracts scoring 0-1 are gaps.

2. **Goal principle alignment** — Does the contract's `[P{N}] Motivating:` match the domain's default goal principle? Cross-reference with the MDS category mapping (Domain→P1, Capability→P4, Interface→P3, Composition→P7, Trust→P4+P2, Observability→P9, Persistence→P8, Lifecycle→P5, Curation→P8). Mismatches require rationale.

3. **Constraining completeness** — Which Magna Carta principles (P1-P4) are missing from `[P{N}] Constraining:`? A Trust category contract without `[P4] Constraining` is a P0 gap.

### 7. CNS Feedback Integration

The TDD cycle is a pre-commit development activity. Post-deployment, the CNS provides runtime contract monitoring per Testing Discipline §7.3. CNS violations feed back into the TDD cycle as triggers for new tracer bullets:

1. **Contract violations** (`cns.contract.violated`) — A runtime contract assertion failed in production. This is a bug where the implementation violated a correct contract, or the contract was too weak to catch the violation. Open a tracer bullet to strengthen the contract to exclude the violation scenario, then fix the implementation. Per Testing Discipline §7.5, the contract now permanently guards against that class of bug.

2. **Coverage drops** (`cns.contract.coverage`) — Variety per domain fell below threshold. The seam watcher (`SeamWatcher::check_drift`) detected that tested behaviors no longer cover what the system actually does. Open a tracer bullet to restore coverage and ensure the contract is still the behavioral specification.

3. **Mutation escapes** — `cargo-mutants` (Testing Discipline §8.5 Q1) detected mutants the test suite didn't catch. Open a tracer bullet with a strengthened contract that excludes the mutation path.

**Principle:** Every CNS contract alert is a candidate tracer bullet. The loop: CNS detects a violation → TDD writes a contract + test that excludes it → implementation fixes it → CNS monitors the new contract.

**Check before closing a CNS alert:** Does the fix have a contract? Does the contract have a test? Is the test traceable to a documented requirement? If any answer is no, the fix is incomplete — the bug will recur.

## Checklist Per Cycle

```
[ ] Contract written before test with all 4 layers:
    [ ] expect: — faithful to requirement, in user's voice
    [ ] [P{N}] Motivating — goal principle declared
    [ ] [P{N}] Constraining — all applicable constraints declared
    [ ] pre:/post:/inv: — complete machine-checkable behavioral specification
[ ] Test is property-based (proptest) where applicable, verifying the behavioral specification
[ ] Test uses public interface only (seam, not internals)
[ ] Test would survive internal refactor
[ ] Code is minimal to satisfy the contract without violating constraining principles
[ ] No speculative features added
[ ] No todo!() or unimplemented!() stubs
[ ] cargo test -p <crate> passes
[ ] cargo clippy -p <crate> -- -D warnings passes
```

## End-of-Session Checklist

```
[ ] Every requirement in scope has a contract + tracer bullet OR a documented deferral
[ ] Every contract's expect: faithfully captures the requirement and carries a [P{N}] annotation
[ ] Every contract's [P{N}] Constraining annotations are complete (minimum applicable P1-P4 Magna Carta)
[ ] No implementation violates constraining principles
[ ] Contract metadata present: all pub fns with contracts have [P{N}] annotations
[ ] Gaps are recorded in OPEN_QUESTIONS.md with deferral rationale
[ ] Any contract expect: or goal_principle change during refactor is flagged for P2 consent
```


## Related Skills

- **coding-guidelines [Guideline]:** Karpathy's four behavioral principles are enforced throughout the TDD workflow — Simplicity First drives minimal implementations, Surgical Changes governs the refactor phase, Goal-Driven Execution maps to contract-anchored tracer bullets, and Think Before Coding surfaces assumptions before contracts are written. The coding-guidelines skill's anti-pattern detection (single-use abstractions, speculative features, impossible-scenario error handling) directly constrains what TDD produces.
- **pragmatic-semantics [Evidence]:** Probabilistic contracts (§7.6) require epistemic classification of contract satisfaction — pragmatic-semantics provides the IS/OUGHT distinction and provenance tracing needed to evaluate `(p, δ, k)`-satisfaction claims against actual execution traces.
- **deep-module [Guideline]:** Used during the REFACTOR phase to deepen modules and apply the deletion test.
- **essentialist [Evidence]:** Paired for eliminative review of contracts and test architectures.
- **improve-codebase-architecture [Evidence]:** Paired for architectural exploration before refactoring — anchors refactors with behavioral contracts.

## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/tdd.yaml`

### PDCA Convergence
- **Threshold:** 0.05 (converged when metric ≤ this)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = all three phases (RED, GREEN, REFACTOR) pass with acceptable coverage

### Energy Budgets
- **Gas (compute cycles):** cap 100000, 100 per iteration
- **rJoule (inference energy):** cap 26000 rJ
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
