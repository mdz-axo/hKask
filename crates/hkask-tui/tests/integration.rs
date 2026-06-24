//! Integration tests for hkask-tui.
//!
//! Covers window creation smoke tests, WindowKind invariant properties,
//! StatusBar rendering, and workspace operations.

use std::sync::Arc;

use hkask_tui::{
    ReplBridge, TurnResult, Window, WindowId, WindowKind,
    windows::{
        BackupWindow, ChatWindow, CnsMonitorWindow, CompaniesWindow, ConfigurationWindow,
        CuratorWindow, DocprocWindow, EditorWindow, KanbanWindow, LogoWindow, MatrixWindow,
        MediaWindow, MemoryWindow, PodsWindow, RegistryWindow, ReplicaWindow, ResearchWindow,
        SidebarWindow, SkillsWindow, TerminalWindow, TrainingWindow, WalletWindow,
    },
};

/// A minimal ReplBridge for testing — returns safe defaults for all methods.
struct MockBridge {
    agent_name: String,
    model_name: String,
    gas_remaining: u64,
    gas_cap: u64,
    cns_alerts: u32,
    context_pressure: f64,
    mcp_loaded: usize,
    mcp_total: usize,
    pod_curator: usize,
    pod_replicant: usize,
    pod_team: usize,
    cns_domains: Vec<(String, bool)>,
}

impl MockBridge {
    fn new() -> Self {
        Self {
            agent_name: "test-agent".into(),
            model_name: "mock-model".into(),
            gas_remaining: 500,
            gas_cap: 1000,
            cns_alerts: 0,
            context_pressure: 0.3,
            mcp_loaded: 2,
            mcp_total: 4,
            pod_curator: 1,
            pod_replicant: 3,
            pod_team: 2,
            cns_domains: vec![
                ("sovereignty".into(), true),
                ("gas".into(), true),
                ("consent".into(), true),
            ],
        }
    }
}

impl ReplBridge for MockBridge {
    fn start_inference(&self, _input: String) {}
    fn poll_inference(&self) -> hkask_tui::InferenceState {
        hkask_tui::InferenceState::Idle
    }
    fn streaming_text(&self) -> String {
        String::new()
    }
    fn send_message_blocking(&self, _input: &str) -> TurnResult {
        TurnResult {
            text: "mock response".into(),
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            gas_cost: 0,
            iterations: 0,
            budget_exhausted: false,
        }
    }
    fn agent_name(&self) -> &str {
        &self.agent_name
    }
    fn model_name(&self) -> &str {
        &self.model_name
    }
    fn gas_remaining(&self) -> u64 {
        self.gas_remaining
    }
    fn gas_cap(&self) -> u64 {
        self.gas_cap
    }
    fn cns_alert_count(&self) -> u32 {
        self.cns_alerts
    }
    fn context_pressure(&self) -> f64 {
        self.context_pressure
    }
    fn mcp_status(&self) -> (usize, usize) {
        (self.mcp_loaded, self.mcp_total)
    }
    fn pod_counts(&self) -> (usize, usize, usize) {
        (self.pod_curator, self.pod_replicant, self.pod_team)
    }
    fn cns_domains(&self) -> Vec<(String, bool)> {
        self.cns_domains.clone()
    }
    fn send_curator_message(&self, _input: &str) -> String {
        "curator ack".into()
    }
}

fn bridge() -> Arc<MockBridge> {
    Arc::new(MockBridge::new())
}

fn window_id() -> WindowId {
    WindowId(uuid::Uuid::new_v4())
}

/// Helper: create a ratatui Terminal backed by TestBackend, draw with a
/// window's render method, and verify no panic occurs. Returns the
/// terminal so the caller can inspect the buffer if desired.
fn render_smoke(
    window: &dyn Window,
    width: u16,
    height: u16,
) -> ratatui::Terminal<ratatui::backend::TestBackend> {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut term = ratatui::Terminal::new(backend).expect("failed to create test terminal");
    term.draw(|f| {
        window.render(f, f.area(), true);
    })
    .expect("render panicked");
    term
}

// ────────────────────────────────────────────────────────────────
// WindowKind invariants
// ────────────────────────────────────────────────────────────────

