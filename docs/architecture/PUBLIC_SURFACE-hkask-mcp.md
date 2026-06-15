# Public Surface Justification — hkask-mcp

**Crate:** `hkask-mcp`  
**Public items in lib.rs:** 17  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-mcp` is the **MCP runtime foundation** — shared infrastructure used by all 10 MCP servers. Its surface is large because it provides the common MCP server framework:

1. **Server runtime** — `run_server()`, `ServerContext`, `ToolSpanGuard`, credential management.
2. **Startup verification** — `verify_startup_gates()` for P4 Gate 1/2/3 enforcement.
3. **Daemon client** — `DaemonClient` for Unix socket communication with the hKask daemon.
4. **Git CAS adapter** — `GitCasAdapter` for content-addressed template storage.
5. **MCP tool macros** — `#[tool]` attribute macro for tool registration.

## Mitigations

- **Shared infrastructure:** Each MCP server would otherwise duplicate daemon communication, startup verification, and tool registration.
- **Macro-based tool registration:** The `#[tool]` macro eliminates boilerplate across 143 tools.

## Deletion Test

Delete `hkask-mcp` and the daemon client, startup gate verification, tool span guards, and MCP server runtime reappear duplicated across all 10 MCP servers. The crate earns its existence.
