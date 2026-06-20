# hkask-memory

Semantic and episodic memory pipelines for hKask.

Implements the memory consolidation pipeline (L2 in the loop architecture):
episodic → consolidation → semantic.

## Pipeline

| Stage | Description |
|-------|-------------|
| `episodic` | Record conversation turns, tool outputs, outcomes |
| `consolidation` | Extract patterns, distill insights from episodic |
| `semantic` | Store consolidated knowledge for long-term retrieval |

## Configuration

| Variable | Description |
|----------|-------------|
| `HKASK_DB_PATH` | SQLite database path |
| `HKASK_DB_PASSPHRASE` | Database encryption passphrase |
