# SYNTHESIS — hKask Adversarial Review

> Single document the project acts on. Each finding has one ID,
> one location, one concept, one principle violated, and one red→green
> test. The raw findings are in `L1-…` through `L6-…`; this file
> is the deduplicated, prioritized, and actionable form.
>
> **Total: 25 unique findings (after dedup of 29 raw findings).**

## Severity legend

| Severity | Meaning | Action |
|----------|---------|--------|
| blocker  | Exploitable attack surface or architectural invariant violation. | Block release. |
| major    | Ambient authority, untested invariant, or principle violation with concrete impact. | Fix in current sprint. |
| minor    | Type width, vocabulary drift, or untested seam. | Fix in next sprint or document. |
| nit      | Drift gate, positive observation, or sub-sprint cleanup. | File issue; don't block. |

## Synthesized findings (prioritized)

### F-SYN-001 — `OcapCapability::String` is a forgeable capability (BLOCKER)

> **Originals:** F-L1-001, F-L2-002, F-L6-001 (3 lenses, 1 finding).
> **Severity:** blocker (the only one in this review).
> **Concept:** `concept:ocap`
> **Location:** `crates/hkask-types/src/curation.rs:104`

**Evidence:** `OcapCapability::String(String)` lets any caller mint
any capability by constructing a string:
`OCAPBoundary::explicit("memory:write:any-webid")`.

**Principle violated:** C4 (Q1 — forgeable), P8 (unverified by a test).

**Root cause:** `ambient_authority` — strings are the ambient authority
of every programming language.

**Fix shape (one PR, one test):**
1. Red: a test that constructs
   `OCAPBoundary::explicit("memory:write:any-webid")` and asserts the
   call site fails to compile.
2. Green: delete the `String` variant of `OcapCapability`; migrate
   `explicit()` to take `OcapTokenKind` via `from_known_action`.
3. Migration table in `OPEN_QUESTIONS.md` (per P7, not `#[deprecated]`).

**Test that proves it:** A compile-fail test in
`crates/hkask-types/tests/no_string_capability.rs`.

### F-SYN-002 — `OCAPBoundary::enforced: bool` allows an unenforceable boundary (MAJOR)

> **Originals:** F-L1-004, F-L2-001 (2 lenses, 1 finding).
> **Severity:** major.
> **Concept:** `concept:ocap_boundary`
> **Location:** `crates/hkask-types/src/curation.rs:106`

**Evidence:** The field is `bool` and every constructor sets it to
`true` unconditionally. A `false` value is *not a boundary* but the
type still claims to be one. The brand is a lie.

**Principle violated:** C4 (Q1), P6 (no stubs).

**Fix shape:** Delete the field. `OCAPBoundary` is enforced by
construction. Update the two constructors (`token`, `explicit`).

**Test:** A compile-fail test asserting `OCAPBoundary::token(_).enforced`
is no longer accessible.

### F-SYN-003 — `is_shared` / `is_public` exist on two types with the same semantics (MAJOR)

> **Originals:** F-L1-005, F-L4-001 (2 lenses, 1 finding).
> **Severity:** major.
> **Concept:** `concept:visibility`
> **Location:** `crates/hkask-types/src/visibility.rs:67` AND
  `crates/hkask-types/src/sovereignty.rs:159,164`

**Evidence:** `Visibility::is_shared` and
`DataSovereigntyBoundary::is_shared` (and `is_public`) carry the same
semantics in two different types.

**Principle violated:** C6 (one way).

**Fix shape:** Move the predicates onto a single `AccessMode` enum;
both `Visibility` and `DataSovereigntyBoundary` carry one.

**Test:** `Visibility::Shared.is_shared() == true` and the
equivalent on the boundary both flow through the same `AccessMode`.

### F-SYN-004 — Episodic triple can become semantic via visibility flip without clearing perspective (MAJOR)

