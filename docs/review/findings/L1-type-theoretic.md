# L1 — Type-Theoretic Review (Hoare)

> Persona: a RustBelt reader, with a soft spot for `unsafe`-adjacent patterns,
> lifetime parameterization, generic bounds, `Send + Sync` propagation,
> and capability ports' `dyn`-compatibility.
>
> Scope: all 11 core crates + 21 MCP servers.
> Method: static reading + targeted `rg` queries against the *type-level* surface.
> Output: findings following the schema in `../charter/CHARTER.md` §3.

## Method

For every `pub` symbol I read, I ask:

1. **Is the type the simplest one that could carry this information?**
2. **Are the generic bounds tight, or `T: Send + Sync + 'static` by reflex?**
3. **Does the type implement `Send + Sync` *intentionally*, or because the
   compiler forced it (and that intent survives review)?**
4. **Is there a `dyn Trait` boundary where there should be a `Box<dyn>` /
   `Arc<dyn>`, or vice versa?**
5. **Are lifetimes named (`'a`, `'de`) or elided? Elision is fine if it's
   unambiguous, suspicious if it's surprising.**

## Findings

### F-L1-001 — `OcapCapability::String(String)` is a brand-laundering pattern

- **id:** F-L1-001
- **location:** `crates/hkask-types/src/curation.rs:104`
- **concept:** `concept:ocap`
- **severity:** minor
- **evidence:**
  ```rust
  pub capability: OcapCapability,
  // ...
  pub enum OcapCapability { Token(OcapTokenKind), String(String) }
  ```
  `OCAPBoundary::explicit(capability: String)` accepts any string as a
  capability, bypassing the typed brand.
- **principle_violated:** C4 (OCAP over ambient authority) — strings are
  forgeable, tokens are not.
- **root_cause_driver:** `over_engineering` — backward-compat shim that
  could be a single migration with a deadline.
- **proposed_fix_shape:** Add a `#[deprecated]` note in the *changelog* (not
  the code, per P7), then remove `String` variant in v0.25.0 with a one-line
  migration table in `OPEN_QUESTIONS.md`.
- **test_that_proves_it:** Red: write a test that asserts all
  `OCAPBoundary::explicit(...)` call sites have been replaced. Green: remove
  the variant; confirm `cargo test --workspace` passes.

### F-L1-002 — `lambda_for_category(&str)` is stringly-typed dispatch

- **id:** F-L1-002
- **location:** `crates/hkask-storage/src/nu_event_store.rs:105`
- **concept:** `concept:span_category`
- **severity:** minor
- **evidence:**
  ```rust
  fn lambda_for_category(category: &str, config: &DecayConfig) -> f64 {
      match category {
          "variety" => config.cybernetics_decay,
          ...
      }
  }
  ```
  The dispatch key is a `&str` and the function is `fn`, not `pub fn`.
- **principle_violated:** C6 (one way to do things — but the way is
  *stringly*, which is *no* way). P8 (the public seam — the match arms —
  is hidden from tests in the form of a hard-coded lookup).
- **root_cause_driver:** `shallow_module` — the dispatch is private and
  the key is a string, so adding a category requires editing the match
  arms *and* the tests, with no compiler guarantee of completeness.
- **proposed_fix_shape:** Introduce `pub enum SpanCategory { Cybernetics,
  Curation, Inference, Episodic, Gas (fallback) }` and dispatch on it.
  Keep the `&str` constructor for backward compat with the wire format.
- **test_that_proves_it:** Red: write a test that asserts every match arm
  in the new enum is covered by an exhaustive match. Green: convert
  `lambda_for_category` to take `SpanCategory`.

### F-L1-003 — `DelegationToken.max_attenuation: u8` is wider than the system cap

- **id:** F-L1-003
- **location:** `crates/hkask-types/src/capability/mod.rs:256`
- **concept:** `concept:attenuation`
- **severity:** nit
- **evidence:**
  ```rust
  pub max_attenuation: u8,
  // ...
  max_attenuation: SYSTEM_MAX_ATTENUATION,  // = 7
  ```
  A `u8` allows 0–255, but the system invariant is 0–7. The runtime
  check at line 544 catches oversized values, but a tighter type would
  catch them at construction.
- **principle_violated:** C6 (functional minimalism — the type is wider
  than its purpose).
- **root_cause_driver:** `over_engineering` — defensive `u8` where a
  `AttenuationLevel(u8)` newtype with a checked constructor would suffice.
- **proposed_fix_shape:** Newtype `pub struct AttenuationLevel(u8)` with
  `new(level: u8) -> Result<Self, _>` enforcing `level <= 7`. Use it
  everywhere `max_attenuation` and `attenuation_level` are used.
