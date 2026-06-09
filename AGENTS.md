# Agent Operating Guide — hKask

**hKask** (ℏKask) — A Minimal Viable Container for Agents | `kask` binary | `hkask-` crate prefix | v0.27.0

---

## Skills (Must Load Before Work)

Activate the relevant skill via `skill` tool when its conditions are met:

| Skill | When to Activate |
|-------|-----------------|
| **coding-guidelines** | Before writing or reviewing any code. Surfaces assumptions, enforces simplicity, surgical changes, goal-driven execution. |
| **tdd** | Building features or fixing bugs. Vertical tracer-bullet RED→GREEN→REFACTOR. Every test carries `// REQ:` tag from spec. |
| **refactor-service-layer** | Extracting duplicated business logic from CLI/API/MCP surfaces into `hkask-services`. Strangler fig pattern, deep-module discipline. |
| **improve-codebase-architecture** | Finding deepening opportunities. Walk codebase for shallow modules, tight coupling, untested seams. |
| **condenser-continuation** | Resuming condenser implementation after context reset. Restores session state, prioritizes remaining tasks, verifies build health. |

---

## Design Constraints (Non-Negotiable)

- **Headless only.** No visual UI, Grafana, dashboards, web frontends, GUIs. CLI/MCP/API only.
- **No monitoring stacks.** Prometheus, Alertmanager, external observability forbidden. CNS provides programmatic observability.
- **No excess complexity.** No `todo!()`, `unimplemented!()`, `#[deprecated]`, unused traits, stubs, feature flags (P1-P8, C1-C8).
- **P8:** Every `#[test]` verifies a stated behavioral property of a public seam.
- **C8:** Test depth matches module depth. Shallow modules get shallow tests; deep modules get deep tests.

Violations get deleted. See `docs/architecture/PRINCIPLES.md`.

---

## Crate Map

| Crate | Purpose |
|-------|---------|
| `hkask-types` | ID types, ν-event, hLexicon |
| `hkask-storage` | SQLite + SQLCipher + sqlite-vec |
| `hkask-memory` | Semantic/episodic pipelines |
| `hkask-cns` | Cybernetic Nervous System (homeostatic self-regulation) |
| `hkask-templates` | Registry, hLexicon, cascade |
| `hkask-agents` | Pods, ACP, bot/replicant, Curation Loop |
| `hkask-ensemble` | Multi-agent chat |
| `hkask-keystore` | OS keychain, AES-256-GCM, HKDF-SHA256 |
| `hkask-mcp` | MCP runtime, dispatch, dynamic tool discovery |
| `hkask-services` | Shared service layer (CLI/API deduplication) |
| `hkask-cli` | CLI commands |
| `hkask-api` | HTTP API (utoipa) |
| `hkask-mcp-doc-knowledge` | Document parsing/chunking MCP server |
| `hkask-mcp-markitdown` | Document conversion + OCR MCP server |

**21 MCP servers:** inference (Okapi), condenser, web, ocap, keystore, cns, git, registry, spec, goal, github, fmp, telnyx, fal, rss-reader, ensemble, episodic, semantic, replicant, doc-knowledge, markitdown
**External:** Okapi (mdz-axo/Okapi), ACP (acp-runtime), MCP (rmcp)

---

## Commands

```bash
cargo check -p <crate>                    # Build & check
cargo test -p <crate>                      # Test (cargo test for workspace)
cargo clippy -p <crate> -- -D warnings     # Lint
cargo run --bin kask -- <subcommand>       # Run
kask chat                                  # Interactive (Curator, default model)
kask chat -m qwen3:8b                      # Specific model
kask chat Alice -m llama3.1:70b            # Named agent + model
echo "hello" | kask chat -f - -m qwen3:8b  # Non-interactive
kask sovereignty verify                    # Magna Carta compliance
```

**Slash commands** (`kask chat`): `/model`, `/model <query>`, `/agent [NAME]`, `/status`

---

## Key Docs

| Topic | Location |
|-------|----------|
| Architecture master | `docs/architecture/hKask-architecture-master.md` |
| Principles (P1-P9) | `docs/architecture/PRINCIPLES.md` |
| MDS Specification | `docs/architecture/MDS.md` |
| Test Program | `docs/specifications/test-program.md` |
| Test Inventory | `docs/status/test-inventory.md` |
| CNS spans (canonical) | `docs/architecture/PRINCIPLES.md` §1.4 |
| Registry & templating | `docs/architecture/interface-and-composition.md` |
| CI/CD | `docs/CI-CD-GUIDE.md` |

---

## Constraint Verification

```bash
if grep -r "grafana\|prometheus\|dashboard\|visual.*ui" crates/ --include="*.rs"; then echo "VIOLATION: Headless"; exit 1; fi
if grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/; then echo "VIOLATION: P6/P7"; exit 1; fi
```
