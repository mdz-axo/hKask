---
title: "ADR-033: Dampener Override Cooldown Semantics"
audience: [architects, security engineers, developers]
last_updated: 2026-06-07
version: "0.27.0"
status: "Draft"
domain: "Technology"
mds_categories: [trust, lifecycle]
---

# ADR-033: Dampener Override Cooldown Semantics

**Date:** 2026-06-07
**Status:** Draft
**Related:** [`MDS.md §7.1-7.2`](MDS.md §7.1-7.2) §4.3, OPEN_QUESTIONS.md FUT-003, FUT-007

## Context

The `Dampener` in `hkask-cns` implements metacognitive override suppression — when an agent's override is processed, all subsequent overrides are suppressed for a cooldown period. The current implementation uses a global cooldown: any issuer's override suppresses all subsequent overrides from all issuers for 120 seconds.

This creates a trust starvation risk: a low-trust issuer (e.g., a bot with `AgentKind::Process`) can suppress overrides from a high-trust issuer (e.g., the human Curator) for the full 120s cooldown window. In adversarial or high-frequency override scenarios, this effectively silences the human operator.

**Problem Statement:** Should `Dampener.override_cooldown` be per-issuer or global?

**Stakeholders:** Agent developers, CNS architects, trust model reviewers

**Constraints:** OCAP model requires all override issuers to present capability tokens; headless constraint means no visual UI for override negotiation

## Decision

**Override cooldown is per-issuer, keyed by `WebID`.**

Each issuer gets its own cooldown window. Override from issuer A does not suppress overrides from issuer B. The cooldown is tracked as `HashMap<WebID, Instant>` with bounded capacity.

### Cooldown Semantics

| Override From | Effect on Same Issuer | Effect on Other Issuers |
|---------------|----------------------|------------------------|
| Issuer A | Suppress A's overrides for 120s | No effect on B, C, etc. |
| Issuer B | Suppress B's overrides for 120s | No effect on A, C, etc. |

### Implementation Details

1. **Data structure:** `HashMap<WebID, Instant>` replaces the current global `Option<Instant>`.
2. **Capacity bound:** Maximum 64 concurrent cooldown entries. When capacity is reached, the oldest entry is evicted (FIFO). This bounds memory growth.
3. **Cooldown duration:** Remains 120 seconds per issuer (configurable via `HKASK_CNS_OVERRIDE_COOLDOWN_SECS`).
4. **Eviction:** Expired entries are pruned on each `check_override()` call (lazy GC).
5. **Capability requirement:** Override callers must present `CnsOverrideCapability` (already required by existing OCAP model).

**Alternatives Considered:**
1. **Global cooldown (current)** — Rejected: A low-trust issuer can starve a high-trust issuer. This violates the trust hierarchy.
2. **Per-issuer with trust-tier scaling** — Deferred to FUT-007: Trust-tier scaling (e.g., 60s for human, 120s for bot) is a natural extension but introduces complexity that should be a separate ADR.
3. **No cooldown** — Rejected: Without cooldown, override storms can cause metacognitive oscillation (the Dampener's core purpose).

**Rationale:** Per-issuer cooldown preserves the Dampener's core purpose (preventing override storms from any single issuer) while eliminating cross-issuer starvation. The 64-entry capacity bound prevents memory growth proportional to active issuers while being sufficient for any realistic deployment (11 core crates, each with at most a few active agents).

## Consequences

### Positive

- High-trust issuers (Curator) cannot be starved by low-trust issuers (bots)
- Override storms from one issuer still get dampened (per-issuer 120s)
- Capacity bound prevents unbounded memory growth
- Lazy GC keeps the implementation simple

### Negative

- `HashMap<WebID, Instant>` is more complex than `Option<Instant>`
- 64-entry capacity bound could be exceeded in extreme deployments (unlikely: 11 crates × ~2 agents each)
- Per-issuer cooldown does not prevent coordinated override storms from multiple issuers (mitigated by separate trust-tier scaling in FUT-007)

### Neutral

- Cooldown duration stays at 120s per issuer (same behavioral guarantee per-issuer)
- The Dampener's dedup logic (same override content filtered) remains unchanged

## Compliance

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P4** (No builder without fallibility) | ✅ | `check_override()` returns `Result`, capacity eviction is explicit |
| **C5** (Every error variant is unique recovery path) | ✅ | `OverrideRejected::CooldownActive(WebID, Duration)` provides issuer-specific recovery |
| **C7** (Divergence must yield) | ✅ | Per-issuer cooldown is a convergence from the current global model |

## Verification

```bash
# Verify per-issuer cooldown data structure
grep -r "HashMap.*WebID.*Instant" crates/hkask-cns/ | wc -l

# Verify capacity bound
grep -r "64" crates/hkask-cns/src/dampener.rs | head -5

# Verify no global cooldown remains
grep -r "Option<Instant>" crates/hkask-cns/src/dampener.rs | wc -l

# Run CNS tests
cargo test -p hkask-cns
```

**Expected Results:**
- Per-issuer `HashMap<WebID, Instant>` replaces global `Option<Instant>`
- 64-entry capacity bound documented and enforced
- All CNS tests pass
- No `Option<Instant>` global cooldown remains

## Related Documents

- [`MDS.md §7.1-7.2`](MDS.md §7.1-7.2) §4.3 — Dampener override model
- [`MDS.md §7.3`](MDS.md §7.3) §1 — Security model
- [`OPEN_QUESTIONS.md`](../OPEN_QUESTIONS.md) FUT-003, FUT-007 — Override cooldown questions

## References

[^dampener]: Ashby, W.R. (1956). *An Introduction to Cybernetics*. Chapman & Hall. Chapter 11: Regulation.
[^ocap]: Miller, M. (2006). *Robust Composition: Towards a National Research Agenda for Object Capability Security*. HP Labs.

---

*ℏKask - A Minimal Viable Container for Agents — ADR-033 — v0.23.0*