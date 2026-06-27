# hkask-fal

Fal.ai API client library for hKask — workflow execution, model dispatch, response parsing.

## Architecture

```
hkask-fal (library crate)
├── FalClient          — HTTP client wrapping Fal's REST API
├── WorkflowNode       — DAG node types (Input, Run, Display)
├── WorkflowResult     — Aggregated output from workflow execution
├── execute_workflow() — Parse DAG → topological sort → execute nodes → collect URLs
├── execute_node()     — Call a single Fal model endpoint
└── Reference resolver — $node.field.path → concrete value
```

This crate is consumed by `hkask-mcp-fal` (the MCP server), and potentially
by other crates that need direct Fal API access. It is **not** an MCP server
itself — it has no tools, no daemon integration, no CNS instrumentation.

## Workflow Model

A workflow is a flat JSON object where each key is a node ID. Three node types:

| Type | JSON tag | Purpose |
|------|----------|---------|
| `Input` | `"input"` | Declares caller-provided values. Available to all other nodes. |
| `Run` | `"run"` | Calls a Fal model. `app` field specifies model ID. |
| `Display` | `"output"` | Collects final results. `fields` use `$references`. |

Data flows between nodes via `$node_id.field.path` references in `Run.input`
and `Display.fields`. The executor resolves these during execution.

## Execution Model

1. **Parse** — Deserialize the flat JSON into `Vec<WorkflowNode>`
2. **Validate** — Ensure at least one Input, one Run, and one Display node exist
3. **Topological sort** — Kahn's algorithm orders nodes by dependency
4. **Execute** — Process nodes in order, resolving `$references` from prior results
5. **Collect** — Extract image URLs from the Display node's resolved fields

## API

```rust
use hkask_fal::FalClient;

let client = FalClient::new("fal-key-xxx".into());

// Execute a workflow plan
let result = client.execute_workflow(&workflow_json).await?;
println!("Output URLs: {:?}", result.output_urls);

// Call a single model
let images = client.execute_node("fal-ai/flux/dev", &input).await?;
```

## Error Types

| Variant | When |
|---------|------|
| `Http` | Network-level failure (DNS, TLS, connection) |
| `Api { status, message }` | Fal API returned non-2xx |
| `InvalidWorkflow` | Malformed JSON, unknown node references |
| `UnresolvedReference` | `$reference` points to undeclared or missing data |
| `CircularDependency` | Dependency cycle detected |
| `MissingRequiredNodes` | Workflow lacks run or output nodes |
| `Json` | Serialization/deserialization failure |

## Tests

```bash
cargo test -p hkask-fal
```

Tests cover: linear DAG sorting, parallel branch sorting, cycle detection,
reference resolution, URL extraction, and structure validation.
