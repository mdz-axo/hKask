# hkask-mcp-fal

Fal workflow execution MCP server — exposes `execute_workflow` tool for
Strategy D PDCA loops.

## Architecture

```
┌──────────────────────────────────────────────────┐
│ hkask-mcp-fal (MCP server binary)                │
│                                                   │
│ FalServer                                         │
│   ├── webid, replicant, daemon (standard MCP)     │
│   └── fal: Arc<FalClient>  ──────┐               │
│                                   │               │
│ Tool: execute_workflow              │               │
│   ├── Parse workflow JSON string  │               │
│   └── Delegate ──────────────────►│ hkask-fal     │
│                                   │ (library)     │
│                                   │               │
│ CNS: auto-mapped to               │ FalClient     │
│   ToolSubsystem::Media            │   .execute_   │
│   → cns.tool.media                │    workflow() │
└───────────────────────────────────┴───────────────┘
```

The server is a thin wrapper around the `hkask-fal` library crate. It handles
MCP protocol concerns (daemon flow, tool registration, CNS instrumentation)
while the library handles the actual Fal API interaction.

## Tools

| Tool | Description |
|------|-------------|
| `execute_workflow` | Execute a workflow plan JSON string. Parses the DAG, topologically sorts nodes by dependency order, resolves `$references` between nodes, calls each Fal model sequentially, and returns output URLs with metadata. |

### Tool Input

```json
{
  "workflow": "{\"input\":{...},\"generate\":{...},\"output\":{...}}"
}
```

The `workflow` parameter is a JSON **string** (not an object) to avoid
`JsonSchema` limitations with `serde_json::Value`.

### Tool Output

```json
{
  "output_urls": ["https://v3.fal.media/files/abc.png"],
  "output_fields": { "image": "https://..." },
  "elapsed_seconds": 12.5
}
```

## Quick Start

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `FAL_KEY` | Recommended | Fal.ai API key for GPU model execution |
| `HKASK_FAL_KEY` | Optional | Alternative credential name (MCP server context) |
| `HKASK_REPLICANT` | Optional | Replicant identity (defaults to "anonymous") |

### Standalone

```bash
FAL_KEY="your-key" hkask-mcp-fal
```

### With kask

The server is auto-discovered by both `kask serve` and `kask chat`:

```bash
# Server mode — auto-started alongside other MCP servers
kask serve

# REPL mode — available via /mcp command (P2 consent gated)
kask chat
/mcp start fal-workflow
```

## CNS Observability

CNS spans are emitted automatically via the `hkask_mcp::execute_tool` wrapper.
The server maps to `ToolSubsystem::Media` → `cns.tool.media` spans.

The library crate (`hkask-fal`) emits `tracing::debug!` events under
`target: "hkask.fal"` for workflow start, node execution, and completion.
These are development-level diagnostics, not CNS spans.
