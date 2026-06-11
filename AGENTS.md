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
| `hkask-agents` | Pods, ACP, bot/replicant, Curation Loop, Ensemble sessions |
| `hkask-keystore` | OS keychain, AES-256-GCM, HKDF-SHA256 |
| `hkask-mcp` | MCP runtime, dispatch, dynamic tool discovery |
| `hkask-services` | Shared service layer (CLI/API deduplication) |
| `hkask-cli` | CLI commands |
| `hkask-api` | HTTP API (utoipa) |
| `hkask-mcp-doc-knowledge` | Document parsing/chunking MCP server |
| `hkask-mcp-markitdown` | Document conversion + OCR MCP server |
| `hkask-mcp-research` | Web search, extraction, browsing, RSS feed research |
| `hkask-mcp-replica` | Authorial style embedding and composition |

**10 MCP servers:** memory, condenser, research, spec, fmp, telnyx, fal, replica, doc-knowledge, markitdown
**Internal cognition:** inference (Okapi), CNS, OCAP, keystore, registry, git (CAS), goals (direct crate calls, not MCP)
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
kask onboard                               # Add a new replicant to an existing install
kask sovereignty verify                    # Magna Carta compliance
kask settings show                          # Show all settings
kask settings show temp                     # Show one setting
kask settings set temp 0.3                  # Set a setting
kask settings reset                         # Reset all to defaults
```

**Slash commands** (`kask chat`): `/model`, `/model <query>`, `/agent [NAME]`, `/status`, `/repl [setting] [value]`, `/start`, `/feedback`

**`/repl` sub-settings** (user-configurable inference params, persisted to `~/.config/hkask/settings.json`):

| Setting | Type | Range | Default | Description |
|---------|------|-------|---------|-------------|
| `loops` | usize | ≥1 | 21 | Max tool-call loop iterations per turn |
| `context` | usize | ≥0 | 3 | Past turns in context (0 = no history) |
| `temp` | f32 | 0.0–2.0 | 0.7 | Sampling temperature |
| `top_p` | f32 | 0.0–1.0 | 0.9 | Nucleus sampling |
| `top_k` | u32 | ≥1 | 40 | Top-k filtering |
| `min_p` | f32 | 0.0–1.0 | 0.0 | Min-p threshold (0.0 = disabled) |
| `typical_p` | f32 | 0.0–1.0 | 0.0 | Locally typical sampling (0.0 = disabled) |
| `max_tokens` | u32 | ≥1 | 512 | Max completion tokens per call |
| `seed` | u32 or `off` | — | random | Deterministic seed (`off` = random) |
| `gas_heuristic` | u64 | ≥1 | 500 | Per-turn gas reservation |
| `gas_cap` | u64 | ≥1 | 10,000 | Total session energy budget cap |
| `auto_condense` | `on`/`off` | — | on | Auto-condense at 87.5% of context window |
| `reset` | — | — | — | Reset all to defaults |

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
| Registry & templating | `docs/architecture/ADR-024-unified-registry.md` |
| CI/CD | `docs/CI-CD-GUIDE.md` |
| API Endpoints | `docs/api/endpoints.md` |

### API Settings Endpoints

`hkask-api` exposes settings via `GET/PUT /api/settings` (same shared `~/.config/hkask/settings.json`):

```http
GET /api/settings            # Returns full SettingsResponse JSON
PUT /api/settings            # Merge-updates with UpdateSettingsRequest body
```

**Fields in both request/response:** `tool_loop_limit`, `context_turns`, `temperature`, `top_p`, `top_k`, `min_p`, `typical_p`, `max_tokens`, `seed`, `gas_heuristic`, `gas_cap`, `auto_condense`. Response also includes `context_length` and `supports_thinking` (read-only model metadata).

### ReplSettings Struct

Defined in `hkask-cli::repl::handlers::repl_settings` — stored as `repl_settings` field on `ReplState`. Serializable via `serde`, loaded from disk at REPL init, mutable during session via `/repl`, and convertible to `hkask_types::LLMParameters` via `to_llm_params()`. Also read by `hkask-services` for shared init paths.

---

## Constraint Verification

```bash
if grep -r "grafana\|prometheus\|dashboard\|visual.*ui" crates/ --include="*.rs"; then echo "VIOLATION: Headless"; exit 1; fi
if grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/; then echo "VIOLATION: P6/P7"; exit 1; fi
```
