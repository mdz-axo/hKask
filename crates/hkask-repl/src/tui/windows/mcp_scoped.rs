//! Shared base for MCP-scoped windows.
//!
//! Each MCP window (Kanban, Companies, Scenarios) provides a dedicated
//! pane where user queries are answered by the LLM using only the
//! tools from one MCP server. This is the `start_scoped_inference`
//! pattern — the LLM acts as an intelligent intermediary that calls
//! the appropriate MCP tools.
//!
//! Future enhancement: add `invoke_mcp_tool` to ReplBridge for direct
//! tool invocation without an LLM round-trip (Phase 3 of the multi-
//! window plan).

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui::repl_bridge::{InferenceRequestId, InferenceState, ReplBridge};
use crate::tui::text_cursor;
use crate::tui::window::{Window, WindowId, WindowKind, WorkspaceAction};

/// A message in the MCP window's conversation log.
#[derive(Debug, Clone)]
struct McpMessage {
    sender: McpSender,
    content: String,
}

#[derive(Debug, Clone, Copy)]
enum McpSender {
    User,
    Agent,
    System,
}

impl McpSender {
    fn prefix(&self) -> &'static str {
        match self {
            McpSender::User => "You",
            McpSender::Agent => "Agent",
            McpSender::System => "System",
        }
    }

    fn color(&self) -> Color {
        match self {
            McpSender::User => Color::Green,
            McpSender::Agent => Color::Cyan,
            McpSender::System => Color::Yellow,
        }
    }
}

/// Shared state for an MCP-scoped window.
pub(crate) struct McpScopedState {
    pub(crate) id: WindowId,
    pub(crate) kind: WindowKind,
    pub(crate) title: String,
    pub(crate) mcp_server: String,
    pub(crate) bridge: Arc<dyn ReplBridge>,
    pub(crate) messages: Vec<McpMessage>,
    pub(crate) input: String,
    pub(crate) cursor_pos: usize,
    pub(crate) scroll_offset: u16,
    pending_request: Option<InferenceRequestId>,
    spinner_frame: u8,
    pending_actions: Vec<WorkspaceAction>,
}

impl McpScopedState {
    pub(crate) fn new(
        id: WindowId,
        kind: WindowKind,
        title: &str,
        mcp_server: &str,
        bridge: Arc<dyn ReplBridge>,
        welcome: &str,
    ) -> Self {
        Self {
            id,
            kind,
            title: title.to_string(),
            mcp_server: mcp_server.to_string(),
            bridge,
            messages: vec![McpMessage {
                sender: McpSender::System,
                content: welcome.to_string(),
            }],
            input: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
            pending_request: None,
            spinner_frame: 0,
            pending_actions: Vec::new(),
        }
    }

    fn add_message(&mut self, sender: McpSender, content: String) {
        self.messages.push(McpMessage { sender, content });
        self.scroll_offset = 0;
    }

    fn send_input(&mut self) {
        if self.pending_request.is_some() {
            return;
        }
        let input = std::mem::take(&mut self.input);
        self.cursor_pos = 0;
        if input.is_empty() {
            return;
        }

        // Check for local slash commands.
        if input.starts_with('/') {
            self.handle_slash(&input);
            return;
        }

        self.add_message(McpSender::User, input.clone());
        let req = self.bridge.start_scoped_inference(input, &self.mcp_server);
        self.pending_request = Some(req);
    }

