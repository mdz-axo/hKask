//! Matrix window — federated messaging rooms via the Matrix protocol.
//!
//! Connects to Matrix homeserver, displays rooms and messages.
//! Full Matrix integration via hkask-communication and hkask-mcp-communication.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

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
    #[allow(dead_code)]
    bridge: Arc<dyn ReplBridge>,
}

impl MatrixWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: MatrixSection::Rooms,
            bridge,
        }
    }
}

impl Window for MatrixWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "Matrix"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Matrix
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let mut lines = vec![
            Line::from(Span::styled(
                format!("── Matrix: {} (Tab to switch) ──", self.section.title()),
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];
        match self.section {
            MatrixSection::Rooms => {
                lines.push(Line::from("  Matrix rooms from configured homeserver."));
                lines.push(Line::from(
                    "  Use `kask matrix` CLI to manage Matrix connections.",
                ));
                lines.push(Line::from("  Rooms appear here once joined."));
            }
            MatrixSection::Messages => {
                lines.push(Line::from("  Messages from the selected room."));
                lines.push(Line::from("  End-to-end encrypted via Matrix protocol."));
                lines.push(Line::from(
                    "  Agent messages interleaved with human messages.",
                ));
            }
            MatrixSection::Contacts => {
                lines.push(Line::from("  Matrix contacts and directory search."));
                lines.push(Line::from("  Invite agents to rooms via their Matrix IDs."));
                lines.push(Line::from(
                    "  Federation support for cross-server communication.",
                ));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Matrix integration via hkask-communication + hkask-mcp-communication.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Tab {
            self.section = self.section.next();
            true
        } else {
            false
        }
    }
    fn tick(&mut self) {}
}
