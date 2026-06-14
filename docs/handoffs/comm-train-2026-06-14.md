# Handoff: Communication & Training MCP Buildout

**Date:** 2026-06-14
**Session scope:** Design and implement hKask's communication infrastructure (Matrix transport, Conduit sidecar, onboarding, pod activation, REPL commands) and training MCP server (provider abstraction, dataset pipeline, LoRA adapter store).
**Progress:** ~85% complete. Core infrastructure built and compiling. Two HIGH-priority integration tasks remain.

---

## 1. Session Context

This session built hKask's communication and training infrastructure from specification through implementation. The communication work went through multiple architectural pivots: rejecting embedded Conduit in favor of a Docker sidecar, deferring the Iamb TUI client, and ultimately refactoring the communication server from a standalone MCP binary into a core crate (`hkask-communication`) owned by the daemon. A ~900-line moderation state machine was identified as a hidden control structure violating Magna Carta principles P1-P4 and was deleted. The training MCP server was built with provider abstraction, dataset pipeline, and LoRA adapter store. All key crates compile cleanly. Two integration tasks remain: starting the 7R7 listener in the daemon and resolving Matrix credentials from the keychain.

---

## 2. What Was Done

### Communication Infrastructure (Core Crate)

- **`crates/hkask-communication/`** — Core crate (not MCP server). Contains:
  - `matrix.rs` — `MatrixTransport` wrapping `matrix_sdk::Client` with 9 live API methods (login, logout, send_message, list_rooms, get_messages, create_room, invite_user, health_check, reconnect)
  - `agent_registration.rs` — `AgentRegistry` for WebID→UserId mapping and thread watchlists
  - `listener.rs` — `SevenR7Listener` passive Matrix room poller that emits CNS observation spans (does NOT classify, escalate, or moderate)
- **Compiles cleanly** (`cargo check -p hkask-communication` passes)

### MCP Communication Wrapper

- **`mcp-servers/hkask-mcp-communication/`** — Thin MCP wrapper over core crate. 9 tools: `tts_speak`, `tts_generate`, `tts_list_voices`, `send_message`, `create_thread`, `invite_agent`, `list_threads`, `monitor_thread`, `tag_agent`
- **Compiles cleanly** (`cargo check -p hkask-mcp-communication` passes)
- Note in `main.rs` L375: *"7R7 listener is started by the daemon, not here."*

### Training MCP Server

- **`mcp-servers/hkask-mcp-training/`** — 6 tools: `training_ingest_qa`, `training_submit`, `training_status`, `training_cancel`, `training_list_adapters`, `training_delete_adapter`
- `providers.rs` — Provider abstraction (axolotl/unsloth)
- `dataset.rs` — Dataset pipeline (ChatML/ShareGPT/Alpaca/RawText normalization)
- `adapters.rs` — LoRA adapter store (in-memory + SQLite)
- **Compiles cleanly** (`cargo check -p hkask-mcp-training` passes)
- **0 tests** (deferred)

### Conduit Docker Sidecar

- `scripts/conduit-docker.yml` — Docker Compose file for Conduit Matrix homeserver
- `scripts/conduit-docker.sh` — Management script with Docker/Podman auto-detection (start, stop, status, register, logs)
- Conduit runs on `http://localhost:8008`, local-only, no federation, no TLS

### Onboarding Matrix Registration

- `crates/hkask-services/src/onboarding.rs` — `OnboardingService::register_matrix_accounts()` creates human (`@firstname-lastname:localhost`) and replicant (`@displayname-bot:localhost`) accounts on Conduit during onboarding
- `OnboardingService::register_system_accounts()` creates Curator + 7R7 bot accounts during bootstrap
- Credentials stored in OS keychain under `matrix-human-username`, `matrix-replicant-username`, `matrix-bot-{name}`
- Non-blocking: if Conduit isn't running, onboarding warns and continues

### Pod Activation Auto-Registration

- `crates/hkask-services/src/context.rs` L641-656 — `AgentService::build()` registers a Matrix auto-registration hook on `PodManager`
- When a pod is activated, the replicant gets a Matrix account on Conduit (`@<pod-name>-bot:localhost`)
- Password stored in keychain under `matrix-pod-<name>`

### REPL Slash Commands

