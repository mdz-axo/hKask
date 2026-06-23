# hKask TUI Specification

**Version:** 0.30.0  
**Status:** Scaffolded (18 windows defined, core interaction surfaces implemented)  
**Framework:** ratatui 0.28 + crossterm 0.28  
**Crate:** `crates/hkask-tui/`

---

## §1. Architecture

### 1.1 Workspace Model

The TUI implements a **Zed-style workspace**: a binary tree of splits (`SplitNode`) hosts stateful `Window` trait objects. The `Workspace` manages focus, layout, resize, tabs, and event routing.

```
TuiSession
  └── Workspace
        ├── Tab bar (Ctrl+1-9)
        ├── SplitNode tree
        │     ├── Leaf: Box<dyn Window>
        │     ├── Horizontal { left, right, ratio }
        │     └── Vertical { top, bottom, ratio }
        ├── Status bar (model, gas, CNS, context %, hints)
        └── Help overlay (? key)
```

### 1.2 Window Trait (7 methods)

```rust
pub trait Window {
    fn id(&self) -> WindowId;
    fn title(&self) -> &str;
    fn kind(&self) -> WindowKind;
    fn render(&self, f: &mut Frame, area: Rect, is_focused: bool);
    fn handle_key(&mut self, key: KeyEvent) -> bool;
    fn can_close(&self) -> bool;
    fn tick(&mut self);
}
```

### 1.3 Bridge Layer

The `ReplBridge` trait decouples the TUI crate from `hkask-cli`. The CLI provides `TuiReplBridge` which wraps `ReplState` behind `Arc<Mutex<>>` and provides:
- Async inference with spinner + streaming
- Live CNS alerts, context pressure, gas tracking
- MCP server enumeration, pod counts
- Curator daemon routing (P12.1)

### 1.4 Keybindings

| Key | Action |
|-----|--------|
| `Ctrl+Q` | Quit |
| `Ctrl+T` | New tab |
| `Ctrl+W` | Close tab |
| `Ctrl+N` | Cycle new window kind (18 kinds) |
| `Ctrl+B` | Toggle sidebar |
| `Ctrl+H/J/K/L` | Navigate focus |
| `Ctrl+Shift+H` | Split horizontal |
| `Ctrl+Shift+J` | Split vertical |
| `Ctrl+=/-` | Resize split |
| `Ctrl+1-9` | Switch tab |
| `Ctrl+P` | Command palette |
| `?` | Help overlay |
| `Enter` | Send message (Chat/Curator/Terminal) |
| `Esc` | Clear input / cancel |
| `PgUp/PgDn` | Scroll |
| `Tab` | Cycle sections (multi-section windows) |

---

## §2. Window Catalog (18 Windows)

### 2.0 Launch Behavior

Default layout on `kask chat --tui`:
```
┌─ Chat ────────────────┐┌─ Curator ──────────────┐
│ REPL ▸ _               ││ CRTR ▸ _                │
└────────────────────────┘└────────────────────────┘
```
65/35 horizontal split. New windows via `Ctrl+N` cycle, `Ctrl+H/J/K/L` navigate.

---

### 2.1 Chat
**File:** `windows/chat.rs`  
**Kind:** `WindowKind::Chat`  
**Status:** Full implementation

