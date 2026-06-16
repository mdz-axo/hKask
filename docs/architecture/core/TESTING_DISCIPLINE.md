---
title: "hKask Testing Discipline"
audience: [engineers, agents, replicants]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Testing Discipline

**External anchor:** Design by Contract (Meyer, 1986), verified through Property-Based Testing (QuickCheck, Claessen & Hughes, 2000).  
**Internal bridge:** TDD skill (`.agents/skills/tdd/SKILL.md`) — the process for writing contract-verified tests.  
**Governing principles:** P4 (Clear Boundaries), P8 (Semantic Grounding), P9 (Homeostatic Self-Regulation).

**Supersedes:** `docs/specifications/specs/test-program.md` (archived 2026-06-15), `docs/specifications/standards/TESTING_STANDARDS.md` (archived 2026-06-15). This document is the single authoritative reference for all hKask testing practices, standards, and philosophy.

---

## 1. The External Discipline: Design by Contract

hKask's testing program is anchored on **Design by Contract** (DbC) as formulated by Bertrand Meyer. This is not a hKask invention — it is a 40-year-old discipline taught in CS curricula, implemented in Eiffel, Ada 2012, SPARK, and Racket, and extended for AI agents in the peer-reviewed Agent Behavioral Contracts framework (2025).

### 1.1 The Three Contract Elements

Every public function has a contract consisting of three assertions:

| Element | Question Answered | Who Is Responsible |
|---------|-------------------|-------------------|
| **Precondition** | What must be true before this function is called? | The **caller** |
| **Postcondition** | What does this function guarantee after it returns? | The **function** |
| **Invariant** | What must always hold across all operations on this type? | The **type** |

A precondition violation is a **bug in the caller**. A postcondition violation is a **bug in the function**. An invariant violation is a **bug in the type's implementation**.

### 1.2 Contract Syntax in hKask Rust

Contracts are expressed as `// REQ:` doc-comments on public functions:

```rust
/// REQ: sovereignty-verify-001
/// pre:  webid is a valid, non-nil WebID
/// post: returns Ok(sovereignty_state) where state.webid == webid
///       OR returns Err(NotFound) if webid has no sovereignty record
/// inv:  does not modify any stored state (read-only)
pub fn verify_sovereignty(webid: &WebID) -> Result<SovereigntyState, SovereigntyError> {
    // ...
}
```

For types with cross-operation invariants:

```rust
/// REQ: wallet-balance-001
/// inv: balance_rj >= 0 (balances are never negative)
/// inv: balance_rj + sum(encumbrances) <= original_deposit_total
pub struct WalletBalance {
    pub balance_rj: u64,
    // ...
}
```

### 1.3 Contract Rules (from Meyer)

1. **Preconditions are caller obligations.** The function does not check them. If the caller violates a precondition, the outcome is undefined (in Rust: the function may panic, but only from the caller's bug, not the function's logic).

2. **Postconditions are function guarantees.** The function must ensure them for all inputs satisfying the precondition. If a postcondition fails, it is a bug in the function.

3. **Invariants hold on entry and exit.** Every public function must preserve the type's invariants. On entry, the invariant is guaranteed (the caller maintained it). On exit, the function must restore it.

4. **Subcontracting (inheritance).** A trait implementation may **weaken** preconditions (accept more) and must **strengthen** postconditions (guarantee more). It must preserve invariants.

5. **Contracts are documentation, specification, and test oracle simultaneously.** A contract serves all three purposes. There is no separate "spec document" for a function's behavior — the contract IS the specification.

---

## 2. The Verification Method: Property-Based Testing

Contracts specify *what* must be true. Property-Based Testing verifies *that* it is true for all inputs.

### 2.1 The Principle

A property-based test does not test a single example. It tests an invariant across randomly generated inputs:

```rust
// REQ: condenser-idempotency-001
// pre:  input is any non-empty string
// post: compress(compress(input)) == compress(input) for all inputs
proptest! {
    #[test]
    fn compression_is_idempotent(input in any::<String>()) {
        prop_assume!(!input.is_empty());  // precondition
        let once = compress(&input);
        let twice = compress(&once);
        prop_assert_eq!(once, twice);     // postcondition
    }
}
```

The `prop_assume!` enforces the precondition — inputs that violate it are skipped. The `prop_assert_eq!` verifies the postcondition. Proptest generates 10,000+ random inputs and shrinks failures to minimal counterexamples.

