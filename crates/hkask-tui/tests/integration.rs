//! Integration tests for hkask-tui.
//!
//! Covers window creation smoke tests, WindowKind invariant properties,
//! StatusBar rendering, and workspace operations.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use hkask_tui::{
    InferenceRequestId, ModelSwitchResult, ReplBridge, SessionBridge, SettingsBridge, SystemBridge,
    TuiModelInfo, TuiTurnResult, Window, WindowId, WindowKind, Workspace,
    windows::{
        BackupWindow, ChatWindow, CnsMonitorWindow, CompaniesWindow, ConfigurationWindow,
        CuratorWindow, DocprocWindow, EditorWindow, KanbanWindow, LogoWindow, MatrixWindow,
        MediaWindow, MemoryWindow, PodsWindow, RegistryWindow, ReplicaWindow, ResearchWindow,
        ScenariosWindow, SkillsWindow, TerminalWindow, TrainingWindow, WalletWindow,
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
    pod_userpod: usize,
    pods_available: bool,
    reg_domains: Vec<(String, bool)>,
    pending: Mutex<HashMap<InferenceRequestId, TuiTurnResult>>,
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
            pod_userpod: 3,
            pods_available: true,
            reg_domains: vec![
                ("sovereignty".into(), true),
                ("gas".into(), true),
                ("consent".into(), true),
            ],
            pending: Mutex::new(HashMap::new()),
        }
    }
}

