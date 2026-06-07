---
title: "utoipa Implementation — API and CLI Documentation"
audience: [API developers, CLI operators, integration engineers]
last_updated: 2026-05-20
version: "1.0.0"
status: "Active"
domain: "Application"
ddmvss_categories: [interface]
---


# utoipa Implementation — API and CLI Documentation

## 1. Statement

utoipa provides automatic OpenAPI specification generation for the hKask HTTP API and Markdown documentation for the CLI. The implementation adds zero runtime overhead — all documentation is generated at build time from type annotations and path macros.[^utoipa-docs]

## 2. Evidence

### 2.1 Code Budget Impact

| Crate | Lines Added | Total Lines | Purpose |
|-------|-------------|-------------|---------|
| `hkask-api` | ~150 | 481 | OpenAPI schemas and path annotations |
| `hkask-cli` | ~200 | 1,112 | Documentation generation commands |
| **Total core crates** | — | 22,251 | —

### 2.2 Dependencies

Workspace-level dependencies (already present in `Cargo.toml`):

```toml
utoipa = { version = "5.5", features = ["axum_extras", "uuid", "chrono"] }
utoipa-axum = "0.2"
```

The `axum_extras` feature enables integration with the axum web framework. The `uuid` and `chrono` features provide schema generation for those types.[^utoipa-crate]

### 2.3 Schema Types

All response and request types derive `ToSchema`:

```rust
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TemplateResponse {
    pub id: String,
    pub template_type: String,
    pub description: String,
    pub source_path: String,
    pub lexicon_terms: Vec<String>,
}
```

**Component schemas:**
- `TemplateResponse` — Template registry entry
- `GrantCapabilityRequest` — Bot capability grant request body
- `CnsHealthResponse` — CNS health status with deficit counters
- `CnsVarietyResponse` — CNS variety counters with entropy
- `ToolResponse` — MCP tool definition
- `ErrorResponse` — Standard error response format
- `ChatRequest` / `ChatResponse` — Curator chat endpoints

### 2.4 API Endpoints

Each endpoint has a `#[utoipa::path]` annotation:

```rust
#[utoipa::path(
    get,
    path = "/api/templates",
    tag = "templates",
    responses(
        (status = 200, description = "List of templates", body = Vec<TemplateResponse>),
        (status = 500, description = "Internal server error"),
    ),
)]
async fn list_templates(State(state): State<ApiState>) -> Json<Vec<TemplateResponse>> {
    // ...
}
```

| Method | Path | Tag | Status Codes |
|--------|------|-----|--------------|
| GET | `/api/templates` | templates | 200, 500 |
| GET | `/api/templates/{id}` | templates | 200, 404, 500 |
| POST | `/api/templates` | templates | 201, 400, 500 |
| GET | `/api/bots/{id}/capabilities` | bots | 200, 500 |
| POST | `/api/bots/{id}/grant` | bots | 200, 400, 500 |
| GET | `/api/mcp/servers` | mcp | 200, 500 |
| GET | `/api/mcp/tools` | mcp | 200, 500 |
| GET | `/api/cns/health` | cns | 200, 500 |
| GET | `/api/cns/alerts` | cns | 200, 500 |
| GET | `/api/cns/variety` | cns | 200, 500 |
| POST | `/api/chat` | chat | 200, 400, 500 |

## 3. Diagram

```mermaid
graph TD
    subgraph "Build Time"
        A[Rust Source Code] -->|derive ToSchema| B[utoipa Macros]
        A -->|path annotations| B
        B --> C[OpenAPI JSON]
        B --> D[CLI Markdown]
    end
    
    subgraph "Runtime"
        E[HTTP Server] -->|serve| F[Swagger UI / ReDoc]
        G[kask CLI] -->|docs command| H[Generated Files]
    end
    
    C --> E
    C --> G
    D --> G
```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-UTOIPA-001
verified_date: 2026-06-07
verified_against: crates/hkask-api/src/openapi.rs:11; crates/hkask-cli/src/main.rs:385
status: VERIFIED
-->

## 4. CLI Documentation Commands

The `kask docs` command provides three subcommands:

### 4.1 OpenAPI Specification

```bash
kask docs openapi [-o OUTPUT]
```

Generates the complete OpenAPI 3.1 specification in JSON format. Output goes to stdout by default, or to a file with `-o`.

### 4.2 CLI Documentation

```bash
kask docs cli [-o OUTPUT]
```

Generates Markdown documentation for all CLI commands, options, and subcommands. The documentation includes usage examples and template type reference.

### 4.3 Complete Documentation

```bash
kask docs all -o OUTPUT_DIR
```

Generates both OpenAPI specification (`openapi.json`) and CLI documentation (`cli.md`) in the specified directory.

## 5. Implications

### 5.1 For API Developers

- **No manual spec maintenance** — OpenAPI spec updates automatically when code changes
- **Type safety** — Schema generation from Rust types prevents documentation drift
- **IDE support** — utoipa annotations provide inline documentation hints

### 5.2 For Integration Engineers

- **Swagger UI ready** — Generated spec works with Swagger UI, ReDoc, and other OpenAPI tools
- **Client generation** — OpenAPI spec can generate client SDKs in multiple languages[^openapi-generators]
- **Contract testing** — Spec serves as the API contract for integration tests

### 5.3 For Operators

- **Single source of truth** — CLI documentation generated from actual command definitions
- **No outdated help text** — Documentation reflects current CLI state
- **Offline access** — Generated Markdown files work without network access

## 6. Verification

```bash
# Verify hkask-api compiles with utoipa annotations
cargo check -p hkask-api

# Generate and inspect OpenAPI spec
cargo run -p hkask-cli -- docs openapi -o docs/openapi.json
jq '.paths | keys' docs/openapi.json

# Generate CLI documentation
cargo run -p hkask-cli -- docs cli -o docs/cli.md

# Generate all documentation
cargo run -p hkask-cli -- docs all -o docs/
```

## 7. Future Enhancements

1. **Swagger UI integration** — Serve interactive API documentation at `/api/docs` endpoint
2. **ReDoc integration** — Alternative documentation UI with better mobile support[^redoc-docs]
3. **CLI shell completions** — Generate bash, zsh, fish completions using clap completions[^clap-completions]
4. **OpenAPI contract tests** — Integration tests that verify endpoints match the generated spec

## References

[^utoipa-docs]: utoipa Contributors. (2026). *utoipa Documentation*. <https://utoipa.dev/>. The primary documentation for the utoipa crate.

[^utoipa-crate]: utoipa Contributors. (2026). *utoipa Crate*. <https://crates.io/crates/utoipa>. The crate specification and feature flags.

[^openapi-generators]: OpenAPI Initiative. (2026). *Code Generators*. <https://github.com/OpenAPI/generator>. List of OpenAPI client and server generators.

[^redoc-docs]: ReDoc Contributors. (2026). *ReDoc Documentation*. <https://redocly.com/docs/redoc/>. Documentation for ReDoc, an open-source API documentation tool.

[^clap-completions]: clap Contributors. (2026). *clap Completions*. <https://docs.rs/clap/latest/clap/_cookbook/_typed_clap_example/index.html#shell-completions>. Documentation for generating shell completions with clap.
