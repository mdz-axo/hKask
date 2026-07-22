---
title: "Diataxis Documentation Architecture — Target Design"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# Diataxis Documentation Architecture — Target Design

**Purpose:** Define the canonical documentation structure for hKask using Diataxis as the structural skeleton. Every document is assigned to exactly one quadrant with a defined reader persona and domain ontology tier.

---

## 1. Quadrant Definitions

| Quadrant | Reader Question | Voice | Purpose |
|----------|----------------|-------|---------|
| **Tutorial** | "I'm new — show me around" | Step-by-step, first-person plural | Learning-oriented |
| **How-To** | "I know what I want — tell me how" | Direct, imperative | Task-oriented |
| **Reference** | "I need the facts" | Neutral, descriptive-only | Information-oriented |
| **Explanation** | "I want to understand why" | Discursive, contextual | Understanding-oriented |

---

## 2. Tutorial Quadrant

### Canonical Entry Point: `docs/tutorial/getting-started.md`

Single end-to-end walkthrough: new developer → working `kask` session. Reader persona: developer new to hKask, familiar with Rust and CLI. Domain tier: Core (5W1H).

**Content outline:**
1. Prerequisites (Rust toolchain, git)
2. Clone and build: `cargo build --release`
3. First invocation: `kask --version`, `kask health`
4. Onboarding: creating a user profile with `kask init`
5. Running a chat: `kask chat`
6. Invoking a skill: installing and running `caveman`
7. Reading Regulation health: `kask regulation status`
8. Next steps → How-To quadrant

### Sub-pages:
| Document | Persona | Outcome |
|----------|---------|---------|
| `docs/tutorial/skill-authoring.md` | Skill author | Build, register, invoke a custom skill |
| `docs/tutorial/mcp-bootstrap.md` | Developer | Bootstrap a new MCP server with `mcp_server!` |

---

## 3. How-To Quadrant

### Canonical Entry Point: `docs/how-to/README.md`

Index of operational procedures. 20 how-to documents:

| # | Document | Procedure | Reader |
|---|----------|-----------|--------|
| 1 | `install-and-run.md` | Compile, install binary, configure env vars | Operator |
| 2 | `configure-feature-gates.md` | Enable/disable `matrix`, `communication`, `tui`, `api`, `hedera` | Developer |
| 3 | `bootstrap-mcp-server.md` | Create MCP server with `mcp_server!` macro + `impl_tool_context!` | Developer |
| 4 | `read-regulation-alerts.md` | Interpret `reg.*` spans, variety counters, algedonic alerts | Operator |
| 5 | `run-qa-pipeline.md` | QA fuzz triage, mutation analysis, autonomous scripts | QA Engineer |
| 6 | `invoke-a-skill.md` | Install, activate, invoke a skill from CLI/API | User |
| 7 | `audit-sovereignty.md` | Inspect OCAP delegation tokens, verify consent records | Security Auditor |
| 8 | `create-agent-pod.md` | Define and deploy an agent pod | Operator |
| 9 | `configure-database-backend.md` | SQLite/SQLCipher ↔ PostgreSQL | Operator |
| 10 | `setup-matrix-transport.md` | Matrix homeserver integration for A2A | Operator |
| 11 | `deploy-k8s.md` | Kubernetes deployment with Conduit sidecar | DevOps |
| 12 | `backup-and-restore.md` | Backup SQLCipher DB, keystore, agent state | Operator |
| 13 | `use-kanban.md` | Boards, tasks, WIP limits for agent coordination | User |
| 14 | `run-kata-cycle.md` | Toyota Kata improvement cycle | Coach |
| 15 | `design-a-skill.md` | PDCA FlowDef manifest, convergence threshold, gas budget | Skill Author |
| 16 | `compose-skills.md` | Bundle multiple skills with cascade ordering | Skill Author |
| 17 | `train-lora-adapter.md` | Fine-tune LoRA adapter for userpod persona | ML Engineer |
| 18 | `configure-guard.md` | Content safety guard with classification policy | Security Engineer |
| 19 | `use-repl.md` | Interactive agent session with slash-command dispatch | User |
| 20 | `use-tui.md` | Terminal UI workspace with multi-window agent interface | User |

