//! Terminal window — embedded command execution.
//!
//! Runs shell commands and captures output. Not a full PTY terminal
//! emulator (that requires thousands of LOC), but sufficient for
//! quick command execution within the TUI.

use std::process::Command;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct TerminalWindow {
    id: WindowId,
    #[allow(dead_code)]
    bridge: Arc<dyn ReplBridge>,
    input: String,
    cursor_pos: usize,
    output: Vec<String>,
    scroll_offset: u16,
}

impl TerminalWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            bridge,
            input: String::new(),
            cursor_pos: 0,
            output: vec!["Terminal ready. Type a command and press Enter.".into()],
            scroll_offset: 0,
        }
    }

    fn execute(&mut self, cmd: &str) {
        self.output.push(format!("$ {}", cmd));
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        let result = if parts.len() == 1 {
            Command::new(parts[0]).output()
        } else {
            Command::new(parts[0]).args(&parts[1..]).output()
        };

        match result {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                for line in stdout.lines() {
                    self.output.push(line.into());
                }
                for line in stderr.lines() {
                    self.output.push(format!("  err: {}", line));
                }
                if !out.status.success() {
                    self.output.push(format!("  exit: {}", out.status));
                }
            }
            Err(e) => {
                self.output.push(format!("  error: {}", e));
            }
        }
    }
}

impl Window for TerminalWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "Terminal"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Terminal
    }

    fn render(&self, f: &mut Frame, area: Rect, is_focused: bool) {
        let vert = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Min(1),
                ratatui::layout::Constraint::Length(3),
            ])
            .split(area);

        // Output area
        let mut lines: Vec<Line> = Vec::new();
        for entry in self.output.iter().rev().skip(self.scroll_offset as usize) {
            lines.push(Line::from(Span::raw(entry.as_str())));
        }
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), vert[0]);

        // Input area
        let border_style = if is_focused {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(border_style);
        let inner = block.inner(vert[1]);
        f.render_widget(block, vert[1]);

        let mut spans = vec![Span::styled("$ ", Style::default().fg(Color::Green).bold())];
        let input_style = if is_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };
        if is_focused && !self.input.is_empty() {
            let before = &self.input[..self.cursor_pos.min(self.input.len())];
            spans.push(Span::styled(before.to_string(), input_style));
            if self.cursor_pos < self.input.len() {
                let at = self.input.chars().nth(self.cursor_pos).unwrap_or(' ');
                spans.push(Span::styled(
                    at.to_string(),
                    Style::default().fg(Color::Black).bg(Color::Green),
                ));
                if self.cursor_pos + 1 < self.input.len() {
                    spans.push(Span::styled(
                        self.input[self.cursor_pos + 1..].to_string(),
                        input_style,
                    ));
                }
            } else {
                spans.push(Span::styled(
                    " ",
                    Style::default().fg(Color::Black).bg(Color::Green),
                ));
            }
        } else {
            spans.push(Span::styled(&self.input, input_style));
            if is_focused {
                spans.push(Span::styled(
                    " ",
                    Style::default().fg(Color::Black).bg(Color::Green),
                ));
            }
        }
        f.render_widget(Paragraph::new(Line::from(spans)), inner);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter => {
                let cmd = std::mem::take(&mut self.input);
                self.cursor_pos = 0;
                if !cmd.is_empty() {
                    self.execute(&cmd);
                }
                true
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return false;
                }
                self.input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
                true
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos);
                }
                true
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
                true
            }
            KeyCode::Right => {
                if self.cursor_pos < self.input.len() {
                    self.cursor_pos += 1;
                }
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
                self.scroll_offset = self.scroll_offset.saturating_add(10);
                true
            }
            KeyCode::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
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
    fn tick(&mut self) {}
}
