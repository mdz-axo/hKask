<p align="center">
  <img src="assets/kask-logo.svg" alt="Kask Logo" width="120"/>
</p>
# ℏKask - A Minimal Viable Container for Replicants

**Version:** v0.31.0 |

---

## Logo & Brand

The Kask logo synthesizes four elements into a single mark:

| Element | Represents | Visual Form |
|---------|-----------|-------------|
| **The Kask (Container)** | Typed container, governed surface | Rectangular amphora with handles |
| **Calligraphy (Art)** | Human craft, temporal mark | Varied stroke width, pressure-sensitive |
| **Curator's Eye (Vision)** | Observation, governance, loyalty | Almond eye with iris, pupil, reflection |
| **Bitemporal Shadow (Perspective)** | Valid-time + transaction-time | Offset shadow, reduced opacity |

> *A simple container, drawn by hand, watching from within, remembering in two times.*

**Core principles:** Recognition <400ms | Scalable 16px–16ft | Monochrome-first | No gradients/effects

Full design principles → [`assets/LOGO-DESIGN-PRINCIPLES.md`](assets/LOGO-DESIGN-PRINCIPLES.md)

---

## Vision

hKask is the minimal viable unit of an agent platform from which a full agent ecosystem can be reconstructed.

**Design Philosophy:** Austere and efficient recombinatorial system. Rust is the loom (fixed logic). YAML/Jinja2 is the thread (mutable content).

---

## Five Anchors

| # | Anchor | Implementation |
|---|--------|----------------|
| 1 | **Agent Enablement** | Bots + Replicants in pods with WebID, A2A, Episodic and Semantic Memory and kask services |
| 2 | **Essential Tools** | 15 MCP servers + Inference Router (DeepInfra, Together AI, fal.ai, OpenRouter, KiloCode) |
| 3 | **User Sovereignty** | OCAP, SQLCipher, keystore, private/public gating |
| 4 | **CNS** | `cns.*` spans, variety counters, algedonic alerts |
| 5 | **Composition** | Templates + manifests compose into iterative PDCA Skill loops (46 skills, 2 templates, 1 bundle) |

---

## Skills & Composition

hKask distinguishes two layers that other systems conflate:

- **Templates** (`.j2` Jinja2 files, 367 total) — One-shot prompt executions. These are what Claude, ChatGPT, and most agent platforms call "skills." In hKask, they are raw material: a template runs once, returns output, and exits.
- **Skills** (46 total) — Iterative PDCA (Plan-Do-Check-Act) loops that compose multiple templates into autonomous search, learning, and implementation cycles. A Skill has a FlowDef manifest with `convergence.threshold > 0`, a `gas.cap`, and a `loop` action. It runs until it converges on a quality threshold, exhausts its energy budget, or escalates to the Curator.

Where other systems give you a prompt, hKask gives you a process.

| Layer | Format | Count | Behavior |
|-------|--------|-------|----------|
| **Templates** | `*.j2` (Jinja2) | 367 | One-shot: execute → return output |
| **Skill manifests** | `manifest.yaml` | 83 | FlowDef: contracts, convergence criteria, gas budget |
| **Skills** | `.agents/skills/` | 46 | PDCA loops: compose templates → iterate → converge \| max_out \| escalate |

Skills execute through the `kask chat` runtime or via the QA pipeline (`kask qa run --script`, planned). The skill system includes discovery, bundling, translation, lifecycle management, and adversarial logic auditing. A Bundle composes multiple Skills but is not itself a PDCA loop.

---

## Crate Structure

### Foundation (14 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-types` | ID types, nu-event, vocabulary, visibility, CNS spans |
| `hkask-storage` | SQLite + SQLCipher, triples, embeddings, blobs, Git CAS |
| `hkask-storage-core` | Storage foundation — Database, Store trait, lock helpers, path sanitization |
| `hkask-database` | Provider-agnostic database driver abstraction (SQLite, PostgreSQL) |
| `hkask-memory` | Semantic/episodic pipelines (consolidation: episodic → semantic) |
| `hkask-cns` | Cybernetic Nervous System |
| `hkask-templates` | Registry, vocabulary, cascade, resolver |
| `hkask-agents` | Pods, ACP, bot/replicant, Curator |
| `hkask-keystore` | OS keychain, AES-256-GCM |
| `hkask-mcp` | MCP runtime, dispatch, security |
| `hkask-cli` | CLI (37 subcommands + REPL) |
| `hkask-api` | HTTP API, utoipa OpenAPI (21 route groups) |
| `hkask-capability` | OCAP delegation tokens |
| `hkask-ports` | Hexagonal port traits |