> **Originals:** F-L6-004.
> **Severity:** major.
> **Concept:** `concept:episodic_memory`, `concept:visibility`
> **Location:** `crates/hkask-storage/src/triples.rs` (write path)

**Evidence:** A `Private` triple with a perspective can be flipped to
`Shared` (or `Public`) without removing the perspective. The
perspective is the *episodic* marker; if it persists across a
visibility flip, the boundary is broken (privacy laundering).

**Principle violated:** C4 (Q4), C6.

**Fix shape:** The visibility-flip path must either (a) refuse the
flip when perspective is set, or (b) clear the perspective. Add a
test for the chosen path.

**Test:** A test that constructs `Private + perspective = X`, flips
to `Shared`, and asserts (a) the flip errors or (b) the result has
`perspective = None`.

### F-SYN-005 — `cns.clone` and `cns.read` spans are emitted from two crates with different semantics (MAJOR)

> **Originals:** F-L5-001.
> **Severity:** major.
> **Concept:** `concept:cybernetics_loop`
> **Location:** `crates/hkask-cli/src/repl/mod.rs` AND
  `crates/hkask-cns/src/cybernetics_loop.rs`

**Evidence:** Same span prefix, two crates, different semantics.

**Principle violated:** C3 (CNS spans are the only observability
primitive — but the namespace is overloaded).

**Fix shape:** Reserve namespaces by `tech:` slug: `cns.cli.clone`,
`cns.cybernetics.clone`. Add a CI test that asserts every `cns.*`
span is emitted from exactly one crate.

**Test:** A static test in `crates/hkask-cns/tests/span_namespace_invariant.rs`.

### F-SYN-006 — Attenuation chain replay: can a revoked token be re-issued under a previous attenuation? (MAJOR)

> **Originals:** F-L2-003, F-L6-002.
> **Severity:** major.
> **Concept:** `concept:attenuation`, `concept:revocation`
> **Location:** `crates/hkask-types/src/capability/mod.rs:245-...`

**Evidence:** I did not read verify + issuance end-to-end. The
invariant is "a revoked token cannot be re-issued under any of its
previous attenuations." Untested.

**Principle violated:** C4 (Q5 — revocation must be complete), P8.

**Fix shape:** Audit `issuance()` for the revocation-log check. If
absent, add it. Add a test that asserts re-mint after revoke fails.

**Test:** "Issuer A revokes token T; re-mint T with same params;
assert Err(TokenError::Revoked)."

### F-SYN-007 — MCP capability gate ordering is asserted for 2 of 21 servers (MAJOR)

> **Originals:** F-L2-006, F-L6-005.
> **Severity:** major.
> **Concept:** `concept:capability_gate`
> **Location:** `mcp-servers/hkask-mcp-{spec,ocap}/src/main.rs` (asserted),
  the other 19 (not asserted)

**Evidence:** v6 test program covers gate ordering for 2 servers.
No cargo-level invariant says "every tool handler has a gate as its
first statement."

**Principle violated:** C4 (Q1, Q4), P8.

**Fix shape:** Static test in `crates/hkask-mcp/tests/capability_gate_invariant.rs`
that walks every MCP server's main.rs and asserts the first
statement of every tool handler is a capability check.

### F-SYN-008 — `MemoryStoragePort` is a port but not a capability (MAJOR)

> **Originals:** F-L2-005.
> **Severity:** major.
> **Concept:** `concept:revocable_forwarder`
> **Location:** `crates/hkask-agents/src/pod/mod.rs`

**Evidence:** `AgentPod::new_with_memory()` accepts an *optional*
`MemoryStoragePort`. The pod then writes triples unconditionally.
The OCAP boundary governs user-facing operations, not the pod's
internal persistence. The membrane is implicit.

**Principle violated:** C4 (Q4).

**Fix shape:** Make the persistence port's optionality explicit:
pods without memory emit a warning span and write no triples. Add
a test that asserts this is the actual behavior.

### F-SYN-009 — `lambda_for_category(&str)` is stringly-typed dispatch (MINOR)

