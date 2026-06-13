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
| `hkask-inference` | Inference router (Ollama, Fireworks, DeepInfra, fal.ai) |
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
| `hkask-mcp-media` | Media generation MCP server (image, video, audio, 3D) |
| `hkask-mcp-doc-knowledge` | Document parsing/chunking MCP server |
| `hkask-mcp-markitdown` | Document conversion + OCR MCP server |
| `hkask-mcp-research` | Web search, extraction, browsing, RSS feed research |
| `hkask-mcp-replica` | Authorial style embedding and composition |

**10 MCP servers:** memory, condenser, research, spec, fmp, communication, media, replica, doc-knowledge, markitdown
**Internal cognition:** inference (hkask-inference — Ollama, Fireworks, DeepInfra, fal.ai), CNS, OCAP, keystore, registry, git (CAS), goals (direct crate calls, not MCP), daemon (Unix socket at ~/.config/hkask/daemon.sock)
**External:** Ollama, Fireworks.ai, DeepInfra, fal.ai, ACP (acp-runtime), MCP (rmcp)

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
kask pod create -t <template> -p <persona.yaml>  # Create agent pod
kask pod activate <pod-id>                   # Activate pod
kask pod list                                # List all pods
kask pod assign <name> <role>                # Assign MCP role to replicant (e.g., kask pod assign Bob research)
kask pod mode <name> server -r <role>        # Put replicant in server mode serving a role
kask pod mode <name> chat                    # Put replicant in chat mode
kask pod mode <name> exit                    # Exit current mode
kask style discover "David Dunning"          # Discover academic author corpus
kask style discover "Author" --no-methods   # Skip LLM concept/method extraction
kask style discover "Author" --no-curate     # Skip interactive curation
kask style discover "J. Smith" --bio "professor of psychology at Cornell"  # Disambiguate
kask style embed-corpus --config <yaml> --db <path> --passphrase <phrase>  # Build corpus
kask list styles                             # List all built style corpora
kask list templates                          # List all registered templates
kask rm styles-hemingway --db <path> --passphrase <phrase>  # Remove a style corpus
kask rm templates-my-template                # Check template existence (removal via YAML)
```

### Replicant Server Mode (MCP)

Replicants can operate in **server mode**, presenting as MCP servers to IDEs (Zed, VSCode) and other hKask agents. The daemon (`~/.config/hkask/daemon.sock`) mediates authentication, role assignment, capability verification, and dual memory encoding.

**Startup flow:**
1. `kask login <replicant>` — authenticate (creates session in UserStore)
2. `kask pod assign <replicant> <role>` — assign MCP role (P4 Gate 2: sovereignty/consent)
3. `kask pod mode <replicant> server -r <role>` — enter server mode (P4 Gate 1: OCAP)
4. IDE spawns MCP binary with `HKASK_REPLICANT=<replicant>`
5. Binary connects to daemon → auth → assignment → capability → serve

**Memory flow:**
- Tool calls → `record_experience()` → daemon `store_experience` → dual encoding (episodic + semantic)
- Every 10 experiences → `generate_narrative()` → inference analyzes session log → stores observations as episodic "narrative"/"thought"
- Existing consolidation pipeline extracts semantic knowledge from both streams

**Mode mutual exclusion (initial):** An agent can be in Chat mode OR Server mode, not both. Concurrency planned for future release.

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
