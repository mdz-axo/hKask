# Web MCP Manifests

## Status: Draft

These manifests define target orchestration patterns for the `hkask-templates` cascade execution engine.

## Variable Binding Syntax

The `${stepN.results[0].url}` variable binding syntax used in these manifests is the **target execution model** for the process engine. It is not currently executed by any runtime.

Each manifest's `steps` define a sequence of MCP tool calls with:
- `tool`: the MCP tool to invoke
- `arguments`: parameters for the tool (may include template variables)
- `output_var`: variable name to store results
- `loop_over`: optional iteration over step results
- `condition`: optional guard expression

## Version Gate

The MCP server version that removes `web_search_and_extract` must not ship until the `hkask-templates` cascade engine can execute `web-deep-research.yaml`.

When the cascade engine supports step-by-step execution with variable binding resolution, each manifest's `status` field should be changed from `draft` to `active`.

## Manifests

| Manifest | Purpose | Key Steps |
|----------|---------|-----------|
| `web-quick.yaml` | Single-provider keyword search | search → raw |
| `web-news-timeline.yaml` | News-oriented search with timeline grouping | search → timeline |
| `web-synthesis.yaml` | Broad web search with synthesis across providers | search → synthesis |
| `web-deep-research.yaml` | Multi-step deep research | search → find_similar → extract loop → synthesize |

## Dependency

- **Process executor**: Lives in `hkask-templates` (or a new `hkask-process` crate)
- **CNS integration**: The executor must emit `cns.*` spans for each step
- **Energy budget**: Each manifest should declare a `max_steps` and `timeout` to enforce bounded execution

---

*This directory is part of hKask v0.21.0 — see `docs/architecture/hKask-architecture-master.md` for the authoritative spec.*