fn all_window_kinds() -> Vec<WindowKind> {
    vec![
        WindowKind::Chat,
        WindowKind::CnsMonitor,
        WindowKind::Backup,
        WindowKind::Registry,
        WindowKind::Pods,
        WindowKind::Kanban,
        WindowKind::Wallet,
        WindowKind::Memory,
        WindowKind::Companies,
        WindowKind::Matrix,
        WindowKind::Configuration,
        WindowKind::Sidebar,
        WindowKind::Curator,
        WindowKind::Terminal,
        WindowKind::Editor,
        WindowKind::Training,
        WindowKind::Media,
        WindowKind::Skills,
        WindowKind::Research,
        WindowKind::Docproc,
        WindowKind::Replica,
        WindowKind::Logo,
    ]
}

#[test]
fn all_22_kinds_exist() {
    assert_eq!(all_window_kinds().len(), 22);
}

#[test]
fn default_title_is_non_empty() {
    for kind in all_window_kinds() {
        assert!(
            !kind.default_title().is_empty(),
            "{:?} has empty title",
            kind
        );
    }
}

#[test]
fn description_is_non_empty() {
    for kind in all_window_kinds() {
        assert!(
            !kind.description().is_empty(),
            "{:?} has empty description",
            kind
        );
    }
}

#[test]
fn allows_multiple_only_for_chat_and_matrix() {
    for kind in all_window_kinds() {
        match kind {
            WindowKind::Chat | WindowKind::Matrix => {
                assert!(kind.allows_multiple(), "{:?} should allow multiple", kind);
            }
            _ => {
                assert!(
                    !kind.allows_multiple(),
                    "{:?} should NOT allow multiple",
                    kind
                );
            }
        }
    }
}

#[test]
fn only_sidebar_and_logo_are_persistent() {
    for kind in all_window_kinds() {
        if kind == WindowKind::Sidebar || kind == WindowKind::Logo {
            assert!(kind.is_persistent());
        } else {
            assert!(!kind.is_persistent(), "{:?} should not be persistent", kind);
        }
    }
}

#[test]
fn all_titles_are_distinct() {
    let mut titles: Vec<&str> = all_window_kinds()
        .iter()
        .map(|k| k.default_title())
        .collect();
    titles.sort_unstable();
    titles.dedup();
    assert_eq!(titles.len(), 22, "duplicate titles: {:?}", titles);
}

// ────────────────────────────────────────────────────────────────
// Rendering smoke tests — every window renders without panicking
// ────────────────────────────────────────────────────────────────

#[test]
fn chat_renders() {
    let b = bridge();
    let w = ChatWindow::new(window_id(), b.agent_name(), b.model_name(), None, b.clone());
    render_smoke(&w, 80, 24);
}

