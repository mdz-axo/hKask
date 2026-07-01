# hkask-mcp-skill — Skill Registry MCP Server

MCP server exposing the skill registry: listing, installing, discovering, and managing skills and their manifests.

**Version:** v0.31.0 | **Crate:** `hkask-mcp-skill`

## Tools (3)

| Tool | Description |
|------|-------------|
| `skill_ping` | Liveness and profile info |
| `skill_list` | List available skill IDs with their descriptions |
| `skill_execute` | Execute a registered skill template with context variables. Renders the skill as a Jinja2 template and runs inference. Use `skill_list` first to discover available skill IDs. |

## Configuration

No environment variables required. Uses the hKask skill registry (`registry/`) for discovery and installation.

## Dependencies

- `hkask-mcp` — MCP runtime and dispatch
- `hkask-services-skill` — Skill service layer
- `hkask-templates` — Template registry and resolver
