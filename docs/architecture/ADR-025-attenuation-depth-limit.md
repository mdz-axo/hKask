---
title: "ADR-025: 7-Level Attenuation Depth Limit"
audience: [architects, security engineers]
last_updated: 2026-05-29
version: "0.27.0"
status: "Active"
domain: "Technology"
ddmvss_categories: [trust]
---

# ADR-025: 7-Level Attenuation Depth Limit

**Date:** 2026-05-29 (retroactive)  
**Status:** Implemented  
**Supersedes:** N/A

## Context

OCAP capability tokens support delegation chains where each delegation creates an attenuated child token. Without a depth limit, delegation chains could grow unboundedly, creating verification latency proportional to chain length. A maximum attenuation depth must be chosen that balances delegation flexibility with verification cost.

## Decision

**Maximum attenuation depth of 7 levels.** This is the hard cap enforced by `DelegationToken::can_attenuate()`.

```rust
pub const MAX_ATTENUATION_LEVEL: u8 = 7;

pub fn can_attenuate(&self) -> bool {
    self.attenuation_level < self.max_attenuation
}
```

The 8th attenuation attempt returns `None`. Chain verification is bounded to O(7) = O(1) constant time.

## Rationale

1. **Miller's principle of least authority.** [^miller-robust] Each attenuation step narrows authority. Beyond some depth, the remaining authority is negligible — further delegation adds no practical value.

2. **Constant-time verification.** Chain verification walks the delegation chain. Bounding depth to 7 ensures verification is O(1) regardless of how many delegations occur.

3. **Empirical grounding.** The matroshka (nesting doll) limit of 7 mirrors the template cascade depth limit. Both follow the same structural recursion bound — what recurses in one dimension recurses in the other.

4. **Cognitive ceiling.** Miller's Law [^miller-magical] identifies 7±2 as the number of items humans can hold in working memory. A delegation chain deeper than 7 exceeds human ability to reason about authority flow.

5. **Ashby's variety bound.** [^ashby-law] Each attenuation reduces variety (authority scope). Seven levels provide 2^7 = 128 distinct attenuation states — sufficient variety for most delegation topologies.

## Consequences

### Positive

- O(1) chain verification regardless of system age
- Bounded recursion matches template cascade limit
- Prevents runaway delegation in autonomous agent systems
- Aligns with human cognitive limits for security auditing

### Negative

- A fixed limit may restrict deeply nested organizational structures
- No mechanism for privilege escalation (by design — this is OCAP, not RBAC)

## Compliance

| Principle | Compliance |
|-----------|-----------|
| P1 (No trait without two consumers) | ✅ `MAX_ATTENUATION_LEVEL` used by `AgentPod::delegate()` and `DelegationToken::attenuate()` |
| C5 (Every error variant is unique recovery path) | ✅ `AgentPodError::AttenuationLimitExceeded` is distinct |
| Miller (Least authority) | ✅ Bounded attenuation enforces authority reduction at each step |

## References

[^miller-robust]: Miller, M. S. (2006). *Robust Composition*. Johns Hopkins University.
[^miller-magical]: Miller, G. A. (1956). The magical number seven, plus or minus two. *Psychological Review*, 63(2), 81–97.
[^ashby-law]: Ashby, W. R. (1956). *An Introduction to Cybernetics*. Wiley.

---

*ℏKask - A Minimal Viable Container for Agents — ADR-025 — v0.21.0*