### 2.2 When to Use Property-Based Tests

| Situation | Use PBT? | Reason |
|-----------|----------|--------|
| Function has mathematical invariants | **Yes** | One proptest replaces dozens of examples |
| Function is pure (no side effects) | **Yes** | Deterministic, easy to verify |
| Function is stateful but has invariants across operation sequences | **Yes** | State machine PBT (generate sequences, verify invariant holds throughout) |
| Function is I/O-bound (network, filesystem) | **No** | Use integration tracer bullet instead |
| Function has no meaningful invariant beyond "doesn't panic" | **Fuzz only** | `catch_unwind` + arbitrary input |

### 2.3 The Contract→PBT Pipeline

The established combination (Hillel Wayne, 2017; `icontract-hypothesis`, 2020; GUMBOX, 2025):

1. **Contracts define the properties.** Preconditions and postconditions ARE the invariants to test.
2. **PBT generates inputs matching preconditions.** `prop_assume!` filters to valid inputs.
3. **PBT verifies postconditions hold.** `prop_assert!` checks the guarantees.
4. **Contracts chain through call stacks.** If `f` calls `g`, and `g` has contracts, `g`'s contracts are verified during `f`'s PBT. Unit PBT becomes integration testing automatically.

### 2.4 The Test Pyramid Under DbC

| Layer | What | Contract Element | Verification |
|-------|------|-----------------|-------------|
| **Unit** | Single function's behavior | Pre/Post conditions | Proptest on the function directly |
| **Integration** | Cross-function chains | Contracts chain through call stack | Proptest on the entry point; called functions' contracts verified transitively |
| **Contract** | Crate boundary behavior | Invariants across operations | Proptest on operation sequences; verifies type invariants hold |
| **Fuzz** | Input surface robustness | Implicit precondition: "any input" | `catch_unwind` + arbitrary input; verifies no panic |
| **System** | End-to-end workflows | Cross-crate invariants | Integration tracer bullet (TDD skill); verifies full vertical slice |

### 2.5 Partial Contract Coverage

The contract chain (§2.3, item 4) requires contracts on ALL called functions to propagate. If a function in the call stack lacks a contract, the chain breaks silently — `f`'s PBT will not verify `g`'s behavior. This is acceptable during migration but must be tracked:

- Functions without contracts are **contract debt** — analogous to test debt (§4.2)
- The contract completeness audit (§12) measures coverage
- New code must not introduce contract debt; existing debt is reduced over time
- When a bug is found in an uncontracted function, the fix must include adding a contract

---

## 3. Ontology — Testing Vocabulary

| Term | Definition | hLexicon Domain |
|------|-----------|-----------------|
| **Seam** | A public interface (`pub` trait, `pub` fn, `pub` struct with `pub` methods) that is the test surface | FlowDef |
| **Contract** | A behavioral specification on a seam: preconditions, postconditions, invariants (Meyer, 1986) | KnowAct |
| **Invariant** | A behavioral property that must hold for all valid inputs or across all operations on a type | KnowAct |
| **Tracer-bullet** | A vertical RED→GREEN cycle: one contract, one test, one implementation. Never horizontal slices | FlowDef |
| **Behavioral test** | A test that exercises the public seam and verifies *what* the system does, not *how*. Survives refactors | KnowAct |
| **Structural test** | A test coupled to implementation detail. Must be rewritten or documented as debt | KnowAct |
| **Deep seam** | Small interface, high leverage (few methods, many behaviors). Prefer testing at deep seams | WordAct |
| **Shallow seam** | Interface as complex as implementation. Deepening candidate, not a testing target | WordAct |
| **Test cycle** | `tracer-bullet` (first test for new behavior), `regression` (test after fix), `property` (proptest/fuzz) | FlowDef |
| **Test debt** | Implementation-coupled tests that exist because a clean seam hasn't been extracted yet | KnowAct |

---

## 4. Test Classification

Every test falls into one of three categories:

| Category | Definition | Survives Refactor? | Priority |
|----------|-----------|-------------------|----------|
| **Public Interface** | Tests behavior through a module's public API or trait seam | ✅ Yes | **Required** |
| **Seam Integration** | Tests interaction between two modules through a shared trait | ✅ Yes | **Required** |
| **Implementation-Coupled** | Tests private methods, internal state, or mocked collaborators | ❌ No | **Flag for rewrite** |

