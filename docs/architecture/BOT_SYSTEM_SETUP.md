# hKask Bot System — Initial Setup

**Version:** v0.21.0  
**Date:** 2026-05-20  
**Status:** 7 core bots defined with manifests and selector templates

---

## Bot Inventory

| # | Bot | Manifest | Selector | Purpose |
|---|-----|----------|----------|---------|
| 1 | `registry-dispatch-bot` | `dispatch.yaml` | `selector.j2` | Template dispatch orchestration |
| 2 | `cns-curator-bot` | `cns-monitoring.yaml` | `alert-selector.j2` | CNS monitoring & algedonic alerts |
| 3 | `memory-curator-bot` | `memory-ops.yaml` | `operation-selector.j2` | Semantic/episodic memory ops |
| 4 | `inference-curator-bot` | `inference-dispatch.yaml` | `model-selector.j2` | LLM model tier selection |
| 5 | `mcp-dispatch-bot` | `mcp-dispatch.yaml` | `tool-selector.j2` | MCP tool routing & OCAP |
| 6 | `ensemble-curator-bot` | `ensemble-orchestration.yaml` | `participant-selector.j2` | Multi-agent coordination |
| 7 | `git-curator-bot` | `git-ops.yaml` | `operation-selector.j2` | Git CAS versioning |

---

## File Structure

```
registry/
├── bots/                          # Bot manifests (7 files)
│   ├── registry-dispatch-bot.yaml
│   ├── cns-curator-bot.yaml
│   ├── memory-curator-bot.yaml
│   ├── inference-curator-bot.yaml
│   ├── mcp-dispatch-bot.yaml
│   ├── ensemble-curator-bot.yaml
│   └── git-curator-bot.yaml
│
├── manifests/                     # Dispatch manifests (7 files)
│   ├── dispatch.yaml
│   ├── cns-monitoring.yaml
│   ├── memory-ops.yaml
│   ├── inference-dispatch.yaml
│   ├── mcp-dispatch.yaml
│   ├── ensemble-orchestration.yaml
│   └── git-ops.yaml
│
└── templates/                     # Selector templates (7 files)
    ├── registry/selectors/selector.j2
    ├── cns/selectors/alert-selector.j2
    ├── memory/selectors/operation-selector.j2
    ├── inference/selectors/model-selector.j2
    ├── mcp/selectors/tool-selector.j2
    ├── ensemble/selectors/participant-selector.j2
    └── git/selectors/operation-selector.j2
```

---

## Bot Manifest Structure

Each bot manifest follows the canonical schema:

```yaml
bot:
  name: <bot-name>
  type: Bot
  binding_contract: true
  editor: curator-or-human-admin

capabilities:
  - tool:<server>:<operation>

rights:
  - read: <resource>
  - write: <resource>
  - execute: <operation>

responsibilities:
  - respond_to: <request_type>
  - emit: <cns_span>
  - enforce: <constraint>

process_manifest: registry/manifests/<manifest>.yaml
```

---

## Dispatch Pattern

All bots use the same 3-step dispatch pattern:

1. **Select** — Render selector template → fast local model → choose operation/template
2. **Populate** — Bind input into selected template's Jinja2 fields
3. **Execute** — Submit rendered document to MCP tool or LLM

**Matroshka Limits:**
- Default max depth: 7
- Enforced by Rust executor
- Configurable per manifest

**CNS Integration:**
- All bots emit `cns.tool.invocation` and `cns.tool.result` spans
- CNS curator monitors variety counters
- Algedonic alert threshold: variety deficit >100

---

## Security Model

| Bot | Security Constraints |
|-----|---------------------|
| `mcp-dispatch-bot` | OCAP verification, rate limiting (100/min), path traversal blocking |
| `memory-curator-bot` | Visibility gating (OCAP-enforced), Bayesian confidence |
| `git-curator-bot` | SHA-only versioning, no SemVer |
| `ensemble-curator-bot` | No swarm consensus, max 7 participants |
| `inference-curator-bot` | Model tier balancing (speed/quality/cost) |
| `cns-curator-bot` | Alert threshold enforcement, calibration |
| `registry-dispatch-bot` | Matroshka depth limit, template validation |

---

## Next Steps

### Immediate (v1.1)
1. **Implement template executor** — Rust loop in `hkask-templates` crate
2. **Create remaining templates** — Populate `templates/<bot>/templates/` directories
3. **Wire MCP adapters** — Connect bot manifests to MCP server tools
4. **Test dispatch flow** — End-to-end bot-mediated operations

### Medium-Term
5. **Add more bots** — `web-curator-bot`, `scholar-bot`, `spandrel-bot` as needed
6. **Refine manifests** — Adjust based on operational data from CNS
7. **Bot discovery** — Registry lookup for A2A communication

---

## Design Principles

| Principle | Implementation |
|-----------|----------------|
| **Bot-mediated subsystems** | Each domain has expert curator bot |
| **Template-mediated coordination** | Bots communicate via self-describing templates |
| **No manual wiring** | Dispatch pattern replaces hard-coded paths |
| **Rust = loom, YAML/Jinja2 = thread** | Fixed executor, mutable content |
| **OCAP enforcement** | Capability tokens gate all operations |
| **CNS observability** | All bot ops emit spans |

---

*ℏKask — Planck's Constant of Agent Systems — v0.21.0*
*7 bots ready for MVP. Learn what works from system use.*
