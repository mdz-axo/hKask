---
title: "ADR-031: Consolidation Authorization via Master Passphrase Derivation + Affirmative Consent"
audience: [architects, security engineers]
last_updated: 2026-06-28
version: "0.30.0"
status: "Active"
domain: "Security"
mds_categories: [trust]
---

# ADR-031: Consolidation Authorization via Master Passphrase Derivation + Affirmative Consent

**Date:** 2026-06-05  
**Status:** Active (amended 2026-06-28)  
**Supersedes:** N/A (extends ADR-027)  
**Related:** ADR-027 (Argon2id + HKDF-SHA256 Master Key Derivation), ADR-040 (Service Layer Extraction)

## Context

Consolidation (episodic→semantic memory promotion and semantic cleanup) is a destructive operation: it deletes low-confidence semantic triples and promotes episodic triples to semantic memory. Without authorization, any caller could trigger irreversible data loss.

Consolidation crosses the episodic/semantic boundary — it requires both `EpisodicMemory` and `SemanticMemory`. Under the Magna Carta, both categories are sovereign and require P2 affirmative consent.

## Decision

**All consolidation operations are routed through `AgentService::consolidate_agent_memory`, which enforces P2 affirmative consent before opening the per-agent memory DB.**

User-facing surfaces additionally verify the master passphrase as an authentication gate. The verification flow:

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

| Surface | Auth gate | Consent gate | Default | Status |
|---------|-----------|--------------|---------|--------|
| CLI (`kask consolidate --passphrase`) | `--passphrase` when targeting a non-Curator agent | `EpisodicMemory` + `SemanticMemory` for target agent | Manual trigger only | Implemented in `crates/hkask-cli/src/commands/consolidation.rs` |
| API (`POST /api/consolidate`) | `passphrase` field; caller WebID must equal target agent | `EpisodicMemory` + `SemanticMemory` for target agent | Manual trigger only | Implemented in `crates/hkask-api/src/routes/consolidation.rs` |
| Chat/REPL (`/consolidate run`) | Single-user session | `EpisodicMemory` + `SemanticMemory` for current agent | Manual trigger only | Implemented in `crates/hkask-cli/src/repl/handlers/consolidation.rs` |
| Curator daemon (`CurationLoop`) | N/A — daemon-internal | `EpisodicMemory` + `SemanticMemory` for Curator WebID | **Disabled** unless `HKASK_CURATOR_AUTO_CONSOLIDATION=1` | Implemented in `crates/hkask-agents/src/curator/curation_loop.rs` |
| MCP Memory | OCAP GovernedTool membrane | ❌ No consolidation tool | By design | By design |

**Authorization model:**

1. **Canonical entry point:** `AgentService::consolidate_agent_memory(agent_name, request)` (`crates/hkask-services-context/src/context_impl.rs`).
   - Derives the target `WebID` from `agent_name`.
   - Checks `ConsentManager::has_consent` for both `DataCategory::EpisodicMemory` and `DataCategory::SemanticMemory`.
   - On missing consent, returns `ServiceError::ConsentDenied` and instructs the caller to grant consent via `kask sovereignty grant --category <category>` or the API consent endpoint.
   - Opens the per-agent memory DB at `agent_paths::agent_memory_db(agent_name)` using the configured passphrase.
   - Runs `ConsolidationService::consolidate` and returns the outcome.

2. **Removed bypass:** The previous direct `consolidation_ops::consolidate(path, passphrase)` entry point has been removed. `hkask-memory::consolidation_ops` now only exports `verify_passphrase` and `check_rate_limit` helpers.

3. **Curator auto-consolidation ("memory condensation"):**
   - Controlled by `ServiceConfig::curator_auto_consolidation_enabled`, read from `HKASK_CURATOR_AUTO_CONSOLIDATION=1` (default `false`).
   - When enabled, `CurationLoop::act()` checks consent for the Curator WebID for both memory categories before calling `ConsolidationBridge::consolidate`.
   - When disabled or consent is missing, the branch is skipped and an escalation entry is posted to the escalation queue, notifying the user.
   - When it runs, a second escalation entry records the event counts.

### Rate Limiting

The API endpoint enforces a coarse-grained rate limit (30-second minimum interval between requests) to prevent CPU denial-of-service via repeated Argon2id derivation (~100ms CPU per call). The implementation uses a global `AtomicU64` timestamp (`crates/hkask-memory/src/consolidation_ops.rs`), which is sufficient for a single-user headless system but is not per-agent.

## Rationale

1. **Reuses existing derivation infrastructure.** The `derive_all_internal_secrets()` function from ADR-027 already produces `capability_key`. No new cryptographic primitives or key storage needed.

2. **P2 affirmative consent is the primary gate.** Passphrase verification proves the caller knows the master key, but consent records the user's explicit authorization to operate on sovereign memory. Both are required.

3. **Threat model: shared-database vs single-user.** In single-user deployments, the REPL session is already the authorized user — consent plus session identity is sufficient. In shared-database deployments, the API endpoint adds caller→target WebID equality and passphrase verification.

4. **Curator auto-consolidation is opt-in.** Defaulting to `false` prevents silent, unattended memory mutation. When enabled, consent must still be granted separately, and every skip/run is recorded in the escalation queue.

5. **Argon2id is intentionally expensive.** The ~100ms derivation cost is a feature, not a bug — it prevents brute-force attacks on the passphrase. Rate limiting prevents this cost from being weaponized as a CPU DoS vector.

6. **All surfaces use the same per-agent memory DB.** The agent's per-agent memory DB contains both episodic and semantic triples. The registry DB (`hkask.db`) is for agent registration, not memory.

## Consequences

### Positive

- Consolidation is protected by both consent and passphrase/auth on all surfaces.
- The previous `consolidation_ops::consolidate` bypass no longer exists.
- Curator auto-consolidation is off by default and double-gated by configuration + consent.
- MCP servers are correctly scoped — no cross-domain consolidation tool.
- Rate limiting prevents API CPU DoS.
- Users receive explicit error messages and escalation-queue notifications when consent is missing.

### Negative

- Every API consolidation request pays ~100ms Argon2id cost (mitigated by rate limiting).
- Curator auto-consolidation requires two independent configuration steps (env flag + consent grant) before it will run.
- The API request field changed from `agent_webid` to `agent_name` so the service layer can derive the WebID and locate the per-agent memory DB.

## Compliance

| Principle | Compliance |
|-----------|-----------|
| P1 (User Sovereignty) | ✅ No root/admin path; every operation carries a WebID |
| P2 (Affirmative Consent) | ✅ All paths check `EpisodicMemory` + `SemanticMemory` consent; denials return actionable messages |
| P4 (Clear Boundaries / OCAP) | ✅ Single entry point in `AgentService`; direct `Database::open` bypass removed |
| ADR-027 (Master key derivation) | ✅ Reuses `derive_all_internal_secrets()` exactly |
| Headless constraint (§1.6) | ✅ No visual UI; notifications are CLI output, API responses, and escalation-queue entries |

---

*ℏKask - A Minimal Viable Container for Replicants — ADR-031 — v0.30.0*