#[test]
fn cns_monitor_renders() {
    let w = CnsMonitorWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn backup_renders() {
    let w = BackupWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn registry_renders() {
    let w = RegistryWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn pods_renders() {
    let w = PodsWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn kanban_renders() {
    let w = KanbanWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn wallet_renders() {
    let w = WalletWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn memory_renders() {
    let w = MemoryWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn companies_renders() {
    let w = CompaniesWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn matrix_renders() {
    let w = MatrixWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn configuration_renders() {
    let w = ConfigurationWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn sidebar_renders() {
    let b = bridge();
    let w = SidebarWindow::new(window_id(), None, b);
    render_smoke(&w, 80, 24);
}

#[test]
fn curator_renders() {
    let w = CuratorWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn terminal_renders() {
    let w = TerminalWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn editor_renders() {
    let w = EditorWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn training_renders() {
    let w = TrainingWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn media_renders() {
    let w = MediaWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn research_renders() {
    let w = ResearchWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn skills_renders() {
    let w = SkillsWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn docproc_renders() {
    let w = DocprocWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn replica_renders() {
    let w = ReplicaWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn all_windows_render_at_multiple_sizes() {
    let b = bridge();
    let sizes = [(40, 12), (80, 24), (120, 40), (200, 80)];

    let windows: Vec<Box<dyn Window>> = vec![
        Box::new(ChatWindow::new(
            window_id(),
            b.agent_name(),
            b.model_name(),
            None,
            b.clone(),
        )),
        Box::new(CnsMonitorWindow::new(window_id(), b.clone())),
        Box::new(BackupWindow::new(window_id(), b.clone())),
        Box::new(RegistryWindow::new(window_id(), b.clone())),
        Box::new(PodsWindow::new(window_id(), b.clone())),
        Box::new(KanbanWindow::new(window_id(), b.clone())),
        Box::new(WalletWindow::new(window_id(), b.clone())),
        Box::new(MemoryWindow::new(window_id(), b.clone())),
        Box::new(CompaniesWindow::new(window_id(), b.clone())),
        Box::new(MatrixWindow::new(window_id(), b.clone())),
        Box::new(ConfigurationWindow::new(window_id(), b.clone())),
        Box::new(SidebarWindow::new(window_id(), None, b.clone())),
        Box::new(CuratorWindow::new(window_id(), b.clone())),
        Box::new(TerminalWindow::new(window_id(), b.clone())),
        Box::new(EditorWindow::new(window_id(), b.clone())),
        Box::new(TrainingWindow::new(window_id(), b.clone())),
        Box::new(MediaWindow::new(window_id(), b.clone())),
        Box::new(SkillsWindow::new(window_id(), b.clone())),
        Box::new(ResearchWindow::new(window_id(), b.clone())),
        Box::new(DocprocWindow::new(window_id(), b.clone())),
        Box::new(ReplicaWindow::new(window_id(), b.clone())),
        Box::new(LogoWindow::new(window_id())),
    ];

    assert_eq!(windows.len(), 22);

    for (w, h) in sizes {
        for window in &windows {
            render_smoke(window.as_ref(), w, h);
        }
    }
}

// ────────────────────────────────────────────────────────────────
// Tab cycling on scaffolded windows
// ────────────────────────────────────────────────────────────────

#[test]
fn kanban_sections_cycle() {
    let mut w = KanbanWindow::new(window_id(), bridge());
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let tab = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);
    // 4 presses = back to start
    render_smoke(&w, 80, 24);
    assert!(w.handle_key(tab));
    assert!(w.handle_key(tab));
    assert!(w.handle_key(tab));
    assert!(w.handle_key(tab));
    render_smoke(&w, 80, 24);
}

#[test]
fn memory_sections_cycle() {
    let mut w = MemoryWindow::new(window_id(), bridge());
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let tab = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);
    render_smoke(&w, 80, 24);
    // 5-state cycle: Episodic -> Semantic -> Triples -> Consolidation -> Chat -> Episodic
    for _ in 0..5 {
        assert!(w.handle_key(tab));
    }
    render_smoke(&w, 80, 24);
}

#[test]
fn terminal_input_roundtrip() {
    let mut w = TerminalWindow::new(window_id(), bridge());
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    render_smoke(&w, 80, 24);
    assert!(w.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)));
    assert!(w.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)));
    assert!(w.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)));
    assert!(w.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)));
    render_smoke(&w, 80, 24);
    assert!(w.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)));
    render_smoke(&w, 80, 24);
}

#[test]
fn editor_text_operations() {
    let mut w = EditorWindow::new(window_id(), bridge());
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    render_smoke(&w, 80, 24);
    assert!(w.handle_key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)));
    assert!(w.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)));
    render_smoke(&w, 80, 24);
    assert!(w.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)));
    render_smoke(&w, 80, 24);
}

// ────────────────────────────────────────────────────────────────
// Domain bridge tests — memory, kanban, registry with mock data
// ────────────────────────────────────────────────────────────────

#[test]
fn memory_shows_live_data_with_bridge() {
    use hkask_tui::bridges::memory::MockMemoryBridge;
    let w = MemoryWindow::new(window_id(), bridge())
        .with_memory_bridge(MockMemoryBridge::with_data().arc());
    render_smoke(&w, 80, 24);
}

#[test]
fn memory_empty_shows_placeholder() {
    use hkask_tui::bridges::memory::MockMemoryBridge;
    let w =
        MemoryWindow::new(window_id(), bridge()).with_memory_bridge(MockMemoryBridge::new().arc());
    render_smoke(&w, 80, 24);
}

#[test]
fn kanban_shows_live_data_with_bridge() {
    use hkask_tui::bridges::kanban::MockKanbanBridge;
    let w = KanbanWindow::new(window_id(), bridge())
        .with_kanban_bridge(MockKanbanBridge::with_sample_data().arc());
    render_smoke(&w, 80, 24);
}

#[test]
fn kanban_empty_shows_placeholder() {
    use hkask_tui::bridges::kanban::MockKanbanBridge;
    let w =
        KanbanWindow::new(window_id(), bridge()).with_kanban_bridge(MockKanbanBridge::new().arc());
    render_smoke(&w, 80, 24);
}

#[test]
fn registry_shows_live_data_with_bridge() {
    use hkask_tui::bridges::registry::MockRegistryBridge;
    let w = RegistryWindow::new(window_id(), bridge())
        .with_registry_bridge(MockRegistryBridge::new().arc());
    render_smoke(&w, 80, 24);
}

#[test]
fn skills_shows_live_data_with_bridge() {
    use hkask_tui::bridges::registry::MockRegistryBridge;
    let w = SkillsWindow::new(window_id(), bridge())
        .with_registry_bridge(MockRegistryBridge::new().arc());
    render_smoke(&w, 80, 24);
}

// ────────────────────────────────────────────────────────────────
// Snapshot tests — verify rendered output contains expected strings
// ────────────────────────────────────────────────────────────────

/// Render a window into a ratatui TestBackend and return the buffer
/// contents as a Vec<String> (one String per visible row).
fn render_snapshot(window: &dyn Window, width: u16, height: u16) -> Vec<String> {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut term = ratatui::Terminal::new(backend).expect("test terminal");
    term.draw(|f| window.render(f, f.area(), true))
        .expect("render");
    let buf = term.backend().buffer().clone();
    let mut lines: Vec<String> = Vec::new();
    for row in 0..height {
        let mut line = String::new();
        for col in 0..width {
            let cell = buf.cell((col, row)).unwrap();
            line.push_str(cell.symbol());
        }
        // Trim trailing whitespace for snapshot comparison
        let trimmed: String = line.trim_end().to_string();
        if !trimmed.is_empty() {
            lines.push(trimmed);
        }
    }
    lines
}

#[test]
fn chat_snapshot_contains_prompt() {
    let b = bridge();
    let agent_name = b.agent_name().to_string();
    let model_name = b.model_name().to_string();
    let w = ChatWindow::new(window_id(), &agent_name, &model_name, None, b);
    let lines = render_snapshot(&w, 80, 24);
    let text = lines.join("\n");
    assert!(text.contains("REPL"), "Chat should show REPL prompt");
}

#[test]
fn cns_monitor_snapshot_shows_domains() {
    let w = CnsMonitorWindow::new(window_id(), bridge());
    let lines = render_snapshot(&w, 80, 24);
    let text = lines.join("\n");
    assert!(!text.is_empty(), "CNS monitor should render content");
}

#[test]
fn wallet_snapshot_shows_gas() {
    let w = WalletWindow::new(window_id(), bridge());
    let lines = render_snapshot(&w, 80, 24);
    let text = lines.join("\n");
    assert!(
        text.contains("Gas Budget") || text.contains("gas"),
        "Wallet should show gas info"
    );
}

#[test]
fn backup_snapshot_shows_commands() {
    let w = BackupWindow::new(window_id(), bridge());
    let lines = render_snapshot(&w, 80, 24);
    let text = lines.join("\n");
    assert!(
        text.contains("snapshot") || text.contains("Backup"),
        "Backup should show commands"
    );
}

#[test]
fn registry_snapshot_shows_sections() {
    let w = RegistryWindow::new(window_id(), bridge());
    let lines = render_snapshot(&w, 80, 24);
    let text = lines.join("\n");
    assert!(
        text.contains("Templates") || text.contains("Registry"),
        "Registry should show sections"
    );
}

#[test]
fn memory_snapshot_shows_tabs() {
    let w = MemoryWindow::new(window_id(), bridge());
    let lines = render_snapshot(&w, 80, 24);
    let text = lines.join("\n");
    assert!(
        text.contains("Episodic") || text.contains("Memory"),
        "Memory should show tabs"
    );
}

#[test]
fn kanban_snapshot_shows_board() {
    let w = KanbanWindow::new(window_id(), bridge());
    let lines = render_snapshot(&w, 80, 24);
    let text = lines.join("\n");
    assert!(
        text.contains("Board") || text.contains("Kanban"),
        "Kanban should show board"
    );
}

#[test]
fn logo_snapshot_renders() {
    let w = LogoWindow::new(window_id());
    let lines = render_snapshot(&w, 80, 30);
    let text = lines.join("\n");
    assert!(text.len() > 50, "Logo should render substantial content");
}

#[test]
fn terminal_snapshot_shows_prompt() {
    let w = TerminalWindow::new(window_id(), bridge());
    let lines = render_snapshot(&w, 80, 24);
    let text = lines.join("\n");
    assert!(
        text.contains("$") || text.contains("Terminal"),
        "Terminal should show prompt"
    );
}

#[test]
fn editor_snapshot_renders() {
    let w = EditorWindow::new(window_id(), bridge());
    let lines = render_snapshot(&w, 80, 24);
    let text = lines.join("\n");
    assert!(!text.is_empty(), "Editor should render content");
}

// ────────────────────────────────────────────────────────────────
// MCP Two-Tab contract — Tab cycles through sections + Chat
// ────────────────────────────────────────────────────────────────

#[test]
fn mcp_tab_kanban_cycles_sections_and_chat() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use hkask_tui::mcp_tabbed::{McpTab, McpTabbedWindow};
    let mut w = KanbanWindow::new(window_id(), bridge());
    let tab = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);

    assert_eq!(w.active_tab(), McpTab::Data);
    render_smoke(&w, 80, 24);

    // Tab x4: Board -> Backlog -> InProgress -> Done -> Chat
    for _ in 0..4 {
        assert!(w.handle_key(tab));
    }
    assert_eq!(w.active_tab(), McpTab::Chat);
    render_smoke(&w, 80, 24);

    // Tab from Chat -> back to Data
    assert!(w.handle_key(tab));
    assert_eq!(w.active_tab(), McpTab::Data);
}

