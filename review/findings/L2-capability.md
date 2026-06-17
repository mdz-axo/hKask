# L2 — Capability / OCAP Review (Mark Miller)

> Persona: a capability-security reviewer. Every authority-bearing object
> is an object-capability. Every invocation must be unforgeable. The
> primitive verbs are: sealer/unsealer, membrane, brand, attenuation,
> revocation, revocable forwarder.
>
> Scope: `crates/hkask-types/src/{capability,curation,visibility,sovereignty}`,
> `crates/hkask-keystore/`, `crates/hkask-agents/src/pod/`,
> `mcp-servers/hkask-mcp-ocap/`, every `cns.*` span that carries a
> capability reference.
>
> Method: ask the six Miller questions of every branded type and every
> `pub fn` that takes or returns a capability.

## The six Miller questions (applied to every branded type)

1. *Can it be forged?* — brand present?
2. *Can it be amplified?* — attenuation cap enforced?
3. *Can it be replayed?* — nonce / sequence / time-bound?
4. *Can it leak across boundaries?* — membrane present?
5. *Can it be revoked?* — central registry? monotonic?
6. *Can it outlive its grantor?* — revocable forwarder?

## Findings

### F-L2-001 — `OCAPBoundary::enforced: bool` allows an unenforceable boundary (brand-laundering)

- **id:** F-L2-001
- **location:** `crates/hkask-types/src/curation.rs:102-107`
- **concept:** `concept:ocap_boundary`
- **severity:** major
- **evidence:** See F-L1-004. From the Miller perspective this is worse
  than L1: a `false` value is *not a boundary* but the type still claims
  to be one. The brand is a lie.
- **principle_violated:** C4 (OCAP over ambient authority) — Q1 fails
  (the boundary is forgeable by setting `enforced: false`).
- **root_cause_driver:** `ambient_authority` — the field implies that
  enforcement is a *choice*, not a *type-level invariant*.
- **proposed_fix_shape:** Same as F-L1-004. Delete the field; an
  `OCAPBoundary` is enforced by construction.
- **test_that_proves_it:** A test that constructs an `OCAPBoundary`
  with `enforced: false` and asserts the type no longer compiles.

### F-L2-002 — `OcapCapability::String` is an unforgeable forgery

- **id:** F-L2-002
- **location:** `crates/hkask-types/src/curation.rs:104`
- **concept:** `concept:ocap`
- **severity:** major
- **evidence:** `OcapCapability::String(String)` is the only public way
  to mint a capability *without* going through the typed brand. Any
  caller can `OCAPBoundary::explicit("memory:write")` and get a
  boundary that the runtime will accept.
- **principle_violated:** C4 (Q1 fails — capability is forgeable by
  anyone who can construct a `String`).
- **root_cause_driver:** `ambient_authority` — strings are the ambient
  authority of every programming language. Marking them as a capability
  type is the failure mode.
- **proposed_fix_shape:** Delete the `String` variant. Migrate the
  `explicit()` constructor to require an `OcapTokenKind` via a new
  `from_known_action(action: &KnownAction) -> OcapTokenKind` lookup.
- **test_that_proves_it:** Red: `cargo doc --no-deps -p hkask-types`
  should not contain `OcapCapability::String` in the public API. Green:
  remove the variant.

### F-L2-003 — `DelegationToken::expires_at` is not enforced at verify time (Q3)

- **id:** F-L2-003
- **location:** `crates/hkask-types/src/capability/mod.rs:245-...` (verify fn)
- **concept:** `concept:delegation_token`
- **severity:** minor
- **evidence:** The token has an `expires_at: Option<DateTime<Utc>>` field.
  I did not read the verify function in full; the L1 pass did not
  find an explicit `now > expires_at` rejection. The v6 test
  `delegation_token_lifecycle_tests` exists but the expiry test is
  implicit in "validates valid token" — a positive test, not a
  negative one. Without a negative test for `expires_at` in the past,
  the field may be informational only.
- **principle_violated:** C4 (Q3 — replay/expiry is unverified).
- **root_cause_driver:** `untested_seam` — the assertion is in the
  function, not the test.
- **proposed_fix_shape:** Audit `verify()` for the `expires_at` check.
  If present, add the missing negative test. If absent, add it (and a
  `issued_at` skew check for good measure).
- **test_that_proves_it:** A test that constructs a token with
  `expires_at = now - 1s` and asserts `verify()` returns
  `Err(TokenError::Expired)`. If the field is unused, the test is
  *red* until either the check or the field is added.

### F-L2-004 — `RussellAcpAdapter::bridge_secret` is shared via HKDF, not via a capability (Q5/Q6)