---

## 4. Reference Quadrant

### Canonical Entry Point: `docs/reference/README.md`

Neutral, complete, descriptive-only. No procedures, no opinions — only what IS.

### 4.1 Crate API Reference (22 documents)

`docs/reference/api/hkask-types.md` — ID types, RegulationRecord, ObservableSpan, visibility, error types
`docs/reference/api/hkask-ports.md` — Hexagonal port traits: InferencePort, ToolPort, LedgerObserver, FederationDispatch
`docs/reference/api/hkask-regulation.md` — RegulationLedger, CyberneticsLoop, GovernedTool, SetPoints, GasBudget
`docs/reference/api/hkask-mcp.md` — MCP runtime, dispatch, DaemonClient, `bootstrap_mcp_server`, `mcp_server!`
`docs/reference/api/hkask-mcp-codegraph.md` — Symbol, Edge, IndexPipeline, AssembledContext (10 MCP tools)
`docs/reference/api/hkask-pods.md` — PodManager, CuratorAgent, ConsentManager, A2ARuntime, PodDeployment
`docs/reference/api/hkask-memory.md` — EpisodicMemory, SemanticMemory, ConsolidationBridge
`docs/reference/api/hkask-inference.md` — InferenceRouter, EmbeddingRouter, FusionOrchestrator
`docs/reference/api/hkask-templates.md` — Registry, SqliteRegistry, ManifestExecutor, SkillLoader
`docs/reference/api/hkask-capability.md` — DelegationToken, CapabilityChecker, TokenRegistry
`docs/reference/api/hkask-guard.md` — ContentGuard, GuardConfig, GuardResult
`docs/reference/api/hkask-database.md` — DatabaseDriver, SqliteDriver, PostgresDriver
`docs/reference/api/hkask-storage.md` — Storage facade, HMemStore, RegulationArchive, EmbeddingStore
`docs/reference/api/hkask-cli.md` — 33 CLI subcommands, flags, env vars
`docs/reference/api/hkask-api.md` — 26 HTTP API route groups, request/response types
`docs/reference/api/hkask-keystore.md` — Keychain, AES-256-GCM, derive_key
`docs/reference/api/hkask-wallet.md` — WalletManager, ChainPort, ApiKeyIssuer, PriceFeed
`docs/reference/api/hkask-ledger.md` — Ledger, LedgerTransaction, Posting
`docs/reference/api/hkask-improv.md` — ImprovSkill, ImprovMode, ImprovCascade
`docs/reference/api/hkask-condenser.md` — CondenserEngine, CompressedOutput, health signals
`docs/reference/api/hkask-communication.md` — MatrixTransport, AgentRegistry, 7R7 listener
`docs/reference/api/hkask-federation.md` — FederationDispatch, CRDT sync
`docs/reference/api/hkask-acp.md` — HkaskAcpAgent, AcpError

### 4.2 Skill/Template/Bundle Registry

`docs/reference/skills/README.md` — Index of all 38 skills + 2 templates + 1 bundle, with FlowDef type, convergence threshold, gas budget, and canonical manifest path.

### 4.3 Regulation Span Registry

`docs/reference/regulation-spans.md` — Complete listing of all domain-specific ObservableSpan enums: AcpSpan, ClassifySpan, ContractSpan, InfraSpan, QaSpan, SeamSpan, SloSpan. Emission points, interpretation, algedonic thresholds.

### 4.4 Magna Carta Reference

`docs/reference/magna-carta.md` — P1-P4 with prohibition levels, enforcement traces (crate/module implementing each), and P4.1 pod-boundary constraint.

### 4.5 MCP Server Reference

