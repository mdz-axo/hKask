---
title: "ADR-031: Consolidation Authorization via Explicit Passphrase + Affirmative Consent"
audience: [architects, security engineers]
last_updated: 2026-07-10
version: "0.30.0"
status: "Active"
domain: "Security"
mds_categories: [trust]
---

# ADR-031: Consolidation Authorization via Explicit Passphrase + Affirmative Consent

**Date:** 2026-06-05  
**Status:** Active (simplified 2026-07-10)
**Supersedes:** N/A (extends ADR-027)  
**Related:** ADR-027 (Argon2id + HKDF-SHA256 Master Key Derivation), archived ADR-040 (Service Layer Extraction — complete)

## Context

Consolidation (episodic→semantic memory promotion and semantic cleanup) is a destructive operation: it deletes low-confidence semantic hMems and promotes episodic hMems to semantic memory. Without authorization, any caller could trigger irreversible data loss.

Consolidation crosses the episodic/semantic boundary — it requires both `EpisodicMemory` and `SemanticMemory`. Under the Magna Carta, both categories are sovereign and require P2 affirmative consent.

## Decision

**All consolidation operations are routed through `AgentService::consolidate_agent_memory`, which enforces P2 affirmative consent before opening the per-agent memory DB.**

User-facing surfaces additionally verify the submitted passphrase directly against the canonical `HKASK_DB_PASSPHRASE` credential. Onboarding stores the user-supplied database passphrase unchanged. Capability signing and SQLCipher encryption are separate concerns; no signing key is reused or transformed into a database credential.

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
   - Checks `ConsentManager::has_consent` for both `DataCategory::EpisodicMemory` and `DataCategory::SemanticMemory`. Missing-consent observations are emitted as `cns.consent.denied` ν-events when the consent manager has a CNS event sink.
   - On missing consent, returns `ServiceError::ConsentDenied` and instructs the caller to grant consent via `kask sovereignty grant --category <category> [--agent <agent_name>]` or the API sovereignty consent endpoint.
   - Opens the per-agent memory DB at `agent_paths::agent_memory_db(agent_name)` using the configured passphrase.
   - Runs `ConsolidationService::consolidate` and returns the outcome.

2. **Removed bypass:** The previous direct `consolidation_ops::consolidate(path, passphrase)` entry point has been removed. `hkask-memory::consolidation_ops` now only exports `verify_passphrase` and `check_rate_limit` helpers.

3. **Curator auto-consolidation ("memory condensation"):**
   - Controlled by `ServiceConfig::curator_auto_consolidation_enabled`, read from `HKASK_CURATOR_AUTO_CONSOLIDATION=1` (default `false`).
   - When enabled, `CurationLoop::act()` checks consent for the Curator WebID for both memory categories before calling `ConsolidationBridge::consolidate`.
   - Consent for the Curator WebID is granted with `kask sovereignty grant --category <category> --agent curator`.
   - When disabled or consent is missing, the branch is skipped and an escalation entry is posted to the escalation queue, notifying the user.
   - When it runs, a second escalation entry records the event counts.

### Rate Limiting

The API endpoint enforces a coarse-grained 30-second rate limit to limit online passphrase guessing. The implementation uses a global `AtomicU64` timestamp, which is sufficient for a single-user headless system but is not per-agent.

## Rationale

1. **Keeps responsibilities explicit.** Database passphrases encrypt databases. A2A and OCAP secrets sign capability tokens. No credential is transformed into another domain's credential.

2. **P2 affirmative consent is the primary gate.** Passphrase verification proves the caller knows the master key, but consent records the user's explicit authorization to operate on sovereign memory. Both are required.

3. **Threat model: shared-database vs single-user.** In single-user deployments, the REPL session is already the authorized user—consent plus session identity is sufficient. In shared-database deployments, the API endpoint adds caller→target WebID equality and direct passphrase verification.

4. **Curator auto-consolidation is opt-in.** Defaulting to `false` prevents silent, unattended memory mutation. When enabled, consent must still be granted separately, and every skip/run is recorded in the escalation queue.

5. **Rate limiting protects the explicit credential.** Consolidation remains rate-limited to reduce online guessing without adding a second derivation representation.