#[test]
fn mcp_tab_companies_cycles_sections_and_chat() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use hkask_tui::mcp_tabbed::{McpTab, McpTabbedWindow};
    let mut w = CompaniesWindow::new(window_id(), bridge());
    let tab = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);

    assert_eq!(w.active_tab(), McpTab::Data);
    // Tab x4: Search -> Profile -> Financials -> Portfolio -> Chat
    for _ in 0..4 {
        assert!(w.handle_key(tab));
    }
    assert_eq!(w.active_tab(), McpTab::Chat);
    assert!(w.handle_key(tab));
    assert_eq!(w.active_tab(), McpTab::Data);
}

#[test]
fn mcp_tab_memory_cycles_sections_and_chat() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use hkask_tui::mcp_tabbed::{McpTab, McpTabbedWindow};
    let mut w = MemoryWindow::new(window_id(), bridge());
    let tab = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);

    assert_eq!(w.active_tab(), McpTab::Data);
    for _ in 0..4 {
        assert!(w.handle_key(tab));
    }
    assert_eq!(w.active_tab(), McpTab::Chat);
    assert!(w.handle_key(tab));
    assert_eq!(w.active_tab(), McpTab::Data);
}

#[test]
fn mcp_tab_training_toggles_chat() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use hkask_tui::mcp_tabbed::{McpTab, McpTabbedWindow};
    let mut w = TrainingWindow::new(window_id(), bridge());
    let right = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);
    let left = KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE);
    assert_eq!(w.active_tab(), McpTab::Data);
    assert!(w.handle_key(right));
    assert_eq!(w.active_tab(), McpTab::Chat);
    assert!(w.handle_key(left));
    assert_eq!(w.active_tab(), McpTab::Data);
}