> **Originals:** F-L1-002.
> **Severity:** minor.
> **Concept:** `concept:span_category`
> **Location:** `crates/hkask-storage/src/nu_event_store.rs:105`

**Evidence:** Dispatch key is `&str`, function is private.

**Fix shape:** `pub enum SpanCategory` with a `From<&str>` shim for
the wire format.

### F-SYN-010 — `DelegationToken.max_attenuation: u8` is wider than the system cap (NIT)

> **Originals:** F-L1-003.
> **Severity:** nit.
> **Concept:** `concept:attenuation`
> **Location:** `crates/hkask-types/src/capability/mod.rs:256`

**Fix shape:** Newtype `AttenuationLevel(u8)` with `new(level) ->
Result<Self, _>` enforcing `level <= 7`.

### F-SYN-011 — `Goal.depth: u8` has no upper bound (NIT)

> **Originals:** F-L1-006.
> **Severity:** nit.
> **Concept:** `concept:sub_goal`
> **Location:** `crates/hkask-storage/src/goals.rs`

**Fix shape:** `MAX_GOAL_DEPTH: u8 = 7` (mirrors attenuation cap);
reject `create_subgoal` past the cap.

### F-SYN-012 — `Dampener.override_cooldown` is global, not per-issuer (DESIGN EXERCISE → FUTURE)

> **Originals:** F-L5-004.
> **Severity:** minor (but design exercise, not a refactor).
> **Concept:** `concept:override_cooldown`
> **Location:** `crates/hkask-cns/src/dampener.rs:58,90-115`

**Evidence:** After any override passes dedup, all subsequent
overrides (from any issuer) within 120s are suppressed.

**Action:** Move to `review/FUTURE.md` as `FUT-003`. Do not act as
a refactor.

### F-SYN-013 — `cns.memory.*` spans have no asserted consumer (MINOR)

> **Originals:** F-L5-002.
> **Severity:** minor.
> **Concept:** `concept:memory_consolidation`

**Fix shape:** Audit; either demote to `tracing::trace!` or add
the consumer.

### F-SYN-014 — `DelegationToken::expires_at` enforcement is unverified by a negative test (MINOR)

> **Originals:** F-L2-003 (independent of F-SYN-006 — that one is
  about re-issuance, this is about expiry).
> **Severity:** minor.
> **Concept:** `concept:delegation_token`

**Fix shape:** Audit `verify()` for the `expires_at` check; add
the negative test.

### F-SYN-015 — `MemoryLoopAdapter` is a forwarder, not a composer (MINOR)

> **Originals:** F-L3-003, F-L4-003.
> **Severity:** minor.
> **Concept:** `concept:memory_loop`
> **Location:** `crates/hkask-agents/src/adapters/memory_loop_adapter.rs`,
  `crates/hkask-memory/`

**Fix shape:** Rename to `MemoryLoopForwarder` and document
responsibility, or fold.

### F-SYN-016 — `PromptCache` and `PromptStrategy` share a noun (MINOR)

> **Originals:** F-L3-004, F-L4-004.
> **Severity:** minor.
> **Concept:** `concept:prompt`
> **Location:** `crates/hkask-templates/src/{prompt_cache.rs, prompt_strategy.rs}`

**Fix shape:** Read both; if parallel, document; if facet, unify.

### F-SYN-017 — `RussellAcpAdapter::bridge_secret` is shared via HKDF, not via a capability (NIT)

> **Originals:** F-L2-004.
> **Severity:** nit.
> **Concept:** `concept:russell_bridge`
> **Action:** Track as `FUT-004` in `review/FUTURE.md`.

### F-SYN-018 — `cns.template.russell_mapping` is a one-off span (NIT)

> **Originals:** F-L5-003.
> **Severity:** nit.
> **Concept:** `concept:russell_bridge`

**Fix shape:** Rename to `cli.russell.mapping` or move to CNS span.

