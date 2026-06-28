---
title: "ADR-031: Consolidation Authorization via Master Passphrase Derivation"
audience: [architects, security engineers]
last_updated: 2026-06-28
version: "0.30.0"
status: "Active"
domain: "Security"
mds_categories: [trust]
---

# ADR-031: Consolidation Authorization via Master Passphrase Derivation

**Date:** 2026-06-05  
**Status:** Active (amended 2026-06-28)  
**Supersedes:** N/A (extends ADR-027)  
**Related:** ADR-027 (Argon2id + HKDF-SHA256 Master Key Derivation), ADR-040 (Service Layer Extraction)

## Context

Consolidation (episodic→semantic memory promotion and semantic cleanup) is a destructive operation: it deletes low-confidence semantic triples and promotes episodic triples to semantic memory. Without authorization, any caller could trigger irreversible data loss.

Consolidation crosses the episodic/semantic boundary — it requires both `EpisodicMemory` and `SemanticMemory`.

## Decision

**User-facing surfaces verify the master passphrase before consolidation.**

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

**Surfaces that expose consolidation:**

| Surface | Authorization | Can consolidate? | Status |
|---------|--------------|-----------------|--------|
| CLI (`kask consolidate --passphrase`) | `--passphrase` flag | ✅ Full | Implemented in `crates/hkask-cli/src/commands/consolidation.rs` |
| API (`POST /api/consolidate`) | `passphrase` field | ✅ Full | Implemented in `crates/hkask-api/src/routes/consolidation.rs` |
| Chat/REPL (`/consolidate run`) | No passphrase (single-user) | ✅ Full | **Not currently implemented** |
| Curator daemon (`CurationLoop`) | None — fires when `pending_escalations_exist` | ✅ Auto | **Implemented without passphrase check** |
| MCP Memory | OCAP GovernedTool membrane | ❌ No consolidation tool | By design |

### Deviation from Original Decision (2026-06-28)

The original ADR restricted consolidation to CLI, API, and Chat surfaces that can verify the master passphrase. The current implementation also auto-fires the `ConsolidationBridge` from `CurationLoop::act()` inside the `pending_escalations_exist` branch (`crates/hkask-agents/src/curator/curation_loop.rs:450-467`). This path does **not** verify the master passphrase; it relies on the Curator daemon's existing OCAP boundary and the fact that the bridge is constructed with the Curator's own per-agent memory DB.

This is an unresolved tension: the auto-consolidation path is structurally inside the Curator's capability envelope but is not covered by the user-facing passphrase authorization described in this ADR. A follow-up decision is required to either:

1. Remove Curator auto-consolidation and require explicit user authorization.
2. Document Curator auto-consolidation as an OCAP-gated daemon exception with a separate attenuation token.
3. Add a configuration switch that disables auto-consolidation by default.

### Rate Limiting

The API endpoint enforces a coarse-grained rate limit (30-second minimum interval between requests) to prevent CPU denial-of-service via repeated Argon2id derivation (~100ms CPU per call). The implementation uses a global `AtomicU64` timestamp (`crates/hkask-memory/src/consolidation_ops.rs:27`), which is sufficient for a single-user headless system but is not per-agent.

## Rationale

1. **Reuses existing derivation infrastructure.** The `derive_all_internal_secrets()` function from ADR-027 already produces `capability_key`. No new cryptographic primitives or key storage needed.

2. **Threat model: shared-database vs single-user.** In single-user deployments (the default), the Curator's REPL session is already the authorized user — no passphrase needed for Chat consolidation. In shared-database deployments, the API endpoint is the attack surface: rate limiting + passphrase verification prevent both unauthorized and DoS attacks.

3. **Consolidation requires both stores and surface authorization.** Episodic→semantic promotion reads from EpisodicMemory and writes to SemanticMemory. `hkask-mcp-memory` has access to both stores but intentionally does not expose a consolidation tool — consolidation is restricted to surfaces where user authorization can be verified (with the Curator daemon exception noted above).

4. **Argon2id is intentionally expensive.** The ~100ms derivation cost is a feature, not a bug — it prevents brute-force attacks on the passphrase. Rate limiting prevents this cost from being weaponized as a CPU DoS vector.

5. **All user-facing surfaces use the same per-agent memory DB.** The agent's per-agent memory DB contains both episodic and semantic triples. The registry DB (`hkask.db`) is for agent registration, not memory.

## Consequences

### Positive

- Consolidation is protected on CLI and API surfaces.
- No new secrets or key storage — reuses the existing HKDF derivation chain.
- MCP servers are correctly scoped — no cross-domain consolidation tool.
- Rate limiting prevents API CPU DoS.

### Negative

- Every API consolidation request pays ~100ms Argon2id cost (mitigated by rate limiting).
- Curator auto-consolidation bypasses the passphrase check, creating an undocumented authorization exception.
- Chat/REPL consolidation is described but not implemented.

## Compliance

| Principle | Compliance |
|-----------|-----------|
| P2 (Affirmative Consent) | ✅ CLI/API require explicit passphrase; ⚠️ Curator auto-consolidation does not |
| P4 (Clear Boundaries / OCAP) | ✅ MCP server blocked by tool absence; ⚠️ Curator auto-consolidation needs explicit OCAP attenuation token |
| ADR-027 (Master key derivation) | ✅ Reuses `derive_all_internal_secrets()` exactly |
| Headless constraint (§1.6) | ✅ No visual UI, all auth is CLI/MCP/API |

---

*ℏKask - A Minimal Viable Container for Replicants — ADR-031 — v0.30.0*
