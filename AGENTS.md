# Agent Operating Guide — hKask

**hKask** (ℏKask) — A Minimal Viable Container for Agents | `kask` binary | `hkask-` crate prefix

---

## System Overview

| Anchor | What It Does |
|--------|-------------|
| **Agent Enablement** | Bots + Replicants run in pods with WebID, ACP |
| **Model Selection** | Switch LLM per-agent via `/model` (CLI) or `model` field (API) |
| **Essential Tools** | 21 MCP servers + Okapi for inference |
| **User Sovereignty** | OCAP, SQLCipher encryption, private/public gating |
| **CNS** | Homeostatic self-regulation: variety sensing, algedonic alerts, OCAP governance, gas budgets |
| **Composition** | Unified registry with `template_type` discriminator |

## Crate Map

| Crate | Purpose |
|-------|---------|
| `hkask-types` | ID types, ν-event, hLexicon |
| `hkask-storage` | SQLite + SQLCipher + sqlite-vec |
| `hkask-memory` | Semantic/episodic pipelines (memory consolidation: episodic → semantic) |
| `hkask-cns` | Cybernetic Nervous System (homeostatic self-regulation) |
| `hkask-templates` | Registry, hLexicon, cascade |
| `hkask-agents` | Pods, ACP, bot/replicant, Curation Loop, Curator Agent |
| `hkask-ensemble` | Multi-agent chat |
| `hkask-keystore` | OS keychain, AES-256-GCM, master key derivation (HKDF-SHA256) |
| `hkask-mcp` | MCP runtime, dispatch, dynamic tool discovery |
| `hkask-cli` | CLI commands |
| `hkask-mcp-doc-knowledge` | Document parsing and chunking MCP server (HTML/text extraction, multi-tier chunking) |
| `hkask-mcp-markitdown` | Document format conversion and OCR MCP server (PDF/MD/HTML/TXT + vision OCR fallback) |
| `hkask-api` | HTTP API (utoipa) |

**MCP Servers:** `inference` (Okapi LLM), `condenser`, `web`, `ocap`, `keystore`, `cns`, `git`, `registry`, `spec`, `goal`, `github`, `fmp`, `telnyx`, `fal`, `rss-reader`, `ensemble`, `episodic`, `semantic`, `replicant`, `doc-knowledge`, `markitdown` (OCR/vision fallback)
**External deps:** Okapi (mdz-axo/Okapi), ACP (acp-runtime), MCP (rmcp)

---

## CNS — Cybernetics & Loop Wiring

The CNS is the homeostatic self-regulation loop combining observability + governance into a single cybernetic feedback cycle.

**CNS spans** (look for these in traces/logs):

> **Canonical source:** [`docs/architecture/PRINCIPLES.md`](docs/architecture/PRINCIPLES.md) §1.4 — that document lists all 21 CNS span namespaces. The table below shows only the spans most relevant to CNS loop wiring.

| Span | What It Covers |
|------|---------------|
| `cns.tool.*` | Tool governance, invocation |
| `cns.prompt.*` | Render, validate, outcome |
| `cns.agent_pod.*` | Lifecycle, delegation |
| `cns.inference.*` | Inference governance (GovernedTool, gas budget) |
| `cns.spec` | Spec drift detection, SpecDriftAlert emission |
| `cns.hhh.persona` | Persona constraint violations stripped from output |
| `cns.cybernetics.backpressure` | Communication queue depth backpressure regulation |

**Algedonic Alert:** Variety deficit > threshold/2 (50) → Warning to Curator; deficit > threshold (100) → Critical to human

**Loop wiring:**

| Wiring | Description |
|--------|-------------|
| **Communication↔Cybernetics queue depth** | `Arc<AtomicU64>` counter shared between CommunicationLoop (writer) and CyberneticsLoop (reader). Lock-free, Relaxed ordering. Backpressure when depth exceeds `communication_backpressure_threshold` (default: 100). |
| **SpecDriftAlert** | `LoopPayload::SpecDriftAlert` from `DefaultSpecCurator` when spec drift exceeds threshold. Flows through Communication Loop to Curation's inbox (alongside NuEvent persistence). |
| **Override cooldown** | `Dampener::override_cooldown` (120s) prevents metacognitive override oscillation. After any override passes dedup, ALL subsequent overrides within cooldown are suppressed. |
| **Persona filter** | Stage 4 of HHH pipeline. Strips persona-constraint-violating content. Loaded from `bot_status()` at init and `/agent` switch. |

