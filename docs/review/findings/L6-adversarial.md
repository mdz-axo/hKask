# L6 — Adversarial User Review (Red-team)

> Persona: an attacker. The system has a curator, a replicant, a
> bot, and 21 MCP servers. I have one WebID. I want to mint
> capabilities I shouldn't have, persist triples I shouldn't see,
> or escape the privacy/visibility boundary.
>
> Method: walk every capability-bearing type and every storage write
> path. For each, ask: *what is the smallest input that crosses the
> trust boundary without being checked?*

## Threat model (assumed)

- The attacker controls a *user* WebID and a *bot* pod. They do not
  control the master passphrase.
- The attacker can call any MCP tool exposed by any MCP server.
- The attacker can read the public architecture docs.

## Findings

### F-L6-001 — `OcapCapability::String` (F-L1-001, F-L2-002) is the single highest-impact attack surface

- **id:** F-L6-001
- **location:** `crates/hkask-types/src/curation.rs:104` (same as F-L1-001)
- **concept:** `concept:ocap`
- **severity:** **blocker**
- **evidence:** An attacker who can construct a `String` can mint any
  capability: `OCAPBoundary::explicit("memory:write:any-webid")`. The
  runtime will accept the string and treat it as a valid capability.
  This is the *easiest* way to escalate from user → admin → system.
- **principle_violated:** C4 (Q1 — forgeable), P8 (the property
  "capabilities cannot be forged" is unverified by a test).
- **root_cause_driver:** `attack_surface` — the `String` variant
  exists *as a feature*, but the feature is the attack.
- **proposed_fix_shape:** Delete the variant. As F-L1-001 and F-L2-002.
- **test_that_proves_it:** A test that constructs
  `OCAPBoundary::explicit("memory:write:any-webid")` and asserts the
  call does not compile after the variant is removed. The test
  *red* state is the vulnerability.

### F-L6-002 — Attenuation chain replay: can a revoked token be re-issued under a previous attenuation?

- **id:** F-L6-002
- **location:** `crates/hkask-types/src/capability/mod.rs:245-...`
- **concept:** `concept:attenuation`, `concept:revocation`
- **severity:** major
- **evidence:** I did not read the verify function end-to-end. The
  invariant is "a revoked token cannot be re-issued under any of its
  previous attenuations" (Task 6, primitive 5). If the verify function
  checks the revocation log but the issuance function does not, the
  attacker can re-mint a token that *looks* revoked and the verify
  function will reject it — that's correct — but if the attacker can
  also re-derive the token from the original issuer's secret, the
  revocation is moot.
- **principle_violated:** C4 (Q5 — revocation must be complete).
- **root_cause_driver:** `attack_surface` — revocation is a property
  of the *issuance* path, not the verify path. The current tests
  may cover only verify.
- **proposed_fix_shape:** Read `issuance()` (or whatever the minting
  function is named). If it does not consult the revocation log,
  add the check. Add a test that asserts: "issuer A revokes token T;
  issuer A then attempts to re-mint T (same nonce, same parameters);
  the re-mint fails with `Err(TokenError::Revoked)`."
- **test_that_proves_it:** The above test. The red state is "the
  re-mint succeeds"; the green state is "the re-mint fails with
  Revoked."

### F-L6-003 — `GoalCapabilityToken` was removed (F6 resolved), but a resurrection path is undocumented

- **id:** F-L6-003
- **location:** OPEN_QUESTIONS F6 (resolved 2026-06-04) — see `joined.ttl`
  `concept:goal_token_resurrection` for the open question
- **concept:** `concept:goal`
- **severity:** minor
- **evidence:** The v6 handoff records that `GoalCapabilityToken` was
  *entirely removed* in v0.23.0. The removal was correct (over-
  engineered ceremony, no payoff). The attack-surface concern is
  that *nothing in the code prevents the token from being re-introduced*.
  If a future contributor re-adds HMAC signing for goals, they may
  re-introduce the revocation problem (F6.1 in OPEN_QUESTIONS).
- **principle_violated:** P6 (no stubs — but the absence of the type
  is what we want to preserve).
