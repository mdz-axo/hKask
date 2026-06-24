# hKask TUI Specification

**Version:** 0.30.0  
**Status:** Implemented (22 windows, 14 with live domain bridges, PTY terminal)
**last_updated:** 2026-06-23  
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
- Live CNS alerts, context pressure, gas tracking, MCP server enumeration, pod counts
- Curator daemon routing (P12.1)

**Domain-specific bridges** (14 total, in `crates/hkask-tui/src/bridges/`) provide live service data to scaffolded windows via separate traits:
- `ConfigDataBridge` → `ReplSettings` (temperature, top_p, tool_loop, gas)
- `RegistryDataBridge` → `SqliteRegistry` (templates, skills, bundles)
- `WalletDataBridge` → `WalletService` (rJoule balance, transactions)
- `MemoryDataBridge` → `EpisodicStoragePort` / `SemanticStoragePort` (usage, consolidation)
- `KanbanDataBridge` → `KanbanService` (boards, tasks by status)
- `BackupDataBridge` → `BackupService` (snapshot metadata, config, verify)
- `MatrixDataBridge` → `MatrixTransport` (connection health, rooms, messages)
- `MediaDataBridge` → MCP `media/gallery_status` (gallery, recent images)
- `TrainingDataBridge` → MCP `training/training_list_adapters` (adapters, deployments)
- `CompaniesDataBridge` → MCP `companies/symbol_search` (search, financials, portfolio)
- `ResearchDataBridge` → MCP `research/web_search` (search, feeds, extract)
- `DocprocDataBridge` → MCP `docproc/docproc_chunk` (chunks, QA, index)
- `ReplicaDataBridge` → MCP `replica/replica_registry` (author replicas)
- `SkillsDataBridge` → MCP `skill/skill_list` (skill corpus, execute)

Each bridge accepts `Option<Arc<dyn Trait>>` — windows gracefully degrade to placeholder text when the bridge is `None`. The CLI implements all 14 on `TuiReplBridge` in `crates/hkask-cli/src/repl/tui_bridges.rs`, wired at `run_tui()` via `TuiSession.with_*_bridge()`. Backup, Media, and Training use `rt_handle.block_on()` for async service calls.

### 1.4 Keybindings

| Key | Action |
|-----|--------|
| `Ctrl+Q` | Quit |
| `Ctrl+N` | New Chat window |
| `Ctrl+T` | New tab |
| `Ctrl+W` | Close window |
| `Ctrl+B` | Toggle sidebar |
| `Ctrl+P` | Command palette (fuzzy search all 22 window kinds) |
| `Tab` | Focus next window |
| `Ctrl+H/J/K/L` | Navigate focus |
| `Ctrl+Shift+H` | Split horizontal |
| `Ctrl+Shift+J` | Split vertical |
| `Ctrl+=/-` | Resize split |
| `Ctrl+1-9` | Switch tab |
| `?` | Help overlay |
| `Enter` | Send message (Chat/Curator/MCP Chat tabs) |
| `Esc` | Clear input / cancel |
| `[` / `]` | Previous/next section (multi-section windows + Chat tab) |
| `PgUp/PgDn` | Scroll |

---

## §2. Window Catalog (22 Windows)

### 2.0 Launch Behavior

Default layout on `kask chat --tui`:
```
┌─ hKask ───────┐┌─ Chat ────────────────┐┌─ Curator ──────────────┐
│                ││ REPL ▸ _               ││ CRTR ▸ _                │
│  [Logo PTY]    │└────────────────────────┘└────────────────────────┘
└────────────────┘
```
Logo (top 25%) + Chat (bottom 75%) on left, Curator (35%) on right. New windows via `Ctrl+N` cycle.

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
**Status:** PTY-backed interactive shell

Uses `portable-pty` to spawn bash/fish/powershell. Features:
- `$` prompt, keystrokes forwarded to child (Enter, arrows, Ctrl+C/D/L, Tab)
- Background reader thread, output capped at 10,000 lines
- PageUp/PageDown scrollback, supports interactive programs

---

### 2.11 Editor
**File:** `windows/editor.rs`  
**Kind:** `WindowKind::Editor`  
**Status:** Live — file open/save

Line-based editor with filesystem integration. Features:
- `Ctrl+S` saves to current filename, `Ctrl+O` reloads
- `with_file(path)` builder, status bar (line/col, modified, filename)
- Line-numbered display, cursor highlighting, character insert/delete
- Line break on Enter, Backspace joins lines

