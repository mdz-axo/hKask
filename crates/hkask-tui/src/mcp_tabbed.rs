//! MCP Two-Tab window trait — unified Chat + Data architecture.
//!
//! MCP-focused windows share scoped Chat and structured Data behavior. Most
//! toggle those views with `Tab`; Scenarios composes the same trait with its
//! own section navigation. See `docs/explanation/tui-architecture.md` for the
//! runtime boundary and current limitations.

use crate::repl_bridge::{InferenceRequestId, InferenceState, ReplBridge};
use crate::widgets::headers;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::window::Window;

/// Which tab is active in an MCP two-tab window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpTab {
    /// Focused chat scoped to this window's MCP server.
    Chat,
    /// Structured data widgets rendering MCP artifacts.
    Data,
}

impl McpTab {
    pub fn next(self) -> Self {
        match self {
            Self::Chat => Self::Data,
            Self::Data => Self::Chat,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Chat => "Chat",
            Self::Data => "Data",
        }
    }
}

/// State for the scoped chat input within an MCP window.
#[derive(Debug, Clone)]
pub struct McpChatState {
    /// Current input buffer for the chat tab.
    pub input: String,
    /// History of chat messages (user + agent).
    pub messages: Vec<McpChatMessage>,
    /// Scoped inference request owned by this window.
    pending_request: Option<InferenceRequestId>,
}

impl McpChatState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            pending_request: None,
        }
    }

    /// Clear the input buffer.
    pub fn clear_input(&mut self) {
        self.input.clear();
    }

    /// Take the current input and return it, clearing the buffer.
    pub fn take_input(&mut self) -> String {
        std::mem::take(&mut self.input)
    }

    /// Add a user message to history.
    pub fn push_user(&mut self, text: String) {
        self.trim_if_needed();
        self.messages.push(McpChatMessage {
            role: "user".to_string(),
            text,
        });
    }

    /// Add an agent message to history.
    pub fn push_agent(&mut self, text: String) {
        self.trim_if_needed();
        self.messages.push(McpChatMessage {
            role: "agent".to_string(),
            text,
        });
    }

    /// Drop oldest messages if history exceeds MAX_MESSAGES.
    fn trim_if_needed(&mut self) {
        const MAX_MESSAGES: usize = 500;
        if self.messages.len() >= MAX_MESSAGES {
            let excess = self.messages.len() - MAX_MESSAGES + 1;
            self.messages.drain(0..excess);
        }
    }
}

/// A single chat message in the MCP-scoped chat tab.
#[derive(Debug, Clone)]
pub struct McpChatMessage {
    pub role: String,
    pub text: String,
}

/// Extension trait for windows that support MCP two-tab layout.
///
/// Windows implementing this trait get a `Tab` key handler that toggles
/// between Chat and Data tabs. The Chat tab provides scoped chat input
/// that includes the window's MCP server context.
pub trait McpTabbedWindow: Window {
    /// Which tab is currently active.
    fn active_tab(&self) -> McpTab;

    /// Set the active tab.
    fn set_active_tab(&mut self, tab: McpTab);

    /// Mutable access to the chat state.
    fn chat_state_mut(&mut self) -> &mut McpChatState;

    /// The name of the MCP server this window is scoped to.
    /// Used in the chat prefix to tell the model which tools are available.
    fn mcp_server_name(&self) -> &str;

    /// Render the Chat tab content.
    fn render_chat_tab(&self, f: &mut Frame, area: Rect);

    /// Render the Data tab content (existing structured view).
    fn render_data_tab(&self, f: &mut Frame, area: Rect);

    /// Handle key events for the Chat tab.
    /// Returns the user message when Enter is pressed with non-empty input.
    fn handle_chat_key(&mut self, key: KeyEvent) -> Option<String> {
        let state = self.chat_state_mut();
        match key.code {
            KeyCode::Char(c) => {
                state.input.push(c);
                None
            }
            KeyCode::Backspace => {
                state.input.pop();
                None
            }
            KeyCode::Enter if state.pending_request.is_some() => None,
            KeyCode::Enter => {
                let text = state.take_input();
                if text.is_empty() {
                    None
                } else {
                    state.push_user(text.clone());
                    Some(text)
                }
            }
            KeyCode::Esc => {
                state.clear_input();
                None
            }
            _ => None,
        }
    }

