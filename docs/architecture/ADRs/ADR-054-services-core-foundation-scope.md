---
title: "ADR-054: Services-Core Foundation Scope"
audience: [architects, developers]
last_updated: 2026-07-17
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle]
---

# ADR-054: Services-Core Foundation Scope

**Date:** 2026-07-17  
**Status:** Active

## Context

`hkask-services-core`'s `lib.rs` historically claimed to be "the universal
foundation that **all** other service-layer modules depend on." A dependency
graph audit of the eleven service-layer crates under `crates/` found this claim to
be false: `hkask-services-research` does not depend on `hkask-services-core`.
Research uses its own `WebError` (provider-shaped: `ProviderUnavailable`,
rate-limit, extraction errors) and its own `RateLimiter`, none of which map
cleanly onto `ServiceError`'s domain-oriented variants.

**Problem Statement:** The foundation crate's doc comment states a Guardrail
("all modules depend on this") that the implementation contradicts, leaving
a spec/impl drift that misleads anyone reasoning about the service-layer
graph.

**Stakeholders:** Anyone reading `hkask-services-core` to understand the
service-layer contract; the architecture review that surfaced the drift.

**Constraints:**
- `WebError` is legitimately provider-shaped; forcing a `core` dependency on
  `research` purely to satisfy the universality claim would be cargo-cult
  coupling (it would pull `ServiceError` + `self_heal` + `model_cache` into a
  crate that needs none of them).
- `core` is load-bearing for ten other service crates; its claim must remain
  trustworthy, not aspirational.

## Decision

**Chosen Approach:** Weaken the spec to match the implementation, and record
the exception explicitly. `hkask-services-core`'s `lib.rs` now states it is the
foundation for *most* service-layer modules and names `hkask-services-research`
as the intentional exception with its own provider-shaped error type. The
decision is recorded here so the exclusion is conscious, not silent drift.

**Alternatives Considered:**
1. **Make `hkask-services-research` depend on `core`** — Rejected: research
   would gain a compile-time dependency on `self_heal`, `model_cache`,
   `verification`, `goal`, `identity`, and `inference_svc` (the broader
   scope-creep problem flagged by the same review) for zero behavioral benefit.
   This would worsen the compile-coupling problem rather than fix honesty.
2. **Merge `WebError` into `ServiceError` as provider-oriented variants** —
   Rejected: `ServiceError` is domain-oriented (`DomainKind`, `ErrorKind`);
   provider/rate-limit errors are a different ontology tier. Conflating them
   violates the pragmatic-semantics ontology-anchoring discipline.
3. **Leave the claim as-is** — Rejected: a false Guardrail claim is worse than
   no claim; it causes downstream reasoners to assume a closure that does not
   hold.

**Rationale:** Per Ousterhout, a module's interface should describe what it
truly provides, not what it aspires to provide[^ousterhout]. A foundation that
claims universality while excluding one member is a contract defect; the
cheapest honest fix is to scope the claim and name the exception. Forcing
coupling to make the claim true would invert the deep-module principle — it
would widen `core`'s reach to serve a claim rather than a need.

## Consequences

### Positive
- `hkask-services-core`'s doc comment is now truthful; spec/impl drift closed.
- `hkask-services-research`'s independent error model is legitimized, not
  treated as an oversight.
- Future readers can trust the foundation claim for the ten crates it covers.
- The exclusion is auditable (this ADR) rather than implicit.

### Negative
- The service layer now has two error systems (`ServiceError`, `WebError`);
  consumers that span both (currently only `hkask-mcp-research`, which already
  bridges) must map between them at the MCP boundary.
- "Foundation for most" is weaker marketing than "universal foundation"; the
  nuance must be carried forward in future docs.

### Neutral
- `hkask-services-research`'s `Cargo.toml` is unchanged; no dependency added or
  removed. The change is documentation + this ADR only.

## Compliance

### Constraint-Driven Design Principles

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P3** (No module directory without encapsulation) | ✅ | No structural change; doc-only |
| **P7** (Prefer deletion over deprecation) | ✅ | The false claim is deleted and replaced with a true one, not deprecated |

### Constraints

| Constraint | Compliance | Evidence |
|-----------|-----------|----------|
| **C7** (Divergence must yield) | ✅ | The spec/impl divergence is resolved by bringing the spec to the implementation's truth |

## Verification

```bash
# The lib.rs claim no longer says "all"
grep -n "universal foundation that all other" crates/hkask-services-core/src/lib.rs
# Expected: no matches

grep -n "hkask-services-research" crates/hkask-services-core/src/lib.rs
# Expected: 1 match naming the exception

# The crate still builds (doc-only change)
cargo check -p hkask-services-core
# Expected: success
```

**Expected Results:**
- The old "all other service-layer modules depend on" phrase is gone.
- `hkask-services-research` is named as the intentional exception.
- `hkask-services-core` compiles.

## Related Documents

- `crates/hkask-services-core/src/lib.rs` — the corrected doc comment
- Architecture review of the core service crates (2026-07-17) — surfaced the drift
- ADR-043 (Eliminate Nested Runtime Panics) — same `core` crate, runtime-panic discipline

## References

[^ousterhout]: Ousterhout, J. (2018). *A Philosophy of Software Design.* Chapter 1 — a module is deep when its interface is simple relative to the complexity it hides; interface claims must match reality.
[^nygard-adr]: Nygard, M. (2011). *Documenting Architecture Decisions.* Relevance. — the ADR format this record follows.

---

*ℏKask - A Minimal Viable Container for UserPods — v0.31.0*