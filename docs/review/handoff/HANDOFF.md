# hKask Adversarial Review — HANDOFF

> **You are a fresh agent, dropped into this repo, asked to continue
> the review.** This file is your index. Read it first; everything
> else is a reference.

## TL;DR

A 9-task adversarial review of the hKask codebase (11 core crates +
21 MCP servers) was run on 2026-06-06. The review produced:

- **1 blocker** finding (`OcapCapability::String` is a forgeable
  capability).
- **7 major** findings, all in the capability / visibility / cybernetics
  surface.
- **12 minor / nit** findings, mostly type width and untested seams.
- **5 positive observations** (drift gates), to be added as CI tests.
- **10 open questions** in `future/FUTURE.md` that block specific
  refactors until decided.

The review's **deliverable is observation**, not code. Code changes
belong in PRs, one finding per PR, with a red test committed first.

## Where everything lives

```
review/
├── charter/
│   └── CHARTER.md                # The contract a reviewer signs before producing findings.
├── graphs/
│   ├── func.ttl                  # 65 concepts (the user's mental model)
│   ├── tech.ttl                  # 32 crates + 1878 public types + 91 span refs
│   ├── joined.ttl                # concept ↔ crate realization edges
│   └── gaps.md                   # 0 unhomed concepts, 1 classified phantom
├── erds/
│   ├── README.md                 # Index + drift-guard
│   └── *.mmd                     # 8 Mermaid ERDs / classDiagrams
├── findings/
│   ├── L1-type-theoretic.md      # 6 findings (Hoare)
│   ├── L2-capability.md          # 6 findings (Mark Miller)
│   ├── L3-composition.md         # 4 findings (Miller / Fowler)
│   ├── L4-minimalism.md          # 4 findings (Planck)
│   ├── L5-cybernetics.md         # 5 findings (Wiener / Beer)
│   ├── L6-adversarial.md         # 5 findings (Red-team)
│   ├── SYNTHESIS.md              # 25 dedup'd findings, prioritized
│   └── T4-hand-off.md            # 25-PR index, one row per F-SYN-XXX
├── future/
│   └── FUTURE.md                 # 10 open questions (FUT-001 .. FUT-010)
└── handoff/
    └── HANDOFF.md                # ← this file
```

## The blocker (read first)

**F-SYN-001**: `OcapCapability::String` lets any caller mint any
capability:

```rust
OCAPBoundary::explicit("memory:write:any-webid")
```

This is a forgeable capability. The fix is to delete the `String`
variant of `OcapCapability` and migrate `explicit()` to take an
`OcapTokenKind` via a `from_known_action` lookup. The full PR spec
is in `findings/SYNTHESIS.md` §F-SYN-001 and the routing is in
`findings/T4-hand-off.md`.

## The 7 majors (in priority order)

1. **F-SYN-002** — `OCAPBoundary::enforced: bool` allows an unenforceable boundary.
2. **F-SYN-003** — `is_shared`/`is_public` collide on two types.
3. **F-SYN-004** — Visibility-flip path doesn't clear perspective.
4. **F-SYN-005** — `cns.clone` and `cns.read` spans emitted from two crates.
5. **F-SYN-006** — Attenuation chain replay: revoke + re-mint may not fail.
6. **F-SYN-007** — MCP capability gate ordering asserted for 2 of 21 servers.
7. **F-SYN-008** — `MemoryStoragePort` is a port, not a membrane.

Each has a one-line fix shape, a single file to touch, and a
compile-fail or red→green test. See `findings/SYNTHESIS.md`.

## The 12 minors + 5 nits (positive observations)

See `findings/SYNTHESIS.md` F-SYN-009 through F-SYN-025. The 5
positives (F-SYN-021 .. F-SYN-025) are CI drift gates — no fix
needed, just tests.

## The 10 open questions (blockers for refactors)

See `future/FUTURE.md`. The most pressing:

- **FUT-004** — Is every MCP server a membrane, or are some passthroughs?
  Blocks F-SYN-007.
- **FUT-001** — Is `SYSTEM_MAX_RECURSION = 7` a constant or a config?
  Blocks F-SYN-010, F-SYN-011.
- **FUT-003** — Is `Dampener.override_cooldown` per-issuer or global?
  Blocks F-SYN-012.

## How to resume work

### If you are a contributor picking up a finding

1. Read `charter/CHARTER.md` (5 min).
2. Find your finding in `findings/SYNTHESIS.md`. Read its evidence,
   principle violated, and test_that_proves_it.