    /// Start a scoped request and bind its completion to this window.
    fn start_chat_request(&mut self, bridge: &dyn ReplBridge, input: String) {
        let request = bridge.start_scoped_inference(input, self.mcp_server_name());
        self.chat_state_mut().pending_request = Some(request);
    }

    /// Poll only the request owned by this window.
    fn poll_chat_request(&mut self, bridge: &dyn ReplBridge) {
        let Some(request) = self.chat_state_mut().pending_request else {
            return;
        };
        match bridge.poll_inference(request) {
            InferenceState::Done(result) => {
                let text = if result.budget_exhausted {
                    "Gas budget exhausted — scoped turn blocked.".to_string()
                } else {
                    result.text
                };
                let state = self.chat_state_mut();
                state.push_agent(text);
                state.pending_request = None;
            }
            InferenceState::Idle => {
                self.chat_state_mut().pending_request = None;
            }
            InferenceState::Thinking => {}
        }
    }

    /// Default render for the Chat tab.
    fn default_render_chat_tab(
        chat_state: &McpChatState,
        mcp_name: &str,
        f: &mut Frame,
        area: Rect,
    ) {
        let inner = area.inner(Margin::new(1, 1));
        let mut lines: Vec<Line> = Vec::new();

        // Header
        lines.push(headers::section_with_color(
            format!("{} Chat (Tab to switch)", mcp_name),
            Color::Magenta,
        ));
        lines.push(Line::from(Span::styled(
            format!("   Tool scope: {} MCP server", mcp_name),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));

        // Chat history (last 10 messages)
        let history_start = chat_state.messages.len().saturating_sub(10);
        for msg in &chat_state.messages[history_start..] {
            let role_style = if msg.role == "user" {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Green)
            };
            let prefix = if msg.role == "user" {
                "REPL ▸ "
            } else {
                "  ←   "
            };
            for line_text in msg.text.lines() {
                lines.push(Line::from(vec![
                    Span::styled(prefix, role_style),
                    Span::raw(line_text),
                ]));
            }
        }

        // Input prompt
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("REPL ▸ {}", chat_state.input),
            Style::default().fg(Color::Cyan),
        )));

        let para = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(para, inner);
    }
}

/// Implement `McpTabbedWindow` for a window whose fields are named
/// `active_tab: McpTab` and `chat_state: McpChatState`.
///
/// Generates the three identical getters plus `mcp_server_name` (from the
/// literal `$mcp_name`) and `render_chat_tab` (delegating to
/// `default_render_chat_tab`). The caller supplies a closure for
/// `render_data_tab` that receives `(&self, &mut Frame, Rect)`.
///
/// # Why a macro, not trait defaults
///
/// `active_tab`/`set_active_tab`/`chat_state_mut` must read private fields
/// (`self.active_tab`, `self.chat_state`) of each concrete window struct, so
/// they cannot have trait-level default implementations. 11 windows share
/// this exact pattern; the macro prevents drift and keeps the trait surface
/// honest about which methods are actually window-specific.
///
/// # Example
///
/// ```ignore
/// impl_mcp_tabbed!(ReplicaWindow, "replica", |this, f, area| {
///     let mut lines = vec![headers::section("Replica ([ ] Chat/Data)"), Line::from("")];
///     f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
/// });
/// ```
#[macro_export]
macro_rules! impl_mcp_tabbed {
    (
        $window:ty,
        $mcp_name:literal,
        |$this:ident, $f:ident, $area:ident| $body:block
    ) => {
        impl $crate::mcp_tabbed::McpTabbedWindow for $window {
            fn active_tab(&self) -> $crate::mcp_tabbed::McpTab {
                self.active_tab
            }
            fn set_active_tab(&mut self, tab: $crate::mcp_tabbed::McpTab) {
                self.active_tab = tab;
            }
            fn chat_state_mut(&mut self) -> &mut $crate::mcp_tabbed::McpChatState {
                &mut self.chat_state
            }
            fn mcp_server_name(&self) -> &str {
                $mcp_name
            }
            fn render_chat_tab(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
                Self::default_render_chat_tab(&self.chat_state, $mcp_name, f, area);
            }
            fn render_data_tab(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
                let $this = self;
                let $f = f;
                let $area = area;
                $body
            }
        }
    };
}
