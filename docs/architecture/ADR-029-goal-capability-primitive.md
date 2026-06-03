---
title: "ADR-029: Goal Capability Primitive — Aligned, Not Unified"
audience: [architects, security engineers]
last_updated: 2026-05-29
version: "1.0.0"
status: "Active"
domain: "Technology"
ddmvss_categories: [trust, capability]
---

# ADR-029: Goal Capability Primitive — Aligned, Not Unified

**Date:** 2026-05-29
**Status:** Implemented
**Supersedes:** N/A
**Related:** ADR-025 (Attenuation Depth Limit), ADR-027 (Master Key Derivation)

## Context

hKask has two capability-token families:

1. **`CapabilityToken`** (`crates/hkask-types/src/capability/`) — the canonical OCAP
   primitive used by `hkask-agents` for pod delegation and ACP message authority.
   It signs all authority-bearing fields, verifies in constant time, carries a
   `max_attenuation` ceiling, supports caveats, and verifies a root-nonce
   attenuation chain.

2. **`GoalCapabilityToken`** (`crates/hkask-types/src/goal_capability.rs`) — a
   smaller token gating the goal coordination substrate (`GoalOp` over a single
   `GoalID`).

An adversarial review (2026-05-29, P0-03) found `GoalCapabilityToken` was a weaker,
divergent reimplementation: its HMAC omitted `operations` and `expires` (forgeable
authority), it lacked a `max_attenuation` field (hardcoding `7`, contradicting
ADR-025), and it compared signatures non-constant-time. This raised the question:
**should the goal token be collapsed into `CapabilityToken`, or hardened as a
distinct type?**

## Decision

**Keep `GoalCapabilityToken` as a distinct type, but align it with the canonical
primitive's security invariants.** (Option B — *align*, not Option A — *unify*.)

Concretely, the hardening applied:

1. The HMAC now binds **every authority-bearing field**: id, goal_id, holder,
   a canonical (sorted, deduplicated, length-delimited) operation set, expiry,
   `attenuation_level`, and `max_attenuation`.
2. Signature verification uses `subtle::ConstantTimeEq`.
3. A `max_attenuation` field initialized to `SYSTEM_MAX_ATTENUATION`, plus a
   `can_attenuate()` method mirroring `CapabilityToken::can_attenuate()`, so both
   families share one delegation-depth semantics (ADR-025).

The two types now share **invariants and the depth constant**, but remain
separate Rust types.

## Rationale

1. **Domain fit without over-generalization.** The goal token's authority alphabet
   is `GoalOp` (Create, Read, Update, Verify, Complete, CreateSubgoal, AddArtifact),
   bound to exactly one `GoalID`. `CapabilityToken` models `(resource, resource_id,
   action)` with caveats and root-nonce chains. Projecting `GoalOp` onto the generic
   action model would either lose the goal-specific operation set or require a
   `GoalOp`-shaped caveat encoding — added indirection for no behavioral gain.

2. **Lean coupling.** [^miller-robust] `goal_capability` lives in `hkask-types` with
   no dependency on the `capability/` module's chain-verification machinery. A goal
   token is verified in O(1) against a single secret; it does not (today) participate
   in cross-agent delegation chains, so it does not need root-nonce chain
   verification. Importing that machinery would be speculative complexity (P5).

3. **Shared invariants, not shared code, are what matter for security.** The
   forgery class is closed by *what the signature covers*, not by *which struct*
   computes it. Both families now sign all authority-bearing fields and verify in
   constant time. The depth bound is a shared constant (`SYSTEM_MAX_ATTENUATION`),
   so ADR-025 holds uniformly.

4. **Reversibility.** If goals later need cross-agent delegation chains (see the
   open crossroads below), promoting `GoalCapabilityToken` to a typed projection
   over `CapabilityToken` is a contained refactor — the public surface
   (`new`, `is_valid`, `can_perform`, `attenuate`) is small and stable.

## Consequences

### Positive

- Goal authority is unforgeable; operations and expiry cannot be amplified
  post-issuance.
- One delegation-depth semantics across both token families (ADR-025).
- No new cross-crate coupling; storage stays a low layer.
- Small, reversible public surface.

### Negative

- Two token types to maintain. Mitigated by shared invariants and a shared depth
  constant; drift is caught by the goal-capability forgery tests
  (`crates/hkask-types/src/goal_capability.rs`) and the CNS denial cybertest
  (`crates/hkask-cns/tests/goal_capability_cybertests.rs`).
- Goal tokens do not (yet) carry caveats or participate in root-nonce chains.

### Alternative Rejected

**Option A — Unify** (`GoalCapabilityToken` as a thin projection over
`CapabilityToken`). Rejected for v0.21: it imports chain-verification and caveat
machinery the goal substrate does not use today, and forces the `GoalOp` alphabet
into the generic action model. Revisit if/when goal tokens cross trust boundaries.

## Compliance

| Principle | Compliance |
|-----------|-----------|
| P1 (No trait without two consumers) | ✅ No new trait introduced; shared `SYSTEM_MAX_ATTENUATION` constant has two consumers (`CapabilityToken`, `GoalCapabilityToken`) |
| P5 (No speculative code) | ✅ Chain verification not imported until a delegation use case exists |
| C5 (Every error variant is a unique recovery path) | ✅ `GoalRepositoryError::{CapabilityDenied, VisibilityDenied, InvalidTransition, Corrupt}` are distinct |
| Miller (No ambient authority) | ✅ Every goal write checks holder ownership in addition to the capability |
| Schneier (Authenticate all authority) | ✅ HMAC binds operations, expiry, and attenuation ceiling; constant-time verify |

## Open Crossroads

Tracked in `docs/OPEN_QUESTIONS.md` F6: capability revocation, operation-set
encoding (length-delimited list vs. bitset), full lineage unification (Option A),
persistence-corruption response policy, and recursion-bound coherence across
attenuation/cascade/subgoal depth.

## References

[^miller-robust]: Miller, M. S. (2006). *Robust Composition*. Johns Hopkins University.

---

*ℏKask - A Minimal Viable Container for Agents — ADR-029 — v0.21.0*
