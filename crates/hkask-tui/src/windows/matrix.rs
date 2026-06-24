//! Matrix window — federated messaging in Matrix rooms.
//!
//! Tab-cycled sections: Rooms, Messages, Contacts. Supports multiple
//! instances. Live data from MatrixDataBridge / matrix-sdk.
//!
//! Adopts the MCP two-tab design (TUI_SPECIFICATION.md §3):
//! - Tab 1 (Chat): Focused chat scoped to the Matrix MCP server
//! - Tab 2 (Data): Rooms, Messages, Contacts sections
//!
//! Tab key: cycles Rooms → Messages → Contacts → Chat → Rooms.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::bridges::MatrixDataBridge;
use crate::mcp_tabbed::{McpChatState, McpTab, McpTabbedWindow};
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

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
    #[allow(dead_code)]
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

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        match self.active_tab {
            McpTab::Chat => {
                Self::default_render_chat_tab(&self.chat_state, "matrix", f, area);
            }
            McpTab::Data => self.render_data_tab(f, area),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Tab {
            match self.active_tab {
                McpTab::Chat => {
                    self.active_tab = McpTab::Data;
                    self.section = MatrixSection::Rooms;
                    return true;
                }
                McpTab::Data => {
                    self.section = self.section.next();
                    if self.section == MatrixSection::Rooms {
                        self.active_tab = McpTab::Chat;
                    }
                    return true;
                }
            }
        }

        match self.active_tab {
            McpTab::Chat => {
                if let Some(msg) = self.handle_chat_key(key) {
                    self.bridge.start_scoped_inference(msg, self.mcp_server_name());
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
    fn tick(&mut self) {}
}

impl McpTabbedWindow for MatrixWindow {
    fn active_tab(&self) -> McpTab {
        self.active_tab
    }
    fn set_active_tab(&mut self, tab: McpTab) {
        self.active_tab = tab;
    }
    fn chat_state_mut(&mut self) -> &mut McpChatState {
        &mut self.chat_state
    }
    fn mcp_server_name(&self) -> &str {
        "matrix"
    }

    fn render_chat_tab(&self, f: &mut Frame, area: Rect) {
        Self::default_render_chat_tab(&self.chat_state, "matrix", f, area);
    }

    fn render_data_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(Span::styled(
                format!(
                    "── Matrix: {} (Tab: next | Tab×3: Chat) ──",
                    self.section.title()
                ),
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];

        if let Some(ref m) = self.matrix {
            let cs = m.connection_status();
            if !cs.connected {
                lines.push(Line::from("  Not connected to Matrix."));
                if !cs.homeserver.is_empty() {
                    lines.push(Line::from(format!("  Homeserver: {}", cs.homeserver)));
                }
            } else {
                match self.section {
                    MatrixSection::Rooms => {
                        let rooms = m.list_rooms();
                        lines.push(Line::from(format!("  {} room(s)", rooms.len())));
                        lines.push(Line::from(""));
                        let room_data: Vec<(String, bool, usize, String, String)> = rooms
                            .iter()
                            .map(|r| {
                                (
                                    r.title.clone(),
                                    r.escalated,
                                    r.member_count,
                                    r.id.clone(),
                                    r.last_active.clone(),
                                )
                            })
                            .collect();
                        for (title, escalated, member_count, _id, _last_active) in &room_data {
                            let esc = if *escalated { " ⚠" } else { "" };
                            lines.push(Line::from(vec![
                                Span::raw("  🏠 "),
                                Span::styled(title.clone(), Style::default().fg(Color::Green)),
                                Span::styled(esc, Style::default().fg(Color::Red)),
                                Span::styled(
                                    format!("  ({})", member_count),
                                    Style::default().fg(Color::DarkGray),
                                ),
                            ]));
                        }
                    }
                    MatrixSection::Messages => {
                        let rooms = m.list_rooms();
                        if let Some(ref first) = rooms.first() {
                            let msgs = m.recent_messages(&first.id, 10);
                            lines.push(Line::from(format!(
                                "  Room: {} — {} recent",
                                first.title,
                                msgs.len()
                            )));
                            lines.push(Line::from(""));
                            let msg_data: Vec<(String, String, String)> = msgs
                                .iter()
                                .map(|msg| {
                                    let body_trunc: String = if msg.body.len() > 60 {
                                        format!(
                                            "{}...",
                                            &msg.body[..msg
                                                .body
                                                .char_indices()
                                                .take(60)
                                                .last()
                                                .map(|(i, _)| i)
                                                .unwrap_or(msg.body.len())]
                                        )
                                    } else {
                                        msg.body.clone()
                                    };
                                    (msg.timestamp.to_string(), msg.sender.clone(), body_trunc)
                                })
                                .collect();
                            for (timestamp, sender, body_trunc) in &msg_data {
                                lines.push(Line::from(vec![
                                    Span::styled(
                                        timestamp.clone(),
                                        Style::default().fg(Color::DarkGray),
                                    ),
                                    Span::raw(" "),
                                    Span::styled(sender.clone(), Style::default().fg(Color::Cyan)),
                                    Span::raw(": "),
                                    Span::styled(
                                        body_trunc.clone(),
                                        Style::default().fg(Color::White),
                                    ),
                                ]));
                            }
                        } else {
                            lines.push(Line::from("  No rooms joined."));
                        }
                    }
                    MatrixSection::Contacts => {
                        lines.push(Line::from(format!(
                            "  Connected as: {}",
                            cs.user_id.as_deref().unwrap_or("unknown")
                        )));
                        lines.push(Line::from(format!("  Homeserver: {}", cs.homeserver)));
                        lines.push(Line::from(""));
                        let rooms = m.list_rooms();
                        let room_titles: Vec<String> =
                            rooms.iter().map(|r| r.title.clone()).collect();
                        let room_counts: Vec<usize> =
                            rooms.iter().map(|r| r.member_count).collect();
                        for (title, member_count) in room_titles.iter().zip(room_counts.iter()) {
                            lines.push(Line::from(format!(
                                "  {} — {} member(s)",
                                title, member_count
                            )));
                        }
                        if room_titles.is_empty() {
                            lines.push(Line::from("  No contacts visible."));
                        }
                    }
                }
            }
        } else {
            match self.section {
                MatrixSection::Rooms => {
                    lines.push(Line::from("  Matrix rooms for federated communication."));
                    lines.push(Line::from(
                        "  Use /matrix join #room:server to join a room.",
                    ));
                }
                MatrixSection::Messages => {
                    lines.push(Line::from("  Messages from Matrix rooms."));
                    lines.push(Line::from("  Use /matrix send <body> to send a message."));
                }
                MatrixSection::Contacts => {
                    lines.push(Line::from("  Contacts from connected Matrix rooms."));
                }
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Matrix is used for CuratorPod federated communication (P8).",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}