3. Find your PR's row in `findings/T4-hand-off.md`. Note the
   *primary file* — touch only that file unless the synthesis
   explicitly says otherwise.
4. Write the red test first. Confirm it fails for the right reason.
5. Implement the fix. Make the test green.
6. Run `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`.
7. Open the PR with the title `F-SYN-NNN: <one-line description>`.

### If you are a reviewer continuing the review

1. Read `charter/CHARTER.md`.
2. The 8 ERDs in `erds/` are the contracts you start from.
3. The semantic graph in `graphs/joined.ttl` is your map. A new
   finding either:
   - Connects to an existing concept (F-SYN-NNN prefix), or
   - Requires a new concept (extend `func.ttl` first, then `joined.ttl`).
4. The 6 lens files are examples of the finding schema. Match the
   schema for every new finding.

### If you are a designer (ADR author)

1. Read `future/FUTURE.md`. Each entry is a one-paragraph ADR away
   from being decided.
2. The capability model in `findings/T6-capability-model.md` is the
   design context.
3. The marker "blocks F-SYN-NNN" tells you which PRs are waiting
   on the decision.

## Conventions used throughout

- **Finding IDs**: `F-L<N>-<NNN>` for raw findings, `F-SYN-NNN` for
  synthesized (dedup'd, prioritized). `FUT-NNN` for open questions.
- **Severity**: blocker / major / minor / nit. Action: see
  `findings/SYNTHESIS.md` §"Severity legend."
- **Concept IDs**: `concept:<slug>` from `func.ttl`. Used in every
  finding to anchor the review to the semantic graph.
- **Tech IDs**: `tech:<crate_slug>` from `tech.ttl`. Used in every
  finding to anchor the review to the code.
- **No new languages**: the review is in markdown, Mermaid, and
  Turtle. Do not add Python, JS, or shell helpers to `review/`.
  If you need a tool, write it as a `#[test]` in a crate.

## What changed in the repo (vs HEAD)

**Zero source mutations.** The review is observation only.

Untracked files (the review's deliverable):

```
review/charter/CHARTER.md
review/graphs/{func,tech,joined}.ttl
review/graphs/gaps.md
review/erds/README.md
review/erds/*.mmd (8)
review/findings/L*.md (6)
review/findings/SYNTHESIS.md
review/findings/T{4,5,6}-*.md
review/future/FUTURE.md
review/handoff/HANDOFF.md
```

All under `review/`. All in the project's existing toolchain
(markdown, Mermaid, Turtle). All hand-authored (the original Python
generators were removed after the user objected to non-Rust code in
the repo).

## Verification commands (re-runnable)

```bash
# Inventory
ls review/charter/CHARTER.md                                       # charter
ls review/erds/*.mmd | wc -l                                       # 8
ls review/findings/L*.md | wc -l                                   # 6
ls review/findings/SYNTHESIS.md                                    # synthesis
ls review/findings/T*.md | wc -l                                   # 3 (T4, T5, T6)
ls review/future/FUTURE.md                                         # future
ls review/handoff/HANDOFF.md                                       # this file

# Synthesis integrity (severity is in the H3 title, not a `severity:` field)
rg -o 'F-SYN-[0-9]+' review/findings/SYNTHESIS.md | sort -u | wc -l  # 25
rg -c '^### F-SYN' review/findings/SYNTHESIS.md                       # 25
rg -c 'BLOCKER' review/findings/SYNTHESIS.md                          # 1  (F-SYN-001)
rg -c '\(MAJOR\)' review/findings/SYNTHESIS.md                        # 7  (F-SYN-002..008)
rg -c '\(MINOR\)' review/findings/SYNTHESIS.md                        # 7  (F-SYN-009,013..016,019,020)
rg -c '\(NIT' review/findings/SYNTHESIS.md                            # 9  (F-SYN-010,011,017,018 + 5 positive)
rg -c '\(DESIGN EXERCISE' review/findings/SYNTHESIS.md                # 1  (F-SYN-012)
echo "Total: $((1+7+7+9+1))"                                          # 25

# Future integrity
rg -c '^## FUT-' review/future/FUTURE.md                              # 10

# No foreign code (review is markdown + Mermaid + Turtle only)
find review/ -name '*.py' -o -name '*.js' -o -name '*.sh' | wc -l  # 0
```

If any of these commands produces an unexpected count, the review
artifact has drifted from the contract. Re-run the inventory; if
the drift is real, the responsible PR is the one that produced the
drift.
