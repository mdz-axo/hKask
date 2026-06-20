# hkask-mcp-replica

Style replica MCP server — embed author corpora, compose prose, and manage style registries.

## Tools (9)

| Tool | Description |
|------|-------------|
| `replica_build` | Build a style replica from corpus |
| `replica_compose` | Compose prose in a style |
| `replica_compare` | Compare two styles |
| `replica_discover` | Discover styles from text |
| `replica_explain` | Explain a style's characteristics |
| `replica_mashup` | Mashup multiple styles |
| `replica_registry` | List registered styles |
| `replica_cache_work` | Cache work for later use |
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
hkask-mcp-replica
```

## Usage

```
"Build a style replica from Hemingway's works"   → replica_build
"Compose a paragraph in Hemingway's style"        → replica_compose
"Compare Orwell vs Huxley"                        → replica_compare
```
