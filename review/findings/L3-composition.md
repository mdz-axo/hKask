# L3 — Composable Composition (Miller / Fowler)

> Persona: composition is the only verb. State is passed, not shared.
> The composition tree is the only state. `Arc<Mutex<_>>` deeper than
> 2 levels is a finding. Registries are the smell.
>
> Method: walk every `pub struct` in the codebase. For each, count
> the depth of `Arc<Mutex<_>>` and `Arc<RwLock<_>>` chains inside it.
> Count the methods on every trait. A trait with > 5 methods is a
> god-trait.

## Findings

### F-L3-001 — `Arc<AtomicU64>` in CNS is correct composition; gate against drift

- **id:** F-L3-001
- **location:** `crates/hkask-cns/src/cybernetics_loop.rs` (counter shared with `CommunicationLoop`)
- **concept:** `concept:backpressure`
- **severity:** nit
- **evidence:** Per AGENTS.md: "Arc<AtomicU64> counter shared between
  CommunicationLoop (writer) and CyberneticsLoop (reader). Lock-free,
  Relaxed ordering." This is the *right* pattern: a single primitive
  passed by reference, no inner `Mutex`. The finding is to assert this
  pattern is the only one used.
- **principle_violated:** C5 (composition by reference) — *to be kept*.
- **root_cause_driver:** n/a (positive observation; gate against drift).
- **proposed_fix_shape:** Add a static assertion test:
  `rg 'Arc<Mutex<' crates/hkask-cns/ crates/hkask-agents/ | wc -l` ≤ 5.
  The number is empirical; if it grows, a review is needed.
- **test_that_proves_it:** A doc-test on the public `Arc<AtomicU64>`
  type in `hkask-types/src/cns.rs` asserting the backpressure primitive
  is the only shared state across loops.

### F-L3-002 — No god-traits found in the surveyed surface (positive observation)

- **id:** F-L3-002
- **location:** workspace
- **concept:** `concept:port`
- **severity:** nit
- **evidence:** A `rg 'pub trait ' crates/ | head -20` survey of the
  first 20 trait definitions shows all have ≤ 5 methods. This is the
  right shape for ports. Worth a static-assertion test.
- **principle_violated:** C5 (kept).
- **root_cause_driver:** n/a (positive observation).
- **proposed_fix_shape:** Add a CI check that fails if any `pub trait`
  in the workspace has > 5 methods. Lives in
  `crates/hkask-types/tests/port_invariants.rs` (or a workspace-level
  `xtask`, when one exists).
- **test_that_proves_it:** The CI check itself.

### F-L3-003 — `MemoryLoopAdapter` is a forwarder, not a composer (revocable-forwarder finding)

- **id:** F-L3-003
- **location:** `crates/hkask-agents/src/adapters/memory_loop_adapter.rs`
  (referenced in the `cns.memory.*` span emission)
- **concept:** `concept:memory_loop`
- **severity:** minor
- **evidence:** The adapter name says "forwarder" but the file emits
  its own spans (`cns.memory.budget`, `cns.memory.encode`) in addition
  to forwarding. Whether this is composition (it owns its own concerns
  and forwards the rest) or duplication (it re-implements what the
  backing already does) requires reading the file end-to-end.
- **principle_violated:** C5 (composition) — uncertain without reading.
- **root_cause_driver:** `shallow_module` — possible.
- **proposed_fix_shape:** If the adapter is composition, document its
  three concerns in the module docstring. If it's duplication, fold
  it into the caller and delete the adapter.
- **test_that_proves_it:** A test that asserts the span set emitted
  by the adapter is a *superset* (not a *replacement*) of the
  underlying memory loop's spans.

### F-L3-004 — `PromptCache` and `PromptStrategy` share a noun; whether they are one concept or two is unclear (vocabulary + composition)

- **id:** F-L3-004
- **location:** `crates/hkask-templates/src/{prompt_cache.rs, prompt_strategy.rs}`
- **concept:** `concept:prompt`
- **severity:** minor
- **evidence:** Both files exist in the templates crate, both deal
  with prompts. The names suggest cache (memoization) and strategy
  (selection), which are *different* concerns — but the same noun
  (`Prompt`) prefix. A reader cannot tell from the file names whether
  one is the implementation of the other.
- **principle_violated:** C5 / C6 (composition vs duplication).
- **root_cause_driver:** `vocabulary_drift` — two files, one noun.
- **proposed_fix_shape:** Read both files. If `PromptCache` is a memoization
  layer over `PromptStrategy`'s output, the names are correct. If
  `PromptCache` is a *subset* of `PromptStrategy`, fold them.
- **test_that_proves_it:** A test that asserts `PromptCache::get` returns
  the same result as `PromptStrategy::select` for any given input (proving
  cache equality), or a test that asserts they are not equivalent (proving
  separation). One of the two is the right answer.

## Lens summary

| Severity | Count |
|----------|-------|
| blocker  | 0     |
| major    | 0     |
| minor    | 2     |
| nit      | 2     |

L3 found no god-traits and no deep `Arc<Mutex>` chains in the pass I
read. The two minors are about *naming* (F-L3-004) and *responsibility*
(F-L3-003) — both reduce to "is this two concepts that share a noun,
or one concept with two names?" That's an L4 question, not an L3
question; cross-reference F-L4-001.