// ────────────────────────────────────────────────────────────────
// Command palette — fuzzy search, selection, dismiss
// ────────────────────────────────────────────────────────────────

#[test]
fn command_palette_filters_and_selects() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use hkask_tui::command_palette::CommandPalette;

    let mut palette = CommandPalette::new();

    // Starts with items visible
    assert!(palette.selected_kind().is_some());

    // Type to filter
    palette.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    palette.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    palette.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
    assert!(palette.selected_kind().is_some());

    // Arrow navigation
    palette.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    palette.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

    // Enter selects
    let result = palette.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(result.is_some());

    // Reset and verify Esc dismisses
    palette.reset();
    let dismiss = palette.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(dismiss, Some(hkask_tui::WindowKind::Chat));

    // Reset and verify Ctrl+P dismisses
    palette.reset();
    let toggle = palette.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    assert_eq!(toggle, Some(hkask_tui::WindowKind::Chat));
}

#[test]
fn command_palette_backspace_clears_filter() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use hkask_tui::command_palette::CommandPalette;

    let mut palette = CommandPalette::new();

    // "xyz" matches no window kind
    palette.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    palette.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
    palette.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    assert!(
        palette.selected_kind().is_none(),
        "unmatched filter returns None"
    );

    // Backspace clears filter, all items show again
    palette.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    palette.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    palette.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    assert!(
        palette.selected_kind().is_some(),
        "empty filter returns first item"
    );
}