---

### 2.12 Training
**File:** `windows/training.rs`  
**Kind:** `WindowKind::Training`  
**Status:** Live — TrainingDataBridge wired

Training session monitor. Displays:
- Active and completed training sessions
- LoRA adapter deployment status
- Training artifacts directory (`~/.config/hkask/adapters/`, `sessions/`)
- Integration path: `axolotl` CLI → `/adapter deploy`

---

### 2.13 Media
**File:** `windows/media.rs`  
**Kind:** `WindowKind::Media`  
**Status:** Live — MediaDataBridge (MCP-backed)

Media gallery browser. Tab-cycled sections:
- **Gallery:** active gallery status (image count, root path)
- **Collections:** recent images with tags and dimensions
- **Recent:** most recently added images

MCP-backed via `media/gallery_status` and `media/gallery_search` tools.

---

### 2.14 Skills
**File:** `windows/skills.rs`  
**Kind:** `WindowKind::Skills`  
**Status:** Live — RegistryDataBridge wired

Skill corpus browser. Tab-cycled sections:
- **Installed:** skills from registry with domain and description
- **Available:** templates available for installation
- **Active:** currently active skill bundles with version

---

### 2.15 Matrix
**File:** `windows/matrix.rs`  
**Kind:** `WindowKind::Matrix`  
**Status:** Live — MatrixDataBridge wired (connection health)

Federated messaging via Matrix protocol. Tab-cycled sections:
- **Rooms:** joined rooms with member counts, escalated ⚠ flag
- **Messages:** recent messages from first room with truncation
- **Contacts:** connection info, homeserver, room membership

Connection status via `MatrixTransport::healthy()` (sync). Room/message listing pending async bridge.

---

### 2.16 Memory
**File:** `windows/memory.rs`  
**Kind:** `WindowKind::Memory`  
**Status:** Live — MemoryDataBridge wired

Agent memory browser. Tab-cycled sections:
- **Episodic:** usage/budget bar + recent triples (entity·attribute=value)
- **Semantic:** triple count, low-confidence filtering
- **Triples:** RDF schema summary (confidence, visibility, owner WebID)
- **Consolidation:** candidate count, semantic totals, budget display

---

### 2.17 Kanban
**File:** `windows/kanban.rs`  
**Kind:** `WindowKind::Kanban`  
**Status:** Live — KanbanDataBridge wired

Task coordination board. Tab-cycled sections:
- **Board:** board name, status counts (backlog/ready/in_progress/review/done), columns
- **Backlog:** unassigned tasks with priority coloring (critical→red, medium→yellow)
- **In Progress:** tasks with agent pod assignments (cyan)
- **Done:** completed tasks with ✓ (green)

---

### 2.18 Companies
**File:** `windows/companies.rs`  
**Kind:** `WindowKind::Companies`  
**Status:** Live — CompaniesDataBridge wired (MCP dispatch)

Organization and entity data. Tab-cycled sections:
- **Search:** company lookup by symbol/name
- **Profile:** detailed company information
- **Financials:** key metrics and market data
- **Portfolio:** tracked companies

---

### 2.19 Research
**File:** `windows/research.rs`  
**Kind:** `WindowKind::Research`  
**Status:** Live — ResearchDataBridge wired (MCP dispatch)

Web research and content extraction. Tab-cycled sections:
- **Search:** web search with RRF fusion across providers
- **Feeds:** RSS/Atom feed monitoring (placeholder)
- **Extract:** URL content extraction to markdown

MCP Chat tab scoped to `research` server tools (`web_search`, `web_extract`).

---

### 2.20 Docproc
**File:** `windows/docproc.rs`  
**Kind:** `WindowKind::Docproc`  
**Status:** Live — DocprocDataBridge wired (MCP dispatch)

Document processing pipeline. Tab-cycled sections:
- **Chunks:** text chunks with token counts and previews
- **QA:** generated question-answer pairs at Bloom's taxonomy levels
- **Index:** vector index status (indexed/total)

MCP Chat tab scoped to `docproc` server tools (`docproc_chunk`, `docproc_generate_qa`, `docproc_query`, etc.).

---

### 2.21 Replica
**File:** `windows/replica.rs`  
**Kind:** `WindowKind::Replica`  
**Status:** Live — ReplicaDataBridge wired (MCP dispatch)

Authorial style replicas. Single data view + Chat tab:
- **Replicas:** built author replicas with centroid counts and status

