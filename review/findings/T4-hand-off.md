# Task 4 — Codebase Deepening (Hand-off Index)

> **Out of scope for the review.** The synthesis is the contract; this
> index is the routing. Each entry points to the synthesis finding and
> names the *single* file the change should touch (or the *fewest* files,
> per the charter's "surgical" rule).
>
> **No code in this file.** Code belongs in PRs, each with a red test
> committed first per the `coding-guidelines` skill (Karpathy rule 4).

## How to pick up a finding

1. Read the synthesis entry. Every entry has: location, evidence,
   principle violated, fix shape, test that proves it.
2. Read the test that proves it *first*. The test is the spec; the
   fix is whatever makes the test pass.
3. Touch only the file(s) named in the *location* field. If the fix
   requires touching > 3 crates, it is a design exercise — promote
   it to `FUTURE.md` and stop.
4. Run `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`.

## The 25 PRs (one per F-SYN-XXX), in priority order

| ID | Sev | Concept | Primary file | Touches | One-line change |
|----|-----|---------|--------------|---------|-----------------|
| F-SYN-001 | **blocker** | `ocap` | `crates/hkask-types/src/curation.rs` | + `OPEN_QUESTIONS.md` migration table | Delete `OcapCapability::String` variant; migrate `explicit()` to `OcapTokenKind`. |
| F-SYN-002 | major | `ocap_boundary` | `crates/hkask-types/src/curation.rs` | (same file) | Delete `OCAPBoundary::enforced` field; update 2 constructors. |
| F-SYN-003 | major | `visibility` | `crates/hkask-types/src/{visibility,sovereignty}.rs` | + 1 new `AccessMode` enum | Move `is_shared`/`is_public` to `AccessMode`; delegate from both. |
| F-SYN-004 | major | `episodic_memory` | `crates/hkask-storage/src/triples.rs` | (single file) | Visibility-flip path: refuse or clear `perspective`. |
| F-SYN-005 | major | `cybernetics_loop` | `crates/hkask-cli/src/repl/mod.rs` + `crates/hkask-cns/src/cybernetics_loop.rs` | (2 files) | Rename `cns.clone` → `cns.cli.clone` / `cns.cybernetics.clone`; same for `cns.read`. |
| F-SYN-006 | major | `attenuation`/`revocation` | `crates/hkask-types/src/capability/mod.rs` | (single file) | Audit `issuance()`; add revocation-log check; add the re-mint test. |
| F-SYN-007 | major | `capability_gate` | `crates/hkask-mcp/tests/capability_gate_invariant.rs` (new) | (new file) | Static test walking all 21 MCP servers' tool handlers. |
| F-SYN-008 | major | `revocable_forwarder` | `crates/hkask-agents/src/pod/mod.rs` | (single file) | Make `MemoryStoragePort::None` *the* no-write behavior; add test. |
| F-SYN-009 | minor | `span_category` | `crates/hkask-storage/src/nu_event_store.rs` | (single file) | Newtype `SpanCategory`; convert `lambda_for_category`. |
| F-SYN-010 | nit | `attenuation` | `crates/hkask-types/src/capability/mod.rs` | (single file) | `AttenuationLevel(u8)` newtype; replace two `u8` fields. |
| F-SYN-011 | nit | `sub_goal` | `crates/hkask-storage/src/goals.rs` | (single file) | `MAX_GOAL_DEPTH = 7`; reject past cap. |
| F-SYN-012 | minor | `override_cooldown` | (none — see FUTURE.md) | (none) | Design exercise; **no refactor**. |
| F-SYN-013 | minor | `memory_consolidation` | `crates/hkask-agents/src/adapters/memory_loop_adapter.rs` | (single file) | Audit `cns.memory.*` spans; demote or wire. |
| F-SYN-014 | minor | `delegation_token` | `crates/hkask-types/src/capability/mod.rs` | (single file) | Audit `verify()` for `expires_at`; add negative test. |
| F-SYN-015 | minor | `memory_loop` | `crates/hkask-agents/src/adapters/memory_loop_adapter.rs` | (single file) | Rename `MemoryLoopAdapter` → `MemoryLoopForwarder`; document. |
| F-SYN-016 | minor | `prompt` | `crates/hkask-templates/src/{prompt_cache, prompt_strategy}.rs` | (2 files) | Read both; classify parallel/facet; act. |
| F-SYN-017 | nit | `russell_bridge` | (none — see FUTURE.md) | (none) | Design exercise; **no refactor**. |
| F-SYN-018 | nit | `russell_bridge` | `crates/hkask-cli/src/commands/russell/mapper.rs` | (single file) | Rename `cns.template.russell_mapping` → `cli.russell.mapping`. |
| F-SYN-019 | minor | `goal` | `crates/hkask-types/src/goal.rs` | (single file) | Doc-comment naming OPEN_QUESTIONS F6; add grep test. |
| F-SYN-020 | minor | `mcp_tool` | `crates/hkask-mcp/tests/fuzz_tool_inputs.rs` (new) | (new file) | `proptest` integration test over all 21 servers. |
| F-SYN-021 | nit | `backpressure` | `crates/hkask-cns/tests/arc_mutex_baseline.rs` (new) | (new file) | `rg 'Arc<Mutex<' | wc -l ≤ baseline` test. |
| F-SYN-022 | nit | `port` | `crates/hkask-types/tests/port_invariants.rs` (new) | (new file) | `pub trait` with > 5 methods fails CI. |
| F-SYN-023 | nit | `backpressure` | `crates/hkask-cns/tests/backpressure_source_invariant.rs` (new) | (new file) | `rg 'BACKPRESSURE' | wc -l == 2` test. |
| F-SYN-024 | nit | `curation_loop` | `crates/hkask-types/src/{curation,goal}.rs` | (2 files) | Doc-comment naming the other enum. |
| F-SYN-025 | nit | `goal` | `crates/hkask-storage/src/goals.rs` + `crates/hkask-templates/src/{registry,ports}.rs` | (3 files) | Read the four `new()`s; classify; act. |

## Anti-pattern (do not do this)

- Do **not** bundle two findings into one PR. A PR that touches F-SYN-001
  and F-SYN-002 in the same commit forces a revert if either fails. The
  one-finding-per-PR rule is the same as the one-responsibility-per-crate
  rule; the granularity is the granularity.
- Do **not** add a `#[deprecated]` attribute. Per P7, fix forward. If a
  migration is needed, document it in `OPEN_QUESTIONS.md`.
- Do **not** introduce a Python, JS, or shell-script helper into
  `review/`. The project is Rust; the review is observation.

## Validation per PR

Before opening the PR:

```bash
cargo test -p <touched-crate>          # unit + integration
cargo test --workspace                 # nothing else broke
cargo clippy --workspace -- -D warnings
rg -c '\bpanic!\b' crates/<touched-crate>/src/   # no new panics on hot paths
rg -c '\.unwrap()\b' crates/<touched-crate>/src/  # no new unwraps (C5)
```

After opening the PR:

- The red test (committed first) is referenced in the PR description.
- The synthesis finding ID is in the PR title: e.g. `F-SYN-001: delete OcapCapability::String`.
- The principle violated is named in the PR body, with the citation
  (`C4 Q1`, `P8`, `DDMVSS-§12`).
- The new test is in the *right* test file per the table above.

## Effort estimate (rough)

| Severity | Count | LoC per fix | Notes |
|----------|-------|-------------|-------|
| blocker  | 1     | ~30         | Type-system change; fast. |
| major    | 7     | ~50–150     | Varies; F-SYN-003 and F-SYN-007 touch more files. |
| minor    | 8     | ~10–50      | Most are rename or newtype. |
| nit      | 9     | ~5–20       | Doc-comments and CI gates. |

Total: roughly 1000–1500 LoC across 25 PRs, of which 7 are non-trivial
(the majors). The blocker and the two `major`s that involve MCP-wide
tests (F-SYN-007, F-SYN-020) are the only ones that need a *workspace-
level* build to validate.
