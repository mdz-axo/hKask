//! Curator window — the Curator daemon's presence in the TUI.
//!
//! Displays CNS alerts, memory summaries, and pattern detection from
//! the Curator daemon. Supports direct Curator addressing via CRTR ▸ prompt.
//! This is the visual realization of P12.1 dual-presence.
//!
//! # RDF Triples
//! ⟨CuratorWindow⟩ receivesFrom ⟨CuratorDaemon⟩ .
//! ⟨CuratorWindow⟩ displays ⟨CnsAlerts, MemorySummaries, PatternDetections⟩ .
//! ⟨CuratorWindow⟩ routesInput ⟨CuratorDaemon⟩ .

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::repl_bridge::ReplBridge;
use crate::text_cursor;
use crate::window::{Window, WindowId, WindowKind};

/// A curator message — may be a CNS alert, memory summary, or direct reply.
#[derive(Debug, Clone)]
struct CuratorEntry {
    kind: CuratorEntryKind,
    content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CuratorEntryKind {
    /// CNS algedonic alert
    CnsAlert,
    /// Direct reply to user's message
    Reply,
    /// User's message to the Curator
    UserMessage,
}

pub struct CuratorWindow {
    id: WindowId,
    bridge: Arc<dyn ReplBridge>,
    /// Curator message history
    entries: Vec<CuratorEntry>,
    /// Input buffer for Curator chat
    input: String,
    cursor_pos: usize,
    scroll_offset: u16,
}

impl CuratorWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        let mut entries = Vec::new();
        entries.push(CuratorEntry {
            kind: CuratorEntryKind::Reply,
            content: format!(
                "Curator daemon active. I monitor CNS health, surface alerts, and provide memory summaries.\nAgent: {} | Model: {}",
                bridge.userpod_name(),
                bridge.model_name(),
            ),
        });

        Self {
            id,
            bridge,
            entries,
            input: String::new(),
            cursor_pos: 0,
            scroll_offset: 0,
        }
    }

    fn add_entry(&mut self, kind: CuratorEntryKind, content: String) {
        self.entries.push(CuratorEntry { kind, content });
        self.scroll_offset = 0;
    }

    fn send_to_curator(&mut self) {
        let input = std::mem::take(&mut self.input);
        self.cursor_pos = 0;
        if input.is_empty() {
            return;
        }

        self.add_entry(CuratorEntryKind::UserMessage, input.clone());

        // Send to Curator daemon via bridge
        let result = self.bridge.send_curator_message(&input);
        self.add_entry(CuratorEntryKind::Reply, result);
    }
}

impl Window for CuratorWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "Curator"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Curator
    }

    fn render(&self, f: &mut Frame, area: Rect, is_focused: bool) {
        // Guard: skip rendering on degenerate areas from deep splits.
        if area.height < 5 {
            return;
        }
        let vert = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Min(1),
                ratatui::layout::Constraint::Length(3),
            ])
            .split(area);

        self.render_entries(f, vert[0]);
        self.render_input(f, vert[1], is_focused);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter => {
                self.send_to_curator();
                true
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return false;
                }
                text_cursor::insert(&mut self.input, &mut self.cursor_pos, c);
                true
            }
            KeyCode::Backspace => {
                text_cursor::backspace(&mut self.input, &mut self.cursor_pos);
                true
            }
            KeyCode::Delete => {
                text_cursor::delete(&mut self.input, self.cursor_pos);
                true
            }
            KeyCode::Left => {
                text_cursor::move_left(&self.input, &mut self.cursor_pos);
                true
            }
            KeyCode::Right => {
                text_cursor::move_right(&self.input, &mut self.cursor_pos);
                true
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                true
            }
            KeyCode::End => {
                self.cursor_pos = self.input.len();
                true
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_add(5);
                true
            }
            KeyCode::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(5);
                true
            }
            KeyCode::Esc => {
                self.input.clear();
                self.cursor_pos = 0;
                true
            }
            _ => false,
        }
    }

    fn tick(&mut self) {
        // Poll CNS for new alerts each frame
        let alerts = self.bridge.cns_alert_count();
        if alerts > 0 {
            // Only add if not already the most recent entry
            let last_is_alert = self
                .entries
                .last()
                .map(|e| e.kind == CuratorEntryKind::CnsAlert)
                .unwrap_or(false);
            if !last_is_alert {
                self.add_entry(
                    CuratorEntryKind::CnsAlert,
                    format!(
                        "Active CNS alerts: {}. Use CNS Monitor for details.",
                        alerts
                    ),
                );
            }
        }
    }
}

impl CuratorWindow {
    fn render_entries(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<Line> = Vec::new();

        for entry in self.entries.iter().rev().skip(self.scroll_offset as usize) {
            let (prefix, color) = match entry.kind {
                CuratorEntryKind::CnsAlert => ("⚠ CNS: ", Color::Rgb(183, 145, 99)),
                CuratorEntryKind::Reply => ("Curator ▸ ", Color::Magenta),
                CuratorEntryKind::UserMessage => ("You ▸ ", Color::Cyan),
            };

            let prefix_span = Span::styled(prefix, Style::default().fg(color).bold());
            for (i, content_line) in entry.content.lines().enumerate() {
                if i == 0 {
                    lines.push(Line::from(vec![
                        prefix_span.clone(),
                        Span::raw(content_line.to_string()),
                    ]));
                } else {
                    lines.push(Line::from(Span::raw(format!("          {}", content_line))));
                }
            }
            lines.push(Line::from(""));
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "Curator is observing. CNS alerts and memory summaries appear here.",
                Style::default().fg(Color::DarkGray),
            )));
        }

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn render_input(&self, f: &mut Frame, area: Rect, is_focused: bool) {
        let border_style = if is_focused {
            Style::default().fg(Color::Magenta)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(border_style);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let mut spans = vec![Span::styled(
            "CRTR ▸ ",
            Style::default().fg(Color::Magenta).bold(),
        )];

        let input_style = if is_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };

        if is_focused && !self.input.is_empty() {
            let (before, at, after) = text_cursor::parts(&self.input, self.cursor_pos);
            spans.push(Span::styled(before.to_string(), input_style));
            if let Some(at) = at {
                spans.push(Span::styled(
                    at.to_string(),
                    Style::default().fg(Color::Black).bg(Color::Magenta),
                ));
                if !after.is_empty() {
                    spans.push(Span::styled(after.to_string(), input_style));
                }
            } else {
                spans.push(Span::styled(
                    " ",
                    Style::default().fg(Color::Black).bg(Color::Magenta),
                ));
            }
        } else {
            spans.push(Span::styled(self.input.clone(), input_style));
            if is_focused {
                spans.push(Span::styled(
                    " ",
                    Style::default().fg(Color::Black).bg(Color::Magenta),
                ));
            }
        }

        f.render_widget(Paragraph::new(Line::from(spans)), inner);
    }
}