`docs/reference/mcp-servers/README.md` — All 15 MCP servers with tool tables, capability tiers, credential requirements.

### 4.6 Generated Documentation

`docs/generated/cli-reference.md` — Auto-generated CLI reference (existing).
`docs/generated/openapi.json` — OpenAPI 3.1.0 spec (existing).

---

## 5. Explanation Quadrant

### Canonical Entry Point: `docs/explanation/README.md`

Background, context, design decisions. "This design exists because…"

### 5.1 Architecture Decisions

| Document | Topic | Domain Tier |
|----------|-------|-------------|
| `docs/explanation/hexagonal-ports.md` | Ports/adapter layout — why hexagonal, trait contracts, dependency inversion | Core |
| `docs/explanation/ocap-mcp-dispatch.md` | OCAP-governed MCP dispatch — capability membrane, PerPodToolBinding, GovernedTool | Core |
| `docs/explanation/regulation-homeostatic-loop.md` | Regulation homeostatic loop — variety engineering, algedonic alerts, set points, cybernetic feedback | Core |
| `docs/explanation/vsm-mapping.md` | Viable System Model mapping — System 1-5 mapping onto hKask subsystems | Core |
| `docs/explanation/nu-event-semantics.md` | ν-event semantics — thin domain events, observability contract, emission points | Core |
| `docs/explanation/good-regulator.md` | The Good Regulator contract — Conant-Ashby theorem applied to Regulation self-regulation | Core |
| `docs/explanation/curator-metacognition.md` | Curator metacognition — escalation, semantic indexing, curation loop | Core |
| `docs/explanation/dual-axis-ontology.md` | P5.4 dual-axis anchoring — PKO + DC/BIBO, 5W1H core, bridge crates | Core |
| `docs/explanation/energy-gas-system.md` | Energy, gas, and rJoule system — Wallet, Ledger, API key lifecycle | Core |
| `docs/explanation/database-driver-abstraction.md` | Database driver abstraction — SQLite/SQLCipher/PostgreSQL, connection pooling | Core |
| `docs/explanation/skill-pdca-model.md` | Skill PDCA model — FlowDef, convergence, gas budget, loop actions | Core |
| `docs/explanation/loom-and-thread.md` | The Loom and the Thread — Rust vs YAML/Jinja2, fixed vs mutable layers | Core |
| `docs/explanation/federation-model.md` | Federation model — CRDT sync, link lifecycle, merged registries | Core |

### 5.2 ADR Archive

Existing ADRs under `docs/architecture/ADRs/` remain in the explanation quadrant.

---

## 6. Directory Structure

