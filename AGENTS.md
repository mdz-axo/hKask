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
| 5 | **Composition** | 3 registries (Prompts, Processes, Cognition) |

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

## Tests

Test public behavior, integration boundaries, and critical invariants. Avoid tests that mirror implementation structure or exist for coverage decoration.

---

## Completion Standard

Before claiming completion:
1. Run `cargo check`, `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`
2. Run `tokei` to verify ≤30,000 lines
3. Report exact commands and whether they passed
4. If verification fails, fix it or state the remaining blocker

---

## Starting Point

1. Read `docs/architecture/hKask-architecture-master.md` (sole authoritative spec)
2. Read `docs/architecture/hKask-implementation-handoff.md` (agent handoff)
3. Begin Phase 0: Workspace skeleton

---

*ℏKask — Planck's Constant of Agent Systems — v2.2*
*As simple as possible, but no simpler.*
