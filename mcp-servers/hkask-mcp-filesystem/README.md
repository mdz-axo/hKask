# hkask-mcp-filesystem

Filesystem and shell access MCP server — OCAP-governed agent tools.

## Architecture

| Component | Description |
|-----------|-------------|
| `FileSystemServer` | Server struct with WebID, replicant identity, and project root |
| Path sandbox | All file I/O canonicalized and verified against `project_root` |
| CNS spans | `cns.tool.filesystem.*` — `file.read`, `file.written`, `file.deleted`, `command.completed`, `command.failed`, `path.rejected` |

## Tools (7)

| Tool | Description |
|------|-------------|
| `fs_read` | Read file contents with optional line ranges and stats |
| `fs_write` | Create or overwrite a file. Creates parent directories if needed |
| `fs_edit` | Apply targeted text replacements to a file |
| `fs_list` | List directory contents (name, path, type, size) |
| `fs_search` | Regex search across files up to configurable depth |
| `fs_delete` | Delete a file or empty directory |
| `shell_exec` | Execute a shell command with timeout and output guard |

## Security

All file I/O is sandboxed to the configured `project_root`. Paths are canonicalized and verified against the root before any read or write. Path traversal (`../`) is rejected at the sandbox boundary.

## Quick Start

```bash
kask mcp start filesystem
```

## CNS Observability

| Span | When |
|------|------|
| `cns.tool.filesystem.file.read` | Any read operation (fs_read, fs_list, fs_search) |
| `cns.tool.filesystem.file.written` | File write or edit |
| `cns.tool.filesystem.file.deleted` | File deletion |
| `cns.tool.filesystem.command.completed` | Shell command exit code 0 |
| `cns.tool.filesystem.command.failed` | Shell command non-zero exit or timeout |
| `cns.tool.filesystem.path.rejected` | Path traversal attempt blocked |