MCP Chat tab scoped to `replica` server tools (`replica_build`, `replica_compose`, `replica_registry`, etc.).

---

### 2.22 Logo
**File:** `windows/logo.rs`  
**Kind:** `WindowKind::Logo`  
**Status:** Persistent — always present (top-left anchor)

Kask amphora logo rendered at reduced scale (40×30 chars) using half-block
Unicode characters (`▀ ▄ █`). Features:
- Rasterized from `assets/kask-logo.svg` (viewBox 400×600) at scale 0.1
- Shared rendering pipeline with splash screen (`splash.rs` → `LogoCanvas`)
- Persistent (unclosable), excluded from `Ctrl+N` cycling
- Bordered with "hKask" title, always present in default layout

---

## §3. MCP Two-Tab Design Pattern (Implemented)

MCP-focused windows (Companies, Kanban, Training, Media, Matrix, Memory, Research, Docproc, Replica, Skills) use a unified two-tab architecture:

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
- `]` cycles forward through sections + Chat tab, `[` cycles backward.
- Chat tab sends scoped inference via `ReplBridge::start_scoped_inference()` — only the window's MCP server tools are available to the model.

---
## §4. Implementation Status



| Layer | Status |

|-------|--------|

| Window trait + WindowKind enum (19 variants) | ✅ Complete |

| SplitNode tree (Leaf/Horizontal/Vertical) | ✅ Complete |

| Workspace (focus, split, resize, tabs, sidebar, help, close) | ✅ Complete |

| ReplBridge trait (16 methods) | ✅ Complete |

| 9 domain-specific bridge traits | ✅ Complete |

| CLI bridge implementations (tui_bridges.rs, live on TuiReplBridge) | ✅ Complete |

| TuiReplBridge (async inference, live CNS/MCP/pods, scoped inference) | ✅ Complete |

| Status bar (model, gas, CNS, context %, hints) | ✅ Live |

| Command palette (Ctrl+P, fuzzy search, 19 window kinds) | ✅ Complete |

| Chat window (full inference, streaming, spinner, export) | ✅ Complete |

| Curator window (P12.1 dual-presence, CNS alerts) | ✅ Live |

| CNS Monitor, Pods, Sidebar (live bridge data) | ✅ Live |

| Wallet, Config, Backup (live domain bridges) | ✅ Live |

| Registry, Skills (live SqliteRegistry data) | ✅ Live |

| Memory, Kanban (MCP two-tab, live service data) | ✅ Live |

| Matrix, Media, Training (MCP two-tab, MCP-backed bridges) | ✅ Live |

| Companies (MCP two-tab, live financial data via hkask-mcp-companies) | ✅ Live |

| Terminal (portable-pty interactive shell) | ✅ Live |

| Editor (file open/save, Ctrl+S/Ctrl+O) | ✅ Live |

| Logo (persistent top-left, SVG-rasterized) | ✅ Complete |

| MCP Chat scoping (start_scoped_inference, per-window tool filtering) | ✅ Complete |

| Help overlay (? key) | ✅ Complete |

| Integration tests (56 total: 8 unit + 48 smoke/rendering/contract) | ✅ Complete |

| Property tests (TuiMode transitions + MCP tab cycles + command palette) | ✅ Passing |

---

## §5. Known Variety Deficits

### 5.1 Terminal Window CNS Blind Spot

The PTY-backed `TerminalWindow` (`windows/terminal.rs`) runs an interactive shell
(`portable-pty` spawning bash/fish/powershell). Shell commands executed inside
the PTY are invisible to the CNS regulator — no CNS spans are emitted for
commands typed into the embedded terminal.

**Impact:** The regulator cannot observe terminal activity. Commands that
modify files, start/stop services, or consume resources are unmonitored. This
is a variety blind spot per Ashby's Law — the CNS's attenuator cannot
compress terminal behavior into a variety counter.

**Mitigation:** The shell process runs as a child of the hKask TUI process,
so resource consumption (CPU, memory) is indirectly observable at the OS level.
The terminal is scoped to the user's shell environment — it does not have
elevated privileges beyond what the user already has.

**Resolution path (v0.31+):** CNS observation of PTY output is blocked by
P1 (User Sovereignty) — capturing private shell content without explicit
consent would violate the Magna Carta. The correct resolution is a consent
gate: the user must explicitly opt in to PTY CNS monitoring. Until then,
this is an acknowledged variety deficit.