- `/matrix` (`/mx`) — No arg lists joined rooms with member counts. With room ID shows last 20 messages with timestamps
- `/msg` (`/dm`) — `/msg <room_id> <text>` sends a message to a Matrix room

### Service Layer

- `crates/hkask-services/src/lifecycle.rs` — `ServerLifecycle` trait (init → start → health → shutdown) with CNS instrumentation
- `crates/hkask-services/src/context.rs` — `AgentService::build()` canonical construction path for all shared infrastructure
- **Compiles cleanly** (`cargo check -p hkask-services` passes)

### Documentation

- `AGENTS.md` updated with Conduit Docker sidecar setup and provider API key instructions

### What Was Deleted

- **Moderation state machine** (~900 lines): `ModerationQueue`, `SqliteModerationQueue`, `EscalationSeverity`, `EscalationState`, `Escalation`, `ClassificationDecision`, `NaiveKeywordClassifier`, `SevenR7Bot` (replaced by `SevenR7Listener`), 17 tests
- **Rationale:** Hidden control structure violating P1 (User Sovereignty), P2 (Affirmative Consent), P3 (Generative Space), P4 (Clear Boundaries). Communication pipe became a judge — not its scope.
- **Iamb TUI integration** — Deferred, never built. Human already has `kask chat` and external Matrix clients.

---

## 3. What Remains

### HIGH — Start 7R7 listener in daemon

**What:** The `SevenR7Listener` exists in `crates/hkask-communication/src/listener.rs` but is never instantiated or started. The daemon (`AgentService::build()` in `crates/hkask-services/src/context.rs`) should create a `MatrixTransport`, log into Matrix, wrap it in a `SevenR7Listener`, and call `.start()`.

**Where:** `crates/hkask-services/src/context.rs`, inside `AgentService::build()`, after the daemon handler setup (around L658-681).

**Dependencies:** Requires Matrix credentials (see next task).

**Strategy:**
1. Resolve Matrix credentials from keychain (see next task)
2. Create `MatrixTransport` with `HKASK_MATRIX_URL` (default `http://localhost:8008`)
3. Call `transport.login(&username, &password).await`
4. Wrap in `Arc<MatrixTransport>`, create `SevenR7Listener::new(matrix, poll_interval_secs)`
5. Call `listener.start().await`
6. Store the `Arc<MatrixTransport>` on `AgentService` so the MCP wrapper and REPL can use it (add a `matrix_transport` field and accessor)

### HIGH — Resolve Matrix credentials from keychain for daemon

**What:** Currently the MCP communication binary reads `HKASK_MATRIX_AGENT_USERNAME` and `HKASK_MATRIX_AGENT_PASSWORD` from environment variables. The daemon should resolve these from the OS keychain where onboarding stores them (`matrix-replicant-username` for the replicant account, or `matrix-bot-curator` for the Curator bot).

**Where:** `crates/hkask-services/src/context.rs`, `AgentService::build()`, and `crates/hkask-keystore/`.

**Strategy:**
1. In `AgentService::build()`, after keychain entries exist, resolve the Matrix username/password:
   - Try `matrix-bot-curator` first (system bot account)
   - Fall back to `matrix-replicant-username` (onboarding replicant account)
   - Use `Keychain::default().get_by_key(...)` to retrieve
2. Pass these to `MatrixTransport::login()`
3. Keep env var fallback for backward compatibility (`HKASK_MATRIX_AGENT_USERNAME`/`PASSWORD`)

### MEDIUM — Training server tests

**What:** Provider implementations, dataset pipeline normalization, and adapter store CRUD need test coverage. Currently 0 tests.

**Where:** `mcp-servers/hkask-mcp-training/src/providers.rs`, `dataset.rs`, `adapters.rs`

**Strategy:**
1. Unit tests for `DatasetPipeline::normalize()` with ChatML/ShareGPT/Alpaca/RawText samples
2. Unit tests for `AdapterStore` CRUD operations (in-memory mode, no SQLite dependency)
3. Provider abstraction tests (mock provider that returns known configs)
4. Tag each test with `// REQ:` referencing the training spec

### MEDIUM — Communication server integration tests

**What:** Integration tests for Matrix transport operations. Requires a running Conduit instance.

**Where:** `crates/hkask-communication/` (add `#[cfg(test)]` module or `tests/` directory)

