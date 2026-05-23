# Agent Operating Guide — hKask

## Project Identity

**hKask** (ℏKask — "Planck's Constant of Agent Systems") is the minimal viable unit of an agent platform.

**Name:** hKask (pronounced *h-bar-kask*)  
**Binary:** `kask`  
**Crate prefix:** `hkask-`

---

## System Overview

| Anchor | What It Does |
|--------|-------------|
| **Agent Enablement** | Bots + Replicants run in pods with WebID, ACP |
| **Essential Tools** | 10 MCP servers + Okapi for inference |
| **User Sovereignty** | OCAP, SQLCipher encryption, private/public gating |
| **CNS** | Observability via `cns.*` spans, variety counters, algedonic alerts |
| **Composition** | Unified registry with `template_type` discriminator |

---

## Crate Map

| Crate | Purpose |
|-------|---------|
| `hkask-types` | ID types, ν-event, hLexicon |
| `hkask-storage` | SQLite + SQLCipher + sqlite-vec |
| `hkask-memory` | Semantic/episodic pipelines |
| `hkask-cns` | Cybernetic Nervous System (observability) |
| `hkask-templates` | Registry, hLexicon, cascade |
| `hkask-agents` | Pods, ACP, bot/replicant |
| `hkask-ensemble` | Multi-agent chat |
| `hkask-keystore` | OS keychain, AES-256-GCM |
| `hkask-mcp` | MCP runtime, dispatch |
| `hkask-cli` | CLI commands |
| `hkask-api` | HTTP API (utoipa) |

**MCP Servers:** `inference` (Okapi LLM), `storage`, `memory`, `embedding`, `condenser`, `ensemble`, `web`, `scholar`, `spandrel` (graph), `doc-knowledge`

**External deps:** Okapi (mdz-axo/Okapi), ACP (acp-runtime), MCP (rmcp)

---

## CNS — Observability

When debugging, look for these span namespaces in traces/logs:

| Span | What It Covers |
|------|---------------|
| `cns.tool.*` | Tool governance, invocation |
| `cns.prompt.*` | Render, validate, outcome |
| `cns.agent_pod.*` | Lifecycle, delegation |
| `cns.connector.*` | External I/O (LLM, embeddings) |

**Algedonic Alert:** Variety deficit >100 → escalation to Curator/human

---

## Agent Types

| Type | Purpose | Interaction | Visibility |
|------|---------|-------------|------------|
| **Bot** | Process execution | Machine-to-machine (A2A) | Public/Shared |
| **Replicant** | Human assistance | Human-to-agent (H2A) | Episodic=Private, Semantic=Public |

**Curator:** Single replicant, system persona, user's counterpart in `kask chat`.

---

## Commands

```bash
# Build & check
cargo check -p <crate>
cargo build --release

# Test
cargo test -p <crate>
cargo test                          # full workspace

# Lint
cargo clippy -p <crate> -- -D warnings
cargo fmt --check

# Run
cargo run --bin kask -- <subcommand>
```

---

## Architecture Reference

For deeper understanding of system behavior:

1. `docs/architecture/hKask-architecture-master.md` — authoritative spec (v0.21.0)
2. `docs/architecture/hKask-erd.md` — entity relationship diagrams
3. `docs/architecture/registry-templating-prompt-v2.md` — registry & templating design

---

## Documentation

| Topic | Location |
|-------|----------|
| GML (Allosteric Thinking) | `docs/gml/README.md` |
| Architecture | `docs/architecture/` |
| CI/CD | `docs/CI-CD-GUIDE.md` |
| Okapi Integration | `docs/P0_OKAPI_INTEGRATION_PLAN.md` |

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
