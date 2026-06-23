//! Editor window — basic text editor for configs, agent YAML, etc.
//!
//! Provides line-based navigation, insert/delete, and basic editing.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct EditorWindow {
    id: WindowId,
    #[allow(dead_code)]
    bridge: Arc<dyn ReplBridge>,
    lines: Vec<String>,
    cursor_line: usize,
    cursor_col: usize,
    scroll_offset: u16,
    filename: Option<String>,
    modified: bool,
}

impl EditorWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            bridge,
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            filename: None,
            modified: false,
        }
    }

    fn insert_char(&mut self, c: char) {
        self.modified = true;
        let line = &mut self.lines[self.cursor_line];
        line.insert(self.cursor_col, c);
        self.cursor_col += 1;
    }

    fn delete_char(&mut self) {
        if self.cursor_col > 0 {
            self.modified = true;
            self.lines[self.cursor_line].remove(self.cursor_col - 1);
            self.cursor_col -= 1;
        } else if self.cursor_line > 0 {
            self.modified = true;
            let cur = self.cursor_line;
            let text = self.lines.remove(cur);
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].len();
            self.lines[self.cursor_line].push_str(&text);
        }
    }

    fn newline(&mut self) {
        self.modified = true;
        let rest = self.lines[self.cursor_line].split_off(self.cursor_col);
        self.cursor_line += 1;
        self.cursor_col = 0;
        self.lines.insert(self.cursor_line, rest);
    }
}

impl Window for EditorWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        if let Some(ref name) = self.filename {
            name.as_str()
        } else {
            "Editor"
        }
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Editor
    }

    fn render(&self, f: &mut Frame, area: Rect, _is_focused: bool) {
        let mut display_lines: Vec<Line> = Vec::new();
        for (i, text) in self
            .lines
            .iter()
            .enumerate()
            .skip(self.scroll_offset as usize)
        {
            let line_num = Span::styled(
                format!("{:3} ", i + 1),
                Style::default().fg(Color::DarkGray),
            );
            if i == self.cursor_line {
                // Show cursor on current line
                let mut spans = vec![line_num];
                let col = self.cursor_col.min(text.len());
                spans.push(Span::raw(&text[..col]));
                if col < text.len() {
                    spans.push(Span::styled(
                        text[col..col + 1].to_string(),
                        Style::default().fg(Color::Black).bg(Color::Cyan),
                    ));
                    spans.push(Span::raw(&text[col + 1..]));
                } else {
                    spans.push(Span::styled(
                        " ",
                        Style::default().fg(Color::Black).bg(Color::Cyan),
                    ));
                }
                display_lines.push(Line::from(spans));
            } else {
                display_lines.push(Line::from(vec![line_num, Span::raw(text.as_str())]));
            }
        }
        f.render_widget(
            Paragraph::new(display_lines).wrap(Wrap { trim: false }),
            area,
        );
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match c {
                        's' => {
                            self.modified = false;
                            return true;
                        } // "save"
                        _ => return false,
                    }
                }
                self.insert_char(c);
                true
            }
            KeyCode::Enter => {
                self.newline();
                true
            }
            KeyCode::Backspace => {
                self.delete_char();
                true
            }
            KeyCode::Delete => {
                if self.cursor_col < self.lines[self.cursor_line].len() {
                    self.modified = true;
                    self.lines[self.cursor_line].remove(self.cursor_col);
                } else if self.cursor_line + 1 < self.lines.len() {
                    self.modified = true;
                    let next = self.lines.remove(self.cursor_line + 1);
                    self.lines[self.cursor_line].push_str(&next);
                }
                true
            }
            KeyCode::Up => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.cursor_col = self.cursor_col.min(self.lines[self.cursor_line].len());
                }
                true
            }
            KeyCode::Down => {
                if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    self.cursor_col = self.cursor_col.min(self.lines[self.cursor_line].len());
                }
                true
            }
            KeyCode::Left => {
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
                true
            }
            KeyCode::Right => {
                if self.cursor_col < self.lines[self.cursor_line].len() {
                    self.cursor_col += 1;
                }
                true
            }
            KeyCode::Home => {
                self.cursor_col = 0;
                true
            }
            KeyCode::End => {
                self.cursor_col = self.lines[self.cursor_line].len();
                true
            }
            KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
                true
            }
            KeyCode::PageDown => {
                self.scroll_offset = self.scroll_offset.saturating_add(10);
                true
            }
            _ => false,
        }
    }
    fn tick(&mut self) {}
}
