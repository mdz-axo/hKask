---
title: "How to Use the REPL — How-To Guide"
audience: [operators, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [domain, lifecycle]
last-verified-against: "3d1a876f"
---

# How to Use the REPL

**Goal:** Use hKask's interactive REPL (`kask chat`) for agent conversations, slash-command dispatch, model switching, skill invocation, and session management.

The REPL is the primary interactive interface for hKask. It provides a readline-based shell with 30+ slash commands, real-time streaming responses, and full access to the CNS, MCP tools, and skill system.

---

## 1. Starting the REPL

```bash
kask chat
```

On first run, hKask runs onboarding to resolve your identity (WebID), master key, and A2A secret. After onboarding, you enter the REPL:

```
ℏKask [Curator]>
```

### Options

| Flag | Effect |
|------|--------|
| `kask chat --tui` | Launch the terminal UI (ratatui-based multi-window interface) |
| `HKASK_TUI=1 kask chat` | Same as `--tui` via environment variable |
| `kask chat --agent <name>` | Start with a specific agent (default: Curator) |
| `kask chat --model <name>` | Start with a specific model |
| `kask chat --template <id>` | Start with a specific template loaded |
| `kask chat --input <file>` | Non-interactive: send file content and exit |

### TUI Mode

The TUI provides a multi-window workspace with:
- Chat window (primary interaction)
- Skills window (`Ctrl+S` — browse, execute, active skills)
- CNS dashboard (alerts, variety counters)
- Pod management (list, inspect, create)

---

## 2. All Slash Commands

Commands start with `/`. Below is the complete command table sorted by category.

### Help & Session

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/help` | `/h`, `/?` | `[COMMAND]` | Show help, or details for a specific command |
| `/quit` | `/q`, `/exit` | | End the session |
| `/clear` | `/cls` | | Clear the screen |
| `/status` | `/st` | | System status (CNS, agent, pod count) |

### Agent & Pod Management

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/agent` | `/a` | `[NAME]` | Switch agent or show current |
| `/agents` | `/ls` | | List registered agents |
| `/pods` | | | List agent pods |

### Model & Inference

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/model` | `/m` | `[NAME\|QUERY]` | Switch model, fuzzy search, or show current |
| `/fusion` | | `[off\|on\|status]` | Show or toggle fusion mode (multi-model deliberation) |
| `/repl` | | `[SETTING] [VALUE]` | Show or set REPL inference settings |

### MCP & Tools

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/tools` | | | List available MCP tools |
| `/mcp` | | `list\|start <server\|all>` | Manage MCP server connections (P2: opt-in) |
| `/invoke` | `/inv` | `<server>/<tool> [args]` | Invoke an MCP tool through GovernedTool |

### Skills & Bundles

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/templates` | `/tpl` | | List registered templates |
| `/bundle` | `/b` | `[SKILL1 SKILL2 ...] \| list \| off \| skills` | Compose, apply, or manage skill bundles |

### Escalations & Metacognition

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/escalations` | `/esc` | | List pending escalations |
| `/resolve` | | `<ID>` | Resolve an escalation |
| `/dismiss` | | `<ID>` | Dismiss an escalation |
| `/metacognition` | `/meta` | | Run a metacognition cycle |

### Memory & History

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/history` | `/hist` | | Show session history (episodic memory recall) |
| `/consolidate` | `/cons` | `[LIMIT] [--floor CONFIDENCE] [--max MAX_TRIPLES]` | Trigger episodic→semantic consolidation |

### Thread Management

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/thread` | `/th` | `list\|switch <id>\|new [title]\|archive <id>` | Manage chat threads — short-term memory across sessions |

### Communication (requires `communication` feature)

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/matrix` | `/mx` | `[ROOM_ID]` | List Matrix rooms, or show messages from a room |
| `/msg` | `/dm` | `<ROOM_ID> <MESSAGE>` | Send a message to a Matrix room |

### Specialized

| Command | Aliases | Args | Description |
|---------|---------|------|-------------|
| `/sovereignty` | `/sov` | | Show sovereignty status |
| `/ask` | | `<AGENT> <MESSAGE>` | Force a specific agent to respond |
| `/improv` | `/imp` | `[plussing\|yes-and\|yes-but\|freestyle\|riff]` | Set or display the active improv interaction mode |
| `/kanban` | `/kb` | `list\|board\|task\|move\|accept\|submit\|decompose\|spawn` | Kanban board and task coordination |
| `/listen` | `/rec`, `/record` | `start [SECONDS] \| stop \| view [FILE]` | Record audio, transcribe, and play back with word-level sync |
| `/talk` | `/speak` | `on \| off \| voice [DESCRIPTION]` | Enable spoken summaries of agent responses (TTS) |
| `/start` | `/tour`, `/onboarding` | | Take a guided tour of hKask's key capabilities |
| `/feedback` | | | Submit onboarding or usability feedback |

---

## 3. Multi-Line Input

The REPL uses Rust's `rustyline` crate with history, tab completion, and multi-line capabilities. For multi-line input:

- **In the readline prompt:** Multi-line is handled through readline's built-in editing. Use standard terminal navigation.
- **Slash commands:** Pass multi-line content through the `/ask` or `/invoke` commands with explicit arguments.

For complex multi-turn interactions, use the `/thread` command to manage persistent conversation contexts.

---

## 4. Context Condensation (`/consolidate`)

The REPL provides explicit episodic→semantic memory consolidation. This extracts semantic triples from episodic memory, creating reusable knowledge that persists across sessions:

```bash
# Show consolidation status
/consolidate

# Run consolidation with defaults (limit=100)
/consolidate run

# Run with custom parameters
/consolidate run --limit 50 --floor 0.33 --max 500
```

Parameters:

| Flag | Description |
|------|-------------|
| `--limit` / `-l` | Maximum episodes to consolidate (default: 100) |
| `--floor` / `-f` | Minimum confidence threshold (0.0–1.0; default: 0.33) |
| `--max` / `-m` | Maximum semantic triples to retain (default: none) |

Output shows pre-consolidation state, consolidation counts, and post-consolidation semantic memory size:

```
Pre-consolidation state:
  Consolidation candidates: 42
  Semantic h_mem count: 156
  Low-confidence h_mems (≤0.33): 12

Consolidation complete:
  Consolidated: 30
  Deleted: 12
  Post-consolidation semantic count: 186
```

---

## 5. Model Switching (`/model`)

### Show Current Model

```
/model
```

Output:

```
Current model: deepseek-v3
Use /model <name> to switch, /model <query> to search
```

### List All Available Models

```
/model list
```

Output shows a table of available models with name, family, parameter size, and file size:

```
Available models (24):
NAME                           FAMILY       PARAMS          SIZE
----------------------------------------------------------------------
deepseek-v3                    deepseek     671B             685.0 GB
llama-3-70b                    llama        70B              40.0 GB
mixtral-8x22b                  mixtral      141B             87.0 GB
...
```

### Exact Match Switch

```
/model deepseek-v3
```

### Fuzzy Search

```
/model llama
```

If multiple models match, you get a filtered list. If exactly one matches, the model switches immediately:

```
Model set to: llama-3-70b
  Family: llama
  Parameters: 70B
```

If no provider is reachable, the model name is stored and used for the next successful inference connection.

---

## 6. Skill Invocation from REPL

Skills are loaded at REPL startup from both `.agents/skills/` (private) and `skills/` (public) zones. Skills are invoked through the `hkask-mcp-skill` MCP server, which must be running (started automatically by the REPL).

> **Note:** The `/skill` slash command shown in some documentation routes through `SkillsDataBridge`, which calls the `hkask-mcp-skill` MCP server's `skill_execute` tool. [VERIFY: needs runtime test] If `/skill` is not available, use the MCP invoke path:

```
/invoke skill skill_execute skill_id=diagnose context="My app crashes on startup"
```

Alternatively, skills can be invoked through bundles:

```
/bundle diagnose coding-guidelines
```

---

## 7. Memory Recall (`/history`)

Recall recent conversation turns from episodic memory:

```
/history
```

Output:

```
Session history (5 turns):
  User: What is the CNS span for tool invocations?
  Curator: The CNS span for tool invocations is cns.tool.{subsystem}...
  User: How do I configure the content guard?
  Curator: The content guard is configured through the HKASK_GUARD_TOKEN_LIMIT...
  User: List all available MCP tools.
  Curator: Here are the available MCP tools...
```

Memory recall is scoped to the current agent and requires OCAP authorization (the token must have `memory:read` capability).

---

## 8. Session Management

### Threads (`/thread`)

Threads provide short-term memory that persists across sessions. They're stored in `agents/{name}/threads.json` and auto-archived on session start based on `short_term_memory_life`.

**List threads:**

```
/thread list
```

Output:

```
Chat Threads — /thread switch <id> to resume

  ● active  abc12345 ← current  12 msgs, Code review session
  ○ archived  def67890  5 msgs, Debugging memory leak
  ● active  ghi11121  8 msgs, API design discussion

  2 active, 1 archived — stm_life: 7 days
```

**Create a new thread:**

```
/thread new "Title for this thread"
```

**Switch to a thread:**

```
/thread switch abc12345
```

Output:

```
Switched to thread: Code review session (12 msgs)
Past conversation history is preserved in episodic memory.
```

**Toggle archive status:**

```
/thread archive def67890
# Archiving an active thread, or activating an archived one
```

### Clean Exit

```
/quit
```

This saves readline history to the history file and exits cleanly.

---

## 9. Common REPL Workflows

### Start a New Coding Session

```
/model deepseek-v3                 # Set the model
/mcp start 1,4-6,9                # Start relevant MCP servers
/tools                             # Verify tools are available
/pods                              # Check pod status
<your prompt>                      # Start chatting
```

### Debug a Skill

```
/skill my-skill "Review src/main.rs for issues"    # [VERIFY: needs runtime test]
/consolidate                                       # Consolidate learnings
/history                                           # Review what was discussed
```

### Audit Session

```
/sovereignty                       # Check consent and boundary state
/status                            # System health overview
/escalations                       # Any pending issues?
/metacognition                     # Run a metacognition cycle
```

### Kanban Coordination

```
/kanban list                       # List boards
/kanban board create "Sprint 12"   # Create a board
/kanban task "Implement X"         # Add a task
/kanban move                       # Move task through columns
```

---

## 10. Troubleshooting

### "No A2A secret resolved"

```
Error: No A2A secret resolved. Run `kask chat` to complete onboarding or set HKASK_MASTER_KEY.
```

Run `kask chat` interactively first to complete onboarding. This resolves your WebID, master key, and A2A secret.

### Matrix commands not available

```
Matrix communication not built — rebuild with `cargo build --features communication`
```

Rebuild:

```bash
cargo build --release --features communication
```

### TUI not available

```
TUI not built — rebuild with `cargo build --features tui`
```

Rebuild with TUI support:

```bash
cargo build --release --features tui
```

### Unknown command

```
Unknown command: /custom — did you mean:
  /consolidate — Trigger episodic→semantic consolidation with optional semantic cleanup
  /curator — Chat with the Supervisor agent
  /clear — Clear the screen
```

The REPL provides fuzzy matching for unknown commands. Type `/help` for the full command list.

### Consent denied during consolidation

```
Consent required: episodic_memory access denied
Grant consent outside the REPL with: kask sovereignty grant --category episodic_memory
                                      kask sovereignty grant --category semantic_memory
```

Run the suggested commands in a separate terminal, then retry consolidation.

---

## Related

- [Invoke a Skill](invoke-a-skill.md) — Detailed skill invocation workflow
- [Read CNS Alerts](read-cns-alerts.md) — CNS spans and algedonic alerts visible from REPL
- [Configure Content Guard](configure-guard.md) — Guard violations visible in REPL interactions
- [Compose Skills](compose-skills.md) — Skill bundling from REPL