6. **All surfaces use the same per-agent memory DB.** The agent's per-agent memory DB contains both episodic and semantic hMems. The registry DB (`hkask.db`) is for agent registration, not memory.

## Consequences

### Positive

- Consolidation is protected by both consent and passphrase/auth on all surfaces.
- The previous `consolidation_ops::consolidate` bypass no longer exists.
- Curator auto-consolidation is off by default and double-gated by configuration + consent.
- MCP servers are correctly scoped — no cross-domain consolidation tool.
- Rate limiting prevents API CPU DoS.
- Users receive explicit error messages and escalation-queue notifications when consent is missing.

### Negative


- Curator auto-consolidation requires two independent configuration steps (env flag + consent grant) before it will run.
- The API request field changed from `agent_webid` to `agent_name` so the service layer can derive the WebID and locate the per-agent memory DB.

## Compliance

| Principle | Compliance |
|-----------|-----------|
| P1 (User Sovereignty) | ✅ No root/admin path; every operation carries a WebID |
| P2 (Affirmative Consent) | ✅ All paths check `EpisodicMemory` + `SemanticMemory` consent; denials return actionable messages |
| P4 (Clear Boundaries / OCAP) | ✅ Single entry point in `AgentService`; direct `Database::open` bypass removed |
| ADR-027 (Master key derivation) | ✅ Reuses `derive_all_internal_secrets()` exactly |
| Headless constraint (§1.6) | ✅ No visual UI; notifications are CLI output, API responses, escalation-queue entries, and CNS spans (`cns.curator.consolidation`) |

## Post-Implementation Simplification (2026-06-28)

An adversarial review of the consolidation fix surfaced 12 anti-patterns. Each was investigated
and classified. Fixed items are listed below; deferred items are documented here.

### Fixed

- **Config flag deduplication** — `read_curator_auto_consolidation_env()` extracted.
- **WebID derivation** — `WebID::for_agent_name(agent_name)` convenience method added.
- **Module naming** — `consolidation_ops` → `consolidation_auth` (module no longer performs consolidation).
- **Category display** — `DataCategory::all_known()` replaces hand-rolled tuple array in sovereignty display.
- **Error variant** — `ServiceError::Forbidden` separates authorization failures (P4) from consent denials (P2).
- **REPL status queries** — `/consolidate` status display now routes through `AgentService::consolidation_status_for()` for fresh reads instead of a stale cached `ConsolidationService`.
- **CNS spans** — Curator auto-consolidation events now emit `cns.curator.consolidation` spans in addition to escalation-queue entries.
- **Integration test** — `consolidate_agent_memory_consent_checks` added, covering both consent-denied and consent-granted paths.

### Deferred — CuratorContext.consent_manager optionality

`CuratorContext.consent_manager` is `Option<Arc<ConsentManager>>` — required in production
(always wired via `build_loops()`) but optional for standalone CLI metacognition and test paths.
The Curation Loop's branch for `None` is 3 lines and defensively handles the absent case.

**Decision:** Retain optionality. Making it required would force all test paths to construct
a full ConsentManager with its keystore/DB dependencies. The optionality enables the
CuratorContext to exist in contexts where sovereignty checks are not applicable.

### Deferred — REPL cannot grant consent

`/sovereignty` in the REPL is status-only. Users who hit consent denial must leave the
REPL and run `kask sovereignty grant`. This is intentional friction: consent grants are
high-stakes authorization operations. Requiring an out-of-band CLI action ensures the
user deliberately authorizes data sharing rather than doing so with a quick slash command.
If user feedback demands inline consent, adding `/sovereignty grant <category>` is a
small, safe change (the REPL already has the `service_context` and `sovereignty()` accessor).

### Deferred — Notifications are escalation-queue-only

The headless system design treats escalation-queue entries as the primary user-notification
mechanism. CNS spans (`cns.curator.consolidation`) are now also emitted for observability.
Adding Matrix messages would require the `communication` feature and a Matrix homeserver,
which is not guaranteed in all deployments. The escalation queue is available in all
configurations (it lives in the primary SQLite DB).

---

*ℏKask - A Minimal Viable Container for UserPods — ADR-031 — v0.31.0*
