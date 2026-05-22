# Agent Operating Guide — hKask

## Project Identity

**hKask** (ℏKask — "Planck's Constant of Agent Systems") is the minimal viable unit of an agent platform.

**Name:** hKask (pronounced *h-bar-kask*)  
**Binary:** `kask`  
**Crate prefix:** `hkask-`  
**Line Budget:** ≤30,000 lines Rust (excluding protocols)

---

## Five Anchors

| # | Anchor | Implementation |
|---|--------|----------------|
| 1 | **Agent Enablement** | Bots + Replicants in pods with WebID, ACP |
| 2 | **Essential Tools** | 10 MCP servers + Okapi |
| 3 | **User Sovereignty** | OCAP, SQLCipher, private/public gating |
| 4 | **CNS** | `cns.*` spans, variety counters, algedonic alerts |
| 5 | **Composition** | **Unified registry** with template_type discriminator |

---

## Repository Shape

```
hkask-workspace/
├── hkask-types         # ID types, ν-event, hLexicon
├── hkask-storage       # SQLite + SQLCipher + sqlite-vec
├── hkask-memory        # Semantic/episodic pipelines
├── hkask-cns           # Cybernetic Nervous System
├── hkask-templates     # Registry, hLexicon, cascade
├── hkask-agents        # Pods, ACP, bot/replicant
├── hkask-ensemble      # Multi-agent chat (NO swarms)
├── hkask-keystore      # OS keychain, AES-256-GCM
├── hkask-mcp           # MCP runtime, dispatch
├── hkask-cli           # CLI commands
├── hkask-api           # HTTP API, utoipa
│
├── hkask-mcp-inference     # Okapi-backed LLM
├── hkask-mcp-storage       # Storage operations
├── hkask-mcp-memory        # Memory operations
├── hkask-mcp-embedding     # Embeddings, similarity
├── hkask-mcp-condenser     # Condensation, summarization
├── hkask-mcp-ensemble      # Multi-agent coordination
├── hkask-mcp-web           # Web search, scrape
├── hkask-mcp-scholar       # Academic research
├── hkask-mcp-spandrel      # Graph analysis
└── hkask-mcp-doc-knowledge # Document extraction
│
└── External (excluded from budget)
    ├── Okapi (mdz-axo/Okapi)
    ├── ACP (acp-runtime)
    └── MCP (rmcp)
```

---

## CNS (Cybernetic Nervous System)

**Namespace:** `cns.*` (replaces `okh.*`)

**Key spans:**
- `cns.tool.*` — tool governance, invocation
- `cns.prompt.*` — render, validate, outcome
- `cns.agent_pod.*` — lifecycle, delegation
- `cns.connector.*` — external I/O (LLM, embeddings)

**Algedonic Alert:** Variety deficit >100 → escalate to Curator/human

---

## Agent Taxonomy

| Type | Purpose | Interaction | Visibility |
|------|---------|-------------|------------|
| **Bot** | Process execution | Machine-to-machine (A2A) | Public/Shared |
| **Replicant** | Human assistance | Human-to-agent (H2A) | Episodic=Private, Semantic=Public |

**Curator:** Single replicant, system persona, user's counterpart in `kask chat`.

---

## Hallucinations (Do NOT Implement)

- Bot reputation systems
- Bot swarms / consensus mechanisms
- Cross-machine sync
- Bot marketplace
- Curator customization
- SemVer versioning (Git-only)
- Separate feedback crate (CNS handles all)
- Promotion pipeline (episodic/semantic categorical)
- Escalation primitive
- Visibility type system (OCAP-enforced)
- OCT-H currency
- Fine-tuning (axolotl)
- OpenCode-style condenser
- OpenHands-style condenser
- UCAN for h-bar (OCAP-only)
- **Three separate registries** (unified registry with `template_type` discriminator)
- **Rust-based template selection** (selection intelligence in Jinja2/LLM)

---

## Essential Commands

```bash
cargo check -p <crate>
cargo test -p <crate>
cargo clippy -p <crate> -- -D warnings
cargo fmt
```

---

## Constraint-Driven Design (P1–P7, C1–C7)

**P1** — No trait without two consumers  
**P2** — No generic without two instantiations  
**P3** — No module directory without encapsulation  
**P4** — No builder without fallibility or complexity  
**P5** — No feature flag without an activator  
**P6** — Delete stubs, don't publish them  
**P7** — Prefer deletion over deprecation  

**C1** — A type must be worn before it's tailored  
**C2** — Distinguish dead from unwired  
**C3** — Unwired code has a shelf life  
**C4** — Repetition is a missing primitive  
**C5** — Every error variant is a unique recovery path  
**C6** — A stub is a debt receipt  
**C7** — When implementations diverge, one must yield  

---

## Workspace Integrity

Before editing:
1. Check `git status --short`
2. Never overwrite uncommitted work you did not create
3. Add dependencies at `[workspace.dependencies]` level first

---

## Code Budget & Testing Policy

**Line Budget:** ≤30,000 lines Rust (excluding blank lines and comments).

### LOC Counting Definition

**Counting method:** `find crates mcp-servers -name "*.rs" -type f -exec cat {} \; | grep -v '^\s*$' | grep -v '^\s*//' | grep -v '^\s*/\*' | grep -v '^\s*\*' | wc -l`

**What Counts Toward Budget:**
- All Rust code in `crates/hkask-*` (core crates)
- All Rust code in `mcp-servers/hkask-mcp-*` (MCP servers)
- Inline `#[cfg(test)]` modules within source files
- Integration tests in `tests/` directories within crates

