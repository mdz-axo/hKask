# Agent Operating Guide — hKask

## Project Identity

**hKask** (ℏKask - "A Minimal Viable Container for Agents") is the minimal viable unit of an agent platform.

**Name:** hKask  
**Binary:** `kask`  
**Crate prefix:** `hkask-`

---

## System Overview

| Anchor | What It Does |
|--------|-------------|
| **Agent Enablement** | Bots + Replicants run in pods with WebID, ACP |
| **Model Selection** | Switch LLM per-agent via `/model` (CLI) or `model` field (API) |
| **Essential Tools** | 14 MCP servers + Okapi for inference |
| **User Sovereignty** | OCAP, SQLCipher encryption, private/public gating |
| **CNS** | Homeostatic self-regulation: variety sensing, algedonic alerts, OCAP governance, energy budgets |
| **Composition** | Unified registry with `template_type` discriminator |

---

## Crate Map

| Crate | Purpose |
|-------|---------|
| `hkask-types` | ID types, ν-event, hLexicon |
| `hkask-storage` | SQLite + SQLCipher + sqlite-vec |
| `hkask-memory` | Semantic/episodic pipelines |
| `hkask-cns` | Cybernetic Nervous System (homeostatic self-regulation) |
| `hkask-templates` | Registry, hLexicon, cascade |
| `hkask-agents` | Pods, ACP, bot/replicant, Curation Loop, Curator Agent |
| `hkask-ensemble` | Multi-agent chat |
| `hkask-keystore` | OS keychain, AES-256-GCM, master key derivation (HKDF-SHA256) |
| `hkask-mcp` | MCP runtime, dispatch |
| `hkask-cli` | CLI commands |
| `hkask-api` | HTTP API (utoipa) |

**MCP Servers:** `inference` (Okapi LLM), `condenser`, `web`, `ocap`, `keystore`, `cns`, `git`, `registry`, `spec`, `github`, `fmp`, `telnyx`, `fal`, `rss-reader`

**External deps:** Okapi (mdz-axo/Okapi), ACP (acp-runtime), MCP (rmcp)

---

## CNS — Cybernetics

The CNS is the homeostatic self-regulation loop, combining observability + governance into a single cybernetic feedback cycle. When debugging, look for these span namespaces in traces/logs:

| Span | What It Covers |
|------|---------------|
| `cns.tool.*` | Tool governance, invocation |
| `cns.prompt.*` | Render, validate, outcome |
| `cns.agent_pod.*` | Lifecycle, delegation |
| `cns.inference` | Inference governance (GovernedTool, energy budget) |

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

# Chat with model selection
kask chat                           # Interactive (Curator, default model)
kask chat -m qwen3:8b               # Interactive with specific model
kask chat Russell -m llama3.1:70b   # Chat with Russell using 70B model
echo "hello" | kask chat -f - -m qwen3:8b  # Non-interactive with model
```

### Slash Commands (inside `kask chat`)

| Command | Purpose |
|---------|---------|
| `/model` | Show current model |
| `/model <name>` | Switch to a specific model |
| `/model <query>` | Fuzzy search models matching query |
| `/agent [NAME]` | Show or switch agent |
| `/status` | System status (CNS, agent, model, pods) |

---

## Architecture Reference

For deeper understanding of system behavior:

1. `docs/architecture/hKask-architecture-master.md` — authoritative index (v0.22.0)
2. `docs/architecture/reference/hKask-erd.md` — entity relationship diagrams
3. `docs/architecture/reference/subsystem-erds.md` — per-crate ERDs grounded in Rust source
4. `docs/architecture/interface-and-composition.md` — registry & templating design (§2-§6)

---

## Design Constraints (Read This First)

**hKask is a headless system with no visual UI.** This is a non-negotiable design principle.

### What This Means

| Constraint | What It Excludes | Why |
|------------|------------------|-----|
| **No Visual UI** | Grafana, dashboards, web frontends, GUIs | CLI/MCP/API only — visual interfaces add complexity without enabling core capabilities |
| **No Monitoring Stacks** | Prometheus, Alertmanager, external observability | CNS provides programmatic observability via spans, variety counters, algedonic alerts |
| **No Excess Complexity** | Unused traits, stubs, deprecations, feature flags | P1-P7, C1-C7 constraints enforce minimal viable complexity |

### How to Operate hKask

All interaction occurs through:
1. **CLI** — `kask <subcommand>` (terminal-based)
2. **MCP** — Machine-to-machine tool calls (14 servers)
3. **API** — HTTP API with OpenAPI docs (programmatic)

All monitoring occurs through:
1. **CNS Spans** — `cns.*` namespaces in structured logs
2. **Variety Counters** — Per-bot/capability tracking
3. **Algedonic Alerts** — Escalation when variety deficit >100

### Verification

Before adding any feature, check:
```bash
# Does this introduce visual UI or monitoring infrastructure?
if grep -r "grafana\|prometheus\|dashboard\|visual.*ui" crates/ --include="*.rs"; then
  echo "VIOLATION: Headless constraint violated"
  exit 1
fi

# Does this introduce excess complexity?
if grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/; then
  echo "VIOLATION: P6/P7 constraints violated"
  exit 1
fi
```

**If you violate these constraints, your work will be deleted.** See `docs/architecture/PRINCIPLES.md` for full rationale.

---

## Documentation

| Topic | Location |
|------|----------|
| GML (Allosteric Thinking) | `docs/gml/README.md` |
| Architecture | `docs/architecture/` |
| CI/CD | `docs/CI-CD-GUIDE.md` |
| Okapi Integration | `docs/architecture/reference/okapi-integration.md` |
| Cybernetic Unit Tests (conventions + commands) | `README.md#cybernetic-unit-tests` |

---

*ℏKask - A Minimal Viable Container for Agents — v0.22.0*
