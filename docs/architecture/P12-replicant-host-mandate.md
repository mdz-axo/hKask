---
title: "Replicant Host Mandate"
audience: [architects, developers, agents]
last_updated: 2026-06-13
version: "0.27.0"
status: "Active"
domain: "Composition"
mds_categories: [domain, trust, composition]
---

# Replicant Host Mandate — P12

**Purpose:** Defines the requirement that every hKask interaction carries a replicant identity. No operation occurs unsupervised.

**Related:** [`PRINCIPLES.md`](PRINCIPLES.md) §2.4, [`AGENTS.md`](../../AGENTS.md), [`MDS.md`](MDS.md)

---

## Principle

**Every interaction with hKask carries a replicant identity.** There is no anonymous or unsupervised agency. Three interaction surfaces map to three host classes:

| Surface | Host | WebID Source | DB | Keychain |
|---------|------|-------------|-----|----------|
| **CLI / REPL** | Human replicant | `kask login <name>` → session in UserStore | `~/.config/hkask/agents/<replicant>.db` | OS keychain via `hkask-keystore` |
| **Daemon / System** | Curator replicant | `Curator` — hardcoded master system agent | `~/.config/hkask/agents/curator.db` | System keychain |
| **API** | 7R7 bots | Bot-managed capability tokens | Per-bot DB within pod | Bot-attested HKDF keys |

---

## Surface Behaviors

### CLI / REPL — Human Host

```
kask login Jacques rZuck
  → authenticates via passphrase → session stored in UserStore
  → DB resolves to ~/.config/hkask/agents/jacques-rzuck.db

kask style embed-corpus --config corpus.yaml
  → reads logged-in identity
  → after completion: records episodic memory "embedded hemingway corpus (1,827 passages)"
  → semantic triples: (corpus:hemingway, was_embedded_by, jacques-rzuck)
```

**Current state:** `embed-corpus` and other CLI commands require explicit `--replicant` and `--passphrase` parameters. The replicant identity is never inferred from the passphrase alone — explicit identification is mandatory per P12.

### Daemon / System — Curator Host

The Curator replicant is the master system agent. It hosts:

- Consolidation pipeline (episodic → semantic, `hkask consolidate`)
- CNS algedonic loop (variety monitoring, alert dispatch)
- Lifecycle transitions (Draft → Active → Deprecated → Superseded → Removed)
- Daemon socket operations (`~/.config/hkask/daemon.sock`)

`CURATOR_PERSONA` is a compile-time constant in `hkask-services/src/embed.rs`:

```rust
const CURATOR_PERSONA: &[u8] = b"Curator";
```

The Curator's WebID is constructed as `WebID::from_persona(CURATOR_PERSONA)` and used as the `owner` field on all system-generated triples.

### API — Bot Host

Programmatic interactions via HTTP API are managed by 7R7 bots:

- Each bot carries a replicant identity with WebID
- Capability tokens bound to the bot's WebID (OCAP P4)
- Bot pods provide isolation boundaries
- `HKASK_REPLICANT` env var identifies the serving replicant

**API key system (pending):** API consumers authenticate via scoped API keys rather than replicant passphrases. Each key is:

- **Issued by** a 7R7 bot with key-issuance capability
- **Scoped** to specific endpoints and actions (read specs, embed corpora, query centroids)
- **Rate-limited** per key with CNS variety monitoring
- **Rotatable** — keys expire after 90 days, renewal requires bot attestation
- **Revocable** — bots monitor usage patterns and revoke anomalous keys

Key lifecycle:

```
7R7 bot issues key → API consumer uses key → CNS monitors usage
  │                                              │
  │   anomaly detected ←─────────────────────────┘
  │       │
  │       └─→ bot revokes key, alerts Curator
  │
  └─→ 90-day expiry → bot sends renewal notice → consumer rotates
```

The 7R7 bots are the **sole key issuers** — no human, no Curator, no daemon can issue API keys. This separation ensures programmatic access is always bot-governed, never human-credentialed.

