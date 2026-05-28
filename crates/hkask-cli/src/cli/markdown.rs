//! CLI markdown documentation generator

/// Generate the CLI help documentation as markdown
pub fn generate_cli_markdown() -> String {
    let mut md = String::new();

    md.push_str("# hKask CLI Documentation\n\n");
    md.push_str(
        "**hKask** (ℏKask — \"Planck's Constant of Agent Systems\") - Command-line interface\n\n",
    );
    md.push_str("## Usage\n\n");
    md.push_str("```bash\n");
    md.push_str("kask [OPTIONS] <COMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("## Options\n\n");
    md.push_str("- `-v`, `--verbose` — Enable verbose output\n");
    md.push_str("- `-r`, `--registry <PATH>` — Registry database path (default: in-memory)\n");
    md.push_str("- `-h`, `--help` — Print help\n");
    md.push_str("- `-V`, `--version` — Print version\n\n");
    md.push_str("## Commands\n\n");
    md.push_str("### `kask chat` — Interactive agent chat\n\n");
    md.push_str("```bash\n");
    md.push_str("kask chat [AGENT]       # Default: Curator\n");
    md.push_str("kask chat russell       # Chat with Russell\n");
    md.push_str("kask chat -f input.txt  # Non-interactive (file input)\n");
    md.push_str("```\n\n");
    md.push_str("Arguments:\n");
    md.push_str("- `[AGENT]` — Agent to chat with (default: Curator)\n\n");
    md.push_str("Options:\n");
    md.push_str("- `-t`, `--template <TEMPLATE>` — Template ID to use\n");
    md.push_str("- `-f`, `--input <INPUT>` — Input file (non-interactive mode)\n\n");
    md.push_str("Slash commands (inside chat):\n");
    md.push_str("- `/help` — Show categorized help, `/help <cmd>` for details\n");
    md.push_str("- `/status` — System status (CNS, agent, pods)\n");
    md.push_str("- `/agent [NAME]` — Show or switch agent\n");
    md.push_str("- `/agents` — List registered agents\n");
    md.push_str("- `/pods` — List agent pods\n");
    md.push_str("- `/templates` — List registered templates\n");
    md.push_str("- `/ensemble` — Multi-agent ensemble (sessions, create, join, send)\n");
    md.push_str("- `/escalations` — List pending escalations\n");
    md.push_str("- `/metacognition` — Run metacognition cycle\n");
    md.push_str("- `/sovereignty` — Show sovereignty status\n");
    md.push_str("- `/history` — Show session turn history\n");
    md.push_str("- `/quit` — End session\n\n");
    md.push_str("### `kask template` — Template management\n\n");
    md.push_str("```bash\n");
    md.push_str("kask template <SUBCOMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("Subcommands:\n");
    md.push_str("- `list` — List all registered templates\n");
    md.push_str("  - `-t`, `--type <TYPE>` — Filter by template type\n");
    md.push_str("- `register` — Register a new template\n");
    md.push_str("  - `-i`, `--id <ID>` — Template ID (e.g., \"prompt/selector\")\n");
    md.push_str("  - `-p`, `--path <PATH>` — Template file path\n");
    md.push_str("  - `-t`, `--type <TYPE>` — Template type (prompt, cognition, process)\n");
    md.push_str("  - `-l`, `--lexicon <LEXICON>` — Lexicon terms (comma-separated)\n");
    md.push_str("  - `-d`, `--description <DESC>` — Description\n");
    md.push_str("- `get <ID>` — Get template details\n");
    md.push_str("- `search <TERM>` — Search templates by lexicon term\n\n");
    md.push_str("### `kask bot` — Bot capability management\n\n");
    md.push_str("```bash\n");
    md.push_str("kask bot <SUBCOMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("Subcommands:\n");
    md.push_str("- `list` — List bot capabilities\n");
    md.push_str("  - `-b`, `--bot-id <BOT_ID>` — Bot WebID\n");
    md.push_str("- `grant` — Grant capability to bot\n");
    md.push_str("  - `-b`, `--bot-id <BOT_ID>` — Bot WebID\n");
    md.push_str(
        "  - `-c`, `--capability <CAPABILITY>` — Capability name (e.g., \"inference:call\")\n\n",
    );
    md.push_str("### `kask pod` — Agent pod management\n\n");
    md.push_str("```bash\n");
    md.push_str("kask pod <SUBCOMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("Subcommands:\n");
    md.push_str("- `create` — Create agent pod from template crate\n");
    md.push_str("  - `-t`, `--template <TEMPLATE>` — Template crate name\n");
    md.push_str("  - `-p`, `--persona <PERSONA>` — Agent persona YAML file path\n");
    md.push_str("  - `-n`, `--name <NAME>` — Pod name (optional, defaults to UUID)\n");
    md.push_str("- `activate <POD_ID>` — Activate agent pod for A2A communication\n");
    md.push_str("- `deactivate <POD_ID>` — Deactivate agent pod\n");
    md.push_str("- `status <POD_ID>` — Show agent pod status\n");
    md.push_str("  - `-v`, `--verbose` — Show verbose details\n");
    md.push_str("- `list` — List all agent pods\n\n");
    md.push_str("### `kask mcp` — MCP server/tool management\n\n");
    md.push_str("```bash\n");
    md.push_str("kask mcp <SUBCOMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("Subcommands:\n");
    md.push_str("- `list-servers` — List MCP servers\n");
    md.push_str("- `list-tools` — List available tools\n");
    md.push_str("- `get-tool <NAME>` — Get tool definition\n\n");
    md.push_str("### `kask cns` — CNS monitoring\n\n");
    md.push_str("```bash\n");
    md.push_str("kask cns <SUBCOMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("Subcommands:\n");
    md.push_str("- `health` — Get CNS health status\n");
    md.push_str("- `alerts` — Get algedonic alerts\n");
    md.push_str("- `variety` — Get variety counters\n\n");
    md.push_str("### `kask docs` — Documentation generation\n\n");
    md.push_str("```bash\n");
    md.push_str("kask docs <SUBCOMMAND>\n");
    md.push_str("```\n\n");
    md.push_str("Subcommands:\n");
    md.push_str("- `openapi` — Generate OpenAPI specification (JSON)\n");
    md.push_str("  - `-o`, `--output <OUTPUT>` — Output file path (default: stdout)\n");
    md.push_str("- `cli` — Generate CLI help documentation (markdown)\n");
    md.push_str("  - `-o`, `--output <OUTPUT>` — Output file path (default: stdout)\n");
    md.push_str("- `all` — Generate all documentation\n");
    md.push_str("  - `-o`, `--output <OUTPUT>` — Output directory\n\n");
    md.push_str("## Examples\n\n");
    md.push_str("```bash\n");
    md.push_str("# Start chat session\n");
    md.push_str("kask chat\n\n");
    md.push_str("# Chat with a specific agent\n");
    md.push_str("kask chat Russell\n\n");
    md.push_str("# List all templates\n");
    md.push_str("kask template list\n\n");
    md.push_str("# Register a new template\n");
    md.push_str("kask template register -i prompt/selector -p templates/selector.j2 -t prompt -l \"select,route,dispatch\"\n\n");
    md.push_str("# Generate OpenAPI spec\n");
    md.push_str("kask docs openapi -o docs/openapi.json\n\n");
    md.push_str("# Generate all documentation\n");
    md.push_str("kask docs all -o docs/\n");
    md.push_str("```\n\n");
    md.push_str("## Template Types\n\n");
    md.push_str("- `prompt` — Prompt templates for LLM interaction\n");
    md.push_str("- `cognition` — Cognitive processing templates\n");
    md.push_str("- `process` — Process execution templates\n\n");
    md.push_str("---\n\n");
    md.push_str(&format!(
        "*hKask v{} — Planck's Constant of Agent Systems*\n",
        env!("CARGO_PKG_VERSION")
    ));

    md
}
