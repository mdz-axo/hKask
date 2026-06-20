# hkask-mcp-spec

Specification authoring, curation, and validation MCP server.

## Tools (13)

| Tool | Description |
|------|-------------|
| `spec_goal_capture` | Capture specification goal |
| `spec_goal_decompose` | Decompose goal into sub-goals |
| `spec_require_writing_quality` | Check writing quality |
| `spec_graph_query` | Query specification graph |
| `spec_graph_coherence` | Check graph coherence |
| `spec_replica_rewrite` | Rewrite spec in replica style |
| `contract_propose` | Propose a contract |
| `contract_accept` | Accept a contract |
| `contract_reject` | Reject a contract |
| `contract_audit` | Audit contracts |
| `contract_list` | List contracts |
| `test_run` | Run specification tests |
| `run` | Main run loop |

## Configuration

| Variable | Description |
|----------|-------------|
| `HKASK_DB_PATH` | SQLite database path |
| `HKASK_DB_PASSPHRASE` | Database encryption passphrase |

## Quick Start

```bash
# The server starts automatically with kask
kask chat
# Or standalone:
hkask-mcp-spec
```
