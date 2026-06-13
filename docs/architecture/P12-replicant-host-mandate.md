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

**Current state:** `embed-corpus` and other CLI commands do not yet auto-resolve the logged-in replicant. DB and passphrase are passed manually via `--db` and `--passphrase`. This is a known gap — see Implementation Status below.

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
| CLI → auto-resolve replicant | ❌ Gap | `embed-corpus`, `compose`, `settings` pass DB manually |
| CLI → experience recording | ❌ Gap | `embed_corpus` stores triples but does not call `store_experience` |
| API → bot auth | ⚠️ Partial | Capability tokens supported; 7R7 bot integration pending |

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