### 4.1 Classifying an Existing Test

Ask: *"If I rewrote the entire internals of this module, would this test still pass?"*

- **Yes** → Public Interface test. Keep.
- **Only if the new internals use the same trait** → Seam integration test. Keep.
- **No** → Implementation-coupled test. Flag for rewrite or removal.

### 4.2 Implementation-Coupled Tests Are Technical Debt

Implementation-coupled tests are not forbidden — they exist because some code currently lacks a clean seam. But they must be tracked:

- Add a `// TEST-DEBT: tests private <detail>` comment above the test
- The debt is resolved when a deeper interface makes the test unnecessary

---

## 5. MDS Category → Test Strategy

Each MDS category has a distinct testing emphasis. The contract defines *what* to test; the strategy defines *how*.

### 7.1 Domain (REQ-DOM-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `WebID`, `NuEvent`, `HLexicon` public APIs |
| **Test type** | Unit: type construction, parsing, validation. Contract: serialization round-trips |
| **Key invariant** | hLexicon round-trips (markdown → YAML → loaded vocabulary) |
| **Anti-pattern** | Testing internal hashmap structure of `HLexicon` |

### 7.2 Capability (REQ-CAP-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `CapabilitySpec`, `DelegationToken`, capability verification traits |
| **Test type** | Integration: capability attenuation chains, per-replicant key derivation |
| **Key invariant** | Fail-closed: no checker = denied, not open |
| **Anti-pattern** | Testing HMAC internals rather than attenuation behavior |

### 7.3 Interface (REQ-IFC-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | CLI ↔ API ↔ MCP equivalence |
| **Test type** | Integration: cross-surface parity (same operation, same result, via all three surfaces) |
| **Key invariant** | `MCP ≡ CLI ≡ API` for every operation |
| **Anti-pattern** | Testing only one surface and assuming the others work |

### 7.4 Composition (REQ-COM-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `SqliteRegistry`, `TemplateResolver`, `ContractValidator` |
| **Test type** | Integration: register → resolve → render round-trips; cascade depth enforcement |
| **Key invariant** | Template cascade terminates within depth limit |
| **Anti-pattern** | Testing Jinja2 string manipulation in isolation |

### 7.5 Trust & Security (REQ-TRU-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `GovernedTool`, key derivation, OCAP verification |
| **Test type** | Unit + integration: deterministic key derivation, attenuation depth limits, fail-closed |
| **Key invariant** | Security boundaries are never relaxed by default |
| **Anti-pattern** | Only testing the happy path; not testing invalid, expired, or wrong tokens |

### 7.6 Observability (REQ-OBS-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `CnsRuntime`, `AlgedonicManager`, `NuEventSink` |
| **Test type** | Unit: span emission, variety counter thresholds; Integration: CNS feedback loop closure |
| **Key invariant** | Algedonic alerts fire at threshold; homeostasis restores after perturbation |
| **Anti-pattern** | Testing `tracing::info!` output format rather than the observer's behavior |

### 7.7 Persistence (REQ-PER-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | Repository traits (`TripleStore`, `SpecStore`, `WalletStore`) |
| **Test type** | Integration: round-trip through SQLite with `TestDb` from harness crate |
| **Key invariant** | Bitemporal queries return correct results; encrypted storage fails without key |
| **Anti-pattern** | Testing SQL query strings rather than repository behavior |

### 7.8 Lifecycle (REQ-LIF-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `main()` entry point, migration functions, bootstrap sequence |
| **Test type** | Integration: bootstrap sequence, schema migration |
| **Key invariant** | Forward-only evolution — no rollback paths |
| **Anti-pattern** | Testing CLI argument parsing in isolation when the real risk is bootstrap ordering |

### 7.9 Curation (REQ-CUR-*)

| Strategy | Details |
|----------|---------|
| **Primary seam** | `SpecCurator`, `SpecStore`, MCP spec tool handlers |
| **Test type** | Integration: spec capture → evaluate → cultivate round-trip |
| **Key invariant** | Coherence threshold gates curation decisions |
| **Anti-pattern** | Testing Jaccard similarity in isolation without testing the full curation pipeline |

---

## 6. The TDD Bridge: From Specification to Contract to Test

