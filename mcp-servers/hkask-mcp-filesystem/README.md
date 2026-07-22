# hkask-mcp-filesystem

Filesystem and shell access MCP server — OCAP-governed agent tools.

## Architecture

| Component | Description |
|-----------|-------------|
| `FileSystemServer` | Server struct with WebID, userpod identity, and project root |
| Path sandbox | All file I/O canonicalized and verified against `project_root` |
| Regulation spans | `reg.tool.filesystem.*` — `file.read`, `file.written`, `file.deleted`, `command.completed`, `command.failed`, `path.rejected` |

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

## Regulation Observability

| Span | When |
|------|------|
| `reg.tool.filesystem.file.read` | Any read operation (fs_read, fs_list, fs_search) |
| `reg.tool.filesystem.file.written` | File write or edit |
| `reg.tool.filesystem.file.deleted` | File deletion |
| `reg.tool.filesystem.command.completed` | Shell command exit code 0 |
| `reg.tool.filesystem.command.failed` | Shell command non-zero exit or timeout |
| `reg.tool.filesystem.path.rejected` | Path traversal attempt blocked |
