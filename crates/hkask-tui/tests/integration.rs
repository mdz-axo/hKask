//! Integration tests for hkask-tui.
//!
//! Covers Chat window rendering, inference ownership, and workspace
//! operations (status bar, layout persistence, keybindings). The TUI now
//! hosts only the Chat window; window-kind invariant tests are limited to
//! the single remaining kind.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use hkask_tui::{
    InferenceRequestId, InferenceState, ModelSwitchResult, ReplBridge, SessionBridge,
    SettingsBridge, SystemBridge, TuiModelInfo, TuiTurnResult, Window, WindowId, WindowKind,
    Workspace, windows::ChatWindow,
};

/// A minimal ReplBridge for testing — returns safe defaults for all methods.
struct MockBridge {
    agent_name: String,
    model_name: String,
    gas_remaining: u64,
    gas_cap: u64,
    reg_alerts: u32,
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
            reg_alerts: 0,
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
    fn reg_alert_count(&self) -> u32 {
        self.reg_alerts
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
    fn poll_inference(&self, request: InferenceRequestId) -> InferenceState {
        self.pending
            .lock()
            .expect("pending lock")
            .remove(&request)
            .map(InferenceState::Done)
            .unwrap_or(InferenceState::Idle)
    }
    fn streaming_text(&self, _request: InferenceRequestId) -> String {
        String::new()
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
    vec![WindowKind::Chat]
}

#[test]
fn chat_kind_is_the_only_kind() {
    assert_eq!(all_window_kinds().len(), 1);
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
fn chat_allows_multiple() {
    for kind in all_window_kinds() {
        assert!(kind.allows_multiple(), "{:?} should allow multiple", kind);
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
    assert_eq!(titles.len(), 1, "duplicate titles: {:?}", titles);
}

// ────────────────────────────────────────────────────────────────
// Rendering smoke tests — Chat renders without panicking
// ────────────────────────────────────────────────────────────────

#[test]
fn chat_renders() {
    let b = bridge();
    let w = ChatWindow::new(window_id(), b.userpod_name(), b.model_name(), b.clone());
    render_smoke(&w, 80, 24);
}

#[test]
fn chat_renders_at_multiple_sizes() {
    let b = bridge();
    let sizes = [(40, 12), (80, 24), (120, 40), (200, 80)];
    for (w, h) in sizes {
        let window = ChatWindow::new(window_id(), b.userpod_name(), b.model_name(), b.clone());
        render_smoke(&window, w, h);
    }
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
    // Default mode is Chat — the prompt should show REPL.
    assert!(
        text.contains("REPL"),
        "Chat should show REPL prompt in default Chat mode; got: {}",
        text
    );
}

#[test]
fn chat_windows_receive_only_their_owned_inference() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let b = bridge();
    let mut first = ChatWindow::new(window_id(), b.userpod_name(), b.model_name(), b.clone());
    let mut second = ChatWindow::new(window_id(), b.userpod_name(), b.model_name(), b.clone());

    // Switch both windows from default Curator mode to Chat mode via /repl
    for c in "/repl".chars() {
        first.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    first.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    for c in "/repl".chars() {
        second.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }
    second.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

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
fn chat_handles_multibyte_cursor_operations() {
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

    for key in keys {
        let event = KeyEvent::new(key, KeyModifiers::NONE);
        chat.handle_key(event);
    }

    render_smoke(&chat, 80, 24);
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
        "Workspace should render Chat REPL prompt (default Chat mode)"
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