### F-SYN-019 — `GoalCapabilityToken` resurrection path is undocumented (MINOR)

> **Originals:** F-L6-003.
> **Severity:** minor.
> **Concept:** `concept:goal`

**Fix shape:** Doc-comment in `crates/hkask-types/src/goal.rs`
naming OPEN_QUESTIONS F6; a grep test asserting zero matches for
`GoalCapabilityToken`.

### F-SYN-020 — MCP tool input fuzzer surface is unasserted (MINOR)

> **Originals:** F-L6-005 (the *fuzz* part, separate from F-SYN-007
  which is the *gate ordering* part).
> **Severity:** minor.
> **Concept:** `concept:mcp_tool`

**Fix shape:** `proptest` integration test in
`crates/hkask-mcp/tests/fuzz_tool_inputs.rs`.

### F-SYN-021 — `Arc<AtomicU64>` in CNS is correct composition; gate against drift (NIT, positive)

> **Originals:** F-L3-001.
> **Severity:** nit (positive observation).
> **Concept:** `concept:backpressure`

**Fix shape:** Static assertion test: `rg 'Arc<Mutex<'` count stays
≤ empirical baseline.

### F-SYN-022 — No god-traits in the surveyed surface (NIT, positive)

> **Originals:** F-L3-002.
> **Severity:** nit (positive observation).
> **Concept:** `concept:port`

**Fix shape:** CI check: any `pub trait` with > 5 methods fails CI.

### F-SYN-023 — `DEFAULT_COMMUNICATION_BACKPRESSURE_THRESHOLD` is exposed in two places (NIT, positive gate)

> **Originals:** F-L5-005.
> **Severity:** nit (positive observation; gate against drift).
> **Concept:** `concept:backpressure`

**Fix shape:** Static test: `rg 'BACKPRESSURE' crates/ | wc -l == 2`.

### F-SYN-024 — `CurationDecision` and `GoalState` are 4-value enums (NIT, positive)

> **Originals:** F-L4-002.
> **Severity:** nit (positive observation).

**Fix shape:** Doc-comment on each naming the other as disjoint.

### F-SYN-025 — `Spec` / `Goal` / `Criterion` / `Artifact` may be parallel or facets (NIT, design call)

> **Originals:** F-L4-004.
> **Severity:** nit (depends on intent).
> **Concept:** `concept:goal`

**Fix shape:** Read the four `new()` constructors; classify
parallel or facet; act accordingly.

## Priority order (synthesized)

1. **F-SYN-001** (blocker) — fix first; blocks v0.24.0.
2. **F-SYN-002, F-SYN-003, F-SYN-004, F-SYN-005, F-SYN-006, F-SYN-007, F-SYN-008**
   (7 majors) — fix in current sprint.
3. **F-SYN-009 through F-SYN-020** (12 minors/nits) — fix or document
   in next sprint.
4. **F-SYN-021 through F-SYN-025** (5 positive observations) — add
   as CI drift gates; no fix needed.

## What is *not* in this synthesis

- The 4 duplicates that were collapsed (see header of each
  synthesized entry).
- F-L5-004's design exercise content (moved to FUTURE.md).
- Any finding the L1-L6 lenses explicitly marked as
  "depends on intent" (F-SYN-016, F-SYN-025) — those are filed as
  drift gates, not refactor candidates.

## Verification commands (re-runnable)

```bash
# Charter + ERD + findings all present
ls review/charter/CHARTER.md
ls review/erds/*.mmd | wc -l   # 8
ls review/findings/L*.md | wc -l   # 6
ls review/findings/SYNTHESIS.md

# Blockers visible
rg 'blocker' review/findings/SYNTHESIS.md

# Findings deduplicated: count unique F-SYN- ids
rg -o 'F-SYN-[0-9]+' review/findings/SYNTHESIS.md | sort -u | wc -l   # 25

# Confirm zero Python scripts were committed in review/
find review/ -name '*.py' | wc -l   # 0
```