**What Is Excluded From Budget:**
- Blank lines
- Code comments (single-line `//` and multi-line `/* */`)
- `hkask-testing` crate (single test crate)
- Jinja2 templates (`.j2` files)
- YAML manifests (`.yaml`, `.yml` files)
- External protocols: Okapi, ACP, rmcp (dependencies)

**Rationale:** The 30k limit pressures the *system* to be minimal, not the *verification*. Rust is the "steel frame" of the building — templates and manifests are the interior walls.

**Status:** Run the LOC count command to verify current budget compliance.

---

## Completion Standard

Before claiming completion:
1. Run `cargo check`, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`
2. Run LOC count: `find crates mcp-servers -name "*.rs" -type f -exec cat {} \; | grep -v '^\s*$' | grep -v '^\s*//' | grep -v '^\s*/\*' | grep -v '^\s*\*' | wc -l`
3. Verify ≤30,000 lines (excluding blanks and comments)
4. Report exact commands and whether they passed
5. If verification fails, fix it or state the remaining blocker

---

## Documentation Budget Policy

**Line Budget:** ≤10,000 lines Markdown (working documentation only).

### Counting Method

```bash
# Count all markdown excluding target/
find . -path ./target -prune -o -type f -name "*.md" -print | xargs wc -l
```

### What Counts Toward Budget

Working documentation that has not yet been formalized or archived:
- `docs/research/` — Research notes, analysis, task reports
- `docs/plans/` — Roadmaps, TODOs, planning documents
- `docs/status/` — Project status, known issues
- `docs/gml/` — GML implementation docs (excluding README)
- `docs/architecture/` — Supporting docs (AGENT_POD_IMPLEMENTATION.md, Curator persona, template-header-standard.md, utoipa-implementation.md, MODEL_CATALOG.md)
- `docs/standards/` — WRITING_EXCELLENCE.md, WRITING_EXCELLENCE_AUDIT.md
- `docs/` root — CI-CD-GUIDE.md, P2_DOCUMENTATION_REFRESH.md, OPTIONAL_FOLLOWUPS_COMPLETE.md
- `monitoring/DEPLOY.md`
- `assets/LOGO-DESIGN-PRINCIPLES.md`
- `hkask-testing/docs/SEMANTIC_MAP.md`
- Root level: `CI-CHANGES.md`, `AGENTS.md`

### What Is Excluded From Budget

Formal, required, or archived documentation:
- **Master TOGAF Documents** (17 files, ~5,285 lines) — See list below
- **User Guides** (5 files, ~3,203 lines) — `docs/user-guides/*.md`
- **README Files** (5 files, ~643 lines) — All `README.md` files
- **Archive** (~107 files, ~25,366 lines) — `docs/archive/**/*.md`
- Blank lines and code comments within markdown

### Master TOGAF Documents (Excluded from Budget)

These 17 files constitute the required TOGAF Lite architecture documentation:

| File | Lines | TOGAF Domain |
|------|-------|--------------|
| `docs/architecture/hKask-architecture-master.md` | 269 | Architecture Vision |
| `docs/architecture/business-architecture.md` | 280 | Business Architecture |
| `docs/architecture/application-architecture.md` | 359 | Application Architecture |
| `docs/architecture/data-architecture.md` | 356 | Data Architecture |
| `docs/architecture/TECHNOLOGY.md` | 303 | Technology Architecture |
| `docs/architecture/security-architecture.md` | 393 | Security Architecture |
| `docs/architecture/PRINCIPLES.md` | 383 | Architecture Principles |
| `docs/architecture/hKask-erd.md` | 356 | Architecture ERD |
| `docs/architecture/magna-carta.md` | 203 | Constitutional Doc |
| `docs/architecture/hKask-hLexicon.md` | 429 | Functional Logic Reference |
| `docs/architecture/registry-templating-prompt-v2.md` | 492 | Registry Specification |
| `docs/architecture/registry-erd.md` | 205 | Registry Data Model |
| `docs/architecture/ADR-021-security-hardening.md` | 173 | Architecture Decision Record |
| `docs/standards/GOVERNANCE.md` | 345 | Governance |
| `docs/standards/DOCUMENTATION_STANDARDS.md` | 353 | Documentation Standards |
| `docs/standards/DEPENDENCY_POLICY.md` | 251 | Dependency Policy |
| `docs/TOGAF_LITE_FOR_OPEN_SOURCE.md` | 135 | TOGAF Methodology |
| **Total** | **5,285** | |

### Rationale

The 10k limit pressures the *documentation* to remain lean and actionable. Working documents must either:
1. **Mature** → Move to User Guides or Master TOGAF docs (excluded from budget)
2. **Archive** → Move to `docs/archive/` when superseded or historical (excluded from budget)
3. **Delete** → Remove when no longer relevant

This prevents documentation drift and ensures the active docs remain current, focused, and useful.

### Budget Compliance Actions

When over budget, prioritize in this order:
1. **Archive session summaries** — Move completed session reports into `docs/archive/` subfolders
2. **Consolidate research** — Merge related research documents into single comprehensive reports
3. **Promote to formal** — Move mature content to User Guides or Master TOGAF docs
4. **Delete obsolete** — Remove TODOs, completed plans, or superseded analysis

**Status:** Run the documentation count command to verify current budget compliance.

---

## Starting Point

1. Read `docs/architecture/hKask-architecture-master.md` (sole authoritative spec, v0.21.0)
2. Read `docs/architecture/hKask-erd.md` (entity relationship diagrams)
3. Read `docs/architecture/registry-templating-prompt-v2.md` (registry & templating design)
4. Read `AGENTS.md` (this operating guide)
5. Begin Phase 0: Workspace skeleton

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
*As simple as possible, but no simpler.*
*Rust is the loom. YAML/Jinja2 is the thread.*
*MVP in progress.*
