# hKask CLI Documentation

**hKask** (ℏKask — "Planck's Constant of Agent Systems") - Command-line interface

## Usage

```bash
kask [OPTIONS] <COMMAND>
```

## Options

- `-v`, `--verbose` — Enable verbose output
- `-r`, `--registry <PATH>` — Registry database path (default: in-memory)
- `-h`, `--help` — Print help
- `-V`, `--version` — Print version

## Commands

### `kask chat` — Curator chat interface

```bash
kask chat [OPTIONS]
```

Options:
- `-t`, `--template <TEMPLATE>` — Optional: template ID to use
- `-f`, `--input <INPUT>` — Optional: input file
- `-i`, `--interactive` — Interactive mode

### `kask template` — Template management

```bash
kask template <SUBCOMMAND>
```

Subcommands:
- `list` — List all registered templates
  - `-t`, `--type <TYPE>` — Filter by template type
- `register` — Register a new template
  - `-i`, `--id <ID>` — Template ID (e.g., "prompt/selector")
  - `-p`, `--path <PATH>` — Template file path
  - `-t`, `--type <TYPE>` — Template type (prompt, cognition, process)
  - `-l`, `--lexicon <LEXICON>` — Lexicon terms (comma-separated)
  - `-d`, `--description <DESC>` — Description
- `get <ID>` — Get template details
- `search <TERM>` — Search templates by lexicon term

### `kask bot` — Bot capability management

```bash
kask bot <SUBCOMMAND>
```

Subcommands:
- `list` — List bot capabilities
  - `-b`, `--bot-id <BOT_ID>` — Bot WebID
- `grant` — Grant capability to bot
  - `-b`, `--bot-id <BOT_ID>` — Bot WebID
  - `-c`, `--capability <CAPABILITY>` — Capability name (e.g., "inference:call")

### `kask pod` — Agent pod management

```bash
kask pod <SUBCOMMAND>
```

Subcommands:
- `create` — Create agent pod from template crate
  - `-t`, `--template <TEMPLATE>` — Template crate name
  - `-p`, `--persona <PERSONA>` — Agent persona YAML file path
  - `-n`, `--name <NAME>` — Pod name (optional, defaults to UUID)
- `activate <POD_ID>` — Activate agent pod for A2A communication
- `deactivate <POD_ID>` — Deactivate agent pod
- `status <POD_ID>` — Show agent pod status
  - `-v`, `--verbose` — Show verbose details
- `list` — List all agent pods

### `kask mcp` — MCP server/tool management

```bash
kask mcp <SUBCOMMAND>
```

Subcommands:
- `list-servers` — List MCP servers
- `list-tools` — List available tools
- `get-tool <NAME>` — Get tool definition

### `kask cns` — CNS monitoring

```bash
kask cns <SUBCOMMAND>
```

Subcommands:
- `health` — Get CNS health status
- `alerts` — Get algedonic alerts
- `variety` — Get variety counters

### `kask docs` — Documentation generation

```bash
kask docs <SUBCOMMAND>
```

Subcommands:
- `openapi` — Generate OpenAPI specification (JSON)
  - `-o`, `--output <OUTPUT>` — Output file path (default: stdout)
- `cli` — Generate CLI help documentation (markdown)
  - `-o`, `--output <OUTPUT>` — Output file path (default: stdout)
- `all` — Generate all documentation
  - `-o`, `--output <OUTPUT>` — Output directory

## Examples

```bash
# Start interactive chat session
kask chat --interactive

# List all templates
kask template list

# Register a new template
kask template register -i prompt/selector -p templates/selector.j2 -t prompt -l "select,route,dispatch"

# Generate OpenAPI spec
kask docs openapi -o docs/openapi.json

# Generate all documentation
kask docs all -o docs/
```

## Template Types

- `prompt` — Prompt templates for LLM interaction
- `cognition` — Cognitive processing templates
- `process` — Process execution templates

---

*hKask v0.1.0 — Planck's Constant of Agent Systems*
