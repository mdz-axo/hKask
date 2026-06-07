# L5 — Cybernetic / CNS Review (Wiener / Beer)

> Persona: the CNS is a homeostatic feedback loop. The primitives are
> variety, algedonic alerts, backpressure, override cooldown, and
> load-shedding. The question for every span emission: *what consumes
> it, and what action does the consumer take?*
>
> Method: walk every `cns.*` span in the codebase. For each, find the
> consumer. If the consumer doesn't exist, it's an alert-orphan.
> For every backpressure counter, find the load-shedding path. If it
> doesn't exist, it's a counter-orphan.

## Span-to-consumer map

| Span prefix | Source | Consumer | Action |
|-------------|--------|----------|--------|
| `cns.agent_pod.*` | `hkask-agents/src/pod/{mod,manager}.rs` | (none asserted in tests) | lifecycle event persistence |
| `cns.algedonic` | `hkask-cns/src/{algedonic, cybernetics_loop}.rs` | `algedonic_alert` emission | threshold R-bar 0.3 / 0.8 (lines 35,40) |
| `cns.cybernetics` | `hkask-cns/src/cybernetics_loop.rs` | `Dampener` | override suppression |
| `cns.cybernetics.backpressure` | `hkask-cns/src/cybernetics_loop.rs` | `Arc<AtomicU64>` | load shedding (queue depth > 100) |
| `cns.clone` | `hkask-cli/src/repl/mod.rs` AND `hkask-cns/src/cybernetics_loop.rs` | (two different ones, different semantics) | unclear |
| `cns.curation` | `hkask-cns/src/cybernetics_loop.rs` | `CurationLoop` | spec drift handling |
| `cns.hhh.*` | `hkask-agents/src/hhh_gate.rs` | `cns.hhh.persona` (filter) | persona stripping |
| `cns.inference.*` | `hkask-agents/src/inference_loop.rs`, `prompt_analysis.rs` | `cns.inference` (governance) | gas-budget enforcement |
| `cns.memory.*` | `hkask-agents/src/adapters/memory_loop_adapter.rs` | (no consumer found in survey) | encode + budget |
| `cns.read` | `hkask-cli/src/repl/{mod,commands}.rs` AND `hkask-cns/src/cybernetics_loop.rs` | (same name, two definitions — vocabulary drift) | unclear |
| `cns.spec.*` | `hkask-agents/src/curator_agent/spec_curator.rs` | `CurationLoop` | spec drift alerts |
| `cns.template.russell_mapping` | `hkask-cli/src/commands/russell/mapper.rs` | (no consumer found) | template mapping |
| `cns.tool` | `hkask-cns/src/governed_tool.rs` | `GovernedTool` | invocation governance |
| `cns.variety` | `hkask-cns/src/cybernetics_loop.rs` | `Algedonic` | threshold check |
| `cns.health` | `hkask-cns/src/cybernetics_loop.rs` | (assumed internal) | health probe |

## Findings

### F-L5-001 — `cns.clone` and `cns.read` are emitted from two different crates with different semantics (vocabulary drift across the observability layer)

- **id:** F-L5-001
- **location:** `crates/hkask-cli/src/repl/mod.rs` (cns.clone, cns.read)
  AND `crates/hkask-cns/src/cybernetics_loop.rs` (cns.clone, cns.read)
- **concept:** `concept:cybernetics_loop`
- **severity:** major
- **evidence:** The same span prefix is emitted from two crates. A
  consumer that filters `cns.clone` will pick up both — but the
  semantics differ (CLI clone command vs cybernetics-loop clone
  operation). This is the same finding as L4-001 in the span namespace.
- **principle_violated:** C3 (CNS spans are the only observability
  primitive — but the namespace is overloaded). C6 (one way).
- **root_cause_driver:** `vocabulary_drift` — span namespaces are a
  vocabulary too.
- **proposed_fix_shape:** Reserve span namespaces by `tech:` slug:
  `cns.cli.clone`, `cns.cybernetics.clone`. Add a CI test that
  asserts every `cns.*` span is emitted from exactly one `tech:`
  crate. Lives in `crates/hkask-cns/tests/span_namespace_invariant.rs`.
- **test_that_proves_it:** Red: write the test for one namespace and
  confirm it fails for `cns.clone`. Green: rename and confirm it passes.

### F-L5-002 — `cns.memory.*` spans have no asserted consumer (alert-orphan)

- **id:** F-L5-002
- **location:** `crates/hkask-agents/src/adapters/memory_loop_adapter.rs`
  (cns.memory.budget, cns.memory.encode)