```
docs/
├── tutorial/
│   ├── getting-started.md
│   ├── skill-authoring.md
│   └── mcp-bootstrap.md
├── how-to/
│   ├── README.md
│   ├── install-and-run.md
│   ├── configure-feature-gates.md
│   ├── bootstrap-mcp-server.md
│   ├── read-regulation-alerts.md
│   ├── run-qa-pipeline.md
│   ├── invoke-a-skill.md
│   ├── audit-sovereignty.md
│   ├── create-agent-pod.md
│   ├── configure-database-backend.md
│   ├── setup-matrix-transport.md
│   ├── deploy-k8s.md
│   ├── backup-and-restore.md
│   ├── use-kanban.md
│   ├── run-kata-cycle.md
│   ├── design-a-skill.md
│   ├── compose-skills.md
│   ├── train-lora-adapter.md
│   ├── configure-guard.md
│   ├── use-repl.md
│   └── use-tui.md
├── reference/
│   ├── README.md
│   ├── api/
│   │   ├── hkask-types.md
│   │   ├── hkask-ports.md
│   │   ├── hkask-regulation.md
│   │   ├── hkask-mcp.md
│   │   ├── hkask-mcp-codegraph.md
│   │   ├── hkask-pods.md
│   │   ├── hkask-memory.md
│   │   ├── hkask-inference.md
│   │   ├── hkask-templates.md
│   │   ├── hkask-capability.md
│   │   ├── hkask-guard.md
│   │   ├── hkask-database.md
│   │   ├── hkask-storage.md
│   │   ├── hkask-cli.md
│   │   ├── hkask-api.md
│   │   ├── hkask-keystore.md
│   │   ├── hkask-wallet.md
│   │   ├── hkask-ledger.md
│   │   ├── hkask-improv.md
│   │   ├── hkask-condenser.md
│   │   ├── hkask-communication.md
│   │   ├── hkask-federation.md
│   │   └── hkask-acp.md
│   ├── skills/
│   │   ├── README.md
│   │   ├── guardrails.md
│   │   ├── core-development.md
│   │   ├── reasoning.md
│   │   ├── kata.md
│   │   ├── meta.md
│   │   ├── specialized.md
│   │   └── templates.md
│   ├── regulation-spans.md
│   ├── magna-carta.md
│   └── mcp-servers/
│       └── README.md
├── explanation/
│   ├── README.md
│   ├── hexagonal-ports.md
│   ├── ocap-mcp-dispatch.md
│   ├── regulation-homeostatic-loop.md
│   ├── vsm-mapping.md
│   ├── nu-event-semantics.md
│   ├── good-regulator.md
│   ├── curator-metacognition.md
│   ├── dual-axis-ontology.md
│   ├── energy-gas-system.md
│   ├── database-driver-abstraction.md
│   ├── skill-pdca-model.md
│   ├── loom-and-thread.md
│   └── federation-model.md
├── architecture/          (existing — ADRs, principles, magna carta)
├── diagrams/              (existing — 32 Mermaid diagrams)
├── specifications/        (existing — DOCUMENTATION_STANDARDS, REQUIREMENTS, REPL, wallet, salience)
├── plans/                 (existing — deployment, TODO, WSS)
├── status/                (existing — PROJECT_STATUS, inventory reports)
├── ci/                    (existing — verify-docs.sh, check-links.sh, check-citations.sh)
├── generated/             (existing — CLI reference, OpenAPI spec)
└── README.md              (portal index)
```

---

## 7. Document Lifecycle

Every document carries:
- **`last-verified-against`** commit hash — mechanically detectable staleness
- **`last_updated`** date — human-readable drift signal
- **`status`** — Active | Draft | Archived
- **`domain`** — Core | Dual-Axis | Domain-Supplement

The CI pipeline (`docs/ci/verify-docs.sh`) checks:
- `last-verified-against` vs HEAD: >30 commits drift → CI WARNING
- Internal hyperlinks resolve → CI ERROR if broken
- Crate references match workspace members → CI ERROR if stale
- Doc examples compile (via `cargo test --doc`) → CI ERROR if fail

---

## 8. Migration from Current Structure

| Current Location | New Location | Migration |
|-----------------|-------------|-----------|
| `docs/user-guides/USERPOD-ONBOARDING-WALKTHROUGH.md` | `docs/tutorial/getting-started.md` (rewrite) | Absorb + expand |
| `docs/user-guides/*` (14 files) | `docs/how-to/*` (20 files) | Split procedural content from reference |
| `docs/architecture/*` | `docs/explanation/*` (new files) + keep architecture/ | Extract explanatory content |
| `docs/specifications/*` | Keep | Already reference-grade |
| `docs/diagrams/*` | Keep + add quadrant tags | Cross-reference to quadrant docs |
| `docs/plans/*` | Keep | Forward-looking plans are a separate category |
| `docs/status/*` | Keep + add new inventory reports | Status documents are meta-documentation |
| `docs/generated/*` | Keep | Auto-generated, rebuild on change |
| `README.md` | Keep as root portal | Update to reference new structure |
| `AGENTS.md` | Keep | Agent operating guide |

---

*Blueprint for Task 3 documentation target architecture. To be reviewed and approved before Task 4 implementation.*