### Key Request Flow

```
Requester                    7R7 Bot                      CNS
   │                            │                           │
   │  POST /api/keys/request    │                           │
   │  {replicant, scope,        │                           │
   │   purpose, webhook}        │                           │
   │─────────────────────────→  │                           │
   │                            │  verify replicant         │
   │                            │  identity via UserStore   │
   │                            │                           │
   │                            │  check scope validity     │
   │                            │  against endpoint registry│
   │                            │                           │
   │                            │  query CNS for requester  │
   │                            │  abuse history ──────────→│
   │                            │                           │
   │                            │  ←── clean history / flags│
   │                            │                           │
   │                            │  allocate energy budget   │
   │                            │  for key scope            │
   │                            │                           │
   │                            │  generate HKDF key        │
   │                            │  store in KeyStore        │
   │                            │                           │
   │  ←── {key_id, key_secret,  │                           │
   │        scope, expires_at,  │                           │
   │        rate_limit}         │                           │
   │                            │                           │
   │  record episodic memory:   │                           │
   │  "key issued to {replicant}│                           │
   │   for {scope}"             │                           │
```

### Approval Criteria

The 7R7 bot approves a key request when ALL of the following hold:

| Criterion | Check | Failure Response |
|-----------|-------|-----------------|
| **Valid replicant** | Requester exists in UserStore with active session | 401 — "Authenticate first" |
| **Clean history** | CNS shows no abuse flags for this replicant in past 90 days | 403 — "Request denied: abuse history" |
| **Valid scope** | Requested endpoints exist in the API endpoint registry | 400 — "Unknown scope: {endpoint}" |
| **Purpose statement** | Non-empty, ≥20 characters, not a repeat of prior denied requests | 400 — "Provide a meaningful purpose" |
| **Rate limit feasible** | Requested rate limit ≤ system maximum for that scope | 400 — "Rate limit exceeds scope maximum" |
| **Funder has balance** | The funding replicant's wallet holds sufficient rJoules for the requested key allocation | 402 — "Insufficient funds: need {required} rJ, have {available} rJ" |

The bot does NOT evaluate whether the purpose is "good" — only that it is stated. Purpose evaluation is the Curator's domain via algedonic review.

### Key Funding Model

API keys are **replicant-funded**, not system-subsidized. Every key draws from a specific replicant's wallet:

```
Replicant wallet (rJoules)
  │
  ├─→ allocates N rJ to key "k1" (scope: embed-corpus)
  ├─→ allocates M rJ to key "k2" (scope: read-specs)
  │
  └─→ remaining balance stays in wallet

Key "k1" uses API → gas consumed → rJ deducted from allocation
  │
  ├─→ allocation > 0? YES → process request
  │                     NO  → 402 "Key allocation exhausted — funder must replenish"
  │
  └─→ CNS tracks: rJ_consumed, rJ_remaining, depletion_eta
```

