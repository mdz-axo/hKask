//! Matrix window — federated messaging. Rooms, Messages, Contacts.
//!
//! `]` forward, `[` backward through sections + Chat tab.

use crate::bridges::MatrixDataBridge;
use crate::impl_mcp_tabbed;
use crate::mcp_tabbed::{McpChatState, McpTab, McpTabbedWindow};
use crate::repl_bridge::ReplBridge;
use crate::widgets::headers;
use crate::window::{Window, WindowId, WindowKind};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatrixSection {
    Rooms,
    Messages,
    Contacts,
}
impl MatrixSection {
    fn next(self) -> Self {
        match self {
            Self::Rooms => Self::Messages,
            Self::Messages => Self::Contacts,
            Self::Contacts => Self::Rooms,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Rooms => Self::Contacts,
            Self::Messages => Self::Rooms,
            Self::Contacts => Self::Messages,
        }
    }
    fn title(&self) -> &str {
        match self {
            Self::Rooms => "Rooms",
            Self::Messages => "Messages",
            Self::Contacts => "Contacts",
        }
    }
}

pub struct MatrixWindow {
    id: WindowId,
    section: MatrixSection,
    active_tab: McpTab,
    chat_state: McpChatState,
    bridge: Arc<dyn ReplBridge>,
    matrix: Option<Arc<dyn MatrixDataBridge>>,
}

impl MatrixWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: MatrixSection::Rooms,
            active_tab: McpTab::Data,
            chat_state: McpChatState::new(),
            bridge,
            matrix: None,
        }
    }
    pub fn with_matrix_bridge(mut self, m: Arc<dyn MatrixDataBridge>) -> Self {
        self.matrix = Some(m);
        self
    }
}

impl Window for MatrixWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        match self.active_tab {
            McpTab::Chat => "Matrix Chat",
            McpTab::Data => "Matrix",
        }
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Matrix
    }
    fn render(&self, f: &mut Frame, area: Rect, _: bool) {
        match self.active_tab {
            McpTab::Chat => <MatrixWindow as McpTabbedWindow>::default_render_chat_tab(
                &self.chat_state,
                "matrix",
                f,
                area,
            ),
            McpTab::Data => self.render_data_tab(f, area),
        }
    }
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(']') => {
                match self.active_tab {
                    McpTab::Chat => {
                        self.active_tab = McpTab::Data;
                        self.section = MatrixSection::Rooms;
                    }
                    McpTab::Data => {
                        self.section = self.section.next();
                        if self.section == MatrixSection::Rooms {
                            self.active_tab = McpTab::Chat;
                        }
                    }
                }
                return true;
            }
            KeyCode::Char('[') => {
                match self.active_tab {
                    McpTab::Chat => {
                        self.active_tab = McpTab::Data;
                        self.section = MatrixSection::Contacts;
                    }
                    McpTab::Data => {
                        self.section = self.section.prev();
                        if self.section == MatrixSection::Contacts {
                            self.active_tab = McpTab::Chat;
                        }
                    }
                }
                return true;
            }
            _ => {}
        }
        match self.active_tab {
            McpTab::Chat => {
                if let Some(msg) = self.handle_chat_key(key) {
                    let bridge = self.bridge.clone();
                    self.start_chat_request(bridge.as_ref(), msg);
                    return true;
                }
                matches!(
                    key.code,
                    KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Enter | KeyCode::Esc
                )
            }
            McpTab::Data => false,
        }
    }
    fn tick(&mut self) {
        let bridge = self.bridge.clone();
        self.poll_chat_request(bridge.as_ref());
    }
}

impl_mcp_tabbed!(MatrixWindow, "matrix", |this, f, area| {
    let mut lines = vec![
        headers::section(format!(
            "Matrix: {} ([ ] to navigate)",
            this.section.title()
        )),
        Line::from(""),
    ];
    if let Some(ref m) = this.matrix {
        let cs = m.connection_status();
        let rooms = m.list_rooms();
        if !cs.connected {
            lines.push(Line::from("  Not connected to Matrix."));
        } else {
            match this.section {
                MatrixSection::Rooms => {
                    lines.push(Line::from(format!("  {} room(s)", rooms.len())));
                    for r in &rooms {
                        let title = r.title.clone();
                        let esc = if r.escalated { " ⚠" } else { "" };
                        lines.push(Line::from(vec![
                            Span::raw("  🏠 "),
                            Span::styled(title, Style::default().fg(Color::Green)),
                            Span::styled(esc, Style::default().fg(Color::Red)),
                        ]));
                    }
                }
                MatrixSection::Messages => {
                    if let Some(ref first) = rooms.first() {
                        let msgs = m.recent_messages(&first.id, 10);
                        lines.push(Line::from(format!(
                            "  Room: {} — {} recent",
                            first.title,
                            msgs.len()
                        )));
                        for msg in &msgs {
                            let sender = msg.sender.clone();
                            let body: String = if msg.body.len() > 60 {
                                format!("{}...", &msg.body[..60])
                            } else {
                                msg.body.clone()
                            };
                            lines.push(Line::from(vec![
                                Span::styled(
                                    format!("{}", msg.timestamp),
                                    Style::default().fg(Color::DarkGray),
                                ),
                                Span::raw(" "),
                                Span::styled(sender, Style::default().fg(Color::Cyan)),
                                Span::raw(": "),
                                Span::styled(body, Style::default().fg(Color::White)),
                            ]));
                        }
                    }
                }
                MatrixSection::Contacts => {
                    lines.push(Line::from(format!(
                        "  Connected as: {}",
                        cs.user_id.as_deref().unwrap_or("unknown")
                    )));
                    lines.push(Line::from(format!("  Homeserver: {}", cs.homeserver)));
                }
            }
        }
    } else {
        lines.push(Line::from("  Matrix rooms for federated communication."));
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
});