---

## Agent Types

| Type | Purpose | Interaction | Visibility |
|------|---------|-------------|------------|
| **Bot** | Process execution | A2A | Public/Shared |
| **Replicant** | Human assistance | H2A | Episodic=Private, Semantic=Public |

**Curator:** Single replicant, system persona, user's counterpart in `kask chat`.

---

## Commands

```bash
cargo check -p <crate>              # Build & check
cargo test -p <crate>                # Test (cargo test for full workspace)
cargo clippy -p <crate> -- -D warnings  # Lint
cargo run --bin kask -- <subcommand>  # Run
kask chat                            # Interactive (Curator, default model)
kask chat -m qwen3:8b                 # Specific model
kask chat Russell -m llama3.1:70b     # Named agent + model
echo "hello" | kask chat -f - -m qwen3:8b  # Non-interactive
```

**Slash commands** (`kask chat`): `/model` show/switch, `/model <query>` fuzzy search, `/agent [NAME]` show/switch, `/status` CNS+agent+model+pods

---

## Design Constraints (Read This First)

**hKask is headless — no visual UI.** Non-negotiable.

| Constraint | Excludes | Why |
|------------|----------|----|
| **No Visual UI** | Grafana, dashboards, web frontends, GUIs | CLI/MCP/API only |
| **No Monitoring Stacks** | Prometheus, Alertmanager, external observability | CNS provides programmatic observability |
| **No Excess Complexity** | Unused traits, stubs, deprecations, feature flags | P1-P8, C1-C8 constraints |

**Interaction:** CLI (`kask <subcommand>`) · MCP (21 servers) · API (HTTP/OpenAPI)
**Monitoring:** CNS spans (`cns.*`) · Variety counters · Algedonic alerts (>100 deficit)

```bash
# Constraint verification
if grep -r "grafana\|prometheus\|dashboard\|visual.*ui" crates/ --include="*.rs"; then echo "VIOLATION: Headless"; exit 1; fi
if grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/; then echo "VIOLATION: P6/P7"; exit 1; fi
```

**Violations get deleted.** See `docs/architecture/PRINCIPLES.md`.

---

## Test Program

**P8:** Every `#[test]` verifies a stated behavioral property of a public seam. Tests without invariants are structural and must be rewritten or removed.

**C8:** Test depth matches module depth. Shallow modules get shallow tests; deep modules get deep tests. If a module is hard to test, deepen the module first.

**TDD Practice:** Vertical tracer-bullet discipline (RED→GREEN per behavior, never horizontal slices). Governed by DDMVSS Curation — every test invariant is evaluated via Merge/Revise/Defer/Discard.

| Topic | Location |
|-------|----------|
| Test Program Specification | `docs/specifications/test-program.md` |
| Test Inventory & Seam Analysis | `docs/status/test-inventory.md` |
| Testing Standards (DDMVSS §12) | `docs/specifications/TESTING_STANDARDS.md` |
| Architecture Principles (P8, C8) | `docs/architecture/PRINCIPLES.md` |
| DDMVSS Testing Protocol | `docs/architecture/DDMVSS.md` §12 |
| Skill-to-DDMVSS Mapping | `docs/specifications/test-program.md` §11 |

---

## Architecture & Docs

| Topic | Location |
|------|----------|
| Architecture master | `docs/architecture/hKask-architecture-master.md` (v0.23.0) |
| ERDs | `docs/architecture/reference/hKask-erd.md`, `subsystem-erds.md` |
| Registry & templating | `docs/architecture/interface-and-composition.md` (§2-§6) |
| DDMVSS Specification | `docs/architecture/DDMVSS.md` (v0.2.2) |
| Testing Protocol (DDMVSS §12) | `docs/architecture/DDMVSS.md` §12 |
| Test Program (DDMVSS self-applying) | `docs/specifications/test-program.md` |
| Test Inventory & Seam Analysis | `docs/status/test-inventory.md` |
| Testing Standards | `docs/specifications/TESTING_STANDARDS.md` |
| GML (Allosteric Thinking) | `docs/gml/README.md` |
| CI/CD | `docs/CI-CD-GUIDE.md` |
| Okapi Integration | `docs/architecture/reference/okapi-integration.md` |
| Cybernetic Unit Tests | `README.md#cybernetic-unit-tests` |

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*
