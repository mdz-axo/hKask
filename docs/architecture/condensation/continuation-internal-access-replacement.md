# Condensation Continuation — Internal Access Replacement & ACP Ports

**Status:** Pending. hKask MCP server consolidation reduced 21→10 servers. Internal servers were removed from the workspace but their callers still use MCP dispatch. This task replaces those dispatch calls with direct function calls.

---

## Background

PRINCIPLES.md §1.2 now lists 10 MCP servers (9 external + 1 memory). The 10 deleted servers (inference, cns, ocap, keystore, git, registry, goal, github, replicant, ensemble) were removed from the workspace and all registration lists. However, callers that previously dispatched to these servers via MCP still need to be updated to use direct crate function calls.

## Target State

### Replace MCP dispatch with direct calls

| Deleted Server | Callers to update | Replacement |
|----------------|-------------------|-------------|
| `hkask-mcp-inference` | `hkask-cli` ensemble dispatch, `hkask-api` chat routes, any `mcp_dispatcher.invoke("inference_*")` | `InferencePort::generate()` / `InferenceService` |
| `hkask-mcp-cns` | Any caller using CNS health/alerts via MCP | `CnsRuntime` / `CnsService` direct methods |
| `hkask-mcp-keystore` | Any caller storing/retrieving secrets via MCP | `Keychain` / `Keystore` crate direct functions |
| `hkask-mcp-ocap` | Any caller checking capabilities via MCP | `GovernedTool::enforce()` + `SovereigntyChecker::can_access()` |
| `hkask-mcp-registry` | Any caller using template/skill registry via MCP | `Registry` / `SqliteRegistry` direct methods |
| `hkask-mcp-git` | Any caller doing CAS operations via MCP | `GitCasAdapter` / `GitCASPort` direct methods |
| `hkask-mcp-goal` | Any caller managing goals via MCP | `SqliteGoalRepository` direct methods |

### ACP Ports

| Deleted Server | Replacement |
|----------------|-------------|
| `hkask-mcp-replicant` | ACP ports for replicant chat (agent-to-agent protocol) |
| `hkask-mcp-ensemble` | ACP ports for multi-agent ensemble coordination |

### Cloud Backup

`hkask-mcp-github` was deleted. Add a `memory_backup` tool to `hkask-mcp-memory` that exports/imports the memory database. The backup target should be configurable (local file by default; cloud storage as future extension).

---

## Approach

### Phase 1 — Audit remaining MCP dispatch calls

Search for all call sites that dispatch to deleted server tool names:

```bash
grep -rn "mcp_dispatcher.invoke\|mcp_runtime.start_server" crates/ --include="*.rs" | grep -v "hkask-mcp-\(condenser\|web\|spec\|fmp\|telnyx\|fal\|rss-reader\|doc-knowledge\|markitdown\|memory\)"
```

Map each call to its replacement direct function.

### Phase 2 — Replace inference dispatch

1. Find all `inference_generate`, `inference_models`, `inference_generate_vision` dispatch calls in CLI and API
2. Replace with `InferenceService::resolve_port()` + `InferencePort::generate()`
3. Update CNS energy tracking: inference now goes through `GovernedTool` wrapping `InferencePort`, not MCP dispatch
4. Update `CompositeEnergyEstimator` routing if needed (currently uses `"hkask-mcp-inference"` as a routing key — may need to change to a new key for internal inference calls)

### Phase 3 — Replace CNS dispatch

1. Find all `cns_*` tool dispatch calls
2. Replace with `CnsRuntime` / `CnsService` method calls from `hkask-services`
3. CNS health, alerts, variety queries should use direct service methods
4. Remove any remaining CNS MCP credential requirements

### Phase 4 — Replace keystore/ocap/registry/git/goal dispatch

1. Audit each caller for the deleted servers
2. Replace with direct crate function calls
3. Ensure OCAP enforcement works without MCP dispatch (sovereignty is now at the `GovernedTool` layer)
4. Verify goal CRUD works via `SqliteGoalRepository`

### Phase 5 — ACP ports

1. Define ACP (Agent Communication Protocol) port trait in `hkask-types::ports`
2. Implement ACP adapter using existing ensemble/replicant infrastructure
3. Wire ACP ports into `ServiceContext` and `CuratorContext`
4. Verify multi-agent chat works via ACP, not MCP

### Phase 6 — Cloud backup tool

1. Add `memory_backup` tool to `hkask-mcp-memory` — exports DB to a configurable path
2. Add `memory_restore` tool — imports DB from a backup file
3. Cloud storage (S3, GitHub, etc.) as future extension

### Phase 7 — Verify

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
# Verify zero MCP dispatch calls to deleted servers
grep -rn "mcp_dispatcher.invoke\|mcp_runtime.start_server" crates/ --include="*.rs"
```

---

## Key Files

| File | Purpose |
|------|--------|
| `crates/hkask-cli/src/commands/models.rs` | Already updated — reference implementation for Phase 2 |
| `crates/hkask-cli/src/commands/ensemble.rs` | May dispatch to inference via MCP |
| `crates/hkask-cli/src/repl/tool_augmented.rs` | Tool call parsing — may still reference inference server |
| `crates/hkask-api/src/routes/chat.rs` | Chat routes — may dispatch to inference |
| `crates/hkask-services/src/inference.rs` | `InferenceService` — direct inference access |
| `crates/hkask-services/src/cns.rs` | `CnsService` — direct CNS access |
| `crates/hkask-services/src/context.rs` | `ServiceContext` — has `inference_port`, `cns_runtime` |
| `crates/hkask-cns/src/composite_energy_estimator.rs` | Inference energy routing key |
| `crates/hkask-types/src/ports/mod.rs` | `InferencePort` trait — add `AcpPort` here |
| `mcp-servers/hkask-mcp-memory/src/main.rs` | Add `memory_backup`/`memory_restore` tools |
| `crates/hkask-agents/src/ports/` | Existing agent ports — ACP wiring |
| `crates/hkask-agents/src/ensemble/` | Ensemble infrastructure — ACP conversion target |

## Verification

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
# No MCP dispatch to internal servers
grep -rn "hkask-mcp-inference\|hkask-mcp-cns\|hkask-mcp-ocap\|hkask-mcp-keystore\|hkask-mcp-registry\|hkask-mcp-git\|hkask-mcp-goal\|hkask-mcp-replicant\|hkask-mcp-ensemble\|hkask-mcp-github" crates/ --include="*.rs" | grep -v "table_energy_estimator\|composite_energy_estimator"
# Ensemble and replicant work via ACP
# Memory server has backup tool
```

---

*This continuation prompt captures all context needed for Phase 2 (full), Phase 4 (ACP), and cloud backup implementation.*