**At issuance:** The requesting replicant specifies how many rJoules to allocate to the key. The 7R7 bot verifies the wallet holds sufficient balance, locks the allocation, and issues the key. The rJoules are not transferred — they are **encumbered** (reserved for the key's use) and deducted as the key consumes gas.

**Replenishment:** The funding replicant can top up a key's allocation at any time via `POST /api/keys/{key_id}/fund`. The bot verifies wallet balance and increases the encumbrance.

**Release:** When a key expires or is revoked, unspent rJoules are released back to the funding replicant's wallet.

**Crypto wallet integration (anticipated):** The rJoule system maps to an on-chain token via `hkask-wallet`. Replicant wallets are HD wallets derived from the replicant's WebID. Gas consumption is settled on-chain periodically (every N blocks) rather than per-request. The 7R7 bots hold the settlement authority — they aggregate key usage, produce a settlement batch, and submit it to the wallet for on-chain confirmation.

### Key Metering

Every API call authenticated by a key is metered through CNS spans:

```
API call with key → cns.api.request span opens
  │
  ├─→ key_id attached to span
  ├─→ scope matched against requested endpoint
  ├─→ rate limit checked: requests_per_minute, tokens_per_day
  ├─→ energy cost computed: gas = f(endpoint_weight, payload_size, response_tokens)
  │
  ├─→ within limits? YES → process request
  │                     NO  → 429 + cns.api.rate_limit_exceeded alert
  │
  └─→ span closes → metrics aggregated:
       • key_id → requests_today, tokens_today, gas_consumed
       • key_id → error_rate, latency_p95
       • key_id → unique_endpoints_touched (variety counter)
```

**Energy budget per key:** Each key draws from a replicant-funded rJoule allocation (not a system budget). The CNS energy budget manager tracks consumption against the allocation. When a key exhausts its allocation, requests return 402 until the funding replicant replenishes. The funding replicant receives a CNS alert when allocation drops below 20%.

**Anomaly detection:** The CNS algedonic loop monitors per-key metrics:
- Sudden spike in error rate → possible abuse → bot investigates
- Variety explosion (touching many new endpoints rapidly) → possible scan → bot rate-limits
- Consistent near-limit usage → legitimate heavy user → bot offers scope upgrade

**Key revocation triggers:**
- 3 consecutive CNS abuse alerts for the same key
- Key used from >5 distinct IPs within 1 hour
- Key used for endpoints outside its declared scope
- Manual revocation via `kask api revoke-key <key_id>` (requires Curator authority)

---

## Memory Flow

Every surface interaction produces experience records:

```
user action → store_experience(replicant, tool, input_summary, outcome)
              ↓
           daemon → dual encoding (episodic + semantic)
              ↓
           consolidation → extract semantic knowledge
              ↓
           Curator observes via algedonic loop
```

The host replicant's identity is:
- The `owner` field on every stored triple
- The `perspective` on CNS spans (`cns.tool.*`)
- The `sender` on ACP messages between bots

---

## Default Prohibition

Without an authenticated replicant:

| Surface | Behavior |
|---------|----------|
| CLI | Commands emit error requesting `kask login <name>` |
| REPL | `/repl` context shows "(not authenticated)" |
| API | Requests without capability tokens return 401 |
| Daemon | Operations default to Curator — no root, no admin, no `sudo` |

Every action has an author. Every triple has an owner. Every CNS span has a perspective.

---

## Implementation Status

| Integration | Status | Notes |
|------------|--------|-------|
| Curator persona constant | ✅ Implemented | `CURATOR_PERSONA` in `embed.rs`, `WebID::from_persona()` |
| Daemon → Curator flow | ✅ Implemented | Daemon operations use Curator WebID |
| MCP servers → replicant auth | ✅ Implemented | `HKASK_REPLICANT` env var + daemon auth query |
| CLI → explicit replicant auth | ✅ Implemented | `--replicant` + `--passphrase` required; DB resolved from UserStore |
| CLI → experience recording | ✅ Implemented | `CliExperienceRecorder` bridges CLI commands to daemon dual-encoding |
| API → bot auth | ⚠️ Planned | Scoped API keys issued by 7R7 bots; CNS-monitored; 90-day rotation. See §API — Bot Host. |

---

## Verification

```bash
# Verify CLI identity
kask login Jacques rZuck
kask settings show

# Verify Curator in triple store
# (query style:gentle-lovelace:centroid → owner must be Curator WebID)

# Verify MCP server auth
HKASK_REPLICANT=Bob kask pod mode Bob server -r replica
```

---

## References

- PRINCIPLES.md §2.4 — P12 definition and traceability
- AGENTS.md — Design constraints and crate map
- MDS.md §1 — 5-category taxonomy
- Magna Carta P1 (User Sovereignty) — every action traces to a sovereign entity
- Magna Carta P2 (Affirmative Consent) — host consent implicit in authentication
- Magna Carta P4 (OCAP) — capability tokens bound to host WebID
