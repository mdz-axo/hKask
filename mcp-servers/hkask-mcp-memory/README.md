# hkask-mcp-memory

Unified episodic + semantic memory MCP server with cloud backup.

## Tools (17)

| Tool | Description |
|------|-------------|
| `episodic_ping` | Episodic health check |
| `episodic_store` | Store episodic memory |
| `episodic_recall` | Recall episodic memory |
| `episodic_budget` | Check memory budget |
| `episodic_consolidate_status` | Consolidation status |
| `semantic_ping` | Semantic health check |
| `semantic_store` | Store semantic memory |
| `semantic_recall` | Recall semantic memory |
| `semantic_search` | Search semantic memory |
| `semantic_embed` | Generate embeddings |
| `semantic_chunk` | Chunk for semantic storage |
| `semantic_centroid` | Compute semantic centroid |
| `semantic_count` | Count semantic entries |
| `semantic_purge` | Purge semantic memory |
| `memory_backup` | Backup memory to cloud |
| `memory_restore` | Restore memory from cloud |
| `run` | Main run loop |

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `HKASK_DB_PATH` | SQLite database path | In-memory |
| `HKASK_DB_PASSPHRASE` | Database encryption passphrase | Required if DB path set |

## Quick Start

```bash
# In-memory by default (no config needed)
kask chat
# With persistence:
HKASK_DB_PATH=./memory.db HKASK_DB_PASSPHRASE=secret kask chat
```
