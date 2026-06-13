# Handoff — Energy/Gas/Payments/API Key System Implementation

**Session date:** 2026-06-13
**Project:** hKask v0.27.0
**Handoff from:** Full Gentle Lovelace replica build + P12 documentation + CLI auth + memory bridge
**Handoff to:** Implement 4 remaining gaps from the energy/gas/payments architecture (gap 4 deferred per essentialist G1)

---

## 1. Session Context

Completed a full documentation sweep, Gentle Lovelace replica design, corpus preparation (11 works), embedding pipeline extension, triple extraction via Gemma 4 classifier, model settings centralization, P12 Replicant Host Mandate documentation, CLI replicant auth, passphrase management, CLI→memory bridge, and the energy/gas/payments/API key architecture doc.

**Progress:** Documentation 100%. Implementation of the economic layer ~40%.

---

## 2. What Was Done

### Gentle Lovelace Replica (Complete)
- 11 works embedded → 1,000 passages, 4 dimension centroids + composite
- 747 passages tagged with Gemma 4 semantic triples (33,377 triples)
- `spec_replica_rewrite` tool on spec MCP server (6th tool)
- `replica_compare` with `document_type` context-sensitive weighting
- `spec_require_writing_quality` with `replica_persona` parameter
- Document-update Task 3 updated with 5th quality dimension

### Literary Replicas (Partial)
- `triple-extractor-literary.yaml` classifier config created
- 5 literary corpus configs updated (Flat budget + `triple_classifier`)
- `TripleExtraction.extra` field for schema-agnostic classifier output
- **Re-run status:** agatha-eliot ✅, hemingway ✅, woolf/jane-wilde/ulysses-s-twain ⚠️ pending

### Infrastructure
- `HkaskSettings` — centralized model defaults in `settings.json`
- `CliExperienceRecorder` — shared CLI→daemon memory bridge
- `BudgetConfig::Flat` variant for corpus.yaml format
- `DeclaredMethod::threshold` for simplified method configs
- Passphrase change + 60-day expiration in UserStore
- CLI `--replicant` + `--passphrase` required for embed-corpus

### Documentation
- P12 Replicant Host Mandate (PRINCIPLES.md + dedicated doc)
- Energy/Gas/Payments/API Key System (dedicated doc, 9 sections)
- Architecture master updated

### Verification
```
cargo check:           workspace clean ✅
cargo test services:   51 passed ✅
cargo test spec:        7 passed ✅
cargo test memory:     12 passed ✅
```

---

## 3. What Remains — 4 Gaps (1 Deferred)

All documented in `docs/architecture/energy-gas-payments-api-keys.md` §8:

| # | Gap | Priority | Crate(s) | Success Criteria |
|---|-----|----------|----------|-----------------|
| 1 | **API key issuance** (7R7 bot endpoint + KeyStore) | HIGH | `hkask-api`, `hkask-storage` | `POST /api/keys/request` returns `{key_id, key_secret, scope, allocation_rj, expires_at}` after 6-gate approval |
| 2 | **CNS `cns.api.request` spans** for key metering | HIGH | `hkask-cns` | Every API call with `Authorization: Bearer hk_...` opens a span tracking `key_id, gas_consumed, allocation_remaining, rate_limit_status` |
| 3 | **Encumbrance system** (wallet lock/release) | MEDIUM | `hkask-wallet` | `encumber(webid, key_id, amount_rj)` locks rJ; `release_encumbrance(key_id)` returns unspent rJ; `consume(key_id, gas_rj)` deducts |
| 5 | **7R7 bot key management capability** | MEDIUM | `hkask-agents` | Bot capability `key:issue`, `key:revoke`, `key:fund` registered in ACP; bot pod can execute key lifecycle operations |

**Gap 4 (on-chain settlement) deferred** — fails essentialist G1 (deletion test): system works without it for single-node deployments. Spec preserved in architecture doc for future multi-node/token economics phase.

---

## 4. Essentialist Pre-Interrogation (G1–G3)

Before implementing any gap, the agent must apply the 3-gate essentialist test. Pre-computed interrogations:

### Gap 1 — API Key Issuance

**G1 (Exist):** What behavior vanishes if we delete `POST /api/keys/request`?
- API consumers have no way to obtain scoped keys. They'd need replicant passphrases — violating P12 (API must be bot-governed). **Earns existence.**

**G2 (Surface):** Minimum public surface?
- `KeyStore` ≤5 methods: `issue_key`, `get_key`, `list_keys_for_replicant`, `revoke_key`, `fund_key`
- `POST /api/keys/request` — one route
- `POST /api/keys/{id}/fund` — one route
- `DELETE /api/keys/{id}` — one route
- **9 public items.** Justify extras or merge `fund_key` into `issue_key`.

