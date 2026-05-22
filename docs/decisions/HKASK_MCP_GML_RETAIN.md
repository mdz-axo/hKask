# hkask-mcp-gml Decision Record

## Status: **RETAINED** (Binary Only)

## Analysis

| Aspect | Finding |
|--------|---------|
| **Implementation** | 1,022 lines — substantial, not a stub |
| **Features** | MWC model, OCAP enforcement, GML computations |
| **Usage** | Standalone MCP server (binary) |
| **Dependencies** | Was referenced by hkask-testing (removed) |
| **Lib Target** | Not needed — no other crates import from it |

## Decision

**Keep as binary-only MCP server.** No lib target required.

### Rationale

1. **Functional implementation** — Not dead code, has real functionality
2. **MCP server pattern** — Other MCP servers in project are also binary-only
3. **No importers** — No other crate needs to link against it
4. **Clean separation** — GML/allosteric thinking is a distinct capability

## Future Considerations

| Scenario | Action |
|----------|--------|
| **hkask-ensemble needs GML** | Extract MWC model to hkask-types or hkask-ensemble |
| **GML library needed** | Create lib.rs with core types, keep main.rs as binary |
| **Unused after 90 days** | Re-evaluate for removal |

## Related

- GML documentation: `docs/gml/README.md`
- MCP server pattern: `docs/architecture/MCP_SERVERS.md`

---
*Decision recorded: 2026-05-22*
*Part of hKask Dependency Governance (Phase 2)*