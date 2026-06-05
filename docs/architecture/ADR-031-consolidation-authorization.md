---
title: "ADR-031: Consolidation Authorization via Master Passphrase Derivation"
audience: [architects, security engineers]
last_updated: 2026-06-05
version: "1.0.0"
status: "Active"
domain: "Security"
ddmvss_categories: [trust]
---

# ADR-031: Consolidation Authorization via Master Passphrase Derivation

**Date:** 2026-06-05
**Status:** Implemented
**Supersedes:** N/A (extends ADR-027)
**Related:** ADR-027 (Argon2id + HKDF-SHA256 Master Key Derivation)

## Context

Consolidation (episodic→semantic memory promotion and semantic cleanup) is a destructive operation: it deletes low-confidence semantic triples and promotes episodic triples to semantic memory. Without authorization, any caller could trigger irreversible data loss.

Three surfaces expose consolidation:

1. **CLI** (`kask consolidate --passphrase`) — single-user, interactive
2. **API** (`POST /api/consolidate`) — headless, network-accessible
3. **Chat** (`/consolidate run`) — single-user, interactive
4. **MCP** (`semantic_consolidate` tool) — OCAP-gated, machine-to-machine

The CLI and API need explicit user authorization since they are directly user-facing. The MCP tool is already gated behind the GovernedTool OCAP membrane (capability tokens required to invoke any tool), so adding passphrase verification would be redundant.

Before this decision, consolidation had no authorization at all — any code path could trigger it without verification.

## Decision

**Consolidation authorization uses the master-passphrase derivation chain (ADR-027) to verify the user's identity.**

The verification flow:

```
User-supplied passphrase
  │
  ├── Argon2id(passphrase, fixed_salt) → 256-bit master key  [~100ms]
  ├── HKDF-SHA256(master_key, "hkask:capability-key") → capability_key
  │
  └── Compare derived capability_key against resolved DB passphrase
```

This is the same derivation chain used during onboarding to store the DB encryption key in the OS keychain (`hkask-db-passphrase`). A correct passphrase produces a capability_key that matches the stored DB passphrase. An incorrect passphrase produces a different key.

**Each surface applies authorization differently:**

| Surface | Authorization | Rationale |
|---------|--------------|-----------|
| CLI | `--passphrase` flag, derives capability_key, compares | Direct user-facing, requires explicit consent |
| API | `passphrase` field in request body, derives capability_key, compares | Network-accessible, must authenticate caller |
| Chat | No passphrase (single-user REPL, already authenticated) | Interactive session, Curator identity already established |
| MCP | None (OCAP GovernedTool membrane) | Tool invocations require capability tokens; redundant to add passphrase |

### Rate Limiting

The API endpoint enforces a coarse-grained rate limit (30-second minimum interval between requests) to prevent CPU denial-of-service via repeated Argon2id derivation (~100ms CPU per call). The CLI and Chat paths are single-user and do not need rate limiting.

## Rationale

1. **Reuses existing derivation infrastructure.** The `derive_all_internal_secrets()` function from ADR-027 already produces `capability_key`. No new cryptographic primitives or key storage needed.

2. **Threat model: shared-database vs single-user.** In single-user deployments (the default), the Curator's REPL session is already the authorized user — no passphrase needed for `/consolidate`. In shared-database deployments, the API endpoint is the attack surface: rate limiting + passphrase verification prevent both unauthorized and DoS attacks.

3. **OCAP membranes are sufficient for MCP.** The GovernedTool membrane (see `hkask-mcp/src/dispatch.rs`) requires valid capability tokens for every tool invocation. Adding passphrase verification on top would be defense-in-depth against a threat (stolen capability token + consolidation intent) that is already mitigated by token scoping and attenuation limits.

4. **Argon2id is intentionally expensive.** The ~100ms derivation cost is a feature, not a bug — it prevents brute-force attacks on the passphrase. Rate limiting prevents this cost from being weaponized as a CPU DoS vector.

## Consequences

### Positive

- Consolidation is now a protected operation across all surfaces
- No new secrets or key storage — reuses the existing HKDF derivation chain
- OCAP gates are recognized as sufficient for MCP (no redundant verification)
- Rate limiting prevents API CPU DoS

### Negative

- Every API consolidation request pays ~100ms Argon2id cost (mitigated by rate limiting)
- Chat `/consolidate` operates without passphrase (acceptable: single-user REPL)
- The REPL ConsolidationService opens a second connection to the registry DB (documented architectural split)

## Compliance

| Principle | Compliance |
|-----------|-----------|
| C5 (Every error variant is unique recovery path) | ✅ Invalid passphrase → 401, Rate limited → 429, Invalid UUID → 400 |
| ADR-027 (Master key derivation) | ✅ Reuses `derive_all_internal_secrets()` exactly |
| Headless constraint (§1.6) | ✅ No visual UI, all auth is CLI/MCP/API |

---

*ℏKask - A Minimal Viable Container for Agents — ADR-031 — v0.22.0*