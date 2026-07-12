---
title: "hkask-mcp — API Reference"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain]
last-verified-against: "e17e69e2"
---

# hkask-mcp — API Reference

MCP (Model Context Protocol) runtime and dispatch layer. Provides server management, capability-based dispatch with OCP, URL validation, adapter lifecycle, and server scaffolding for all 16 hKask MCP server binaries.

## Public Modules

| Module | Description |
|---|---|
| `daemon` | Unix socket transport for MCP binary ↔ hKask daemon communication. Types: `DaemonClient`, `DaemonHandler`, `DaemonListener`, `DaemonRequest`, `DaemonResponse` |
| `dispatch` | Tool dispatch through the GovernedTool membrane. Types: `McpDispatcher`, `RawMcpToolPort` |
| `git_cas` | Git CAS adapter: `GixCasAdapter` |
| `runtime` | MCP server runtime: `McpRuntime`, `McpServer`, `McpTool`, `ServerStartError` |
| `server` | Server scaffolding: `CapabilityTier`, `CredentialRequirement`, `ExperienceCallback`, `McpError`, `ServerContext`, `ToolContext`, plus helper functions |
| `startup` | P4 Gate 1/2/3 startup verification for MCP server binaries: `StartupGateResult`, `verify_startup_gates()` |

## Key Public Types

### `MCPBootstrap`

Result of the standard MCP server daemon bootstrap flow, used by all 12 MCP server binaries.

**Fields:**
| Field | Type | Description |
|---|---|---|
| `replicant` | `String` | Replicant identity for capability tokens |
| `daemon_client` | `Option<DaemonClient>` | Daemon client (None when daemon unavailable) |

### `DaemonClient`

Unix socket client for communication with the hKask daemon. Constructed via `DaemonClient::new()`. Provides event recording capabilities for `record_via_daemon()`.

### `ToolContext`

Trait implemented by all 14 hKask MCP server structs. Canonical interface for identity and outcome recording.

**Methods:**
- `webid(&self) -> &WebID` — agent identity for capability tokens and ownership
- `record_tool_outcome(&self, tool: &str, outcome: &str)` — record tool execution result via daemon

### `ServerContext`

Context passed to MCP server factory closures during bootstrap. Provides identity and credential resolution.

### `McpError`

Error type for MCP server operations.

### `CredentialRequirement`

Declares a required credential for an MCP server.

### `CapabilityTier`

Capability tier classification for MCP tools.

### `McpRuntime`

Runtime managing MCP server lifecycles. Implements `McpServer` management.

### `McpServer`

An individual MCP server within the runtime, with associated `McpTool` instances.

### `McpDispatcher`

Tool dispatcher through the GovernedTool membrane. Implements `ToolPort`. Handles capability-based dispatch with OCP verification.

### `RawMcpToolPort`

A raw (un-governed) tool port used within the dispatch pipeline.

## Constants

### `BUILTIN_SERVERS`

Canonical registry of all built-in MCP servers. Type: `&[(&str, &str)]` — `(server_id, binary_name)` pairs.

| Server ID | Binary Name |
|---|---|
| `memory` | `hkask-mcp-memory` |
| `condenser` | `hkask-mcp-condenser` |
| `research` | `hkask-mcp-research` |
| `companies` | `hkask-mcp-companies` |
| `communication` | `hkask-mcp-communication` |
| `curator` | `hkask-mcp-curator` |
| `media` | `hkask-mcp-media` |
| `docproc` | `hkask-mcp-docproc` |
| `training` | `hkask-mcp-training` |
| `replica` | `hkask-mcp-replica` |
| `kanban` | `hkask-mcp-kata-kanban` |
| `skill` | `hkask-mcp-skill` |
| `filesystem` | `hkask-mcp-filesystem` |
| `codegraph` | `hkask-mcp-codegraph` |

All consumers that start MCP servers must use this list. Subsets are permitted for intentionally-sandboxed environments but must reference this constant as the upper bound.

## Public Functions

### `bootstrap_mcp_server()`

