//! Kanban window — interactive multi-column task board.
//!
//! Navigation: h/l or ←/→ to switch columns, j/k or ↑/↓ to select tasks.
//! m to advance the selected task one status forward.
//! Tab toggles between board view and scoped chat.

use crate::bridges::kanban::{KanbanDataBridge, KanbanTaskSummary};
use crate::mcp_tabbed::{McpChatState, McpTab, McpTabbedWindow};
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::sync::Arc;

/// Column definitions for the kanban board.
const COLUMN_STATUSES: &[&str] = &["backlog", "ready", "in_progress", "review", "done"];
const COLUMN_TITLES: &[&str] = &["Backlog", "Ready", "In Progress", "Review", "Done"];

/// Maximum tasks to fetch per column.
const TASKS_PER_COLUMN: usize = 50;

pub struct KanbanWindow {
    id: WindowId,
    active_tab: McpTab,
    chat_state: McpChatState,
    bridge: Arc<dyn ReplBridge>,
    kanban: Option<Arc<dyn KanbanDataBridge>>,

    // Interaction state
    col_idx: usize,
    row_idx: usize,
    status_message: Option<String>,
}

impl KanbanWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            active_tab: McpTab::Data,
            chat_state: McpChatState::new(),
            bridge,
            kanban: None,
            col_idx: 0,
            row_idx: 0,
            status_message: None,
        }
    }

    pub fn with_kanban_bridge(mut self, kb: Arc<dyn KanbanDataBridge>) -> Self {
        self.kanban = Some(kb);
        self
    }

    /// Get tasks for a column, clamped to visible area height.
    fn column_tasks(&self, status: &str) -> Vec<KanbanTaskSummary> {
        self.kanban
            .as_ref()
            .map(|kb| kb.tasks_by_status(status, TASKS_PER_COLUMN))
            .unwrap_or_default()
    }

    /// Advance selected task to the next status column.
    fn move_task_forward(&mut self) {
        let kb = match self.kanban.as_ref() {
            Some(k) => k,
            None => return,
        };

        let tasks = kb.tasks_by_status(COLUMN_STATUSES[self.col_idx], TASKS_PER_COLUMN);
        let task = match tasks.get(self.row_idx) {
            Some(t) => t,
            None => return,
        };

        // Determine target status
        let target_idx = match self.col_idx {
            0 => 1, // backlog → ready
            1 => 2, // ready → in_progress
            2 => 3, // in_progress → review
            3 => 4, // review → done
            _ => {
                self.status_message = Some("Already done — cannot advance".into());
                return;
            }
        };

        match kb.move_task(&task.id, COLUMN_STATUSES[target_idx]) {
            Ok(_moved) => {
                self.status_message = Some(format!(
                    "Moved #{} → {}",
                    &task.id[..task.id.len().min(8)],
                    COLUMN_TITLES[target_idx]
                ));
                // Adjust row_idx: stay in bounds after removal
                let new_count = tasks.len().saturating_sub(1);
                if self.row_idx >= new_count && new_count > 0 {
                    self.row_idx = new_count - 1;
                } else if new_count == 0 {
                    self.row_idx = 0;
                }
            }
            Err(e) => {
                self.status_message = Some(e);
            }
        }
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

    fn render(&self, f: &mut Frame, area: Rect, is_focused: bool) {
        match self.active_tab {
            McpTab::Chat => Self::default_render_chat_tab(&self.chat_state, "kanban", f, area),
            McpTab::Data => self.render_board(f, area, is_focused),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Tab toggles between Data and Chat
        if key.code == KeyCode::Tab {
            self.active_tab = self.active_tab.next();
            self.status_message = None;
            return true;
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
            McpTab::Data => self.handle_board_key(key),
        }
    }

    fn tick(&mut self) {}
}

impl KanbanWindow {
    fn handle_board_key(&mut self, key: KeyEvent) -> bool {
        use KeyCode::*;

        // Dismiss status message on any key
        self.status_message = None;

        match key.code {
            // Column navigation: h / Left
            Char('h') | Left => {
                if self.col_idx > 0 {
                    self.col_idx -= 1;
                    self.row_idx = 0;
                }
                true
            }
            // Column navigation: l / Right
            Char('l') | Right => {
                if self.col_idx < 4 {
                    self.col_idx += 1;
                    self.row_idx = 0;
                }
                true
            }
            // Row navigation: k / Up
            Char('k') | Up => {
                if self.row_idx > 0 {
                    self.row_idx -= 1;
                }
                true
            }
            // Row navigation: j / Down
            Char('j') | Down => {
                let max = self
                    .column_tasks(COLUMN_STATUSES[self.col_idx])
                    .len()
                    .saturating_sub(1);
                if self.row_idx < max {
                    self.row_idx += 1;
                }
                true
            }
            // Move task forward
            Char('m') if key.modifiers.is_empty() => {
                self.move_task_forward();
                true
            }
            // Page Up / Page Down for scrolling
            PageUp => {
                self.row_idx = self.row_idx.saturating_sub(5);
                true
            }
            PageDown => {
                let count = self.column_tasks(COLUMN_STATUSES[self.col_idx]).len();
                self.row_idx = (self.row_idx + 5).min(count.saturating_sub(1));
                true
            }
            // Home / End
            Home => {
                self.row_idx = 0;
                true
            }
            End => {
                let count = self.column_tasks(COLUMN_STATUSES[self.col_idx]).len();
                self.row_idx = count.saturating_sub(1);
                true
            }
            _ => false,
        }
    }

    fn render_board(&self, f: &mut Frame, area: Rect, is_focused: bool) {
        let vert = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // columns
                Constraint::Length(1), // detail bar
                Constraint::Length(1), // key hints
            ])
            .split(area);

        // --- Column layout ---
        self.render_columns(f, vert[0], is_focused);

        // --- Detail bar ---
        self.render_detail_bar(f, vert[1], is_focused);

        // --- Key hints ---
        self.render_key_hints(f, vert[2]);
    }

    fn render_columns(&self, f: &mut Frame, area: Rect, is_focused: bool) {
        let col_widths = [
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
        ];
        let cols = Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints(col_widths)
            .split(area);

        for (i, col_area) in cols.iter().enumerate() {
            let is_selected_col = i == self.col_idx && is_focused;
            self.render_column(f, *col_area, i, is_selected_col);
        }
    }

    fn render_column(&self, f: &mut Frame, area: Rect, col_idx: usize, is_selected: bool) {
        let tasks = self.column_tasks(COLUMN_STATUSES[col_idx]);
        let count = tasks.len();
        let status = COLUMN_TITLES[col_idx];

        // Calculate available rows for task display (minus borders and title)
        let inner_height = area.height.saturating_sub(2); // borders
        if inner_height == 0 {
            return;
        }

        // Build task lines with selection highlight
        let mut lines: Vec<Line> = Vec::new();

        for (task_idx, task) in tasks.iter().enumerate() {
            if lines.len() as u16 >= inner_height {
                break;
            }
            let is_task_selected = is_selected && task_idx == self.row_idx;
            lines.push(self.task_line(task, is_task_selected));
        }

        let border_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let title = format!(" {} ({}) ", status, count);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title);

        f.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn task_line(&self, task: &KanbanTaskSummary, selected: bool) -> Line<'_> {
        let done_prefix = task.status == "done";
        let prefix = if done_prefix { "✓" } else { "•" };

        let (prio_color, prio_text) = match task.priority.as_deref() {
            Some("critical") => (Color::Red, "!!!"),
            Some("high") => (Color::Red, "!!"),
            Some("medium") => (Color::Yellow, "!"),
            Some("low") => (Color::DarkGray, "·"),
            _ => (Color::DarkGray, "·"),
        };

        let title_style = if selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if done_prefix {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        let mut title = task.title.clone();
        // Truncate to fit column width (approx 25 chars for 1/5 of 120-char terminal)
        if title.len() > 30 {
            title.truncate(27);
            title.push('…');
        }

        let assignee = task.assignee.as_deref().unwrap_or("");
        let has_assignee = !assignee.is_empty();

        let selected_bg_style = Style::default().fg(Color::Black).bg(Color::Yellow);

        let mut spans = vec![
            Span::raw(" "),
            Span::styled(format!("{prefix} {title}"), title_style),
        ];
        if selected {
            if has_assignee {
                spans.push(Span::styled(
                    format!(" [{}] [{}]", prio_text, assignee),
                    selected_bg_style,
                ));
            } else {
                spans.push(Span::styled(format!(" [{}]", prio_text), selected_bg_style));
            }
        } else if has_assignee {
            spans.push(Span::styled(
                format!(" [{}]", prio_text),
                Style::default().fg(prio_color),
            ));
            spans.push(Span::styled(
                format!(" [{}]", assignee),
                Style::default().fg(Color::Cyan),
            ));
        } else {
            spans.push(Span::styled(
                format!(" [{}]", prio_text),
                Style::default().fg(prio_color),
            ));
        }
        Line::from(spans)
    }

    fn render_detail_bar(&self, f: &mut Frame, area: Rect, _is_focused: bool) {
        let detail = if let Some(ref msg) = self.status_message {
            Span::styled(msg.clone(), Style::default().fg(Color::Yellow))
        } else {
            let tasks = self.column_tasks(COLUMN_STATUSES[self.col_idx]);
            match tasks.get(self.row_idx) {
                Some(task) => {
                    let id_short = &task.id[..task.id.len().min(8)];
                    let assignee = task.assignee.as_deref().unwrap_or("unassigned");
                    let prio = task.priority.as_deref().unwrap_or("-");
                    Span::styled(
                        format!(
                            "  #{id_short}  {title}  [{prio}]  {assignee}",
                            title = task.title
                        ),
                        Style::default().fg(Color::Cyan),
                    )
                }
                None => Span::styled(
                    format!("  {} — empty", COLUMN_TITLES[self.col_idx]),
                    Style::default().fg(Color::DarkGray),
                ),
            }
        };

        f.render_widget(Paragraph::new(Line::from(detail)), area);
    }

    fn render_key_hints(&self, f: &mut Frame, area: Rect) {
        let hints = Span::styled(
            " h/l cols  j/k tasks  m=advance  PgUp/PgDn  Home/End  Tab=chat ",
            Style::default().fg(Color::DarkGray),
        );
        f.render_widget(Paragraph::new(Line::from(hints)), area);
    }
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
        // Used by McpTabbedWindow default dispatch — but we override render() directly.
        // Delegate to board render with focused=true for Data tab rendering.
        self.render_board(f, area, true);
    }
}