- **test_that_proves_it:** Red: a test asserting `AttenuationLevel::new(8)`
  fails at construction. Green: replace `u8` fields with the newtype.

### F-L1-004 — `OcapBoundary::enforced: bool` is a foot-gun, not a feature

- **id:** F-L1-004
- **location:** `crates/hkask-types/src/curation.rs:106`
- **concept:** `concept:ocap_boundary`
- **severity:** minor
- **evidence:**
  ```rust
  pub struct OCAPBoundary {
      pub capability: OcapCapability,
      pub enforced: bool,
  }
  ```
  Every constructor in the file (`token`, `explicit`) sets `enforced: true`
  unconditionally. The `bool` field is never `false` in the codebase
  (grep confirms zero `enforced: false` outside the type definition).
- **principle_violated:** P6 (no stubs — a field that has no live
  production value is a stub). C6 (one way).
- **root_cause_driver:** `shallow_module` — the field exists "for
  flexibility" but has no consumer.
- **proposed_fix_shape:** Remove the field. Make `enforced` a private
  invariant: an `OCAPBoundary` *is* enforced by construction.
- **test_that_proves_it:** Red: a test asserting `OCAPBoundary::token(_).enforced`
  field access compiles after the field is removed. Green: remove the field,
  update all call sites.

### F-L1-005 — `Visibility::is_private` / `is_public` are dead code; `is_shared` is also dead

- **id:** F-L1-005
- **location:** `crates/hkask-types/src/visibility.rs:57-69`
- **concept:** `concept:visibility`
- **severity:** nit
- **evidence:**
  ```rust
  #[allow(dead_code)] // reserved for future crate-internal use
  pub(crate) fn is_private(&self) -> bool { ... }
  #[allow(dead_code)] // reserved for future crate-internal use
  pub(crate) fn is_public(&self) -> bool { ... }
  pub fn is_shared(&self) -> bool { ... }   // not marked dead, but ALSO dead
  ```
  `rg -n '\bis_(private|public|shared)\b'` shows all three are *defined*
  on `Visibility` but *never called* in the workspace. (The other
  `is_shared`/`is_public` matches are on the unrelated
  `DataSovereigntyBoundary` in `crates/hkask-types/src/sovereignty.rs:159,164`
  — a different type with the same predicate name, which is itself a
  vocabulary-drift finding under L4.)
- **principle_violated:** P6 (no stubs — "reserved for future use" is a
  stub). C6 (one way).
- **root_cause_driver:** `shallow_module` — `is_private`/`is_public` are
  speculatively exposed; `is_shared` was kept when its siblings were
  marked dead.
- **proposed_fix_shape:** Delete all three methods. Call sites that need
  the predicate should `match self { Visibility::Private => ..., ... }`
  inline. The `DataSovereigntyBoundary::is_shared`/`is_public` rename is
  a separate L4 finding.
- **test_that_proves_it:** Red: `rg '\bVisibility\b.*\bis_(private|public|shared)\b' crates/ mcp-servers/`
  should return zero production matches. Green: remove the methods.

### F-L1-006 — `Goal.depth: u8` has no upper bound in the type

- **id:** F-L1-006
- **location:** `crates/hkask-storage/src/goals.rs` (`create_subgoal`),
  inferred from `crates/hkask-types/src/goal.rs` and the test plan
- **concept:** `concept:sub_goal`
- **severity:** nit
- **evidence:** No upper bound is asserted in the type or the storage
  repository for `sub_goal.depth`. The attenuation cap of 7 is
  architectural, but sub-goal depth has no analog.
- **principle_violated:** C6 / DDMVSS — the attenuation cap of 7 is
  explicit; the sub-goal depth cap is not.
- **root_cause_driver:** `untested_seam` — the spec curator's tolerance
  check is not a structural cap.
- **proposed_fix_shape:** Add a constant `MAX_GOAL_DEPTH: u8 = 7` (mirrors
  `SYSTEM_MAX_RECURSION` and the attenuation cap). Reject
  `create_subgoal` when `parent.depth + 1 > MAX_GOAL_DEPTH`.
- **test_that_proves_it:** Red: `create_subgoal` of a depth-7 goal
  returns `Err(GoalError::MaxDepthExceeded)`. Green: implement the cap.

## Lens summary

| Severity | Count |
|----------|-------|
| blocker  | 0     |
| major    | 0     |
| minor    | 4     |
| nit      | 3     |

All findings are about *type width* (F-L1-001, F-L1-003, F-L1-004,
F-L1-005, F-L1-006) and *stringly dispatch* (F-L1-002). No `unsafe` was
found in the survey; no `Send + Sync` soundness issue was found in the
passes I read. The crates use `Arc<Mutex<_>>` patterns in CNS, which
L5 covers, not L1.
