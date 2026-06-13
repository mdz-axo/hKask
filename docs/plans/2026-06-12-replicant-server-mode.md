---
title: "Handoff — Replicant Server Mode Architecture"
audience: [architects, developers]
last_updated: 2026-06-12
version: "0.27.0"
status: "Active"
domain: "Technology"
mds_categories: [composition, trust, lifecycle]
---

# Handoff — Replicant Server Mode Architecture

**Session date:** 2026-06-12
**Project:** hKask v0.27.0
**Status:** Build complete, operational loops wired, documentation updated

---

## 1. Session Context

This session designed and implemented the **Replicant Server Mode** architecture: MCP servers are served by replicants operating in "server mode," with a Unix domain socket daemon mediating authentication, P4 dual-gate verification, dual memory encoding, and internal narrative generation. All 10 MCP servers were converted to the thin launcher pattern. CLI commands for role assignment and mode switching were added. Architecture documentation (ADR-035, master doc, AGENTS.md) and test inventory were updated. The build is complete — all loops are connected and operational.

---

## 2. What Was Done

### Agent Mode (`hkask-agents`)

- **`AgentMode` enum** (`Chat` | `Server`) added to `crates/hkask-agents/src/pod/types.rs`
- **Mode fields** on `AgentPod`: `mode: Option<AgentMode>`, `assigned_mcp_roles: Vec<String>` in `crates/hkask-agents/src/pod/mod.rs`
- **Mode transition methods**: `enter_server_mode(role)`, `enter_chat_mode()`, `exit_mode()`, `is_in_server_mode()`, `is_in_chat_mode()` — with P4 dual-gate enforcement (activation check + role assignment check + mutual exclusion)
- **Error variants**: `ModeConflict`, `ModeRequiresActivation`, `RoleNotAssigned`
- **4 unit tests** with `// REQ: P4-dual-gate` tags: `mode_requires_activation`, `mode_mutual_exclusion`, `role_not_assigned_denied`, `mode_exit_and_switch`
- **Daemon accessors** on `PodManager`: `find_pod_by_name()`, `get_pod_webid()`, `is_assigned_to_role()`, `has_capability()`, `assign_role()`, `set_mode()` in `crates/hkask-agents/src/pod/manager.rs`

### Daemon Transport (`hkask-mcp`)

- **`daemon.rs`** (NEW): `DaemonClient`, `DaemonListener`, `DaemonHandler` trait, JSON newline-delimited protocol
- **Well-known path**: `~/.config/hkask/daemon.sock` (XDG config dir)
- **Protocol types**: `DaemonRequest` (auth_query, assignment_query, capability_query, store_experience), `DaemonResponse` (auth_response, assignment_response, capability_response, store_response, error)
- **5 integration tests**: `daemon_auth_query_authenticated`, `daemon_auth_query_unauthenticated`, `daemon_assignment_query`, `daemon_capability_query`, `daemon_store_experience_dual_encoding`
- **`Clone` derived** on `DaemonClient` for use in factory closures

### Daemon Handler (`hkask-services`)

- **`daemon_handler.rs`** (NEW): `ServiceDaemonHandler` implementing `DaemonHandler`
- **`check_auth`**: verifies replicant exists in UserStore, checks for active sessions (MutexGuard dropped before await)
- **`check_assignment`**: delegates to `PodManager::is_assigned_to_role()`
- **`check_capability`**: delegates to `PodManager::has_capability()`
- **`store_experience`**: creates PodContext, dual-encodes to episodic (first-person, private) + semantic (third-person, public, generalized via `generalize_value()`)
- **Narrative generation**: every 10 stored experiences triggers `generate_narrative()` — queries episodic "mcp_session" triples, calls inference with system prompt, parses observations, stores as episodic "narrative"/"thought"
- **Constants**: `NARRATIVE_THRESHOLD = 10`, `NARRATIVE_SYSTEM_PROMPT` (agent reflects on session patterns)
- **Wired in `AgentService::build()`**: daemon handler created with `pod_manager`, `user_store`, `inference_port`; daemon listener bound and served in background tokio task
- **Accessor**: `AgentService::daemon_handler()` added

### MCP Server Conversion (all 10)

Each server now follows the **thin launcher + narrative recording** pattern:

| Server | Role | Key Tools with `record_experience` |
|--------|------|-----------------------------------|
| `hkask-mcp-research` | `research` | `web_search`, `web_extract` |
| `hkask-mcp-condenser` | `condenser` | `condenser_compress`, `condenser_thread_summary` |
| `hkask-mcp-memory` | `memory` | `episodic_store`, `semantic_store`, `semantic_search` |
| `hkask-mcp-spec` | `spec` | `spec_goal_capture`, `spec_goal_decompose`, `spec_graph_query` |
| `hkask-mcp-replica` | `replica` | `replica_build`, `replica_compose`, `replica_mashup` |
| `hkask-mcp-doc-knowledge` | `doc_knowledge` | `doc_knowledge_chunk`, `doc_knowledge_parse`, `doc_knowledge_store_qa` |
| `hkask-mcp-markitdown` | `markitdown` | `markitdown_convert`, `markitdown_ocr` |
| `hkask-mcp-fmp` | `fmp` | `fmp_company_profile`, `fmp_quote`, `fmp_search` |
| `hkask-mcp-communication` | `communication` | `tts_speak`, `tts_generate` |
| `hkask-mcp-fal` | `fal` | `fal_generate_image` |

**Pattern applied to each:**
- `replicant: String` and `daemon: Option<DaemonClient>` fields on server struct
- `record_experience()` helper (fire-and-forget via `tokio::spawn`)
- `try_daemon_flow()` — auth → assignment → capability verification
- `main()` reads `HKASK_REPLICANT` from env, graceful fallback if daemon unavailable
- `chrono.workspace = true` added to Cargo.toml where missing

### CLI Commands (`hkask-cli`)

- **`kask pod assign <name> <role>`**: `PodAction::Assign` variant, `PodService::assign_role()`, handler in `commands/pod.rs`
- **`kask pod mode <name> server -r <role>`**: `PodAction::Mode` variant, `PodService::set_mode()`, handler in `commands/pod.rs`
- **`kask pod mode <name> chat`**: enter chat mode
- **`kask pod mode <name> exit`**: exit current mode
- Re-exports added to `commands/mod.rs`

### Documentation

- **`AGENTS.md`**: new commands, Replicant Server Mode section (startup flow, memory flow, mode mutual exclusion), daemon in internal cognition
- **`docs/architecture/hKask-architecture-master.md`**: new §Daemon & Replicant Server Mode with mermaid diagram, ADR-035 in decision records, date bumped to 2026-06-12
- **`docs/architecture/ADR-035-replicant-server-mode.md`** (NEW): full ADR with context, decision, architecture, startup/memory flow, CLI commands, consequences, Magna Carta compliance table, verification commands
- **`docs/status/test-inventory.md`**: v2.1.0, 187 tests across 18 crates (↑ from 130)

### Pre-existing Issues Fixed (unrelated, blocking compilation)

- `hkask-storage/src/agent_registry.rs`: Rust 2024 edition string prefix issues (added spaces)
- `hkask-agents/src/registry_loader.rs`: borrow-after-move with `voice_description`/`voice_id` (cloned before moves)
- `hkask-cli/src/commands/agent.rs`: missing `voice_description`/`voice_id` fields in `AgentDefinition` initializer

---

## 3. What Remains

### HIGH — Integration Test

**What:** End-to-end test exercising the full flow: daemon start → replicant login → assign role → enter server mode → spawn MCP binary → verify auth/assignment/capability → make tool calls → verify narrative generation triggered.

**Where:** New integration test in `hkask-services/tests/` or `hkask-cli/tests/`.

**Dependencies:** Requires hKask running with inference available for narrative generation verification.

**Strategy:** Use the mock `DaemonHandler` pattern from `hkask-mcp/src/daemon.rs` tests, extended to cover the full `ServiceDaemonHandler` flow. Or a tokio-based integration test that starts a real daemon listener on a temp socket.

### MEDIUM — Concurrent Chat+Server Mode

**What:** Remove mode mutual exclusion. An agent can be in Chat mode AND Server mode simultaneously. User can chat with Bob about what he's learning from serving MCP tools.

**Where:** `crates/hkask-agents/src/pod/mod.rs` — remove the `ModeConflict` check in `enter_server_mode()` and `enter_chat_mode()`. `AgentPod.mode` changes from `Option<AgentMode>` to a bitfield or `HashSet<AgentMode>`.