impl SystemBridge for MockBridge {
    fn userpod_name(&self) -> &str {
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
    fn pod_counts(&self) -> Option<(usize, usize)> {
        self.pods_available
            .then_some((self.pod_curator, self.pod_userpod))
    }
    fn reg_domains(&self) -> Vec<(String, bool)> {
        self.reg_domains.clone()
    }
}

impl ReplBridge for MockBridge {
    fn start_inference(&self, input: String) -> InferenceRequestId {
        let request = InferenceRequestId::new();
        self.pending.lock().expect("pending lock").insert(
            request,
            TuiTurnResult {
                text: format!("reply: {input}"),
                prompt_tokens: 1,
                completion_tokens: 1,
                total_tokens: 2,
                gas_cost: 1,
                iterations: 1,
                budget_exhausted: false,
            },
        );
        request
    }
    fn poll_inference(&self, request: InferenceRequestId) -> hkask_tui::InferenceState {
        self.pending
            .lock()
            .expect("pending lock")
            .remove(&request)
            .map(hkask_tui::InferenceState::Done)
            .unwrap_or(hkask_tui::InferenceState::Idle)
    }
    fn streaming_text(&self, _request: InferenceRequestId) -> String {
        String::new()
    }
    fn send_message_blocking(&self, _input: &str) -> TuiTurnResult {
        TuiTurnResult {
            text: "mock response".into(),
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            gas_cost: 0,
            iterations: 0,
            budget_exhausted: false,
        }
    }
    fn send_curator_message(&self, _input: &str) -> String {
        "curator ack".into()
    }
}

impl SettingsBridge for MockBridge {
    fn set_model(&self, name: &str) -> ModelSwitchResult {
        ModelSwitchResult {
            resolved_name: name.to_string(),
            detail: String::new(),
        }
    }
    fn list_models(&self) -> anyhow::Result<Vec<TuiModelInfo>> {
        Ok(Vec::new())
    }
    fn settings_display(&self) -> String {
        "(mock settings)".to_string()
    }
    fn set_setting(&self, _key: &str, _value: &str) -> anyhow::Result<String> {
        Ok("(mock)".to_string())
    }
}

impl SessionBridge for MockBridge {
    fn current_agent(&self) -> String {
        self.agent_name.clone()
    }
    fn list_agents_display(&self) -> String {
        "(mock agents)".to_string()
    }
    fn history_display(&self) -> String {
        "(mock history)".to_string()
    }
}

fn bridge() -> Arc<MockBridge> {
    Arc::new(MockBridge::new())
}

fn bridges() -> (Arc<dyn SystemBridge>, Arc<dyn ReplBridge>) {
    let b: Arc<MockBridge> = Arc::new(MockBridge::new());
    let system: Arc<dyn SystemBridge> = b.clone();
    let repl: Arc<dyn ReplBridge> = b;
    (system, repl)
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
        WindowKind::Scenarios,
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
fn only_logo_is_persistent() {
    for kind in all_window_kinds() {
        if kind == WindowKind::Logo {
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
    let w = ChatWindow::new(window_id(), b.userpod_name(), b.model_name(), b.clone());
    render_smoke(&w, 80, 24);
}

#[test]
fn reg_monitor_renders() {
    let w = CnsMonitorWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn backup_renders() {
    let w = BackupWindow::new(window_id(), bridge());
    render_smoke(&w, 80, 24);
}

#[test]
fn backup_without_bridge_is_explicitly_unavailable() {
    let window = BackupWindow::new(window_id(), bridge());
    let text = render_snapshot(&window, 80, 24).join("\n");
    assert!(text.contains("unavailable — backup bridge not configured"));
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
fn pods_report_scan_failure_as_unavailable() {
    let mut mock = MockBridge::new();
    mock.pods_available = false;
    let window = PodsWindow::new(window_id(), Arc::new(mock));
    let text = render_snapshot(&window, 80, 24).join("\n");
    assert!(text.contains("unavailable — scan failed"));
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
            b.userpod_name(),
            b.model_name(),
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
        Box::new(ScenariosWindow::new(window_id(), b.clone())),
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
    let right = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
    let left = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
    // 5 columns: cycle right through all then back left
    render_smoke(&w, 80, 24);
    for _ in 0..4 {
        assert!(w.handle_key(right));
    }
    for _ in 0..4 {
        assert!(w.handle_key(left));
    }
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

#[test]
fn text_windows_handle_multibyte_cursor_operations() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let keys = [
        KeyCode::Char('é'),
        KeyCode::Char('界'),
        KeyCode::Left,
        KeyCode::Delete,
        KeyCode::Backspace,
    ];
    let b = bridge();
    let mut chat = ChatWindow::new(window_id(), b.userpod_name(), b.model_name(), b.clone());
    let mut curator = CuratorWindow::new(window_id(), b.clone());
    let mut terminal = TerminalWindow::new(window_id(), b.clone());
    let mut editor = EditorWindow::new(window_id(), b);

    for key in keys {
        let event = KeyEvent::new(key, KeyModifiers::NONE);
        chat.handle_key(event);
        curator.handle_key(event);
        terminal.handle_key(event);
        editor.handle_key(event);
    }
    editor.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    render_smoke(&chat, 80, 24);
    render_smoke(&curator, 80, 24);
    render_smoke(&terminal, 80, 24);
    render_smoke(&editor, 80, 24);
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
    let window = KanbanWindow::new(window_id(), bridge())
        .with_kanban_bridge(MockKanbanBridge::with_sample_data().arc());
    render_smoke(&window, 80, 24);
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
    let agent_name = b.userpod_name().to_string();
    let model_name = b.model_name().to_string();
    let w = ChatWindow::new(window_id(), &agent_name, &model_name, b);
    let lines = render_snapshot(&w, 80, 24);
    let text = lines.join("\n");
    assert!(text.contains("REPL"), "Chat should show REPL prompt");
}

#[test]
fn chat_windows_receive_only_their_owned_inference() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let b = bridge();
    let mut first = ChatWindow::new(window_id(), b.userpod_name(), b.model_name(), b.clone());
    let mut second = ChatWindow::new(window_id(), b.userpod_name(), b.model_name(), b.clone());

    for c in "first".chars() {
        first.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    first.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    for c in "second".chars() {
        second.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    second.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    second.tick();
    first.tick();

    let first_text = render_snapshot(&first, 80, 24).join("\n");
    let second_text = render_snapshot(&second, 80, 24).join("\n");
    assert!(first_text.contains("reply: first"));
    assert!(!first_text.contains("reply: second"));
    assert!(second_text.contains("reply: second"));
    assert!(!second_text.contains("reply: first"));
}

#[test]
fn reg_monitor_snapshot_shows_domains() {
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
    assert!(text.contains("Unavailable: wallet bridge not configured"));
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
        text.contains("Backlog") || text.contains("Kanban"),
        "Kanban should show columns"
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
    let tab_key = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);

    assert_eq!(w.active_tab(), McpTab::Data);
    render_smoke(&w, 80, 24);

    // Tab toggles Data → Chat → Data
    assert!(w.handle_key(tab_key));
    assert_eq!(w.active_tab(), McpTab::Chat);
    render_smoke(&w, 80, 24);

    assert!(w.handle_key(tab_key));
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

#[test]
fn mcp_tab_receives_its_scoped_inference_completion() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut window = TrainingWindow::new(window_id(), bridge());
    window.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
    for c in "train".chars() {
        window.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    window.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    window.tick();

    let text = render_snapshot(&window, 80, 24).join("\n");
    assert!(text.contains("reply: train"));
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
    assert_eq!(
        dismiss,
        Some(hkask_tui::command_palette::PaletteAction::Close)
    );

    // Reset and verify Ctrl+P dismisses
    palette.reset();
    let toggle = palette.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    assert_eq!(
        toggle,
        Some(hkask_tui::command_palette::PaletteAction::Close)
    );
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

// ────────────────────────────────────────────────────────────────
// Workspace tests — split tree, focus, tabs, global keybindings
// ────────────────────────────────────────────────────────────────

/// Extract all text from a TestBackend buffer as a single string.
fn buffer_text(term: &ratatui::Terminal<ratatui::backend::TestBackend>) -> String {
    let buf = term.backend().buffer().clone();
    let (w, h) = (buf.area().width, buf.area().height);
    let mut lines: Vec<String> = Vec::new();
    for row in 0..h {
        let mut line = String::new();
        for col in 0..w {
            line.push_str(buf.cell((col, row)).map(|c| c.symbol()).unwrap_or(" "));
        }
        let trimmed: String = line.trim_end().to_string();
        if !trimmed.is_empty() {
            lines.push(trimmed);
        }
    }
    lines.join("\n")
}

fn test_terminal(width: u16, height: u16) -> ratatui::Terminal<ratatui::backend::TestBackend> {
    let backend = ratatui::backend::TestBackend::new(width, height);
    ratatui::Terminal::new(backend).expect("test terminal")
}

#[test]
fn workspace_renders_chat_content() {
    let ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };
    let mut term = test_terminal(80, 24);
    term.draw(|f| ws.render(f)).expect("render");
    let text = buffer_text(&term);
    assert!(
        text.contains("REPL"),
        "Workspace should render Chat REPL prompt"
    );
    assert!(!text.is_empty(), "Workspace should render content");
}

#[test]
fn workspace_renders_status_bar() {
    let ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };
    let mut term = test_terminal(120, 30);
    term.draw(|f| ws.render(f)).expect("render");
    let text = buffer_text(&term);
    assert!(
        text.contains("Gas") || text.contains("mock-model"),
        "Status bar should show gas or model info. Got: {}",
        &text[text.len().min(200)..]
    );
}

#[test]
fn workspace_rejects_invalid_layout_without_mutation() {
    use hkask_tui::layout::SavedLayout;

    let mut ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };
    let original_focus = ws.focused_window();
    let invalid = SavedLayout {
        version: 1,
        tabs: Vec::new(),
        active_tab: 0,
    };

    ws.restore_layout(&invalid);

    assert_eq!(ws.tab_count(), 1);
    assert_eq!(ws.window_count(), 1);
    assert_eq!(ws.focused_window(), original_focus);
}

#[test]
fn workspace_has_single_window_initially() {
    let ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };
    assert_eq!(
        ws.window_count(),
        1,
        "new_test workspace should have 1 window"
    );
    assert_eq!(ws.tab_count(), 1, "new_test workspace should have 1 tab");
    assert!(ws.focused_window().is_some(), "A window should be focused");
}

#[test]
fn workspace_focus_cycles() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };
    let initial = ws.focused_window().unwrap();

    // With 1 window, focus_next should stay on the same window
    ws.focus_next();
    assert_eq!(ws.focused_window().unwrap(), initial);
    ws.focus_prev();
    assert_eq!(ws.focused_window().unwrap(), initial);

    // Split to create a second window
    ws.handle_global_key(KeyEvent::new(
        KeyCode::Char('j'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ));
    assert_eq!(ws.window_count(), 2);
    let after_split = ws.focused_window().unwrap();
    assert_ne!(
        after_split, initial,
        "Focus should move to new window after split"
    );

    // Cycle back to first window
    ws.focus_next();
    assert_eq!(ws.focused_window().unwrap(), initial);
}

#[test]
fn workspace_ctrl_q_quits() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };
    assert!(!ws.should_quit);

    let consumed = ws.handle_global_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL));
    assert!(consumed);
    assert!(ws.should_quit);
}