```rust
pub async fn bootstrap_mcp_server(
    server_name: &str,
    target: &str,
    host_env_var: &str,
) -> MCPBootstrap
```

Standard MCP server bootstrap flow:
1. Load `.env`
2. Verify P4 startup gates (auth, role, tools) against the daemon
3. If daemon is unavailable, warn and fall back to direct/standalone mode

**Parameters:**
- `server_name` — short name (e.g., `"communication"`)
- `target` — tracing target (e.g., `"hkask.mcp.communication"`)
- `host_env_var` — env var for replicant identity (defaults to `"HKASK_MCP_HOST"`; curator uses `"HKASK_CURATOR_REPLICANT"`)

### `run_server()`

```rust
pub async fn run_server<S, F>(
    name: &str,
    version: &str,
    factory: F,
    credentials: Vec<CredentialRequirement>,
) -> Result<(), McpError>
where
    S: rmcp::ServiceExt<rmcp::RoleServer> + rmcp::Service<rmcp::RoleServer>,
    F: FnOnce(ServerContext) -> Result<S, McpError>,
```

Canonical entry point for all hKask MCP servers. Each server's `main.rs` calls this directly. Delegates to `run_stdio_server()`.

### `run_server_with_preloaded()`

```rust
pub async fn run_server_with_preloaded<S, F>(
    name: &str,
    version: &str,
    factory: F,
    credentials: Vec<CredentialRequirement>,
    preloaded: HashMap<String, String>,
) -> Result<(), McpError>
```

Same as `run_server()` but with preloaded `.env` credentials via a `HashMap<String, String>`.

### Helper Functions (from `server` module)

- `validate_identifier(name: &str, value: &str, max_len: usize) -> Result<(), McpError>` — validates an identifier field
- `api_get()` / `api_put()` — API helper functions
- `execute_tool()` — tool execution helper
- `load_dotenv()` — load environment variables
- `record_via_daemon()` — record tool outcome through daemon
- `run_stdio_server()` — run an MCP server with stdio transport
- `run_stdio_server_with_preloaded()` — stdio server with preloaded credentials
- `resolve_credential()` — resolve a credential requirement
- `tool_internal_error()` — produce a tool internal error response

## Macros

### `mcp_server!`

Defines an MCP server struct with standard fields (`webid: WebID`, `replicant: String`, `daemon: Option<DaemonClient>`) plus any domain-specific fields. Generates a `new()` constructor and `ToolContext` impl via `impl_tool_context!`.

**Usage:**
```ignore
mcp_server!(struct SkillServer {
    inference_port: Arc<dyn InferencePort>,
    skills: HashMap<String, SkillDef>,
});
```

Also supports a variant with no custom fields:
```ignore
mcp_server!(struct MinimalServer;);
```

### `impl_tool_context!`

Generates a `ToolContext` impl for an MCP server struct. Expects `webid`, `replicant`, and `daemon` fields.

**Usage:**
```ignore
impl_tool_context!(CommunicationServer);
```

### `validate_field!`

Validates an identifier field and returns early on error via the span. Eliminates repeated 3-line validation patterns.

**Usage:**
```ignore
validate_field!(span, "session_id", &session_id, 256);
```

## Re-exports

`DaemonClient`, `DaemonHandler`, `DaemonListener`, `DaemonRequest`, `DaemonResponse`, `McpDispatcher`, `RawMcpToolPort`, `GixCasAdapter`, `ToolInfo` (from `hkask_ports`), `McpRuntime`, `McpServer`, `McpTool`, `ServerStartError`, `CapabilityTier`, `CredentialRequirement`, `ExperienceCallback`, `McpError`, `ServerContext`, `ToolContext`, `api_get()`, `api_put()`, `execute_tool()`, `load_dotenv()`, `record_via_daemon()`, `resolve_credential()`, `run_stdio_server()`, `run_stdio_server_with_preloaded()`, `tool_internal_error()`, `validate_identifier()`, `StartupGateResult`, `verify_startup_gates()`.
