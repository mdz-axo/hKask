//! MCP Two-Tab window trait — unified Chat + Data architecture.
//!
//! MCP-focused windows (Kanban, Training, Media, Matrix, Memory, Companies)
//! adopt a two-tab design per TUI_SPECIFICATION.md §3:
//! - Tab 1 (Chat): Focused chat scoped to one MCP server's tools
//! - Tab 2 (Data): Structured UI widgets rendering MCP artifacts
//!
//! Toggle between tabs with the `Tab` key.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Style, Stylize};
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
}

impl McpChatState {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
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
        self.messages.push(McpChatMessage {
            role: "user".to_string(),
            text,
        });
    }

    /// Add an agent message to history.
    pub fn push_agent(&mut self, text: String) {
        self.messages.push(McpChatMessage {
            role: "agent".to_string(),
            text,
        });
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
        lines.push(Line::from(Span::styled(
            format!("── {} Chat (Tab to switch) ──", mcp_name),
            Style::default().fg(Color::Magenta).bold(),
        )));
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

    /// Default Tab key handler — toggles between Chat and Data.
    fn handle_tab_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Tab {
            let next = self.active_tab().next();
            self.set_active_tab(next);
            return true;
        }
        false
    }
}
