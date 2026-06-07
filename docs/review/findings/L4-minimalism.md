# L4 — Functional Minimalism (Planck / hKask ethos)

> Persona: "There is one way." Vocabulary is finite. Each concept has
> exactly one name, one type, one place. If a synonym exists, one of
> the two is wrong.
>
> Method: build a vocabulary card index from `func.ttl` and grep the
> workspace for near-synonyms. Where two terms carry the same
> definition, one is a finding.

## The vocabulary card index

From `review/graphs/func.ttl` + the codebase, here are the terms that
*should* have exactly one name and one home:

| Concept | Canonical name | Realization |
|---------|----------------|-------------|
| visibility | `Visibility` (enum) | `hkask-types/src/visibility.rs` |
| sovereignty-boundary | `DataSovereigntyBoundary` (struct) | `hkask-types/src/sovereignty.rs` |
| ocap-boundary | `OCAPBoundary` (struct) | `hkask-types/src/curation.rs` |
| attenuation-level | `u8` (today), `AttenuationLevel` (proposed) | `hkask-types/src/capability/mod.rs` |
| sub-goal-depth | `u8` (today), `AttenuationLevel`-analog (proposed) | `hkask-storage/src/goals.rs` |
| span-category | `&str` (today), `SpanCategory` (proposed) | `hkask-storage/src/nu_event_store.rs` |

## Findings

### F-L4-001 — `is_shared` / `is_public` exist on two different types with the same semantics (vocabulary collision)

- **id:** F-L4-001
- **location:** `crates/hkask-types/src/visibility.rs:67` AND
  `crates/hkask-types/src/sovereignty.rs:159,164`
- **concept:** `concept:visibility`
- **severity:** minor
- **evidence:**
  ```rust
  // hkask-types/src/visibility.rs:67
  impl Visibility {
      pub fn is_shared(&self) -> bool { ... }
      // (also: is_private, is_public, both #[allow(dead_code)])
  }
  // hkask-types/src/sovereignty.rs:159,164
  impl DataSovereigntyBoundary {
      pub fn is_shared(&self, category: &DataCategory) -> bool { ... }
      pub fn is_public(&self, category: &DataCategory) -> bool { ... }
  }
  ```
  Two types, two `is_shared` predicates, same meaning ("is the access
  mode shared/public?"). A reader of one cannot find the other
  without `rg`-ing.
- **principle_violated:** C6 (one way to do things — but the way has
  *two names and two implementations*). C2 (ports over concretes —
  there is no port for "is-access-shared?").
- **root_cause_driver:** `vocabulary_drift` — the two predicates
  predate the `Visibility` enum (or vice versa); the symmetry was
  never enforced.
- **proposed_fix_shape:** Move the `is_shared` / `is_public` predicates
  onto a single `AccessMode` enum, with both `Visibility` and
  `DataSovereigntyBoundary` carrying one. The single source of truth
  is then `AccessMode::is_shared(category)` and friends. Rename
  `DataSovereigntyBoundary::is_shared` to `AccessMode::is_shared` and
  delegate.
- **test_that_proves_it:** A test that asserts
  `Visibility::Shared.is_shared() == true` and
  `DataSovereigntyBoundary::default().is_shared(&DataCategory::Episodic) == ...`
  both flow through the same `AccessMode::is_shared` path. Compile-time
  rename + runtime test.

### F-L4-002 — `CurationDecision` and `GoalState` are 4-value enums with disjoint meanings, but the *number* matches — a vocabulary coincidence, not a concept (positive observation)

- **id:** F-L4-002
- **location:** `crates/hkask-types/src/curation.rs:9-21`,
  `crates/hkask-types/src/goal.rs:39-...`
- **concept:** `concept:curation_loop`
- **severity:** nit
- **evidence:** Both have exactly 4 variants. They are *not* the same
  concept. This is a positive observation: the codebase did not
  collapse the two into a single `Status` enum.
- **principle_violated:** n/a (positive).
- **proposed_fix_shape:** None. Add a doc comment on each enum naming
  the other as "disjoint concept with same cardinality" so future
  refactors don't merge them.
- **test_that_proves_it:** A doc-test on each enum that fails to
  compile if a `From<GoalState> for CurationDecision` impl is added
  (i.e. asserts the types are unrelated).

### F-L4-003 — `MemoryLoopAdapter` *and* `memory_loop` in the same crate — same noun, three files?

- **id:** F-L4-003
- **location:** `crates/hkask-agents/src/adapters/memory_loop_adapter.rs`,
  `crates/hkask-memory/`
- **concept:** `concept:memory_loop`
- **severity:** minor
- **evidence:** `memory_loop` is a concept in the graph and a *crate*
  (`hkask-memory`). The `MemoryLoopAdapter` is the third appearance of
  the noun in three places. The 7-line ERD (cns_loops.mmd) treats
  "MemoryLoop" as one concept with three names. This is the case
  that needs to be either consolidated (one crate, no adapter) or
  named (adapter is a *forwarder*, not a *memory loop*).
- **principle_violated:** C6 (one way).
- **root_cause_driver:** `duplicate_concept` — the noun means three
  things in three places.
- **proposed_fix_shape:** Rename `MemoryLoopAdapter` to
  `MemoryLoopForwarder` (or whatever its actual responsibility is).
  Document in the module docstring which of the three meanings it is.
- **test_that_proves_it:** A test that asserts the adapter's
  *responsibility* in a single sentence (e.g. "forwards lifecycle
  events to the memory loop while emitting its own cns.memory.* spans").
  The test is a doc-comment; the assertion is the rename.

### F-L4-004 — `Spec`, `Goal`, `Criterion`, `Artifact` are four types with similar `pub fn` signatures — vocabulary without duplication?

- **id:** F-L4-004
- **location:** `crates/hkask-storage/src/goals.rs`,
  `crates/hkask-templates/src/{registry.rs, ports.rs}`
- **concept:** `concept:goal`
- **severity:** nit
- **evidence:** All four have `pub fn new(...)` constructors, `pub fn
  id(&self) -> &IdType` accessors, and `pub fn update_*` mutators with
  similar signatures. Whether this is *parallel* (good — four
  types, four parallel APIs) or *duplicate* (bad — one `Entity` type
  with four facets) is a design call.
- **principle_violated:** C6 if duplicate; none if parallel.
- **root_cause_driver:** n/a (depends on design intent).
- **proposed_fix_shape:** Read the four `new()` constructors. If they
  share > 80% of their arguments, the types are parallel and the
  duplication is fine. If they share < 50%, the types are facets of
  a larger entity and should be unified.
- **test_that_proves_it:** A snapshot test that lists the argument
  names of each `new()`; if the intersection is > 80%, declare "parallel"
  with a doc-comment; if < 50%, the finding is *red* and the unification
  is the fix.

## Lens summary

| Severity | Count |
|----------|-------|
| blocker  | 0     |
| major    | 0     |
| minor    | 2     |
| nit      | 2     |

L4 found no outright duplicates in the surveyed surface. The
vocabulary collision (`is_shared` on two types, F-L4-001) is the
single L4 finding worth acting on; the others are positive
observations or "depends on intent" cases that become findings only
after reading more code.