- **concept:** `concept:memory_consolidation`
- **severity:** minor
- **evidence:** I did not find a consumer in the surveyed surface.
  The spans are emitted; whether anything reads them is unverified.
- **principle_violated:** C3 (CNS span without consumer = observability
  without action = noise).
- **root_cause_driver:** `alert_orphan`.
- **proposed_fix_shape:** Audit the spans: if they are *only* for
  tracing visibility, change them to `tracing::trace!` (lower
  cardinality). If they are *for* the cybernetics loop, add the
  consumer in `hkask-cns/`.
- **test_that_proves_it:** A static test in `crates/hkask-cns/` that
  walks the `cns.*` namespace and asserts every emission has a
  paired `tracing::span!` consumer in the same or a downstream crate.
  The test compiles a list of emissions from `rg` and a list of
  consumers from `rg`, and asserts the second is a superset of the
  first.

### F-L5-003 — `cns.template.russell_mapping` is a one-off span that should be a CNS span or a tracing event, not both

- **id:** F-L5-003
- **location:** `crates/hkask-cli/src/commands/russell/mapper.rs`
- **concept:** `concept:russell_bridge`
- **severity:** nit
- **evidence:** A single span prefix used by a single mapping command.
  Not part of the cybernetic vocabulary; it's CLI command telemetry.
- **principle_violated:** C3 (span namespace is the cybernetics
  vocabulary, not the CLI vocabulary).
- **root_cause_driver:** `shallow_module` — the CLI reused the span
  primitive because it was convenient.
- **proposed_fix_shape:** Either (a) rename to `cli.russell.mapping`
  to mark it as CLI telemetry, or (b) move the mapping to a CNS
  span if the cybernetics loop should observe it.
- **test_that_proves_it:** A doc-comment on the span that names it
  as CLI or CNS. The test is the rename.

### F-L5-004 — `Dampener.override_cooldown` is global, not per-issuer (FUTURE design exercise)

- **id:** F-L5-004
- **location:** `crates/hkask-cns/src/dampener.rs:58,90-115`
- **concept:** `concept:override_cooldown`
- **severity:** minor
- **evidence:** The cooldown is a single `Duration` field on the
  `Dampener` struct, not keyed by issuer. After any override passes
  dedup, all subsequent overrides (from any issuer) within 120s are
  suppressed.
- **principle_violated:** C4 (Q6 — an override from issuer A can
  suppress an override from issuer B, which means B's authority is
  bound by A's history).
- **root_cause_driver:** `shallow_module` — the design is correct
  for *single-issuer* systems, underspecified for *multi-issuer*.
- **proposed_fix_shape:** This is a *design exercise*, not a refactor.
  Move to `review/FUTURE.md` as `FUT-003`. Do not act on it as a
  refactor.
- **test_that_proves_it:** A doc-test on `Dampener` that asserts the
  current behavior (global cooldown) and the planned behavior
  (per-issuer cooldown) in two separate test cases.

### F-L5-005 — `DEFAULT_COMMUNICATION_BACKPRESSURE_THRESHOLD = 100.0` is a magic number exposed in two places (gate against drift)

- **id:** F-L5-005
- **location:** `crates/hkask-types/src/cns.rs:63` (definition),
  `crates/hkask-cns/src/set_points.rs:36` (re-export)
- **concept:** `concept:backpressure`
- **severity:** nit
- **evidence:** The constant lives in `hkask-types` (the right place
  for shared primitives) and is re-exported by `hkask-cns`. This is
  correct composition, not a duplicate. The finding is to *gate
  against drift*: if the threshold is ever defined in two places, it
  will be wrong.
- **principle_violated:** C6 (one way) — *to be kept*.
- **root_cause_driver:** n/a (positive observation; gate against drift).
- **proposed_fix_shape:** A static test: `rg 'BACKPRESSURE' crates/ | wc -l`
  should equal exactly 2 (definition + re-export). If it grows, a
  review is needed.
- **test_that_proves_it:** The static test.

## Lens summary

| Severity | Count |
|----------|-------|
| blocker  | 0     |
| major    | 1     |
| minor    | 2     |
| nit      | 2     |

L5's major is about *span namespace pollution* (F-L5-001) — the same
finding L4-001 makes for the type vocabulary. The principle (one
name, one place) applies equally to the observability surface; the
review just measures it with a different tool.

The two positive observations (F-L5-004 global cooldown, F-L5-005
single-source threshold) are exactly the gates the next reviewer
should not have to re-derive.
