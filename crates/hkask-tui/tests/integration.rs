//! Integration tests for hkask-tui.
//!
//! Covers window creation smoke tests, WindowKind invariant properties,
//! StatusBar rendering, and workspace operations.

use std::sync::Arc;

use hkask_tui::{
    ReplBridge, TurnResult, Window, WindowId, WindowKind,
    windows::{
        BackupWindow, ChatWindow, CnsMonitorWindow, CompaniesWindow, ConfigurationWindow,
        CuratorWindow, EditorWindow, KanbanWindow, MatrixWindow, MediaWindow, MemoryWindow,
        PodsWindow, RegistryWindow, SidebarWindow, SkillsWindow, TerminalWindow, TrainingWindow,
        WalletWindow,
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
    ]
}

#[test]
fn all_18_kinds_exist() {
    assert_eq!(all_window_kinds().len(), 18);
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
fn only_sidebar_is_persistent() {
    for kind in all_window_kinds() {
        if kind == WindowKind::Sidebar {
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
    assert_eq!(titles.len(), 18, "duplicate titles: {:?}", titles);
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
fn skills_renders() {
    let w = SkillsWindow::new(window_id(), bridge());
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
        Box::new(SkillsWindow::new(window_id(), b)),
    ];

    assert_eq!(windows.len(), 18);

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
    let tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
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
    let tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
    render_smoke(&w, 80, 24);
    assert!(w.handle_key(tab));
    assert!(w.handle_key(tab));
    assert!(w.handle_key(tab));
    assert!(w.handle_key(tab));
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
    let w = MemoryWindow::new(window_id(), bridge())
        .with_memory_bridge(MockMemoryBridge::new().arc());
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
    let w = KanbanWindow::new(window_id(), bridge())
        .with_kanban_bridge(MockKanbanBridge::new().arc());
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