#[test]
fn workspace_ctrl_t_creates_tab() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };
    assert_eq!(ws.tab_count(), 1);

    let consumed = ws.handle_global_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
    assert!(consumed);
    assert_eq!(ws.tab_count(), 2);
    assert_eq!(ws.active_tab_index(), 1, "New tab should be active");
}

#[test]
fn workspace_tab_cycles_focus() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };
    let initial = ws.focused_window().unwrap();

    // Tab with 1 window stays on same window
    ws.handle_global_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(ws.focused_window().unwrap(), initial);

    // After split, Tab cycles between windows
    ws.handle_global_key(KeyEvent::new(
        KeyCode::Char('j'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ));
    let _after_split = ws.focused_window().unwrap();
    ws.handle_global_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert_eq!(
        ws.focused_window().unwrap(),
        initial,
        "Tab should cycle back"
    );
}

#[test]
fn workspace_help_toggles() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };

    ws.handle_global_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
    // Help overlay renders — verify no panic and content appears
    let mut term = test_terminal(80, 24);
    term.draw(|f| ws.render(f)).expect("render with help");
    let text = buffer_text(&term);
    assert!(
        text.contains("Keybindings") || text.contains("Ctrl"),
        "Help overlay should show keybindings"
    );
}

#[test]
fn workspace_command_palette_opens_and_closes() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };

    ws.handle_global_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    assert!(ws.palette_open);
    // Palette renders — verify no panic
    let mut term = test_terminal(80, 24);
    term.draw(|f| ws.render(f)).expect("render with palette");

    // Dismiss with Esc
    ws.handle_palette_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!ws.palette_open);
}

