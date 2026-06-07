---
name: tdd
description: Test-driven development with red-green-refactor loop. Use when building features or fixing bugs with TDD, mentions "red-green-refactor", wants integration tests, or asks for test-first development.
---

# Test-Driven Development

Adapted from Matt Pocock's TDD skill.

## Philosophy

**Core principle**: Tests verify behavior through public interfaces, not implementation details. Code can change entirely; tests shouldn't.

**Good tests** are integration-style: they exercise real code paths through public APIs. They describe *what* the system does, not *how* it does it. A good test reads like a specification. These tests survive refactors because they don't care about internal structure.

**Bad tests** are coupled to implementation. They mock internal collaborators, test private methods, or verify through external means. If you rename an internal function and tests fail, those tests were testing implementation, not behavior.

### Spec-Anchored Testing

Every tracer bullet starts from a specification requirement, not from intuition. The specification is the source of truth for *what* to test. Without spec anchoring, tests validate behavior that may not matter and miss behavior that does.

**Traceability chain**: Specification → REQ-ID → `// REQ:` comment → test → implementation.

If no specification exists for the feature, create a minimal one before planning tests. A feature without a spec cannot be spec-anchored.

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
4. Use `tempfile` for tests needing filesystem — never write to the project tree.
5. Prefer `assert!` with meaningful messages over `assert_eq!` when the message adds diagnostic value.
6. Test error paths — verify error variants, not just happy paths.
7. **No `todo!()` or `unimplemented!()`** — write minimal stubs that return sensible defaults or errors, not panics.
8. **Every test carries a `// REQ:` comment** naming the spec requirement it validates. Format: `// REQ: REQ-XXX — requirement summary`. If a test has no REQ-ID, it tests implementation detail, not a functional requirement.

## Workflow

### 1. Spec-Anchored Planning

Before writing any code:

**Step 1 — Extract requirements from specifications:**
- Identify the relevant specification document(s) for the change
- Extract functional requirements with their DDMVSS REQ-IDs (e.g., `REQ-TRU-001`)
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
- Each behavior must reference a REQ-ID from the specification
- List behaviors to test (not implementation steps)
- Get user approval on the plan

Ask: "Which specification requirements does this change touch? What should the public interface look like? Which requirements are most critical to test?"

**You can't test everything.** Focus on requirements in the change scope, prioritized by risk.

### 2. Tracer Bullet

Write ONE test that confirms ONE thing about the system:

```
RED:   Write test for first behavior → test fails
GREEN: Write minimal code to pass → test passes
```

Each test must include a `// REQ:` comment:
```rust
// REQ: REQ-TRU-001 — Fail-closed capability checker
#[test]
fn capability_check_denies_when_no_checker() { ... }
```

### 3. Incremental Loop

For each remaining behavior:

```
RED:   Write next test → fails
GREEN: Minimal code to pass → passes
```

Rules:
- One test at a time
- Only enough code to pass current test
- Don't anticipate future tests
- Keep tests focused on observable behavior
- Each test carries its `// REQ:` tag

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

1. **List every REQ-ID** from the specifications that were in scope for this change
2. **For each REQ-ID**, check: is there a tracer bullet with this tag?
3. **Gaps** — requirements without a tracer bullet — must be addressed:
   - Write a tracer bullet for the gap, OR
   - Document the gap in `OPEN_QUESTIONS.md` with a deferral rationale

This step catches the "tested but wrong" problem (tests that don't validate real requirements) and the "untested requirement" problem (spec requirements with no coverage).

## Checklist Per Cycle

```
[ ] Test describes behavior, not implementation
[ ] Test uses public interface only (seam, not internals)
[ ] Test would survive internal refactor
[ ] Test carries a // REQ: tag referencing a specification requirement
[ ] Code is minimal for this test
[ ] No speculative features added
[ ] No todo!() or unimplemented!() stubs
[ ] cargo test -p <crate> passes
[ ] cargo clippy -p <crate> -- -D warnings passes
```

## End-of-Session Checklist

```
[ ] Every spec requirement in scope has a tracer bullet OR a documented deferral
[ ] No // REQ: tag references a non-existent requirement
[ ] TRACEABILITY_MATRIX.md is updated for new REQ-ID coverage
[ ] Gaps are recorded in OPEN_QUESTIONS.md with deferral rationale
```