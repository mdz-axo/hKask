---
title: "P12 — Authenticated Host Mandate"
audience: [architects, developers, agents]
last_updated: 2026-07-20
version: "0.31.0"
status: "Active"
domain: "Trust"
mds_categories: [trust, lifecycle]
---

# P12 — Authenticated Host Mandate

**Every action has an accountable host identity. No anonymous agency.**

hKask has three actor classes, each with an accountable host identity (WebID):

- **Human users** — each acts through their own **userpod** (1:1; Solid-Pod-modeled,
  persistent for the life of the account). The userpod is the unit of user agency.
- **The curator** — the system daemon (runs under systemd; the one surviving
  "userpod" by name). It coordinates userpods and owns the `SemanticIndex`.
- **AI tools** — skills and MCP servers, invoked by a userpod or the curator. They
  never act under their own identity — every tool action is attributed to the hosting
  userpod or curator.

## Surface-Host Mapping (P12.1)

| Surface | Host | WebID Source | Storage | Keychain |
|---------|------|-------------|---------|----------|
| **CLI / REPL** | Human user (via userpod) + Curator daemon | `kask login <name>` → UserStore session | `~/.config/hkask/agents/<userpod>.db` | OS keychain via `hkask-keystore` |
| **Daemon / System** | Curator daemon | `Curator` — system daemon | `~/.config/hkask/agents/curator.db` | System keychain |
| **API** | Userpods | Userpod-managed capability tokens | Per-userpod DB | Userpod-attested HKDF keys |

## Dual-presence pattern

The CLI/REPL surface hosts both the user's userpod AND the Curator daemon in a single
loop. The user speaks; the Curator observes, surfaces Regulation alerts, provides memory
summaries, and can be addressed directly via `kask curator chat`. This is not two
separate sessions — it is one conversation with two participants. The user's userpod is
the sovereign host; the Curator daemon is the system's presence.

## Regulation span authority

Every `reg.*` span carries a `userpod` (or `owner`) WebID as its authority field. No
span is emitted without an accountable host. This is the observability expression of
P12: every regulated variable is attributed.

## What changed in the v0.31.0 consolidation

The prior "Authenticated Host Mandate" framed a bot/userpod taxonomy (P10). That taxonomy
is retired; P10 is refocused to **user agency**. The mandate is renamed to "Authenticated
Host Mandate" to reflect that the accountable host is the userpod (or curator), not a
"userpod" or "bot" role. The generic "agent" concept is preserved for A2A interop —
userpods present as agents in A2A — but the hKask-specific bot/userpod distinction is
gone. See `PRINCIPLES.md` §1.4 (P10 — User Agency, P12 — Authenticated Host Mandate).