Primary AI interaction surface. Features:
- `TuiMode` state machine: `Chat` ↔ `Command` ↔ `Curator`
- Prompt prefixes: `REPL ▸` (cyan), `CMD ▸` (yellow), `CRTR ▸` (magenta)
- Async inference with spinner (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` animation)
- Streaming text display (3-char chunked reveal during inference)
- Slash commands: `/help`, `/clear`, `/model`, `/status`, `/mcp`, `/export`, `/curator`
- Message types: User, Agent, Curator, CNS Alert (Richmond Gold), Tool
- `/export` writes `kask-chat-YYYYMMDD-HHMMSS.md` with full history
- 8 property tests for `TuiMode` transitions

---

### 2.2 Curator
**File:** `windows/curator.rs`  
**Kind:** `WindowKind::Curator`  
**Status:** Scaffolded with live CNS

P12.1 dual-presence window. Displays CNS alerts, memory summaries, and pattern detection from the Curator daemon. Features:
- `CuratorEntryKind`: CnsAlert, MemorySummary, Pattern, Reply, UserMessage
- `CRTR ▸` prompt with magenta styling
- Live CNS alert polling each frame via bridge
- Sends messages to Curator daemon via `bridge.send_curator_message()`

---

### 2.3 CNS Monitor
**File:** `windows/cns_monitor.rs`  
**Kind:** `WindowKind::CnsMonitor`  
**Status:** Live bridge data

Cybernetic nervous system health display. Shows:
- Active alert count (atomic counter from bridge)
- Domain health status (green ✓ / red ✗)
- Gas budget remaining/cap
- Context window pressure
- MCP server loaded/total count
- Pod counts (curator, replicant, team)

---

### 2.4 Pods
**File:** `windows/pods.rs`  
**Kind:** `WindowKind::Pods`  
**Status:** Live filesystem counts

Three-tier pod deployment status. Data from `PodRegistry::scan_by_kind()`:
- Tier 1: CuratorPod — SemanticIndex owner, CNS coordination
- Tier 2: TeamPods — shared bot episodic storage
- Tier 3: ReplicantPods — human+replicant pair, private episodic
- Per-pod SQLCipher, no cross-pod access (P4.1, P11.1)

---

### 2.5 Wallet
**File:** `windows/wallet.rs`  
**Kind:** `WindowKind::Wallet`  
**Status:** Scaffolded

Gas budget and rJoule management. Features:
- Gas gauge widget with color transitions
- rJoule balance display (balance, reserved for gas holds)
- Gas budget (remaining, cap, replenish rate)
- Transaction history placeholder
- **Future:** Buy rJoule, deposits/withdrawals, API key management via `hkask-wallet`

---

### 2.6 Registry
**File:** `windows/registry.rs`  
**Kind:** `WindowKind::Registry`  
**Status:** Scaffolded

Browse templates, skills, styles, and bundles. Tab-cycled sections:
- **Templates:** WordAct/FlowDef/KnowAct manifests
- **Skills:** PDCA FlowDef loops
- **Styles:** Prose composition templates
- **Bundles:** Composite skill bundles

---

### 2.7 Backup
**File:** `windows/backup.rs`  
**Kind:** `WindowKind::Backup`  
**Status:** Scaffolded

Backup operations reference. Shows commands:
- `/backup snapshot` — create new snapshot
- `/backup restore` — restore from snapshot
- `/backup list` — list all snapshots
- `/backup verify` — verify integrity
- `/backup prune` — remove old snapshots
- Storage: `~/.config/hkask/backups/`, encrypted SQLCipher

---

### 2.8 Configuration
**File:** `windows/configuration.rs`  
**Kind:** `WindowKind::Configuration`  
**Status:** Scaffolded

REPL inference and system settings display. Shows:
- Inference params (model, temperature, top-p, max tokens)
- Tool loop settings (limit, context turns, auto-condense)
- Energy budget (gas heuristic, cap, context used)
- System info (MCP servers, agent name)

---

### 2.9 Sidebar
**File:** `windows/sidebar.rs`  
**Kind:** `WindowKind::Sidebar`  
**Status:** Live bridge data, persistent

Composite status panel. Tab-cycled sections:
- **CNS Health:** domain status, alerts, gas
- **MCP Servers:** loaded/total count
- **Pods:** three-tier deployment counts
- **Context:** window pressure, model metadata
- **Keys:** full keybinding reference

Persistent (cannot be closed). Opened via `Ctrl+B`.

---

### 2.10 Terminal
**File:** `windows/terminal.rs`  
**Kind:** `WindowKind::Terminal`  
**Status:** Scaffolded (command execution, not full PTY)

Embedded shell command execution. Features:
- `$` prompt with green styling
- Command execution via `std::process::Command`
- Captures stdout and stderr
- Exit code display
- Scrollable output history

**Note:** Not a full PTY terminal emulator. For interactive programs, use the system terminal.

---

### 2.11 Editor
**File:** `windows/editor.rs`  
**Kind:** `WindowKind::Editor`  
**Status:** Scaffolded

Basic text editor for configs, agent YAML, scripts. Features:
- Line-numbered display
- Cursor navigation (arrows, Home/End, PgUp/PgDn)
- Character insert/delete
- Line break on Enter
- Backspace joins lines
- Ctrl+S marks as saved (placeholder)
- Modified flag tracking

---

### 2.12 Training
**File:** `windows/training.rs`  
**Kind:** `WindowKind::Training`  
**Status:** Scaffolded

Training session monitor. Displays:
- Active and completed training sessions
- LoRA adapter deployment status
- Training artifacts directory (`~/.config/hkask/adapters/`, `sessions/`)
- Integration path: `axolotl` CLI → `/adapter deploy`

---

### 2.13 Media
**File:** `windows/media.rs`  
**Kind:** `WindowKind::Media`  
**Status:** Scaffolded

Media gallery browser. Tab-cycled sections:
- **Gallery:** supported formats, directory path
- **Collections:** group related media files
- **Recent:** recently generated images, audio, transcripts

Integrates with media MCP server and `/listen`/`/talk` commands.

---

### 2.14 Skills
**File:** `windows/skills.rs`  
**Kind:** `WindowKind::Skills`  
**Status:** Scaffolded

Skill corpus browser. Tab-cycled sections:
- **Installed:** skills from `.agents/skills/` and `registry/` (46 templates)
- **Available:** registry listing with named entries
- **Active:** currently active skill bundles

Integrates with `/skill list`, `/skill status`, `/bundle compose/apply`.

---

### 2.15 Matrix
**File:** `windows/matrix.rs`  
**Kind:** `WindowKind::Matrix`  
**Status:** Scaffolded

Federated messaging via Matrix protocol. Tab-cycled sections:
- **Rooms:** joined Matrix rooms
- **Messages:** room message history (E2E encrypted)
- **Contacts:** directory search, agent invitations

Integrates with `hkask-communication` and `hkask-mcp-communication`.

---

### 2.16 Memory
**File:** `windows/memory.rs`  
**Kind:** `WindowKind::Memory`  
**Status:** Scaffolded

Agent memory browser. Tab-cycled sections:
- **Episodic:** private, agent-scoped experiences (per-pod SQLCipher)
- **Semantic:** shared, public knowledge (CuratorPod SemanticIndex)
- **Triples:** RDF subject-predicate-object with confidence/visibility/owner
- **Consolidation:** episodic→semantic triggers and confidence floor

Memory model: ν-events → episodic (private) → semantic (public) → SemanticIndex

---

### 2.17 Kanban
**File:** `windows/kanban.rs`  
**Kind:** `WindowKind::Kanban`  
**Status:** Scaffolded

Task coordination board. Tab-cycled sections:
- **Board:** overview of all columns
- **Backlog:** unassigned tasks
- **In Progress:** tasks with agent pod assignments
- **Done:** completed tasks with verification

Integrates with `hkask-services-kanban` and Kata coaching loop.

---

### 2.18 Companies
**File:** `windows/companies.rs`  
**Kind:** `WindowKind::Companies`  
**Status:** Scaffolded

Organization and entity data. Tab-cycled sections:
- **Search:** company lookup by name/domain/industry
- **Profile:** detailed company information
- **People:** key personnel and contacts
- **Relations:** subsidiaries, competitors, partners

Powered by `hkask-mcp-companies` + Firecrawl integration.

---

## §3. MCP Two-Tab Design Pattern (Future)

MCP-focused windows (Companies, Kanban, Training, Media, Matrix, Research, etc.) will adopt a unified two-tab architecture:

```
┌─ [MCP Name] ────────────────────────┐
│┌─ Tab: Chat ───┐┌─ Tab: Data ───────┐│
││                ││                   ││
││ Focused chat   ││ UI widgets and    ││
││ using this     ││ artifacts from    ││
││ MCP's tools    ││ the MCP server    ││
││                ││                   ││
││ REPL ▸ _       ││ [structured view] ││
│└────────────────┘└───────────────────┘│
└──────────────────────────────────────┘
```

- **Tab 1 (Chat):** A focused chat that only has access to one MCP server's tools. The system prompt includes only that MCP's tool definitions.
- **Tab 2 (Data):** Structured UI widgets rendering MCP artifacts — tables, trees, cards, galleries. Non-chat interaction surface.
- Toggle between tabs with `Tab` key within the window.

---

## §4. Implementation Status

| Layer | Status |
|-------|--------|
| Window trait + WindowKind enum (18 variants) | ✅ Complete |
| SplitNode tree (Leaf/Horizontal/Vertical) | ✅ Complete |
| Workspace (focus, split, resize, tabs, sidebar, help, close) | ✅ Complete |
| ReplBridge trait (15 methods) | ✅ Complete |
| TuiReplBridge (async inference, live CNS/MCP/pods) | ✅ Complete |
| Status bar (model, gas, CNS, context %, hints) | ✅ Live |
| Chat window (full inference, streaming, spinner, export) | ✅ Complete |
| Curator window (P12.1 dual-presence, CNS alerts) | ✅ Live |
| CNS Monitor, Pods, Sidebar (live bridge data) | ✅ Live |
| Wallet, Registry, Backup, Configuration | ✅ Scaffolded |
| Terminal, Editor | ✅ Scaffolded |
| Training, Media, Skills, Matrix, Memory, Kanban, Companies | ✅ Scaffolded |
| Help overlay (? key) | ✅ Complete |
| 8 property tests (TuiMode transitions) | ✅ Passing |