    fn handle_slash(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let primary = parts
            .first()
            .map(|s| s.trim_start_matches('/'))
            .unwrap_or("");

        match primary {
            "help" | "?" => {
                self.add_message(
                    McpSender::System,
                    format!(
                        "Commands: /help /close /clear /open <kind> /split h|v /focus\n\
                         This window is scoped to the {} MCP server.",
                        self.mcp_server
                    ),
                );
            }
            "close" => {
                self.pending_actions.push(WorkspaceAction::CloseFocused);
            }
            "clear" => {
                self.messages.clear();
                self.scroll_offset = 0;
            }
            "open" => {
                if let Some(kind_str) = parts.get(1) {
                    if let Some(kind) = WindowKind::from_str(kind_str) {
                        self.pending_actions.push(WorkspaceAction::OpenWindow(kind));
                        self.add_message(
                            McpSender::System,
                            format!("Opening {} window...", kind.default_title()),
                        );
                    } else {
                        self.add_message(
                            McpSender::System,
                            format!(
                                "Unknown window kind: {}. Try: chat kanban companies scenarios",
                                kind_str
                            ),
                        );
                    }
                } else {
                    self.add_message(
                        McpSender::System,
                        "Available: /open chat /open kanban /open companies /open scenarios".into(),
                    );
                }
            }
            "split" => {
                let dir = match parts.get(1).copied().unwrap_or("") {
                    "h" | "horizontal" => crate::tui::window::SplitDirection::Horizontal,
                    _ => crate::tui::window::SplitDirection::Vertical,
                };
                self.pending_actions.push(WorkspaceAction::Split(dir));
            }
            "focus" => {
                self.pending_actions.push(WorkspaceAction::FocusNext);
            }
            _ => {
                self.add_message(
                    McpSender::System,
                    format!(
                        "Unknown command: /{}. Type /help for available commands.",
                        primary
                    ),
                );
            }
        }
    }

    pub(crate) fn tick(&mut self) {
        if let Some(req) = self.pending_request {
            let state = self.bridge.poll_inference(req);
            match state {
                InferenceState::Thinking => {
                    self.spinner_frame = self.spinner_frame.wrapping_add(1);
                }
                InferenceState::Done(result) => {
                    self.add_message(McpSender::Agent, result.text);
                    self.pending_request = None;
                }
                InferenceState::Idle => {
                    self.pending_request = None;
                }
            }
        }
    }

    pub(crate) fn render(&self, f: &mut Frame, area: Rect, is_focused: bool) {
        if area.height < 3 || area.width < 10 {
            return;
        }

        // Layout: message log (top) + input line (bottom, 1 line)
        let input_h = 1u16;
        let log_h = area.height.saturating_sub(input_h);
        let log_area = Rect::new(area.x, area.y, area.width, log_h);
        let input_area = Rect::new(area.x, area.y + log_h, area.width, input_h);

        // Render message log
        let mut lines: Vec<Line> = Vec::new();
        for msg in &self.messages {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{}: ", msg.sender.prefix()),
                    Style::default()
                        .fg(msg.sender.color())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(&msg.content),
            ]));
        }

        // Spinner if thinking
        if self.pending_request.is_some() {
            let spinner = match self.spinner_frame % 4 {
                0 => "⠋",
                1 => "⠙",
                2 => "⠹",
                _ => "⠸",
            };
            lines.push(Line::from(vec![
                Span::styled(
                    "Agent: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{} thinking...", spinner),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }

        let log = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));
        f.render_widget(log, log_area);

        // Render input line
        let prompt_style = Style::default().fg(if is_focused {
            Color::Yellow
        } else {
            Color::DarkGray
        });
        let prompt_span = Span::styled("> ", prompt_style);

        let input_line = if is_focused {
            let (before, after) = text_cursor::parts(&self.input, self.cursor_pos);
            Line::from(vec![
                prompt_span,
                Span::raw(before),
                Span::styled("|", Style::default().fg(Color::Yellow)),
                Span::raw(after),
            ])
        } else {
            Line::from(vec![prompt_span, Span::raw(&self.input)])
        };
        f.render_widget(Paragraph::new(input_line), input_area);
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> bool {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Enter) => {
                self.send_input();
                true
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                if self.cursor_pos > 0 {
                    text_cursor::backspace(&mut self.input, &mut self.cursor_pos);
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Delete) => {
                text_cursor::delete(&mut self.input, &mut self.cursor_pos);
                true
            }
            (KeyModifiers::NONE, KeyCode::Left) => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Right) => {
                if self.cursor_pos < self.input.len() {
                    self.cursor_pos += 1;
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Up) => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                self.scroll_offset += 1;
                true
            }
            (KeyModifiers::NONE, KeyCode::Home) => {
                self.cursor_pos = 0;
                true
            }
            (KeyModifiers::NONE, KeyCode::End) => {
                self.cursor_pos = self.input.len();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                text_cursor::insert(&mut self.input, &mut self.cursor_pos, c);
                true
            }
            _ => false,
        }
    }

    pub(crate) fn drain_actions(&mut self) -> Vec<WorkspaceAction> {
        std::mem::take(&mut self.pending_actions)
    }
}