The TDD skill (`.agents/skills/tdd/SKILL.md`) defines the *process* for writing tests. This discipline defines *what* the tests must verify. The bridge is the contract.

### 6.1 The Traceability Chain

```
Specification (spec/goal/capture)
    │
    ▼
GoalSpec.criteria ──→ "The system shall verify sovereignty for any valid WebID"
    │
    ▼
Contract (// REQ: pre/post/inv) ──→ "pre: valid WebID; post: returns state or NotFound"
    │
    ▼
Property-Based Test ──→ proptest!(|webid in any_webid()| { ... })
    │
    ▼
Implementation ──→ pub fn verify_sovereignty(...)
```

Every link is traceable. The `// REQ:` tag on the test references the spec_id. The contract on the function references the same spec_id. The TDD skill's gap-check verifies that every spec criterion has a matching `// REQ:` tag.

### 6.2 TDD Cycle with Contracts

The TDD skill's RED→GREEN→REFACTOR cycle, augmented with contracts:

```
RED:   Write the contract as a doc-comment on the function signature.
       Write a property-based test that:
         - Generates inputs satisfying the precondition
         - Calls the function
         - Asserts the postcondition holds
       Test fails (function not implemented).

GREEN: Write minimal code to satisfy the contract.
       Test passes for all generated inputs.

REFACTOR:
       - Can the precondition be weakened? (accept more inputs)
       - Can the postcondition be strengthened? (guarantee more)
       - Can invariants be added? (cross-operation guarantees)
       - Does the implementation still satisfy the contract?
```

### 6.3 Contract-First, Not Test-First

The TDD skill says "write the test first." This discipline adds: **write the contract before the test.** The contract is the specification of the test. Without a contract, the test verifies *something*, but it's not clear what.

Order:
1. **Contract** — `// REQ: pre: ... post: ...`
2. **Test** — proptest verifying the contract
3. **Implementation** — minimal code satisfying the contract

### 6.4 Vertical Slicing, Not Horizontal

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

### 6.5 Skill Integration

The following skills govern testing practices and are anchored on this discipline:

| Skill | Testing Role | When to Use |
|-------|-------------|-------------|
| `tdd` | Red-green-refactor with vertical slicing, contract-first ordering | Building features, fixing bugs |
| `diagnose` | Build feedback loop before hypothesizing | Bug reports, performance regressions |
| `improve-codebase-architecture` | Identify shallow modules and deepen seams | When code is hard to test through its interface |
| `coding-guidelines` | Surgical changes, simplicity first, goal-driven | All code changes |
| `pragmatics` | Meta-cognitive codebase review against principles | Architecture analysis, principle compliance audit |

---

## 7. Principle Alignment

### 7.1 P4 — Clear Boundaries (OCAP)

**Contracts ARE OCAP membranes made testable.** An OCAP boundary is a capability gate. A contract is the behavioral specification of that gate. The precondition defines what capabilities the caller must possess. The postcondition defines what capabilities are returned or exercised.

Contract tests at crate boundaries detect **semantic drift** — when a type changes in a way that's type-compatible but behaviorally different. The compiler can't catch this. Contracts can.

### 7.2 P8 — Semantic Grounding

**Every contract is an IS statement about behavior.** Preconditions, postconditions, and invariants are declarative claims about what the system does. They are not OUGHT statements ("the system should...") — they are IS statements ("the system, when called with X, returns Y"). The proptest verifies the IS claim against reality.

The `// REQ:` tag traces the contract to a specification requirement. The specification is the OUGHT. The contract is the IS. The test verifies that IS matches OUGHT.

### 7.3 P9 — Homeostatic Self-Regulation

**The test suite is a feedback loop.** Under the Good Regulator Theorem (Conant & Ashby, 1970), every good regulator must be a model of the system it regulates. The test suite IS that model. If the tests don't model the system's actual failure modes, they're not a good regulator.