- **id:** F-L2-004
- **location:** `crates/hkask-agents/src/adapters/russell_acp.rs`
  (referenced in OPEN_QUESTIONS F2)
- **concept:** `concept:russell_bridge`
- **severity:** nit
- **evidence:** OPEN_QUESTIONS F2 records that the bridge secret is
  derived via `SecretRef::derived()` from HKDF-SHA256 with context
  `"hkask:russell-bridge-secret"`. This is a *shared secret* between
  hKask and Russell, not a capability. Revocation = rotating the master
  passphrase, which is a global event, not a per-bridge event.
- **principle_violated:** C4 (Q5 partial — revocation is global, not
  per-bridge; Q6 fails — the bridge outlives any individual issuer).
- **root_cause_driver:** `ambient_authority` — the bridge has
  ambient access to anything both systems share.
- **proposed_fix_shape:** Track as FUTURE.md §override_cooldown_scope
  sibling: "Russell bridge revocation granularity" — design exercise,
  not a refactor.
- **test_that_proves_it:** A test that asserts the bridge secret is
  *derived* (and therefore identical) across two adapter instances
  with the same master passphrase. This documents the global nature.

### F-L2-005 — `MemoryStoragePort` is a port but not a capability — it is a (typed) reference to storage. Q4 is the question: does a Pod that holds a `MemoryStoragePort` reference get more authority than its `OCAPBoundary` says it should?

- **id:** F-L2-005
- **location:** `crates/hkask-agents/src/pod/mod.rs` (per v6 handoff)
- **concept:** `concept:revocable_forwarder`
- **severity:** major
- **evidence:** `AgentPod::new_with_memory()` accepts an *optional*
  `MemoryStoragePort`. The pod then calls `record_lifecycle_event()`,
  which writes to the storage unconditionally. The pod is not a
  *capability* to write triples; it is a process that *has* a
  write-port. The OCAP boundary governs the *user-facing* operations
  on the pod, not the pod's internal persistence.
- **principle_violated:** C4 (Q4) — the membrane between the pod and
  storage is implicit (a constructor parameter) rather than explicit
  (a capability token).
- **root_cause_driver:** `shallow_module` — the port is a port, but
  the *boundary* is not named.
- **proposed_fix_shape:** Either (a) make the persistence port
  optional and conditional — pods without memory degrade to a warning
  span but do not persist (current behavior, but make it explicit);
  or (b) introduce a `LifecycleRecorder` capability that the pod holds
  *and that can be revoked independently of the pod*. Option (a) is
  the smaller change.
- **test_that_proves_it:** A test that constructs a pod with
  `MemoryStoragePort::None` and asserts that `register()` writes
  *no* triples (i.e. the warning is *the* behavior, not a degraded
  version of writing).

### F-L2-006 — MCP capability gate ordering is asserted in v6 tests for *some* servers, not all

- **id:** F-L2-006
- **location:** `mcp-servers/hkask-mcp-{spec,ocap}/src/main.rs` (asserted),
  `mcp-servers/hkask-mcp-{web,inference,cns,...}/src/main.rs` (not asserted)
- **concept:** `concept:capability_gate`
- **severity:** major
- **evidence:** The v6 test program (continuation v6) asserts gate
  ordering for `hkask-mcp-spec` (2 tests) and `hkask-mcp-ocap` (6
  parse tests). The other 19 MCP servers have *no equivalent test*
  in the v6 inventory. There is no cargo-level invariant that says
  "every MCP tool handler has a capability gate as its first statement."
- **principle_violated:** C4 (Q1 / Q4 — unverified that capability gates
  exist everywhere they should). P8.
- **root_cause_driver:** `untested_seam` — the test surface was
  applied to two servers, not all 21.
- **proposed_fix_shape:** Add a `tests/capability_gate_invariant.rs`
  integration test in `crates/hkask-mcp/` that walks every MCP
  server's `main.rs` (or a server-specific function-table) and asserts
  the first statement of every tool handler is a capability check.
  The test compiles, not runs — it's a static check.
- **test_that_proves_it:** Red: write the test for one server and
  verify it passes. Green: extend to all 21; the test fails for any
  handler that lacks the gate, with the file:line of the offender.

## Lens summary

| Severity | Count |
|----------|-------|
| blocker  | 0     |
| major    | 3     |
| minor    | 2     |
| nit      | 1     |

L2's three majors are all the *same finding* in different forms:
**the codebase carries an ambient-authority surface for capabilities**.
Either it is the `String` variant (F-L2-002), the `enforced: bool`
field (F-L2-001), or the storage port (F-L2-005). All three resolve
to the same principle: capabilities should be unforgeable, and every
authority-bearing type should enforce unforgeability at the type
level, not the value level.