**Dependencies:** None. Pure agent-mode change.

**Timeline:** Planned for 3-6 months from now (2026-06-12).

### MEDIUM — `kask daemon` Explicit Command

**What:** Explicit `kask daemon start` / `kask daemon stop` commands. Currently daemon auto-starts with `AgentService::build()` (whenever hKask starts).

**Where:** New `DaemonAction` in `crates/hkask-cli/src/cli/actions.rs`, handler in `commands/daemon.rs`.

**Strategy:** `start` ensures the socket is bound and the serve loop is running. `stop` cancels the serve loop and removes the socket file. Mostly for operational visibility — the auto-start covers normal usage.

### LOW — Narrative Threshold Tuning

**What:** `NARRATIVE_THRESHOLD = 10` is a fixed constant. May need to be configurable per replicant or per session.

**Where:** `crates/hkask-services/src/daemon_handler.rs` — make threshold configurable via `ServiceConfig` or `ReplSettings`.

### LOW — Remaining MCP Server Test Coverage

**What:** Several MCP servers have 0 tests (memory, replica, doc-knowledge, markitdown, communication, fal). Per C8, shallow modules get shallow tests — these are API proxies and pass-throughs, so 0 tests is compliant. But basic tool schema validation tests would improve coverage.

---

## 4. Recommended Skills and Tools

| Skill | Why |
|-------|-----|
| **coding-guidelines** | Before any code changes — enforce simplicity, surgical changes, goal-driven execution |
| **tdd** | For the integration test and any new features — RED→GREEN→REFACTOR with `// REQ:` tags |
| **condenser-continuation** | If resuming after context reset — restores session state from this handoff |
| **magna-carta-verifier** | When implementing concurrent mode — verify P1-P4 compliance of the new mode transitions |

**Verification commands:**
```bash
cargo check -p hkask-agents -p hkask-mcp -p hkask-services -p hkask-cli
cargo test -p hkask-agents -p hkask-mcp -p hkask-services
cargo clippy -p hkask-agents -p hkask-mcp -p hkask-services -- -D warnings
```

---

## 5. Key Decisions to Preserve

1. **Server mode is an agent property, not an MCP runtime property.** `AgentMode` lives in `hkask-agents`, not `hkask-mcp`. The MCP binary is a thin launcher — the agent is the identity behind it. Do not reintroduce a separate "MCP server mode" concept.

2. **Unix domain socket at `~/.config/hkask/daemon.sock`** — chosen over localhost HTTP and env var discovery. Rationale: kernel-enforced local-only (P1), natural fail-closed (P2), zero configuration (P3), OS-level capability enforcement (P4). See ADR-035 §Architecture for full analysis.

3. **Dual memory encoding at experience time, not deferred consolidation.** Every `store_experience` writes both episodic (first-person, private) and semantic (third-person, public, generalized) simultaneously. Consolidation still runs later for batch processing, but the semantic seed is planted at encode time. This mirrors human memory: every experience generates both specific and generalizable knowledge.

4. **Narrative generation every 10 experiences.** The agent "thinks about" what it's observing in the MCP session — parallel to how chat-mode agents think about conversation turns. Uses the existing inference port. Observations stored as episodic "narrative"/"thought" triples. Threshold is a fixed constant for now; may become configurable later.

5. **Passphrase never stored with MCP binary.** Authentication flows through hKask's UserStore sessions. MCP binary sends `auth_query` to daemon; daemon checks session existence. User authenticates via `kask login` (existing flow). This enforces P2 Affirmative Consent without exposing credentials to out-of-process binaries.

6. **Mode mutual exclusion (initial).** Chat and Server modes are mutually exclusive. This simplifies state management during the debugging phase. Concurrent mode is planned for 3-6 months. Do not remove the mutual exclusion guard prematurely — wait for the concurrency phase.

7. **Graceful fallback, not hard requirement.** MCP binaries operate without daemon if it's unavailable (direct mode, with warning). This ensures MCP servers work during development and transition. The daemon is infrastructure, not a gatekeeper that blocks functionality.

8. **`HKASK_REPLICANT` env var as the identity bridge.** The MCP binary discovers which replicant it serves from this env var. Defaults to "anonymous" if unset. This is the single configuration point for IDE integration — Zed sets it in `context_servers` config.

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0 — Handoff 2026-06-12*