**Strategy:**
1. Use the Docker sidecar (`scripts/conduit-docker.sh start`) in CI
2. Test: login → create room → send message → get messages → list rooms → invite → logout
3. Test: health_check, reconnect after disconnect
4. Test: AgentRegistry resolve and monitor_thread

### LOW — Conduit auto-restart on health failure

**What:** The daemon could call `docker restart hkask-conduit` if `conduit_health_check()` fails repeatedly.

**Where:** CNS monitoring loop or a dedicated health-check loop in `AgentService::build()`

**Strategy:** Add a background task that periodically calls `conduit_health_check()`. After N consecutive failures, attempt `docker restart hkask-conduit` (requires Docker socket access). Emit CNS spans for each state transition.

### Deferred — E2EE key material

**What:** Matrix end-to-end encryption keys in `hkask-keystore`.

**Blocker:** SQLCipher/SQLite linking conflict between `hkask-storage` (rusqlite 0.39) and `matrix-sdk-sqlite` (rusqlite 0.37). Do not attempt until the dependency conflict is resolved.

### Design Work Needed — Moderation policy layer

**What:** The deleted state machine was wrong, but the spec still says "7R7 monitors, Curator moderates, CNS escalates." The *how* needs to be designed as an agent/skill/template/LLM system, not hardcoded Rust.

**Infrastructure ready:** Matrix transport, CNS spans, 7R7 listener. The policy layer is not.

---

## 4. Recommended Skills and Tools

| Skill | Why |
|-------|-----|
| **coding-guidelines** | Before writing any code — surfaces assumptions, enforces simplicity, surgical changes |
| **tdd** | For training server tests and communication integration tests — RED→GREEN→REFACTOR with `// REQ:` tags |
| **condenser-continuation** | If context resets during this work, restore session state and verify build health |

### Build/Test Commands

```bash
# Check all relevant crates
cargo check -p hkask-communication -p hkask-mcp-communication -p hkask-mcp-training -p hkask-services

# Run existing tests
cargo test -p hkask-services   # lifecycle tests (3 passed)
cargo test -p hkask-agents      # pod tests (5 passed)

# Lint
cargo clippy -p hkask-communication -p hkask-services -- -D warnings

# Conduit management
./scripts/conduit-docker.sh start
./scripts/conduit-docker.sh status
```

---

## 5. Key Decisions to Preserve

1. **Conduit as Docker sidecar, not embedded library.** Rejected embedding Conduit as a Rust library dependency because: (a) simpler deployment, (b) no compile-time cost, (c) no SQLite linking conflicts with `matrix-sdk-sqlite` vs `rusqlite 0.39`. Do not reverse this without solving the SQLite conflict.

2. **Communication server is infrastructure, not a tool surface.** Matrix connectivity is always-on, shared by daemon, REPL, pod activation, and CNS. Making it a separate MCP binary created three duplicate Matrix connections. The refactoring to `crates/hkask-communication` as a core crate owned by the daemon is the correct architecture. The MCP binary (`hkask-mcp-communication`) is now a thin wrapper — it should stay thin.

3. **7R7 is a passive listener, not a moderator.** The deleted moderation state machine embedded classification, escalation, and judgment in the communication pipe. The correct architecture: Matrix messages flow through → CNS observes → agent layer (Curator + skills + templates + LLM) decides what content means. The `SevenR7Listener` emits CNS spans only. Do not add classification logic to the listener.

4. **Matrix credentials in keychain, not env vars.** Onboarding stores Matrix credentials in the OS keychain (`matrix-human-username`, `matrix-replicant-username`, `matrix-bot-{name}`). The daemon should resolve from keychain first, with env var fallback for backward compatibility. Do not make env vars the primary resolution path.

5. **Non-blocking Matrix operations.** If Conduit isn't running, onboarding warns and continues. Pod activation Matrix registration is non-blocking and non-fatal. The system must function without Matrix. Do not make Matrix a hard dependency for core operations.

6. **Iamb TUI is deferred, not deleted.** The human already has `kask chat` (dual-presence Curator coordination) and external Matrix clients (FluffyChat/Element). Adding a third TUI would be redundant. Do not build Iamb integration without a specific user request justifying the redundancy.

7. **Training server uses daemon flow for memory.** `TrainingServer` connects to the daemon socket for experience recording and semantic memory. This is the correct pattern — MCP servers should not own independent memory stores. Do not add independent storage to the training server.