**G3 (Contract):** Pass-through risk?
- 7R7 bot approval logic is the route handler, not a separate `KeyApprovalService` trait.
- `KeyStore` is a distinct domain with its own schema — not a `TripleStore` wrapper.

### Gap 2 — CNS API Spans

**G1 (Exist):** Without `cns.api.request` spans: no per-key metering, no rate limits, no allocation tracking, no anomaly detection. Economic model collapses. **Earns existence.**

**G2 (Surface):** 3 public items: span type, alert type, rate limit checker. Clean.

**G3 (Contract):** Span is a new domain concept — not a wrapper. Rate limit state is in-memory, not a `RateLimitStore` abstraction.

### Gap 3 — Encumbrance System

**G1 (Exist):** Without encumbrance: keys draw from system pool (violates replicant-funded model). No reserve/release. **Earns existence.**

**G2 (Surface):** 4 public items: `encumber`, `release_encumbrance`, `consume`, `Encumbrance` struct. Clean.

**G3 (Contract):** Encumbrance is a first-class wallet concept. `consume` must be atomic — no separate check+deduct pair.

### Gap 5 — 7R7 Bot Key Management

**G1 (Exist):** Without bot capability: no entity can manage keys. **Earns existence** — capability registration only, bot pod stub deferred.

**G2 (Surface):** 2 public items: 3 ACP capabilities + 1 capability set. Clean.

**G3 (Contract):** Capabilities registered in ACP registry, not hardcoded.

---

## 5. Coding Guidelines Constraints

Activate **coding-guidelines** skill before implementation. Specific constraints:

1. **Think Before Coding:** Verify crate dependencies before writing. State assumptions.
2. **Simplicity First:** Gap 1's 6-gate approval is a single function. Gap 2's rate limiter is an in-memory `HashMap`. Gap 3's encumbrance is wallet methods, not a separate service.
3. **Surgical Changes:** Touch only listed crates. Don't refactor adjacent code.
4. **Goal-Driven Execution:** Every gap has a verifiable success criterion. Loop until met.

---

## 6. Grill-Me Self-Interrogation

Before writing code for each gap, answer internally:

### Gap 1 — API Key Issuance
- "What happens if two replicants request keys simultaneously? Is `KeyStore::issue_key` atomic?"
- "Where does the key secret live after issuance? Stored hashed or plaintext?"
- "How does the 7R7 bot receive the request — API route handler or ACP message?"

### Gap 2 — CNS API Spans
- "What happens to rate limit state on process restart? Acceptable?"
- "Rate-limited vs. allocation-exhausted — separate span fields or separate span types?"
- "Where does `endpoint_weight` table live? Hardcoded or configurable?"

### Gap 3 — Encumbrance
- "What if `consume` is called after `release_encumbrance`? State machine needed?"
- "Can a replicant double-encumber the same rJ? How prevented?"
- "Encumbrance persisted to wallet DB or in-memory?"

### Gap 5 — Bot Capability
- "What prevents a non-7R7 bot from registering `key:issue`?"
- "How is the capability token bound to a specific 7R7 bot's WebID?"
- "Minimum bot pod configuration to exercise these capabilities?"

---

## 7. Order of Attack

```
Gap 3 (Encumbrance) → Gap 1 (Key Issuance) → Gap 2 (CNS Spans) → Gap 5 (Bot Capability)
```

**Rationale:** Encumbrance is the foundation — keys can't be issued without wallet lock/release. Key issuance depends on encumbrance. CNS spans depend on keys existing. Bot capability is the final integration.

---

## 8. Key Files

| File | Role |
|------|------|
| `docs/architecture/energy-gas-payments-api-keys.md` | Canonical spec for all gaps |
| `docs/architecture/P12-replicant-host-mandate.md` | API key system context |
| `crates/hkask-wallet/src/` | Encumbrance (Gap 3) |
| `crates/hkask-storage/src/` | KeyStore schema (Gap 1) |
| `crates/hkask-api/src/routes/` | Key issuance routes (Gap 1) |
| `crates/hkask-cns/src/` | API spans + rate limiter (Gap 2) |
| `crates/hkask-agents/src/` | Bot capability registration (Gap 5) |
| `crates/hkask-services/src/experience.rs` | Pattern reference for daemon bridge |

---

## 9. Verification

```bash
# After each gap:
cargo check -p <affected-crate>
cargo test -p <affected-crate>

# Full workspace after all gaps:
cargo check
cargo test --workspace
```

---

## Broader Arc

```
Documentation Sweep (done) → Gentle Lovelace (done) → Corpus Prep (done)
  → Embedding Pipeline (done) → Integration (done) → P12 + Auth (done)
  → Energy/Gas/Payments Architecture (done) → Implementation ← NOW
```

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
