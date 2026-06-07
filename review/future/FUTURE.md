# Task 8 — Future (Open Questions & Underspecified Aspects)

> **Function:** capture every finding, observation, or implication from
> Tasks 0–7 that this review *cannot* resolve, so a future agent (or a
> future human) can pick up the work without re-reading the codebase.
>
> The synthesis's "Fix shape" sections are the *refactor* hand-off;
> this file is the *design* hand-off. Entries here are *not* bugs;
> they are decisions that need a human or an ADR.

## The minimum required entries

The action plan specified that FUT-001 through FUT-006 must be present;
absence of any one of them is itself a finding. I have them all.

---

## FUT-001 — Whether the 7-level attenuation limit is a hard architectural constant or a configurable cap

- **raised_by:** T3 (L1-003, L1-006, L2 in general)
- **category:** capability
- **question:** Is `SYSTEM_MAX_RECURSION = 7` (and by extension
  `SYSTEM_MAX_ATTENUATION = 7`) a *physical constant* of the system
  (like Planck's constant for the OCAP domain) or a *configurable
  cap* that may be raised? The codebase currently has it as a `const`
  in `crates/hkask-types/src/capability/mod.rs:7`, and a *parallel*
  `MAX_GOAL_DEPTH: u8 = 7` is *proposed* in F-SYN-011 but not yet
  defined. If 7 is configurable, both need a `set_max()` (or similar)
  on the same `Config` struct. If 7 is a constant, the second `7`
  should be a *named constant* that *aliases* the first, not a new
  literal.
- **decision_needed_from:** ADR
- **blocking:** F-SYN-010 (AttenuationLevel newtype), F-SYN-011
  (MAX_GOAL_DEPTH), and any future L1-006 follow-ups. If 7 is
  configurable, the newtype is wrong; the right shape is a
  config struct.
- **minimum_resolvable_artifact:** A one-paragraph ADR with the
  question, two options, and the chosen answer. The synthesis
  F-SYN-010 / F-SYN-011 PRs then proceed accordingly.

## FUT-002 — Whether `lambda_for_category` should be public

- **raised_by:** T2 (ERD #4), T3 (F-L1-002 / F-SYN-009)
- **category:** observability
- **question:** `lambda_for_category` is `fn` (private) in
  `crates/hkask-storage/src/nu_event_store.rs:105`. The dispatch
  table is fixed (5 categories) and the test surface asserts each
  arm. Should the dispatch key be a `pub enum SpanCategory` (per
  F-SYN-009), and should `lambda_for_category` be `pub`?
- **decision_needed_from:** ADR (small)
- **blocking:** F-SYN-009. If the answer is "yes, make it pub", the
  fix is one PR. If "no, keep it private", F-SYN-009 is moot.
- **minimum_resolvable_artifact:** An ADR paragraph saying "the
  dispatch table is part of the storage implementation, not a public
  port; new categories require a code change." Then F-SYN-009 is
  closed as "by design."

## FUT-003 — Whether `Dampener.override_cooldown` is per-issuer or global

- **raised_by:** T3 (F-L5-004 / F-SYN-012)
- **category:** trust
- **question:** The `Dampener` struct holds a single `Duration` field
  and a single `Instant` (`last_override`). After *any* override
  passes dedup, *all* subsequent overrides (from any issuer) within
  120s are suppressed. Is this the intended semantics? For a
  multi-issuer system, the answer is "probably not" — issuer A's
  override should not bound issuer B's.
- **decision_needed_from:** Design exercise (spec change)
- **blocking:** Any future multi-issuer scenario. Currently
  hKask has effectively one issuer per pod, so the global cooldown
  is correct *for now*. The design exercise is for v1.0+.
- **minimum_resolvable_artifact:** A design note saying either
  (a) "Dampener is a single-issuer primitive; multi-issuer requires
  a `DashMap<IssuerWebID, Dampener>` wrapper at the consumer level,"
  or (b) "Dampener becomes keyed by issuer as a breaking change in
  v1.0." Either is fine; the choice blocks the design of any
  multi-issuer test in the future.

## FUT-004 — Whether the MCP gateway is a membrane or a passthrough

- **raised_by:** T2 (ERD #8), T3 (F-L2-006 / F-SYN-007)
- **category:** composition
- **question:** The plan and the L2 review treat MCP servers as
  *membranes* (Mark Miller sense: they wrap a canonical realization
  and translate capabilities into possibly-attenuated ones). The
  `cns.*` span map shows that some MCP servers emit their own spans
  in addition to the canonical span, which is consistent with
  membrane behavior. But the *capability* dimension is unverified:
  the gate exists for 2 of 21 servers (per F-SYN-007). Is the
  gateway a membrane for *all* servers, or is it a passthrough for
  *some* and a membrane for *others*?
- **decision_needed_from:** ADR (architectural)
- **blocking:** F-SYN-007. If the answer is "all MCP servers are
  membranes, no exceptions," the test is universal. If the answer
  is "mcp-X is a passthrough," the test must enumerate the
  exceptions.
- **minimum_resolvable_artifact:** An ADR that names each of the
  21 servers and classifies it as "membrane" or "passthrough,"
  with the criteria for the classification. The F-SYN-007 test
  then either asserts "all are membranes" (universal) or enumerates
  the passthroughs.

## FUT-005 — Whether `SpecId` is a brand or a plain ID

- **raised_by:** T3 (L2 in general), the `joined.ttl` review
- **category:** capability
- **question:** The `Spec` type has an `id: SpecID` field. Other
  branded types in the system have explicit `Brand` markers
  (e.g. `OcapTokenKind`, `WebID`). Does `SpecID` carry a similar
  brand, or is it a plain `String`? If plain, anyone can construct
  a `Spec` referencing any other `Spec`; if branded, only the
  curator can.
- **decision_needed_from:** Read `crates/hkask-types/src/spec.rs`
  (or wherever `SpecID` is defined) and the `SpecStore` consumer.
- **blocking:** None today. Potentially a finding in a future
  capability review if `SpecID` turns out to be a string.
- **minimum_resolvable_artifact:** A two-line note in the Spec
  type's module docstring: "`SpecID` is a brand; the only way to
  construct one is through the curator or the spec-goal-capture
  MCP tool." Or, if it's a string: "`SpecID` is a plain string;
  trust is established by the *consumer*, not the type."

## FUT-006 — Whether `GoalCapabilityToken` (removed in v0.23.0) has any resurrection path

- **raised_by:** T3 (F-L6-003 / F-SYN-019), the v6 handoff
- **category:** lifecycle
- **question:** Per OPEN_QUESTIONS F6, `GoalCapabilityToken` was
  *entirely removed* in v0.23.0. The removal was correct
  (over-engineered ceremony). The question is: what stops a future
  contributor from re-adding HMAC signing for goals? F-SYN-019
  proposes a grep test as a regression guard. Is that enough, or
  should the absence be enforced by a compile-time assertion (e.g.
  a `compile_error!` in a guarded module)?
- **decision_needed_from:** Human (or senior reviewer)
- **blocking:** None today. The grep test is a *soft* guard; a
  *hard* guard would be a `compile_error!("do not reintroduce
  GoalCapabilityToken — see OPEN_QUESTIONS F6")` placed in the
  `goal.rs` module.
- **minimum_resolvable_artifact:** A one-line decision: "soft guard
  (grep test) is sufficient; the OPEN_QUESTIONS citation is enough
  to deter re-introduction." Or: "hard guard via compile_error! is
  the right shape; F-SYN-019 is upgraded to a build-time assertion."

## Additional entries (beyond the plan's minimum six)

These are findings from the review that resolved into design
exercises rather than refactors.

## FUT-007 — Per-issuer override cooldown (F-SYN-012)

- **raised_by:** F-SYN-012
- **category:** regulation
- **question:** Same as FUT-003 but framed as the F-SYN-012
  follow-up. If FUT-003 is "single-issuer is fine for v0.x," this
  question becomes a v1.0+ issue. If FUT-003 is "must be
  per-issuer in v0.24," this is current work.
- **decision_needed_from:** Whichever path FUT-003 takes.
- **minimum_resolvable_artifact:** Same as FUT-003.

## FUT-008 — Russell bridge revocation granularity (F-SYN-017)

- **raised_by:** F-L2-004, F-SYN-017
- **category:** trust
- **question:** `RussellAcpAdapter` uses an HKDF-derived shared
  secret. Revocation = master rotation, which is global. If
  multiple Russell bridges need independent revocation, the
  current design does not support it. (This is *not* a problem
  today — there is one bridge — but it is a future problem.)
- **decision_needed_from:** ADR when the second bridge is needed.
- **minimum_resolvable_artifact:** Same as FUT-003 — the ADR
  records the design for future bridges.

## FUT-009 — Span namespace `cns.cli.*` vs `cns.cybernetics.*` (F-SYN-005)

- **raised_by:** F-L5-001, F-SYN-005
- **category:** observability
- **question:** F-SYN-005 proposes renaming `cns.clone` →
  `cns.cli.clone` / `cns.cybernetics.clone`. The principle (one
  span prefix per emitting crate) should be ADR'd before the
  rename, because the rename is *incompatible* with any consumer
  that filters on the old prefix.
- **decision_needed_from:** ADR (small)
- **blocking:** F-SYN-005.
- **minimum_resolvable_artifact:** An ADR paragraph with the
  naming rule, an enumeration of the *current* violations, and the
  proposed renames. The PR for F-SYN-005 then cites the ADR.

## FUT-010 — `fuzz_tool_inputs` (F-SYN-020) — `proptest` vs `cargo-fuzz`

- **raised_by:** F-SYN-020
- **category:** testing
- **question:** F-SYN-020 proposes a `proptest` integration test
  for MCP tool inputs. A heavier alternative is `cargo-fuzz`, which
  requires nightly and a separate CI lane. The trade-off: `proptest`
  catches *most* parser-level panics; `cargo-fuzz` catches *all*
  panics via libFuzzer.
- **decision_needed_from:** Human (CI cost).
- **blocking:** F-SYN-020.
- **minimum_resolvable_artifact:** A note in the MCP crate's CI
  config saying either "proptest is enough" or "add cargo-fuzz to
  the nightly lane."

## Index of all open questions

| ID | Category | Severity in this review | Status |
|----|----------|-------------------------|--------|
| FUT-001 | capability | blocks F-SYN-010, F-SYN-011 | undecided |
| FUT-002 | observability | blocks F-SYN-009 | undecided |
| FUT-003 | trust | blocks F-SYN-012 | undecided |
| FUT-004 | composition | blocks F-SYN-007 | undecided |
| FUT-005 | capability | no immediate block | unverified |
| FUT-006 | lifecycle | blocks F-SYN-019's *strength* | undecided |
| FUT-007 | regulation | sibling of FUT-003 | undecided |
| FUT-008 | trust | future-only | undecided |
| FUT-009 | observability | blocks F-SYN-005 | undecided |
| FUT-010 | testing | blocks F-SYN-020 | undecided |

## What this file is *not*

- This is **not** a bug tracker. Bugs go in synthesis. Design
  questions go here.
- This is **not** a backlog. The synthesis is the backlog. This
  file is the *unblocking* layer: every entry here, when decided,
  unblocks one or more synthesis findings.
- This is **not** a `DOCS/` file. It lives in `review/` because
  it is part of the review, not part of the system.
