# Reviewer's Charter — hKask Adversarial Review

> Every reviewer (human or sub-agent) must read this and agree to it before producing a finding. The charter is what keeps the review convergent.

## 1. The four operative rules (from `coding-guidelines`)

| # | Rule | Operative test |
|---|------|----------------|
| 1 | **Think Before Coding** | The finding names the assumption, the alternative considered, and the chosen seam. If it doesn't, it's an opinion, not a finding. |
| 2 | **Simplicity First** | The proposed fix removes a concept or collapses an edge. If it adds a concept, it must be a *required* capability port — not a helper, trait, or config flag. |
| 3 | **Surgical** | Touches only the file(s) named; matches existing style (Result types, error variants, span namespaces, port conventions). If the fix requires touching > 3 crates, it's a design exercise, not a refactor — move it to `review/FUTURE.md`. |
| 4 | **Goal-Driven** | Each finding names the property it preserves (P8 invariant) and the test that proves it (red → green). "It looks cleaner" is not a property. |

## 2. The hKask-specific rules (P1–P8, C1–C8)

- **P6 — No stubs.** A `pub type` with no concept link in the semantic graph is a deletion candidate.
- **P7 — No deprecations.** Fix forward; never `#[deprecated]`.
- **P8 — Tests verify a stated behavioral property of a public seam.** A test without an invariant is structural and must be rewritten or removed.
- **C1 — One responsibility per crate.** If a crate's name doesn't appear in the answer to "what does this *do*?", the crate is too wide.
- **C2 — Ports over concretes.** Every cross-crate dependency goes through a port in `hkask-templates` with a `template_type` discriminator.
- **C3 — CNS spans are the only observability primitive.** If a code path emits a `log::info!` or `println!` that isn't inside a `tracing::span!(cns.*, …)`, it's a finding.
- **C4 — OCAP over ambient authority.** Every authority-bearing object is a capability; every invocation is unforgeable.
- **C5 — Composition by reference.** State is passed, not shared. `Arc<Mutex<…>>` deeper than 2 levels is a finding.
- **C6 — Functional minimalism.** One way to do each thing. Duplicate types are findings.
- **C7 — Recursive structure.** The review tool is itself a capability with a port, a test, and a CI gate.
- **C8 — Test depth matches module depth.** Deep modules get deep tests.

## 3. The finding schema (no finding is valid without all 9 fields)

```yaml
- id: F-<LENS>-<NNN>            # e.g. F-L2-007
  location: <file>:<line range>  # or `concept:<node-id>` for cross-cutting findings
  concept: <node id in joined.ttl>
  severity: {blocker | major | minor | nit}
  evidence: <1–3 lines, verbatim from code or graph>
  principle_violated: P{n} | C{n} | DDMVSS-§{n}
  root_cause_driver: <vocabulary_drift | ambient_authority | duplicate_concept
                     | missing_test | untested_seam | alert_orphan
                     | attack_surface | shallow_module | over_engineering>
  proposed_fix_shape: <one sentence; no code>
  test_that_proves_it: <one sentence; TDD red → green>
```

## 4. The principle of minimum surface

- The review produces **findings**, not code. Code is produced in a separate phase (Task 4), one PR per finding cluster, with a red test committed first.
- A finding that requires a design exercise (not a refactor) is moved to `review/FUTURE.md` with category `design_exercise`.
- A finding that is purely stylistic and has no principle behind it is dropped.

## 5. The "Planck constant" of the review

The review has exactly 9 tasks (T0–T8). Each task's output is the next task's input. There are no side branches. If a task grows a side branch, the side branch is a finding, not work.