### Infrastructure (16 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-inference` | Inference router (provider dispatch, model selection, Fal.ai workflow DAG execution) |
| `hkask-communication` | Matrix transport, agent registry, 7R7 listener |
| `hkask-improv` | Constructive interaction protocol (plussing, yes-and, yes-but, freestyling, riffing) |
| `hkask-condenser` | Context condensation engine (7 tools, 90 tests) |
| `hkask-codegraph` | Native code understanding engine (tree-sitter, FTS5, recursive CTE traversal, context assembly) |
| `hkask-acp` | Agent Client Protocol — IDE integration for coding agents |
| `hkask-adapter` | Trained adapter lifecycle — store, expertise, endpoint lifecycle, provider cost model |
| `hkask-test-harness` | Test infrastructure (TestDb, TestWebId, mocks, strategies) |
| `hkask-mcp-cloud-gateway` | Cloud MCP gateway for remote tool dispatch |
| `hkask-guard` | Content safety guard — mandatory LLM boundary scanning, OWASP LLM Top 10 aligned |
| `hkask-repl` | Interactive REPL — slash-command dispatch, tab completion, fuzzy matching |
| `hkask-forecast` | Superforecasting computation engine (Fermi decomposition, Bayesian updating, Brier scoring) |
| `hkask-storage-guard` | Autonomous disk space management loop — monitors /data volume, prunes old exports |
| `hkask-git-cas` | Git content-addressable storage (BLAKE3-hashed object store) |
| `hkask-goal` | Goal specification and completion verification |
| `hkask-identity` | Human identity & access-control user records (HumanUser, OAuth providers, roles) — Loop 6 Access Guard |

### Services (14 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-services-core` | Service-layer foundation — ServiceError, ServiceConfig, HkaskSettings |
| `hkask-services-context` | AgentService context, CNS runtime, cybernetic loops |
| `hkask-services-runtime` | Runtime services — text classification, provider intelligence, daemon handler |
| `hkask-services-chat` | Chat session management and history |
| `hkask-services-compose` | Style composition — exemplar retrieval, prose generation, centroid-distance validation against an author's voice |
| `hkask-services-corpus` | Document corpus management and indexing |
| `hkask-services-inference` | Inference provider intelligence and dispatch services |
| `hkask-services-kata-kanban` | Toyota Kata coaching/improvement + Kanban board coordination |
| `hkask-services-onboarding` | First-run and user onboarding |
| `hkask-services-research` | Research pipeline services (web search, extraction, feed management) |
| `hkask-services-self-heal` | Autonomous self-healing loop services |
| `hkask-services-skill` | Skill discovery, publishing, hashing, auditing, and bundle composition |
| `hkask-services-verification` | Magna Carta verification — manifest-driven structural audits of codebase sovereignty/consent provisions |
| `hkask-services-wallet` | Gas budgeting, price feeds, CNS integration |

### Wallet, Identity & Ledger (3 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-wallet` | rJoule wallet — self-custody multi-chain deposits, API key issuance, Hinkal privacy |
| `hkask-wallet-types` | Wallet value types and data structures |
| `hkask-ledger` | Double-entry accounting ledger (cost, crypto, securities) |

### Ontology & Interface (2 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-federation` | Cross-instance agent federation protocol |
| `hkask-tui` | Terminal UI (ratatui-based interactive console) |

### Ontology Bridges (5 crates)
| Crate | Purpose |
|-------|--------|
| `hkask-bridge-dublincore` | Dublin Core + BIBO + CiTO vocabulary bridge (bibliographic metadata, resource typing, citation relationships) |
| `hkask-bridge-eso` | Epistemic Science Ontology bridge (hypotheses, evidence, theories, models, falsification, uncertainty) |
| `hkask-bridge-fibo` | Financial Industry Business Ontology bridge (competitive advantage, valuation, capital allocation, risk, economic profit) |
| `hkask-bridge-golem` | GOLEM narrative/literary ontology bridge (characters, events, themes, literary devices, interpretive relationships) |
| `hkask-bridge-pko` | Procedural Knowledge Ontology bridge (procedures, steps, actions, executions, issues, feedback) |

