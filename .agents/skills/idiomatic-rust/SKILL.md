---
name: idiomatic-rust
visibility: public
description: >
  Idiomatic Rust design through Graydon Hoare's lens. Convergent inquiry loop:
  anchor design problems against Hoare's principles, propose type-driven
  solutions with code examples, challenge and refine through adversarial
  review, and converge toward deeper, more idiomatic designs. Covers type
  design, ownership patterns, error handling, unsafe boundaries, idiom
  auditing, and refactoring toward deeper modules.
---

# Idiomatic Rust — Thinking Like Graydon Hoare

This skill runs a **convergent inquiry loop** — not a passive reference document. When activated, it anchors your Rust design problem against Graydon Hoare's eight principles, proposes type-driven solutions with code examples drawn from the current Rust ecosystem, challenges the design through adversarial review, and iterates until the design converges toward genuine idiomatic depth.

## The Inquiry Loop

```
Understand → Design → Challenge → Converge → (loop)
```

Each phase produces structured, scored output. If the convergence metric exceeds the threshold, the critique's refinement directives feed back into the design phase for another iteration.

## Hoare's Eight Principles

These are the operating system of this skill. Every design decision is evaluated against them.

| # | Principle | What It Means |
|---|-----------|---------------|
| 1 | **Make invalid states unrepresentable** | A well-designed type makes wrong usage impossible. If `None` is invalid, the type should not be `Option<T>`. If a state transition can happen out of order, the type should prevent it. The compiler rejects invalid programs — your types should too. |
| 2 | **Ownership is architecture** | Who creates? Who observes? Who mutates? Who destroys? These are architectural decisions, not memory management details. The borrow checker is your design partner — if you're fighting it, the design is wrong. |
| 3 | **Fearless refactoring** | If it compiles, the type system verified the change. Algebraic types, exhaustive pattern matching, and the borrow checker together mean refactoring is a mechanical operation, not a gamble. |
| 4 | **Zero-cost abstraction** | What you don't use, you don't pay for. What you do use, you couldn't hand-code better. Traits, generics, and iterators compile to the same machine code as their hand-written equivalents. |
| 5 | **Explicit over implicit** | The reader never guesses. Type inference is local (never cross-function). Conversions require `From`/`Into`. Every panic is documented. The cost of every operation is visible at the call site. |
| 6 | **Composition over inheritance** | No class hierarchies. Traits define behavior; structs compose data. A type's capabilities are the sum of the traits it implements. |
| 7 | **Errors as values** | Errors are data — `Result<T, E>`, never exceptions. The caller decides: propagate, handle, convert. No hidden control flow. `#[must_use]` on Results. |
| 8 | **Unsafe as contract** | `unsafe` is not "dangerous code." It is code where the programmer, not the compiler, upholds the invariants. Every `unsafe` block must document: what invariants it assumes, why they hold, what breaks if they're violated. |

## Phase 1: Understand & Anchor

Anchor the design problem against all eight principles before writing code.

**What the agent does:**
- Enumerate the **invariants** — what must always be true
- List every **invalid state** the current design permits
- Draw the **ownership DAG** — who creates, observes, mutates, destroys each value
- Map the **error domain** — what can fail, where, and how
- Identify which principles the design violates, ranked by severity
- Produce specific **improvement targets** ordered by impact

**Template:** `idiomatic-rust/idiomatic-rust-inquiry`

## Phase 2: Design & Exemplify

Propose type-driven solutions with code. Not pseudocode — real Rust with `impl` blocks, trait bounds, and error propagation.

**What the agent does:**
- Design types that make invalid states unrepresentable (newtypes, enums, non-empty types)
- Architect ownership (single owner, shared immutable, shared mutable, borrowed)
- Design errors (`thiserror` for libraries, `anyhow` for applications, never `unwrap()` in library code)
- Compose traits (many small traits, `impl Trait` returns, standard derives)
- Cite the ecosystem: std patterns, key crates, relevant RFCs, API guidelines

**Template:** `idiomatic-rust/idiomatic-rust-design`

## Phase 3: Challenge & Refine

Adversarial review. Find what's wrong — gaps, edge cases, unchallenged assumptions.

