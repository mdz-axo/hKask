//! Kanban window — task board for agent coordination.
//!
//! Displays kanban board with columns (Backlog, In Progress, Review, Done).
//! Each card is a task that can be assigned to agents. Integrates with
//! hkask-services-kanban for task lifecycle management.
//!
//! # Architecture
//! ⟨Kanban⟩ displays ⟨Columns, Cards, Assignments⟩ .
//! ⟨Kanban⟩ integratesWith ⟨hkask-services-kanban⟩ .
//!
//! # MCP Two-Tab Design (future)
//! Tab 1: Kanban chat — focused chat using kanban MCP tools
//! Tab 2: Board view — visual kanban board with drag-drop cards

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
enum KanbanSection {
    Board,
    Backlog,
    InProgress,
    Done,
}

impl KanbanSection {
    fn next(self) -> Self {
        match self {
            Self::Board => Self::Backlog,
            Self::Backlog => Self::InProgress,
            Self::InProgress => Self::Done,
            Self::Done => Self::Board,
        }
    }
    fn title(&self) -> &str {
        match self {
            Self::Board => "Board",
            Self::Backlog => "Backlog",
            Self::InProgress => "In Progress",
            Self::Done => "Done",
        }
    }
}

pub struct KanbanWindow {
    id: WindowId,
    section: KanbanSection,
    #[allow(dead_code)]
    bridge: Arc<dyn ReplBridge>,
}

impl KanbanWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: KanbanSection::Board,
            bridge,
        }
    }
}

impl Window for KanbanWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "Kanban"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Kanban
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let mut lines = vec![
            Line::from(Span::styled(
                format!("── Kanban: {} (Tab to switch) ──", self.section.title()),
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];
        match self.section {
            KanbanSection::Board => {
                lines.push(Line::from(
                    "  Columns: Backlog | In Progress | Review | Done",
                ));
                lines.push(Line::from(
                    "  Each card is a task assigned to an agent pod.",
                ));
                lines.push(Line::from("  Cards flow left-to-right as work progresses."));
                lines.push(Line::from(""));
                lines.push(Line::from("  Use `kask kanban` CLI for board management."));
            }
            KanbanSection::Backlog => {
                lines.push(Line::from("  Tasks awaiting assignment."));
                lines.push(Line::from(
                    "  Created via /kanban create or kata PDCA cycles.",
                ));
            }
            KanbanSection::InProgress => {
                lines.push(Line::from("  Tasks currently being worked on."));
                lines.push(Line::from(
                    "  Agent pods execute tasks within their OCAP boundary.",
                ));
            }
            KanbanSection::Done => {
                lines.push(Line::from("  Completed tasks with verification status."));
                lines.push(Line::from("  Triggers memory consolidation on completion."));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Kanban integrates with Kata coaching loop for task-scoped scientific thinking.",
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
