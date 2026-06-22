# hkask-mcp-skill

Skill invocation MCP server — exposes registered skills as callable tools.

Part of hKask's Inference loop (L1). Skills are Jinja2 templates (WordAct / KnowAct / FlowDef) registered in the template registry. This server loads them at startup and makes each available as an invocable endpoint.

## Tools (3)

| Tool | Description |
|------|-------------|
| `skill_ping` | Liveness check, version info, skill count |
| `skill_list` | List all available skill IDs with descriptions |
| `skill_execute` | Render a skill template with context, run inference, return result |

## Usage

The server is registered as `("skill", "hkask-mcp-skill")` in the REPL's built-in server list. Start it with:

```
/mcp start skill
```

Skills are loaded from the bootstrapped template registry at server startup. Each skill's Jinja2 template is read from disk. Skills with unreadable template files are silently skipped.

## Tool: skill_execute

```
{
  "skill_id": "coding-guidelines.guidelines-assess",
  "context": {
    "feature": "authentication module",
    "language": "rust"
  }
}
```

1. Looks up `skill_id` in the loaded skill index
2. Renders the Jinja2 template with `context` variables
3. Runs inference via the centralized `InferenceRouter`
4. Returns the model's response

Parameters use the `Parameters<SkillExecuteRequest>` pattern (rmcp struct extraction).

## Dependencies

- `hkask-templates` — template registry bootstrap and `RegistryIndex`
- `hkask-inference` — centralized inference router
- `hkask-mcp` — MCP server framework (`run_server`, `ToolSpanGuard`, `CapabilityTier`)
- `minijinja` — Jinja2 template rendering
- `rmcp` — MCP protocol (server handler, tool macros)