// ────────────────────────────────────────────────────────────────
// Companies window — graceful degradation with None bridge
// ────────────────────────────────────────────────────────────────

#[test]
fn companies_renders_without_bridge() {
    let w = CompaniesWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn companies_renders_with_bridge() {
    use hkask_tui::bridges::companies::MockCompaniesBridge;
    let w = CompaniesWindow::new(window_id(), bridge())
        .with_companies_bridge(MockCompaniesBridge::with_sample().arc());
    render_smoke(&w, 80, 24);
}

#[test]
fn companies_all_sections_no_panic_without_bridge() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut w = CompaniesWindow::new(window_id(), bridge());
    let tab = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);
    for _ in 0..5 {
        render_smoke(&w, 80, 24);
        w.handle_key(tab);
    }
}

#[test]
fn companies_all_sections_no_panic_with_bridge() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use hkask_tui::bridges::companies::MockCompaniesBridge;
    let mut w = CompaniesWindow::new(window_id(), bridge())
        .with_companies_bridge(MockCompaniesBridge::with_sample().arc());
    let tab = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);
    for _ in 0..5 {
        render_smoke(&w, 80, 24);
        w.handle_key(tab);
    }
}

// ────────────────────────────────────────────────────────────────
// Research window — snapshot, bridge, MCP tab, graceful degradation
// ────────────────────────────────────────────────────────────────

#[test]
fn research_snapshot_shows_sections() {
    let w = ResearchWindow::new(window_id(), bridge());
    let lines = render_snapshot(&w, 80, 24);
    let text = lines.join("\n");
    assert!(
        text.contains("Search") || text.contains("Research"),
        "Research should show sections"
    );
}

#[test]
fn research_renders_with_bridge() {
    use hkask_tui::bridges::research::MockResearchBridge;
    let w = ResearchWindow::new(window_id(), bridge())
        .with_research_bridge(MockResearchBridge::with_sample().arc());
    render_smoke(&w, 80, 24);
}

#[test]
fn research_empty_shows_placeholder() {
    use hkask_tui::bridges::research::MockResearchBridge;
    let w = ResearchWindow::new(window_id(), bridge())
        .with_research_bridge(MockResearchBridge::new().arc());
    render_smoke(&w, 80, 24);
}

#[test]
fn mcp_tab_research_cycles_sections_and_chat() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use hkask_tui::mcp_tabbed::{McpTab, McpTabbedWindow};
    let mut w = ResearchWindow::new(window_id(), bridge());
    let tab = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);

    assert_eq!(w.active_tab(), McpTab::Data);
    for _ in 0..3 {
        assert!(w.handle_key(tab));
    }
    assert_eq!(w.active_tab(), McpTab::Chat);
    render_smoke(&w, 80, 24);
    assert!(w.handle_key(tab));
    assert_eq!(w.active_tab(), McpTab::Data);
}

#[test]
fn research_all_sections_no_panic_without_bridge() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut w = ResearchWindow::new(window_id(), bridge());
    let tab = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);
    for _ in 0..4 {
        render_smoke(&w, 80, 24);
        w.handle_key(tab);
    }
}

// ────────────────────────────────────────────────────────────────
// Docproc window — snapshot, bridge, MCP tab, graceful degradation
// ────────────────────────────────────────────────────────────────

#[test]
fn docproc_snapshot_shows_sections() {
    let w = DocprocWindow::new(window_id(), bridge());
    let lines = render_snapshot(&w, 80, 24);
    let text = lines.join("\n");
    assert!(
        text.contains("Chunks") || text.contains("Docproc"),
        "Docproc should show sections"
    );
}