- **root_cause_driver:** `attack_surface` — regression risk.
- **proposed_fix_shape:** Add a comment in `crates/hkask-types/src/goal.rs`
  naming OPEN_QUESTIONS F6 and stating "do not reintroduce capability
  tokens for goals; goals are scoped by `&WebID` only." This is a
  doc-comment, not code; the test is *future contributors read it*.
- **test_that_proves_it:** A grep test:
  `rg 'GoalCapabilityToken' crates/ mcp-servers/` should return zero
  matches. If it returns a non-zero count, the type was resurrected.
  Lives in `crates/hkask-types/tests/goal_no_capability_token.rs`.

### F-L6-004 — Episodic triple can become semantic via the `shared` visibility flip

- **id:** F-L6-004
- **location:** `crates/hkask-storage/src/triples.rs` (write path)
- **concept:** `concept:visibility`, `concept:episodic_memory`
- **severity:** major
- **evidence:** A triple is created with `Visibility::Private` and a
  perspective. The visibility can be flipped to `Shared` (or `Public`)
  in a later write — and the perspective is *not* removed in the flip.
  This means a private episodic triple (with perspective = attacker
  WebID) can become a shared semantic triple (with perspective = still
  the attacker's WebID, but now visible to everyone). The perspective
  is supposed to be the *episodic* marker; if it persists across a
  visibility flip, the boundary is broken.
- **principle_violated:** C4 (Q4 — privacy laundering), C6 (one
  visibility, one meaning).
- **root_cause_driver:** `attack_surface` — visibility is mutable
  state, perspective is not. The invariant "perspective is removed
  when visibility transitions from Private to Shared/Public" is
  not asserted.
- **proposed_fix_shape:** The visibility-flip path must either (a)
  refuse the flip when perspective is set, or (b) clear the
  perspective on flip. Add a test that asserts the chosen path is
  enforced.
- **test_that_proves_it:** Red: a test that constructs a `Private +
  perspective = X` triple, flips it to `Shared`, and asserts either
  (a) the flip returns `Err(VisibilityError::PerspectiveLocked)` or
  (b) the resulting triple has `perspective = None`. Green:
  implement the check.

### F-L6-005 — MCP tool input fuzzer surface is unasserted (parallel to F-L2-006)

- **id:** F-L6-005
- **location:** `mcp-servers/hkask-mcp-*/src/main.rs` (all 21)
- **concept:** `concept:mcp_tool`
- **severity:** minor
- **evidence:** Each MCP tool accepts `Parameters<T>` for typed
  arguments, but the *parsing* of those arguments is delegated to
  `serde_json`. A fuzzer-style test that feeds malformed JSON to
  every tool would catch panics in the parsing path. The v6 test
  program does not include property-based / fuzz tests for MCP
  inputs.
- **principle_violated:** C4 (Q4 — input boundary unverified).
- **root_cause_driver:** `untested_seam` — the test surface is positive
  (typed inputs work) but not negative (malformed inputs don't panic).
- **proposed_fix_shape:** Add a `proptest` integration test in
  `crates/hkask-mcp/tests/fuzz_tool_inputs.rs` that, for each MCP
  tool type, generates 1000 random `serde_json::Value`s and asserts
  none of them cause a panic (the result is `Err(...)` or `Ok(...)`,
  never a `panic!`).
- **test_that_proves_it:** The proptest itself, run in CI.

## Lens summary

| Severity | Count |
|----------|-------|
| blocker  | 1     |
| major    | 2     |
| minor    | 2     |
| nit      | 0     |

L6's blocker (F-L6-001) is the same finding as L1-001 and L2-002 —
the `OcapCapability::String` variant. The three lenses all surface it
because it is the single most exploitable ambient-authority surface
in the system. **Delete it.** This is the highest-priority finding
in the entire review and the only one I would block v0.24.0 on.

The two majors (F-L6-002 revocation replay, F-L6-004 visibility
laundering) are both about *the value of a mutable field that does
not enforce its own invariant*. The type system can fix both.