**What the agent does:**
- Find **gaps** — scenarios the design doesn't cover
- Test **edge cases** — empty input, max values, concurrency, error chains, shutdown
- Challenge **assumptions** — try to construct the invalid state the type claims to prevent
- Find **connections** — what std type resembles this? What crate solves a similar problem? Which RFC applies?
- Produce **refinement directives** — specific, actionable changes with expected improvement

This is the **few-shot refinement step.** If the critique finds issues, the directives feed back into Phase 2 for another design iteration. Each iteration improves the design against the critique's specific targets.

**Template:** `idiomatic-rust/idiomatic-rust-challenge`

## Phase 4: Converge

Synthesize design quality, critique depth, and connection richness into a single convergence metric.

**Template:** `idiomatic-rust/idiomatic-rust-convergence`

## When to Use

- "Make this Rust code more idiomatic" / "Is this Rust code idiomatic?"
- "Design a Rust type for X" / "What's the right Rust type for this?"
- "Review this Rust code for safety violations" / "Is this unsafe block correct?"
- "Refactor this Rust module toward deeper design"
- "What crate or std type should I use for this pattern?"
- "How would Graydon Hoare design this?"

## Key Resources

This skill's reasoning draws from:

- **std library:** `std::iter`, `std::collections`, `std::sync`, `std::ops`, `std::convert`
- **Key crates:** `serde` (serialization), `tokio` (async), `thiserror`/`anyhow` (errors), `rayon` (parallelism), `tracing` (observability)
- **RFCs:** [RFC 243](https://rust-lang.github.io/rfcs/0243-trait-based-exception-handling.html) (`?` operator), [RFC 1522](https://rust-lang.github.io/rfcs/1522-conservative-impl-trait.html) (`impl Trait`), [RFC 195](https://rust-lang.github.io/rfcs/0195-associated-items.html) (associated items), [RFC 236](https://rust-lang.github.io/rfcs/0236-error-conventions.html) (error conventions)
- **API Guidelines:** [rust-lang.github.io/api-guidelines](https://rust-lang.github.io/api-guidelines/)
- **The Book:** [doc.rust-lang.org/book](https://doc.rust-lang.org/book/)
- **The Reference:** [doc.rust-lang.org/reference](https://doc.rust-lang.org/reference/)
- **The Nomicon:** [doc.rust-lang.org/nomicon](https://doc.rust-lang.org/nomicon/) (unsafe Rust)

## Related Skills

- **coding-guidelines** — Karpathy's four behavioral principles (pairs with idiomatic-rust for surgical Rust changes)
- **deep-module** — Module depth evaluation (pairs with idiomatic-rust for Rust module design)
- **essentialist** — Recursive eliminative simplification (pairs with idiomatic-rust for stripping non-idiomatic complexity)
- **tdd** — Test-driven development (pairs with idiomatic-rust for type-first test design)
- **diagnose** — Bug diagnosis (pairs with idiomatic-rust for ownership and lifetime debugging)
- **pragmatic-semantics** — Epistemic discipline for classifying design claims (IS vs OUGHT, domain anchoring)

## Registry Templates

| Template | Type | Purpose |
|----------|------|--------|
| `idiomatic-rust/idiomatic-rust-inquiry` | KnowAct | Anchor design problem against Hoare's eight principles |
| `idiomatic-rust/idiomatic-rust-design` | KnowAct | Propose type-driven solutions with code examples |
| `idiomatic-rust/idiomatic-rust-challenge` | KnowAct | Adversarial review with refinement directives |
| `idiomatic-rust/idiomatic-rust-convergence` | KnowAct | Compute convergence metric for the inquiry loop |

## Registry Manifest

**Type:** Skill | **Manifest:** `registry/manifests/idiomatic-rust.yaml`

### PDCA Convergence
- **Threshold:** 0.15 (converged when metric ≤ this)
- **Improvement ratio:** 0.05 (min relative reduction per iteration)
- **Improvement gate:** threshold_only
- **Max iterations:** 3
- **Convergence meaning:** 0 = design is maximally idiomatic

### Energy Budgets
- **Gas (compute cycles):** cap 120000, 100 per iteration
- **rJoule (inference energy):** cap 3 rJ
- **System constant:** 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
