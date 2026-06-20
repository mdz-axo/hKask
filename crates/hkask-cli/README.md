# hkask-cli

CLI commands for hKask — the primary user interface.

39 subcommand groups + interactive REPL with `/model` slash commands.

## Key Subcommands

| Group | Commands |
|-------|----------|
| `chat` | Interactive Curator chat |
| `agent`, `bot`, `pod`, `replicant` | Agent lifecycle |
| `template`, `skill`, `bundle` | Composition |
| `mcp` | MCP server management |
| `cns`, `loops` | CNS observability |
| `sovereignty` | Magna Carta enforcement |
| `backup`, `git` | Storage and backup |
| `spec`, `docs` | Specification and docs |
| `kata` | Toyota Kata coaching |
| `qa` | Fuzz triage and mutation analysis |
| `daemon`, `serve`, `matrix` | Server modes |

## Configuration

| Variable | Description |
|----------|-------------|
| `HKASK_DB_PATH` | SQLite database path |
| `HKASK_DB_PASSPHRASE` | Database encryption passphrase |
