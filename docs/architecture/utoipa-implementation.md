# utoipa Implementation — hKask API & CLI Documentation

## Summary

utoipa has been implemented for automatic OpenAPI specification generation and CLI documentation.

## Code Budget Impact

- **hkask-api**: 481 lines (includes utoipa annotations and documentation endpoints)
- **hkask-cli**: 1,112 lines (includes documentation generation commands)
- **Total core crates**: ~22,251 lines (well within 30,000 line budget)
- **Remaining budget**: ~7,749 lines

## Implementation Details

### hkask-api

**Dependencies added** (workspace level, already present):
```toml
utoipa = { version = "5.5", features = ["axum_extras", "uuid", "chrono"] }
utoipa-axum = "0.2"
```

**Files modified**:
- `crates/hkask-api/src/lib.rs` — OpenAPI router creation, response types with `ToSchema`
- `crates/hkask-api/src/openapi.rs` — OpenAPI specification with tags and metadata
- `crates/hkask-api/src/routes.rs` — Path operations with utoipa annotations

**Schema types** (all derive `ToSchema`):
- `TemplateResponse` — Template information
- `GrantCapabilityRequest` — Bot capability grant request
- `CnsHealthResponse` — CNS health status
- `CnsVarietyResponse` — CNS variety counters
- `ToolResponse` — MCP tool definition
- `ErrorResponse` — Error responses
- `ChatRequest` / `ChatResponse` — Chat endpoints

**API Endpoints documented**:
| Method | Path | Tag | Description |
|--------|------|-----|-------------|
| GET | `/api/templates` | templates | List templates |
| GET | `/api/templates/{id}` | templates | Get template by ID |
| POST | `/api/templates` | templates | Register template |
| GET | `/api/bots/{id}/capabilities` | bots | List bot capabilities |
| POST | `/api/bots/{id}/grant` | bots | Grant capability |
| GET | `/api/mcp/servers` | mcp | List MCP servers |
| GET | `/api/mcp/tools` | mcp | List tools |
| GET | `/api/cns/health` | cns | CNS health status |
| GET | `/api/cns/alerts` | cns | Algedonic alerts |
| GET | `/api/cns/variety` | cns | Variety counters |
| POST | `/api/chat` | chat | Curator chat |

### hkask-cli

**New command**: `kask docs`

**Subcommands**:
- `kask docs openapi [-o OUTPUT]` — Generate OpenAPI specification (JSON)
- `kask docs cli [-o OUTPUT]` — Generate CLI documentation (Markdown)
- `kask docs all -o OUTPUT` — Generate all documentation to directory

**Usage examples**:
```bash
# Generate OpenAPI spec to stdout
kask docs openapi

# Save OpenAPI spec to file
kask docs openapi -o docs/openapi.json

# Generate CLI documentation
kask docs cli -o docs/cli.md

# Generate all documentation
kask docs all -o docs/
```

## OpenAPI Specification

The generated OpenAPI spec includes:
- Complete API documentation with request/response schemas
- Tagged endpoints for organization
- Server configuration (`/api`)
- Component schemas for all types

## CLI Documentation

The generated Markdown documentation includes:
- Command overview and options
- All subcommands with parameters
- Usage examples
- Template type reference

## Future Enhancements

1. **Swagger UI integration** — Serve interactive API documentation at `/api/docs`
2. **ReDoc integration** — Alternative API documentation UI
3. **CLI completion generation** — Use clap completions for shell autocompletion
4. **API endpoint tests** — Integration tests using generated OpenAPI spec

## Verification

To verify the implementation:

```bash
# Check hkask-api compilation
cargo check -p hkask-api

# Generate OpenAPI spec
cargo run -p hkask-cli -- docs openapi

# Generate CLI documentation
cargo run -p hkask-cli -- docs cli
```

---

*Implementation completed: 2026-05-20*
*hKask v0.1.0 — Planck's Constant of Agent Systems*