- **Contract violations are CNS events [OUGHT — requires `cns.contract.violated` span implementation].** A failed contract test SHOULD emit a `cns.contract.violated` algedonic signal. The span is registered in canonical CNS span registry (`crates/hkask-types/src/cns.rs`, `CnsSpan`) and `hkask-types::event::CANONICAL_NAMESPACES`. Implementation is pending (see `docs/plans/contract-first-migration-plan-v0.27.0.md` §5.4). Until implemented, contract violations are detected through CI test failures and the contract coverage trend gate.
- **Test coverage is variety.** The CNS tracks test coverage per domain as variety (Ashby's Law). A drop in variety triggers an alert via `cns.contract.coverage`.
- **Mutation testing measures regulator quality.** `cargo-mutants` injects bugs; the percentage caught measures how well the test suite models the system.

### 7.4 P6 — Space for Replicants

**Replicants propose contracts for their own behavior.** An agent can open a PR containing:
- A contract (`// REQ: pre: ... post: ...`) describing its intended behavior
- A property-based test verifying the contract
- The agent's WebID as the authenticated author (P12)

A human operator provides affirmative consent (P2) to merge. The contract becomes part of the agent's regulatory model — the agent is saying "this is how I should behave, verify it."

**Contract conflict resolution:** If two replicants propose conflicting contracts for the same function, the conflict is resolved by:
1. The human operator identifies the conflict during PR review
2. The conflicting replicants are prompted to reconcile (improv plussing mode)
3. If reconciliation fails, the human selects one contract and documents the rejection rationale
4. The rejected contract is archived as a curation decision for future reference

### 7.5 P7 — Evolutionary Architecture

**Contracts evolve from actual failures, not speculation.** When a bug escapes to production:
1. The bug is a contract violation (a postcondition that was too weak, or an invariant that was missing)
2. Strengthen the contract to exclude the bug
3. The proptest now fails on the counterexample
4. Fix the implementation
5. The contract now permanently guards against that class of bug

Contracts accumulate the scar tissue of every production incident. They become the real engineering artifact — the implementation is replaceable; the contract is not.

### 7.6 Probabilistic Contracts for LLM Agents

Design by Contract assumes deterministic functions. LLM agents are non-deterministic. The Agent Behavioral Contracts framework (2025) extends DbC for this case with **`(p, δ, k)`-satisfaction**:

- **p:** Probability threshold (e.g., 0.95 = contract must hold in 95% of executions)
- **δ:** Tolerance bound (how far from the postcondition is acceptable)
- **k:** Recovery window (how many steps the agent has to self-correct before violation is reported)

For hKask, probabilistic contracts apply to:
- Inference output validation (exact match impossible)
- Agent behavior assertions (non-deterministic by design)
- Improv mode compliance (creative variation is expected)

Probabilistic contracts use the same `// REQ:` syntax with an additional `prob:` field:

```rust
/// REQ: improv-plussing-001
/// pre:  mode is Plussing, context is non-empty
/// post: response builds on input (yes-and pattern) with prob ≥ 0.90, δ = semantic similarity ≥ 0.7
/// prob: p=0.90, δ=0.7, k=3
pub async fn plussing_respond(&self, input: &str) -> String {
    // ...
}
```

---

## 8. Rules for the Testing Program

### 8.1 Test Location

| Test Type | Location | Convention |
|-----------|----------|------------|
| Unit (same-module) | `#[cfg(test)] mod tests` inside source file | For testing public interface of a single module |
| Integration | `tests/` directory at crate root | For testing cross-module behavior through crate public API |
| Contract | `tests/contract/` directory at crate root | For testing behavioral contracts at crate boundaries |
| Fuzz | `tests/` directory at crate root | For panic-free verification on arbitrary input |
| MCP server | `#[cfg(test)] mod tests` in `main.rs` | For testing tool handlers and response types |

### 8.2 Contract Rules

| Rule | Description |
|------|-------------|
| **C1** | Every `pub fn` must have a contract (`// REQ: pre: ... post: ...`) |
| **C2** | Every type with cross-operation invariants must declare them (`// REQ: inv: ...`) |
| **C3** | Preconditions are caller obligations — the function does not check them |
| **C4** | Postconditions are function guarantees — verified by proptest |
| **C5** | Contracts are the specification — there is no separate "spec document" for function behavior |
| **C6** | Contract violations are bugs — fix the implementation, not the contract (unless the contract was wrong) |

### 8.3 Testing Rules

| Rule | Description |
|------|-------------|
| **T1** | Every contract must have at least one property-based test verifying it |
| **T2** | Property-based tests use `prop_assume!` to enforce preconditions |
| **T3** | Property-based tests use `prop_assert!` to verify postconditions and invariants |
| **T4** | Every test carries a `// REQ:` tag traceable to a specification requirement |
| **T5** | Integration tracer bullets follow the TDD skill's vertical slice pattern |
| **T6** | Fuzz tests verify that all input surfaces handle arbitrary input without panicking |
| **T7** | No `todo!()`, `unimplemented!()`, or `#[deprecated]` in test code (P5) |
| **T8** | Use `#[cfg(test)]` module for unit tests; `tests/` for integration; `#[tokio::test]` for async |
| **T9** | Use `tempfile` or `hkask-test-harness` for filesystem/database — never write to the project tree |
| **T10** | Prefer `assert!` with meaningful messages; test error paths, not just happy paths |

### 8.4 Process Rules

| Rule | Description |
|------|-------------|
| **P1** | Contract first, test second, implementation third |
| **P2** | One contract + one test per TDD cycle (vertical slice, not horizontal) |
| **P3** | Refactor only when GREEN — never while RED |
| **P4** | After every bug fix, strengthen the contract to exclude that class of bug |
| **P5** | Replicants may propose contracts and tests; humans provide consent to merge (P2, P6) |
| **P6** | Every test action carries an authenticated author (TestWebId or replicant WebID) (P12) |

### 8.5 Quality Rules

| Rule | Description |
|------|-------------|
| **Q1** | Mutation testing runs periodically; target ≥70% mutant detection |
| **Q2** | CNS monitors test coverage as variety per domain; drops trigger algedonic alerts |
| **Q3** | Contract completeness is auditable: `grep -r "// REQ: pre:" crates/ --include="*.rs"` |
| **Q4** | Every specification criterion has a matching `// REQ:` tag (TDD gap-check) |

---

## 9. Verification & Audit

### 9.1 Verification Gates

| Gate | Command | Expected |
|------|---------|----------|
| Build | `cargo check --workspace` | Pass |
| Tests | `cargo test --workspace` | All pass |
| Lint | `cargo clippy --workspace -- -D warnings` | No warnings |
| Format | `cargo fmt --check` | No diffs |
| Prohibitions | `grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/ --include="*.rs"` | Zero |
| Headless | `grep -r "grafana\|prometheus\|dashboard\|visual.*ui" crates/ --include="*.rs"` | Zero |
| Contract coverage | `grep -rn "// REQ:.*pre:" crates/ mcp-servers/ --include="*.rs" \| wc -l` | Increasing over time |
| Test debt | `grep -r "TEST-DEBT" crates/ --include="*.rs" \| wc -l` | Decreasing over time |

### 9.2 Contract Completeness Audit

```bash
# Count public functions
pub_fns=$(grep -rn "pub fn\|pub async fn" crates/ mcp-servers/ --include="*.rs" \
  | grep -v "cfg(test)" | grep -v "/tests/" | wc -l)

# Count contracted functions (those with // REQ: pre:)
contracted=$(grep -rn "// REQ:.*pre:" crates/ mcp-servers/ --include="*.rs" | wc -l)

echo "Public functions: $pub_fns"
echo "Contracted: $contracted"
echo "Coverage: $(( contracted * 100 / pub_fns ))%"
```

Target: 100% of public functions have contracts. New code must not reduce this percentage.

---

## 10. References

### External Discipline

- Meyer, B. (1986). "Design by Contract." *IEEE Computer*. The foundational text.
- Meyer, B. (1992). *Eiffel: The Language*. Prentice Hall. Chapters on assertions and contracts.
- Claessen, K. & Hughes, J. (2000). "QuickCheck: A Lightweight Tool for Random Testing of Haskell Programs." *ICFP*. The foundational PBT text.
- Wayne, H. (2017). "Property Tests + Contracts = Integration Tests." Documents the DbC+PBT combination.
- Hatcliff, J. et al. (2025). "GUMBOX: Automated Property-Based Testing from AADL Component Contracts." *Springer STTT*. Peer-reviewed contract→PBT pipeline.
- mristin (2020). `icontract-hypothesis`. Working implementation of DbC+PBT in Python.

### Agentic AI Extension

- Agent Behavioral Contracts (2025). Formal DbC for LLM agents with probabilistic compliance. `C = (P, I, G, R)`.

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
- `docs/plans/test-harness-maturation-plan-v0.27.0.md` — Implementation plan
- `docs/architecture/core/MDS.md` — Minimal Domain Specification

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
