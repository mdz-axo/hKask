---
title: "ADR-055: Per-Provider Circuit Breaker (Deferred)"
audience: [architects, developers, security-reviewers]
last_updated: 2026-07-20
version: "0.31.0"
status: "Active"
domain: "Application"
mds_categories: [composition, trust, lifecycle]
---

# ADR-055: Per-Provider Circuit Breaker (Deferred)

**Date:** 2026-07-20  
**Status:** Active (deferred enhancement)  
**Supersedes:** None

## Context

The `hkask-mcp-research` server's `ProviderPool` queries all matching providers
in parallel via `join_all` during a compound search. If a provider returns
errors on every call (e.g., expired API key, revoked endpoint, chronic
timeout), it is retried on every subsequent `web_search` with no backoff or
circuit-breaking.

Both the 2026-07-17 review ([`research-mcp-adversarial-review-2026-07-17.md`](../../status/research-mcp-adversarial-review-2026-07-17.md))
and the 2026-07-20 follow-up ([`research-mcp-adversarial-review-2026-07-20.md`](../../status/research-mcp-adversarial-review-2026-07-20.md))
identified this as a variety deficit per Ashby's Law of Requisite Variety:

> The regulator lacks variety to handle chronic provider failure. A provider
> that returns errors on every call is retried on every `web_search` with no
> circuit breaker or exponential backoff.

[^ashby]: Ashby, W. R. (1956). *An Introduction to Cybernetics.* Chapman & Hall. — Ashby's Law of Requisite Variety: the regulator must have at least as much variety as the disturbance it regulates.

## Decision

**Defer the circuit breaker as a separate enhancement.** The 2026-07-20
review addressed 11 findings; the circuit breaker is enhancement #12, not a
correctness bug. The current behavior is:

- A failing provider is recorded in `providers_failed` on each compound search.
- The compound search still succeeds if at least one provider returns results.
- The `COMPOUND_PROVIDER_TIMEOUT_SECS` (10s) cap (fixed in the 2026-07-17
  review, G3) prevents a hanging provider from blocking the entire search.

The missing capability is: skipping a provider that has failed N consecutive
times for a cooldown period, rather than retrying it on every call.

## Rationale

1. **Not a correctness bug.** The current behavior produces correct results;
   it just wastes time and API calls on a chronically-failing provider.
2. **The 10s timeout (G3) bounds the damage.** A hanging provider cannot
   block the search indefinitely; it times out after 10s and is recorded as
   failed.
3. **The fix requires state.** A circuit breaker needs per-provider failure
   counters and cooldown timestamps — mutable state on `ProviderPool` (which
   is currently immutable behind `Arc`). This is a structural change, not a
   one-line fix.
4. **The 2026-07-20 fixes addressed higher-priority issues first.** The
   stored-SSRF (N11), dead OCAP (N1), and transaction safety (N3) are
   security/correctness issues; the circuit breaker is a performance/reliability
   enhancement.

[^fowler-cb]: Fowler, M. (2014). *Circuit Breaker.* https://martinfowler.com/bliki/CircuitBreaker.html — the canonical circuit breaker pattern description.

## Consequences

- **Positive:** The decision preserves focus on correctness over
  performance optimization. The 2026-07-20 fixes are shippable without
  entangling them with a state-management change.
- **Negative:** A chronically-failing provider wastes one HTTP request + 10s
  timeout per compound search. For users with 5+ providers where one is
  broken, this adds ~10s latency to every search until the provider is fixed
  or the API key is removed.
- **Mitigation:** Users can remove the broken provider's API key from their
  environment, which excludes it from the pool at startup.

## Future Implementation

When implemented, the circuit breaker should:

1. Track per-provider consecutive failure count (e.g., `AtomicU32` per
   provider).
2. After N consecutive failures (default: 5), enter "open" state for a
   cooldown period (default: 60s).
3. In "open" state, skip the provider entirely (record as
   `ProviderError { kind, error: "circuit open" }`).
4. After the cooldown, enter "half-open" state: try one request. If it
   succeeds, reset the counter. If it fails, re-enter "open" state.

The state should live on `ProviderPool` (or a new `ProviderHealthTracker`
struct wrapped in `Arc<Mutex<...>>`), not on individual providers — the
providers are stateless structs.

## Cross-links

- [Research MCP Adversarial Review (2026-07-17)](../../status/research-mcp-adversarial-review-2026-07-17.md) — G3 (compound timeout), variety deficit note
- [Research MCP Adversarial Review (Follow-Up 2026-07-20)](../../status/research-mcp-adversarial-review-2026-07-20.md) — item #7 (this ADR)
- [Architecture Patterns](../../explanation/architecture-patterns.md) — hexagonal ports, ProviderPool adapter
