---
title: "ADR-031: Consolidation Authorization via Master Passphrase Derivation"
audience: [architects, security engineers]
last_updated: 2026-06-05
version: "2.0.0"
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

Consolidation crosses the episodic/semantic boundary — it requires both `EpisodicMemory` and `SemanticMemory`. This means it can only be performed by surfaces that have access to both stores.

Three surfaces expose consolidation:

1. **CLI** (`kask consolidate --passphrase`) — single-user, interactive
2. **API** (`POST /api/consolidate`) — headless, network-accessible
3. **Chat** (`/consolidate run`) — single-user, interactive

Two MCP servers provide memory operations but **cannot perform consolidation**:

4. **MCP Episodic** (`hkask-mcp-episodic`) — episodic store/recall only; read-only consolidation status
5. **MCP Semantic** (`hkask-mcp-semantic`) — semantic store/recall/embed/search only

The CLI, API, and Chat need explicit user authorization since they are directly user-facing. MCP servers are OCAP-gated and do not expose consolidation because they lack both stores.

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

**Each surface applies authorization and connects the same way:**

| Surface | Authorization | DB | Can consolidate? |
|---------|--------------|-----|-----------------|
| CLI | `--passphrase` flag, derives capability_key, compares | `hkask-memory-{agent}.db` | ✅ Full (episodic→semantic + cleanup) |
| API | `passphrase` field in request body, derives capability_key, compares | `hkask-memory-{agent}.db` | ✅ Full (episodic→semantic + cleanup) |
| Chat | No passphrase (single-user REPL, already authenticated) | `hkask-memory-{agent}.db` | ✅ Full (episodic→semantic + cleanup) |
| MCP Episodic | OCAP GovernedTool membrane | `hkask-memory-{agent}.db` | ❌ Read-only status |
| MCP Semantic | OCAP GovernedTool membrane | `hkask-memory-{agent}.db` | ❌ No consolidation tool |

All three consolidation surfaces (CLI, API, Chat) build `ConsolidationService` from the agent's per-agent memory DB (`hkask-memory-{agent}.db`) using the same pattern: open DB → `EpisodicMemory` + `SemanticMemory` → `ConsolidationBridge` → `ConsolidationService`.

Both MCP servers connect to the same per-agent memory DB via `HKASK_MEMORY_DB`, but each only has access to one memory type (EpisodicMemory or SemanticMemory), so they cannot perform cross-store consolidation.

### Rate Limiting

The API endpoint enforces a coarse-grained rate limit (30-second minimum interval between requests) to prevent CPU denial-of-service via repeated Argon2id derivation (~100ms CPU per call). The CLI and Chat paths are single-user and do not need rate limiting.

## Rationale

1. **Reuses existing derivation infrastructure.** The `derive_all_internal_secrets()` function from ADR-027 already produces `capability_key`. No new cryptographic primitives or key storage needed.

2. **Threat model: shared-database vs single-user.** In single-user deployments (the default), the Curator's REPL session is already the authorized user — no passphrase needed for `/consolidate`. In shared-database deployments, the API endpoint is the attack surface: rate limiting + passphrase verification prevent both unauthorized and DoS attacks.

3. **Consolidation requires both stores.** Episodic→semantic promotion reads from EpisodicMemory and writes to SemanticMemory. Individual MCP servers only have one store, so consolidation is structurally impossible there. This is correct — MCP tools are scoped to their domain.

4. **Argon2id is intentionally expensive.** The ~100ms derivation cost is a feature, not a bug — it prevents brute-force attacks on the passphrase. Rate limiting prevents this cost from being weaponized as a CPU DoS vector.

5. **All consolidation surfaces use the same DB.** The agent's per-agent memory DB (`hkask-memory-{agent}.db`) contains both episodic and semantic triples. The registry DB (`hkask.db`) is for agent registration, not memory. Previously, the API and Chat paths incorrectly used in-memory or registry DBs for consolidation, meaning they operated on the wrong data.

## Consequences

### Positive

- Consolidation is now a protected operation across all surfaces
- No new secrets or key storage — reuses the existing HKDF derivation chain
- All consolidation surfaces use the same per-agent memory DB (correct data)
- MCP servers are correctly scoped — no cross-domain consolidation
- Rate limiting prevents API CPU DoS

### Negative

- Every API consolidation request pays ~100ms Argon2id cost (mitigated by rate limiting)
- Chat `/consolidate` operates without passphrase (acceptable: single-user REPL)
- MCP servers cannot consolidate — agents must use CLI or API (by design)

## Compliance

| Principle | Compliance |
|-----------|-----------|
| C5 (Every error variant is unique recovery path) | ✅ Invalid passphrase → 401, Rate limited → 429, Invalid UUID → 400 |
| ADR-027 (Master key derivation) | ✅ Reuses `derive_all_internal_secrets()` exactly |
| Headless constraint (§1.6) | ✅ No visual UI, all auth is CLI/MCP/API |

---

*ℏKask - A Minimal Viable Container for Agents — ADR-031 — v0.22.0*