### MCP Servers (15 crates)
- `hkask-mcp-condenser` — Context condensation (thin wrapper around hkask-condenser)
- `hkask-mcp-research` — Web search, extraction, and feed-based research
- `hkask-mcp-skill` — Skill invocation (exposes registered skills as callable MCP tools)
- `hkask-mcp-curator` — Curator daemon tools (algedonic log, escalations, memory recall, semantic search)
- `hkask-mcp-companies` — Company financial data (FMP + EODHD dual-provider)
- `hkask-mcp-communication` — Thin MCP wrapper over core communication crate
- `hkask-mcp-media` — Media generation (image, video, audio, 3D, workflows via fal.ai)
- `hkask-mcp-replica` — Authorial style embedding and prose composition
- `hkask-mcp-docproc` — Unified document processing (format conversion, OCR, chunking, QA generation)
- `hkask-mcp-memory` — Unified episodic + semantic memory with cloud backup
- `hkask-mcp-training` — Model training (QA pairs and training data for fine-tuning pipelines)
- `hkask-mcp-kata-kanban` — Kata-Kanban workflow coordination
- `hkask-mcp-filesystem` — OCAP-governed filesystem + shell access (7 tools: fs.read/write/…, shell.exec)
- `hkask-mcp-codegraph` — Code understanding tools (query, traverse, impact, analysis, context, structure, stats, reindex, feedback, embed, dead_code)
- `hkask-mcp-scenarios` — Scenario planning (MAIA event-tree forecasting, Fermi decomposition, Bayesian calibration)

---

## Current Metrics

| Metric | Value |
|--------|-------|
| **Core LOC (crates/src/)** | ~182,500 |
| **MCP Server LOC (src/)** | ~49,600 |
| **Total LOC** | ~232,100 |
| **Core Crates** | 54 (14 foundation + 16 infra + 14 services + 3 wallet/identity/ledger + 2 ontology/interface + 5 bridges) |
| **MCP Servers** | 15 |
| **Workspace Members** | 69 (54 crates + 15 MCP servers, excluding fuzz targets) |
| **Tests** | ~2,166 (`#[test]` + `#[tokio::test]` annotations across workspace) |
| **CLI Subcommands** | 37 |
| **API Route Groups** | 23 |
| **Build/Clippy/Fmt/Test/UnusedDeps** | All passing |
| **Skills** | 46 (83 registry manifests, 367 Jinja2 templates) |
| **Codegraph** | 11 MCP tools (query, traverse, impact, analysis, context, structure, stats, reindex, feedback, embed, dead_code) |
| **QA Pipeline** | Fuzz triage, mutation analysis, autonomous script runner |

---

## Commands

```bash
# Verification
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check

# Dependency hygiene (nightly)
RUSTFLAGS="-D unused_crate_dependencies" cargo +nightly check --workspace

# Documentation health
bash docs/ci/verify-docs.sh
bash docs/ci/check-links.sh
```

---

## Documentation

hKask follows the [Diataxis](https://diataxis.fr/) documentation methodology. The documentation portal at [`docs/README.md`](docs/README.md) is the canonical entry point.

- [`AGENTS.md`](AGENTS.md) — Agent operating guide (capability catalog, tooling policy, prohibitions)
- [`docs/README.md`](docs/README.md) — Documentation portal (Diataxis index of all active docs)
- [`docs/how-to/getting-started.md`](docs/how-to/getting-started.md) — End-to-end walkthrough: clone → build → chat → skill invoke
- [`docs/how-to/`](docs/how-to/) — Task-oriented guides: install, configure, bootstrap MCP, invoke skills, audit sovereignty
- [`docs/reference/`](docs/reference/) — API reference, skill registry, CNS span registry, Magna Carta
- [`docs/explanation/`](docs/explanation/) — Architecture decisions: hexagonal ports, CNS loop, OCAP dispatch, ν-events
- [`docs/architecture/`](docs/architecture/) — ADRs, master architecture, provider/federation/database architecture
- [`docs/specifications/DOCUMENTATION_STANDARDS.md`](docs/specifications/DOCUMENTATION_STANDARDS.md) — Documentation standards (metadata, citations, diagrams, lifecycle)
- [`docs/OPEN_QUESTIONS.md`](docs/OPEN_QUESTIONS.md) — Underspecified aspects and open design decisions
- [`.github/workflows/ci.yml`](.github/workflows/ci.yml) — CI pipeline (fmt → clippy + unused-deps + build → test + doc → invariants)
- [`.github/workflows/audit.yml`](.github/workflows/audit.yml) — Weekly dependency audit (cargo-deny + cargo-audit)
- [`docs/ci/verify-docs.sh`](docs/ci/verify-docs.sh) — Documentation health check (10-step verification, runs in CI)

---

## Design Philosophy

**As simple as possible, but no simpler.**

- **No silent draws on reserve** — Every change cited
- **No hallucinations** — All features traceable to spec
- **No speculation** — Code not needed today is debt
- **No ceremony** — Direct, technical, concise

**The Loom and the Thread:**

| Layer | Technology | Mutability |
|-------|------------|------------|
| **Hard (Kernel)** | Rust | Fixed, stable |
| **Soft (Material)** | YAML, Jinja2, MD | Mutable, evolving |

---

*ℏKask - A Minimal Viable Container for Replicants — v0.31.0*
*Rust is the loom. YAML/Jinja2 is the thread.*
*CI green. 54 crates. 15 MCP servers. 46 PDCA skill loops. 367 templates.*