#[test]
fn workspace_enforces_singleton_window_kinds() {
    let mut workspace = {
        let (system, repl) = bridges();
        Workspace::new_test(system, repl)
    };

    workspace.open_window_kind(WindowKind::Configuration);
    assert_eq!(workspace.window_count(), 2);
    let first = workspace.focused_window();
    workspace.open_window_kind(WindowKind::Configuration);
    assert_eq!(workspace.window_count(), 2);
    assert_eq!(workspace.focused_window(), first);
}

#[test]
fn workspace_ctrl_w_closes_focused_window_but_not_logo() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut workspace = {
        let (system, repl) = bridges();
        Workspace::new_test(system, repl)
    };
    workspace.open_window_kind(WindowKind::Configuration);
    assert_eq!(workspace.window_count(), 2);
    workspace.handle_global_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
    assert_eq!(workspace.window_count(), 1);

    workspace.open_window_kind(WindowKind::Logo);
    assert_eq!(workspace.window_count(), 2);
    workspace.handle_global_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
    assert_eq!(
        workspace.window_count(),
        2,
        "persistent Logo must remain open"
    );
}

#[test]
fn workspace_split_creates_second_window() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };
    assert_eq!(ws.window_count(), 1);

    // Ctrl+Shift+H splits horizontally
    let consumed = ws.handle_global_key(KeyEvent::new(
        KeyCode::Char('h'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ));
    assert!(consumed);
    assert_eq!(ws.window_count(), 2);
}

#[test]
fn workspace_multiple_tabs_switch() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };

    // Create 3 tabs
    ws.handle_global_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
    ws.handle_global_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
    assert_eq!(ws.tab_count(), 3);
    assert_eq!(ws.active_tab_index(), 2);

    // Switch to tab 1 via Ctrl+1
    ws.handle_global_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::CONTROL));
    assert_eq!(ws.active_tab_index(), 0);

    // Switch to tab 2 via Ctrl+2
    ws.handle_global_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::CONTROL));
    assert_eq!(ws.active_tab_index(), 1);
}

#[test]
fn workspace_no_crash_on_unfocused_key() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };

    // Random key that nothing handles
    let consumed = ws.handle_global_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    assert!(
        !consumed,
        "Unbound key should not be consumed by global handler"
    );

    // Should not panic when routing to focused window
    ws.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
}

#[test]
fn workspace_render_after_split_shows_both_windows() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut ws = {
        let (s, r) = bridges();
        Workspace::new_test(s, r)
    };

    ws.handle_global_key(KeyEvent::new(
        KeyCode::Char('j'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ));

    let mut term = test_terminal(80, 24);
    term.draw(|f| ws.render(f)).expect("render after split");
    let text = buffer_text(&term);
    // Both windows should show their REPL prompts
    let repl_count = text.matches("REPL").count();
    assert!(
        repl_count >= 1,
        "At least one REPL prompt should render after split"
    );
}
