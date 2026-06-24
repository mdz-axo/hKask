//! Kanban window — task board for agent coordination.
//!
//! `]` forward, `[` backward through Board→Backlog→InProgress→Done→Chat.

use crate::bridges::KanbanDataBridge;
use crate::mcp_tabbed::{McpChatState, McpTab, McpTabbedWindow};
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use std::sync::Arc;

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
    fn prev(self) -> Self {
        match self {
            Self::Board => Self::Done,
            Self::Backlog => Self::Board,
            Self::InProgress => Self::Backlog,
            Self::Done => Self::InProgress,
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
    active_tab: McpTab,
    chat_state: McpChatState,
    bridge: Arc<dyn ReplBridge>,
    kanban: Option<Arc<dyn KanbanDataBridge>>,
}

impl KanbanWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: KanbanSection::Board,
            active_tab: McpTab::Data,
            chat_state: McpChatState::new(),
            bridge,
            kanban: None,
        }
    }
    pub fn with_kanban_bridge(mut self, kb: Arc<dyn KanbanDataBridge>) -> Self {
        self.kanban = Some(kb);
        self
    }
}

impl Window for KanbanWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        match self.active_tab {
            McpTab::Chat => "Kanban Chat",
            McpTab::Data => "Kanban",
        }
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Kanban
    }
    fn render(&self, f: &mut Frame, area: Rect, _: bool) {
        match self.active_tab {
            McpTab::Chat => Self::default_render_chat_tab(&self.chat_state, "kanban", f, area),
            McpTab::Data => self.render_data_tab(f, area),
        }
    }
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(']') => {
                match self.active_tab {
                    McpTab::Chat => {
                        self.active_tab = McpTab::Data;
                        self.section = KanbanSection::Board;
                    }
                    McpTab::Data => {
                        self.section = self.section.next();
                        if self.section == KanbanSection::Board {
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
                        self.section = KanbanSection::Done;
                    }
                    McpTab::Data => {
                        self.section = self.section.prev();
                        if self.section == KanbanSection::Done {
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
                    self.bridge
                        .start_scoped_inference(msg, self.mcp_server_name());
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

impl McpTabbedWindow for KanbanWindow {
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
        "kanban"
    }
    fn render_chat_tab(&self, f: &mut Frame, area: Rect) {
        Self::default_render_chat_tab(&self.chat_state, "kanban", f, area);
    }
    fn render_data_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(Span::styled(
                format!("── Kanban: {} ([ ] to navigate) ──", self.section.title()),
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];
        if let Some(ref kb) = self.kanban {
            let boards = kb.board_list();
            let counts = kb.status_counts();
            let bk = kb.tasks_by_status("backlog", 20);
            let ip = kb.tasks_by_status("in_progress", 20);
            let dn = kb.tasks_by_status("done", 20);
            match self.section {
                KanbanSection::Board => {
                    if let Some(ref board) = boards.first() {
                        lines.push(Line::from(format!(
                            "  Board: {} ({})",
                            board.name, board.id
                        )));
                        lines.push(Line::from(format!("  Total tasks: {}", board.task_count)));
                    }
                    lines.push(Line::from(format!(
                        "  Backlog: {}   In Progress: {}   Review: {}   Done: {}",
                        counts.backlog, counts.in_progress, counts.review, counts.done
                    )));
                }
                KanbanSection::Backlog => {
                    lines.push(Line::from(format!(
                        "  {} task(s) in backlog",
                        counts.backlog
                    )));
                    for t in &bk {
                        let title = t.title.clone();
                        let prio = t.priority.as_deref().unwrap_or("-");
                        let c = match prio {
                            "critical" | "high" => Color::Red,
                            "medium" => Color::Yellow,
                            _ => Color::DarkGray,
                        };
                        lines.push(Line::from(vec![
                            Span::raw("  • "),
                            Span::styled(title, Style::default().fg(Color::White)),
                            Span::styled(format!("  [{}]", prio), Style::default().fg(c)),
                        ]));
                    }
                }
                KanbanSection::InProgress => {
                    lines.push(Line::from(format!(
                        "  {} task(s) in progress",
                        counts.in_progress
                    )));
                    for t in &ip {
                        let title = t.title.clone();
                        let a = t.assignee.as_deref().unwrap_or("unassigned");
                        lines.push(Line::from(vec![
                            Span::raw("  • "),
                            Span::styled(title, Style::default().fg(Color::Yellow)),
                            Span::styled(format!("  [{}]", a), Style::default().fg(Color::Cyan)),
                        ]));
                    }
                }
                KanbanSection::Done => {
                    lines.push(Line::from(format!("  {} task(s) completed", counts.done)));
                    for t in &dn {
                        let title = t.title.clone();
                        lines.push(Line::from(vec![
                            Span::raw("  ✓ "),
                            Span::styled(title, Style::default().fg(Color::Green)),
                        ]));
                    }
                }
            }
        } else {
            match self.section {
                KanbanSection::Board => lines.push(Line::from("  No kanban service connected.")),
                _ => lines.push(Line::from("  No kanban service connected.")),
            }
        }
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}
