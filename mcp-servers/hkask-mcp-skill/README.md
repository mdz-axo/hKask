# hkask-mcp-skill — Skill Registry MCP Server

MCP server exposing the skill registry: listing, installing, discovering, and managing skills and their manifests.

**Version:** v0.31.0 | **Crate:** `hkask-mcp-skill`

## Tools (3)

| Tool | Description |
|------|-------------|
| `skill_health` | Liveness and profile info |
| `skill_list` | List available skill IDs with their descriptions |
| `skill_install` | Install a skill from the registry |

## Configuration

No environment variables required. Uses the hKask skill registry (`registry/`) for discovery and installation.

## Dependencies

- `hkask-mcp` — MCP runtime and dispatch
- `hkask-services-skill` — Skill service layer
- `hkask-templates` — Template registry and resolver
