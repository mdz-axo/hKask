//! Kanban window — task board for agent coordination.
//!
//! Displays kanban board with columns (Backlog, In Progress, Review, Done).
//! Each card is a task that can be assigned to agents.
//!
//! Adopts the MCP two-tab design (TUI_SPECIFICATION.md §3):
//! - Tab 1 (Chat): Focused chat scoped to the Kanban MCP server
//! - Tab 2 (Data): Board, Backlog, InProgress, Done sections
//!
//! Tab key behavior: cycles Board → Backlog → InProgress → Done → Chat → Board.
//!
//! # Architecture
//! ⟨Kanban⟩ displays ⟨Columns, Cards, Assignments⟩ .
//! ⟨Kanban⟩ integratesWith ⟨hkask-services-kanban⟩ .

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::bridges::KanbanDataBridge;
use crate::mcp_tabbed::{McpChatState, McpTab, McpTabbedWindow};
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
    active_tab: McpTab,
    chat_state: McpChatState,
    #[allow(dead_code)]
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
        "Kanban"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Kanban
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        match self.active_tab {
            McpTab::Chat => {
                Self::default_render_chat_tab(&self.chat_state, "kanban", f, area);
            }
            McpTab::Data => self.render_data_tab(f, area),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Tab {
            match self.active_tab {
                McpTab::Chat => {
                    // Tab from Chat → switch to Data (first section)
                    self.active_tab = McpTab::Data;
                    self.section = KanbanSection::Board;
                    return true;
                }
                McpTab::Data => {
                    // Tab from Data: cycle sections, then wrap to Chat
                    self.section = self.section.next();
                    if self.section == KanbanSection::Board {
                        // Wrapped around → switch to Chat tab
                        self.active_tab = McpTab::Chat;
                    }
                    return true;
                }
            }
        }

        match self.active_tab {
            McpTab::Chat => {
                if let Some(_msg) = self.handle_chat_key(key) {
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
                format!(
                    "── Kanban: {} (Tab: next | Tab×4: Chat) ──",
                    self.section.title()
                ),
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];

        if let Some(ref kb) = self.kanban {
            let boards = kb.board_list();
            let counts = kb.status_counts();

            match self.section {
                KanbanSection::Board => {
                    if let Some(ref board) = boards.first() {
                        lines.push(Line::from(vec![
                            Span::raw("  Board: "),
                            Span::styled(
                                format!("{} ({})", board.name, board.id),
                                Style::default().fg(Color::Green),
                            ),
                        ]));
                        lines.push(Line::from(format!("  Total tasks: {}", board.task_count)));
                    }
                    lines.push(Line::from(format!(
                        "  Backlog: {}   In Progress: {}   Review: {}   Done: {}",
                        counts.backlog, counts.in_progress, counts.review, counts.done
                    )));
                    lines.push(Line::from(""));
                    lines.push(Line::from("  Columns:"));
                    if let Some(ref board) = boards.first() {
                        for col in &board.columns {
                            lines.push(Line::from(format!("    • {}", col)));
                        }
                    }
                }
                KanbanSection::Backlog => {
                    lines.push(Line::from(format!(
                        "  {} task(s) in backlog",
                        counts.backlog
                    )));
                    lines.push(Line::from(""));
                    let tasks = kb.tasks_by_status("backlog", 20);
                    if tasks.is_empty() {
                        lines.push(Line::from("  No tasks awaiting assignment."));
                    } else {
                        for t in &tasks {
                            let title = t.title.to_string();
                            let prio = t.priority.as_deref().unwrap_or("-");
                            let color = match prio {
                                "critical" | "high" => Color::Red,
                                "medium" => Color::Yellow,
                                _ => Color::DarkGray,
                            };
                            lines.push(Line::from(vec![
                                Span::raw("  • "),
                                Span::styled(title, Style::default().fg(Color::White)),
                                Span::styled(format!("  [{}]", prio), Style::default().fg(color)),
                            ]));
                            if !t.labels.is_empty() {
                                let labels = t.labels.join(", ");
                                lines.push(Line::from(format!("    labels: {}", labels)));
                            }
                        }
                    }
                }
                KanbanSection::InProgress => {
                    lines.push(Line::from(format!(
                        "  {} task(s) in progress",
                        counts.in_progress
                    )));
                    lines.push(Line::from(""));
                    let tasks = kb.tasks_by_status("in_progress", 20);
                    if tasks.is_empty() {
                        lines.push(Line::from("  No tasks currently being worked on."));
                    } else {
                        for t in &tasks {
                            let title = t.title.to_string();
                            let assignee =
                                t.assignee.as_deref().unwrap_or("unassigned").to_string();
                            lines.push(Line::from(vec![
                                Span::raw("  • "),
                                Span::styled(title, Style::default().fg(Color::Yellow)),
                                Span::styled(
                                    format!("  [{}]", assignee),
                                    Style::default().fg(Color::Cyan),
                                ),
                            ]));
                        }
                    }
                }
                KanbanSection::Done => {
                    lines.push(Line::from(format!("  {} task(s) completed", counts.done)));
                    lines.push(Line::from(""));
                    let tasks = kb.tasks_by_status("done", 20);
                    if tasks.is_empty() {
                        lines.push(Line::from("  No completed tasks."));
                    } else {
                        for t in &tasks {
                            let title = t.title.to_string();
                            lines.push(Line::from(vec![
                                Span::raw("  ✓ "),
                                Span::styled(title, Style::default().fg(Color::Green)),
                            ]));
                        }
                    }
                }
            }
        } else {
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
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Kanban integrates with Kata coaching loop for task-scoped scientific thinking.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}