#[test]
fn docproc_renders_with_bridge() {
    use hkask_tui::bridges::docproc::MockDocprocBridge;
    let w = DocprocWindow::new(window_id(), bridge())
        .with_docproc_bridge(MockDocprocBridge::with_sample().arc());
    render_smoke(&w, 80, 24);
}

#[test]
fn docproc_empty_shows_placeholder() {
    use hkask_tui::bridges::docproc::MockDocprocBridge;
    let w = DocprocWindow::new(window_id(), bridge())
        .with_docproc_bridge(MockDocprocBridge::new().arc());
    render_smoke(&w, 80, 24);
}

#[test]
fn mcp_tab_docproc_cycles_sections_and_chat() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use hkask_tui::mcp_tabbed::{McpTab, McpTabbedWindow};
    let mut w = DocprocWindow::new(window_id(), bridge());
    let tab = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);

    assert_eq!(w.active_tab(), McpTab::Data);
    for _ in 0..3 {
        assert!(w.handle_key(tab));
    }
    assert_eq!(w.active_tab(), McpTab::Chat);
    assert!(w.handle_key(tab));
    assert_eq!(w.active_tab(), McpTab::Data);
}

#[test]
fn docproc_all_sections_no_panic_without_bridge() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut w = DocprocWindow::new(window_id(), bridge());
    let tab = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);
    for _ in 0..4 {
        render_smoke(&w, 80, 24);
        w.handle_key(tab);
    }
}

// ────────────────────────────────────────────────────────────────
// Replica window — snapshot, bridge, MCP tab, graceful degradation
// ────────────────────────────────────────────────────────────────

#[test]
fn replica_snapshot_shows_content() {
    let w = ReplicaWindow::new(window_id(), bridge());
    let lines = render_snapshot(&w, 80, 24);
    let text = lines.join("\n");
    assert!(!text.is_empty(), "Replica should render content");
}

#[test]
fn replica_renders_with_bridge() {
    use hkask_tui::bridges::replica::MockReplicaBridge;
    let w = ReplicaWindow::new(window_id(), bridge())
        .with_replica_bridge(MockReplicaBridge::with_sample().arc());
    render_smoke(&w, 80, 24);
}

#[test]
fn replica_empty_shows_placeholder() {
    use hkask_tui::bridges::replica::MockReplicaBridge;
    let w = ReplicaWindow::new(window_id(), bridge())
        .with_replica_bridge(MockReplicaBridge::new().arc());
    render_smoke(&w, 80, 24);
}

#[test]
fn mcp_tab_replica_toggles_chat() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use hkask_tui::mcp_tabbed::{McpTab, McpTabbedWindow};
    let mut w = ReplicaWindow::new(window_id(), bridge());
    let right = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);
    let left = KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE);

    assert_eq!(w.active_tab(), McpTab::Data);
    assert!(w.handle_key(right));
    assert_eq!(w.active_tab(), McpTab::Chat);
    assert!(w.handle_key(left));
    assert_eq!(w.active_tab(), McpTab::Data);
}

// ────────────────────────────────────────────────────────────────
// Skills window — extended MCP tab and bridge tests
// ────────────────────────────────────────────────────────────────

#[test]
fn skills_renders_with_skills_bridge() {
    use hkask_tui::bridges::skills::MockSkillsBridge;
    let w = SkillsWindow::new(window_id(), bridge())
        .with_skills_bridge(MockSkillsBridge::with_sample().arc());
    render_smoke(&w, 80, 24);
}

#[test]
fn mcp_tab_skills_cycles_sections_and_chat() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use hkask_tui::mcp_tabbed::{McpTab, McpTabbedWindow};
    let mut w = SkillsWindow::new(window_id(), bridge());
    let tab = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);

    assert_eq!(w.active_tab(), McpTab::Data);
    for _ in 0..3 {
        assert!(w.handle_key(tab));
    }
    assert_eq!(w.active_tab(), McpTab::Chat);
    assert!(w.handle_key(tab));
    assert_eq!(w.active_tab(), McpTab::Data);
}

#[test]
fn skills_all_sections_no_panic_without_bridge() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut w = SkillsWindow::new(window_id(), bridge());
    let tab = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);
    for _ in 0..4 {
        render_smoke(&w, 80, 24);
        w.handle_key(tab);
    }
}
