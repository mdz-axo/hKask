---
title: "Bootstrap an MCP Server"
audience: [developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "MCP"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# Bootstrap an MCP Server

**Goal:** Create a new hKask MCP server with N tools and register it in `BUILTIN_SERVERS`.

hKask has 15 MCP servers (memory, condenser, research, companies, communication, curator,
media, docproc, training, replica, kanban, skill, filesystem, codegraph, scenarios).
Every server follows the same bootstrap pattern defined in `hkask-mcp`.

---

## Prerequisites

- hKask source tree with `crates/hkask-mcp/` built
- A new crate under `mcp-servers/` named `<your-mcp-package>`
- Familiarity with the `rmcp` crate (the MCP protocol library hKask uses)

### Cargo.toml Dependencies

Add to your new crate's `Cargo.toml`:

```toml
[dependencies]
hkask-mcp = { path = "../../crates/hkask-mcp" }
hkask-types = { path = "../../crates/hkask-types" }
hkask-ports = { path = "../../crates/hkask-ports" }
hkask-inference = { path = "../../crates/hkask-inference" }  # if you need inference
rmcp = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
dotenvy = { workspace = true }
```

---

## Step 1: Define the Server Struct

Use the `mcp_server!` macro from `hkask-mcp`. It generates the struct with mandatory fields
(`webid`, `replicant`, `daemon`) plus your domain-specific fields, along with a `new()`
constructor and a `ToolContext` implementation.

```rust
// mcp-servers/<your-mcp-package>/src/lib.rs

use hkask_mcp::mcp_server;
use std::sync::Arc;

// If you need inference:
use hkask_ports::InferencePort;

mcp_server! {
    /// Example MCP server — demonstrates the bootstrap pattern.
    pub struct ExampleServer {
        /// Optional inference port for LLM calls.
        inference_port: Option<Arc<dyn InferencePort>>,
        /// Your domain-specific state.
        items: std::collections::HashMap<String, String>,
    }
}
```

### What `mcp_server!` Generates

The macro expands to:

```rust
pub struct ExampleServer {
    pub webid: hkask_types::WebID,       // Agent identity
    pub replicant: String,                // Replicant serving this MCP server
    pub daemon: Option<hkask_mcp::DaemonClient>,  // Event recording
    pub inference_port: Option<Arc<dyn InferencePort>>,
    pub items: std::collections::HashMap<String, String>,
}

impl ExampleServer {
    pub fn new(
        webid: hkask_types::WebID,
        replicant: String,
        daemon: Option<hkask_mcp::DaemonClient>,
        inference_port: Option<Arc<dyn InferencePort>>,
        items: std::collections::HashMap<String, String>,
    ) -> Self { /* ... */ }
}

impl hkask_mcp::server::ToolContext for ExampleServer {
    fn webid(&self) -> &hkask_types::WebID { &self.webid }
    fn record_tool_outcome(&self, tool: &str, outcome: &str) {
        hkask_mcp::record_via_daemon(&self.daemon, &self.replicant, tool, outcome);
    }
}
```

The server struct can have zero custom fields. Use the `;` variant:

```rust
mcp_server! {
    /// Minimal server with no extra fields.
    pub struct MinimalServer;
}
```

---

## Step 2: Define Tool Methods

Annotate methods with `#[tool(description = "...")]` and use `execute_tool` for CNS span emission:

```rust
use hkask_mcp::server::execute_tool;
use rmcp::tool;

#[tool(description = "Liveness check")]
async fn example_ping(&self) -> String {
    execute_tool(self, "example_ping", async {
        Ok(serde_json::json!({
            "status": "ok",
            "server": "example",
        }))
    }).await
}

#[tool(description = "Store an item by key")]
async fn example_store(
    &self,
    Parameters(StoreRequest { key, value }): Parameters<StoreRequest>,
) -> String {
    execute_tool(self, "example_store", async {
        // Your domain logic here
        Ok(serde_json::json!({ "stored": key }))
    }).await
}
```

`execute_tool` wraps your logic with CNS span emission (`cns.tool.{tool_name}`) and error
mapping. Always use it for tool methods — never return raw `Result` from a `#[tool]` function.

For request types, derive `serde::Deserialize` and `rmcp::schemars::JsonSchema`:

```rust
use rmcp::schemars;
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StoreRequest {
    pub key: String,
    pub value: String,
}
```

The `Parameters<T>` wrapper is from `rmcp` and handles JSON-RPC parameter deserialization.

---

## Step 3: Apply the `tool_router` Macro

The canonical pattern uses rmcp's `#[tool_router(server_handler)]` attribute on the `impl` block
that contains your `#[tool]`-annotated methods. The macro generates the `Server` trait
implementation (`tool_box`, `call_tool`, `into_service`) automatically — no manual `Server`
impl is needed.

```rust
use rmcp::tool_router;

#[tool_router(server_handler)]
impl ExampleServer {
    #[tool(description = "Liveness check")]
    pub async fn example_ping(&self) -> String {
        execute_tool(self, "example_ping", async {
            Ok(serde_json::json!({
                "status": "ok",
                "server": "example",
            }))
        }).await
    }

    // ... other #[tool] methods ...
}
```

The `#[tool_router(server_handler)]` attribute must be placed on the `impl` block that holds
all your `#[tool]`-annotated methods. It wires them into the MCP protocol handler so that
`run_server()` can call `serve()` on the returned server.

---

## Step 4: Write the `run()` Function

Every hKask MCP server has a `run()` function that accepts the bootstrap result and calls
`run_server()` with a factory closure. The server is constructed **inside** the closure —
this is the canonical pattern used by all hKask MCP servers (see `hkask-mcp-codegraph`):

```rust
use hkask_mcp::{DaemonClient, McpError, run_server};

pub async fn run(
    replicant: String,
    daemon_client: Option<DaemonClient>,
) -> Result<(), McpError> {
    let db_path = std::env::var("EXAMPLE_DB").ok();
    run_server(
        "hkask-mcp-example",
        env!("CARGO_PKG_VERSION"),
        |_ctx| {
            // Build the server inside the closure.
            // The WebID is created fresh here; the closure may also
            // inspect _ctx for credential resolution if needed.
            let webid = hkask_types::WebID::new();
            let server = ExampleServer::new(
                webid,
                replicant.clone(),
                daemon_client.clone(),
                Some(inference_port()?),
                std::collections::HashMap::new(),
            );
            Ok(server)
        },
        vec![], // credentials (see CredentialRequirement)
    ).await
}
```

Key points:

- The factory closure receives a `ServerContext` and returns `Result<S, McpError>`.
- Any configuration read from environment variables (e.g., `HKASK_CODEGRAPH_DB`) is captured
  **outside** the closure and moved in, so it is read once at startup.
- `replicant` and `daemon_client` are cloned into the closure — they are `String` /
  `Option<DaemonClient>` (both `Clone`).

Two variants are available:

- **`run_server()`** — standard stdio server with MCP negotiation
- **`run_server_with_preloaded()`** — passes preloaded env vars (used by servers that
  need `HKASK_CURATOR_REPLICANT` or other secrets before MCP handshake)

---

## Step 5: Write the Binary Entry Point (`main.rs`)

The binary entry point calls `bootstrap_mcp_server()` then `run()`:

```rust
// mcp-servers/<your-mcp-package>/src/main.rs

#[tokio::main]
async fn main() -> Result<(), hkask_mcp::McpError> {
    // 1. Load .env, verify P4 startup gates, get replicant identity
    let bootstrap = hkask_mcp::bootstrap_mcp_server(
        "example",
        "hkask.mcp.example",
        "HKASK_MCP_HOST",
    ).await;

    // 2. Pass bootstrap result to the server's run()
    hkask_mcp_example::run(
        bootstrap.replicant,
        bootstrap.daemon_client,
    ).await
}
```

### What `bootstrap_mcp_server` Does

1. **Loads `.env`** — calls `dotenvy::dotenv().ok()`
2. **Reads replicant identity** — from the env var you specify (default: `HKASK_MCP_HOST`)
3. **Verifies P4 startup gates** — calls `verify_startup_gates()` against the daemon:
   - Gate 1: Authentication (`auth_query`)
   - Gate 2: Role assignment (`assignment_query`)
   - Gate 3: Per-tool capability check (`capability_query`)
4. **Falls back to direct mode** — if the daemon is unavailable, warns and returns
   `daemon_client: None` (standalone operation)

### Startup Gate Behaviour

| Gate | Failure | Result |
|------|---------|--------|
| Gate 1 (auth) | Replicant not authenticated | `McpError::Auth` — server fails to start |
| Gate 2 (assignment) | Replicant not assigned to role | `McpError::RoleAssignment` — server fails to start |
| Gate 3 (capability) | Some tools denied | Non-fatal — server starts, denied tools are unavailable |
| Daemon unavailable | Cannot reach daemon socket | Falls back to direct mode (`daemon_client: None`) |

Gate 3 capability denials are **non-fatal** — the server starts in degraded mode. This matches
the OCAP principle that tools are individually gated, not the whole server.

---

## Step 6: Register in BUILTIN_SERVERS

Add your server to the canonical registry in `crates/hkask-mcp/src/lib.rs`:

```rust
pub const BUILTIN_SERVERS: &[(&str, &str)] = &[
    ("memory", "hkask-mcp-memory"),
    ("condenser", "hkask-mcp-condenser"),
    ("research", "hkask-mcp-research"),
    // ... existing entries ...
    ("example", "<your-mcp-package>"),   // ← add this line
];
```

The tuple is `(server_id, binary_name)`. The `server_id` is used by:
- `kask pod create --mcp example`
- `kask mcp start example`
- `McpRuntime::start_server("example")`

---

## Testing the Server

### Manual Test (Stdio)

```bash
# Build the binary
cargo build -p <your-mcp-package>

# Run it (accepts JSON-RPC on stdin/stdout)
HKASK_MCP_HOST=test-replicant cargo run -p <your-mcp-package>

# In another terminal, send an MCP request
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | \
  HKASK_MCP_HOST=test-replicant cargo run -p <your-mcp-package>
```

### Integration Test

Create `mcp-servers/<your-mcp-package>/tests/tools.rs`:

```rust
use hkask_mcp::McpTool;
use hkask_mcp::server::ToolContext;

#[tokio::test]
async fn example_ping_returns_ok() {
    let server = hkask_mcp_example::ExampleServer::new(
        hkask_types::WebID::new(),
        "test-replicant".into(),
        None,
        None,
        std::collections::HashMap::new(),
    );

    let result = server.example_ping().await;
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["status"], "ok");
}
```

### Daemon Mode Test

Start the hKask daemon first, then test the server with P4 gate verification:

```bash
# Terminal 1: start daemon
kask daemon start

# Terminal 2: run server with daemon
HKASK_MCP_HOST=alice cargo run -p <your-mcp-package>
# Should see: "P4 gates verified" or "Daemon unavailable — falling back to direct mode"
```

---

## Common Pitfalls

### 1. Missing `#[tool]` Attribute

**Symptom:** `rmcp` fails with "tool not found".

**Fix:** Every public async method that should be an MCP tool must have `#[tool(description = "...")]`.

### 2. Using `impl_tool_context!` Manually

**Symptom:** Compile error about missing `ToolContext` impl.

**Fix:** `mcp_server!` already calls `impl_tool_context!` for you. Don't duplicate it.

### 3. Forgetting `execute_tool` Wrapper

**Symptom:** No CNS spans emitted for tool invocations.

**Fix:** Always wrap tool logic in `execute_tool(self, "tool_name", async { ... }).await`. This
ensures CNS span emission and error canonicalization.

### 4. Wrong `host_env_var`

**Symptom:** Server starts as `"anonymous"` instead of the actual replicant.

**Fix:** Most servers use `"HKASK_MCP_HOST"`. The curator uses `"HKASK_CURATOR_REPLICANT"`.
Check the existing servers for the correct convention.

### 5. Not Adding to BUILTIN_SERVERS

**Symptom:** `kask pod create --mcp example` says "unknown server".

**Fix:** Add your `(server_id, binary_name)` to `BUILTIN_SERVERS` in
`crates/hkask-mcp/src/lib.rs`.

### 6. Daemon Socket Permission

**Symptom:** "Daemon unavailable — falling back to direct mode" every time.

**Fix:** The daemon Unix socket is at `~/.hkask/daemon.sock`. Ensure the daemon is running
(`kask daemon start`) and the user has read/write permission on the socket.

### 7. Tool Name Conflicts

**Symptom:** `McpRuntime` reports duplicate tool names across servers.

**Fix:** Tool names are global across all MCP servers. Use a prefix convention
(e.g., `example_ping`, `example_store`).

### 8. Replicant Identity Not Set

**Symptom:** Server starts but all capability checks fail.

**Fix:** Ensure `HKASK_MCP_HOST` (or your `host_env_var`) is set and the replicant is
registered with the daemon (`kask pod create` registers the replicant).
