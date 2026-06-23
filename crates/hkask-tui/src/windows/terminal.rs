//! Terminal window — PTY-backed interactive shell.
//!
//! Uses portable-pty to spawn a shell (bash/sh) with a pseudo-terminal,
//! forwarding keystrokes to the child process and displaying output.
//! Supports interactive programs and Ctrl+C/D/L.

use std::cell::Cell;
use std::io::Read;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use portable_pty::{CommandBuilder, MasterPty, PtySize};
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
    output: Arc<Mutex<Vec<String>>>,
    output_rx: Receiver<String>,
    scroll_offset: u16,
    master: Option<Box<dyn MasterPty>>,
    pty_writer: Option<Box<dyn std::io::Write + Send>>,
    pending_cols: Cell<u16>,
    pending_rows: Cell<u16>,
    last_cols: Cell<u16>,
    last_rows: Cell<u16>,
}

impl TerminalWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        let output = Arc::new(Mutex::new(vec![
            "Terminal ready — PTY shell active.".into(),
        ]));
        let (tx, rx) = mpsc::channel();
        let (master, writer) = spawn_shell(output.clone(), tx);
        Self {
            id,
            bridge,
            input: String::new(),
            cursor_pos: 0,
            output,
            output_rx: rx,
            scroll_offset: 0,
            master: Some(master),
            pty_writer: Some(writer),
            pending_cols: Cell::new(80),
            pending_rows: Cell::new(24),
            last_cols: Cell::new(80),
            last_rows: Cell::new(24),
        }
    }

    fn send_input(&mut self, text: &str) {
        if let Some(ref mut writer) = self.pty_writer {
            let _ = writer.write_all(text.as_bytes());
            let _ = writer.flush();
        }
    }

    fn resize_pty(&mut self, cols: u16, rows: u16) {
        if let Some(ref master) = self.master {
            let _ = master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            });
        }
    }
}

fn shell_command() -> CommandBuilder {
    let mut cmd = CommandBuilder::new(if cfg!(windows) {
        "powershell.exe"
    } else if std::env::var("SHELL").unwrap_or_default().contains("fish") {
        "fish"
    } else {
        "bash"
    });
    if !cfg!(windows) {
        cmd.arg("-l");
    }
    cmd
}

fn spawn_shell(
    output: Arc<Mutex<Vec<String>>>,
    tx: Sender<String>,
) -> (Box<dyn MasterPty>, Box<dyn std::io::Write + Send>) {
    let pty_system = portable_pty::native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("failed to open PTY");

    let cmd = shell_command();
    let _child = pair
        .slave
        .spawn_command(cmd)
        .expect("failed to spawn shell");
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("clone reader");
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]);
                    let mut out = output.lock().unwrap();
                    for line in text.lines() {
                        out.push(line.to_string());
                    }
                    let excess = out.len().saturating_sub(5_000);
                    if excess > 0 {
                        out.drain(..excess);
                    }
                    let _ = tx.send(text.to_string());
                }
                Err(_) => break,
            }
        }
    });

    let master = pair.master;
    let writer: Box<dyn std::io::Write + Send> = Box::new(master.take_writer().unwrap());
    (master, writer)
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
        self.pending_cols.set(area.width);
        self.pending_rows.set(area.height.saturating_sub(3)); // minus input bar
        let vert = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Min(1),
                ratatui::layout::Constraint::Length(3),
            ])
            .split(area);

        let lines: Vec<Line> = {
            let output = self.output.lock().unwrap();
            let total = output.len();
            let visible = vert[0].height as usize;
            let skip = self.scroll_offset as usize;
            if total <= skip {
                vec![Line::from("")]
            } else {
                let end = total.saturating_sub(skip);
                let start = end.saturating_sub(visible);
                output[start..end]
                    .iter()
                    .map(|s| Line::from(s.clone()))
                    .collect()
            }
        };

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), vert[0]);

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
            let cp = self.cursor_pos.min(self.input.len());
            let before = &self.input[..cp];
            spans.push(Span::styled(before.to_string(), input_style));
            if cp < self.input.len() {
                let at = self.input.chars().nth(cp).unwrap_or(' ');
                spans.push(Span::styled(
                    at.to_string(),
                    Style::default().fg(Color::Black).bg(Color::Green),
                ));
                if cp + 1 < self.input.len() {
                    spans.push(Span::styled(self.input[cp + 1..].to_string(), input_style));
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
                self.send_input(&format!("{}\n", cmd));
                true
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match c {
                        'c' => {
                            self.send_input("\x03");
                            return true;
                        }
                        'd' => {
                            self.send_input("\x04");
                            return true;
                        }
                        'l' => {
                            self.send_input("\x0c");
                            return true;
                        }
                        _ => return false,
                    }
                }
                self.input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
                self.send_input(&c.to_string());
                true
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos);
                    self.send_input("\x08");
                }
                true
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.send_input("\x1b[D");
                }
                true
            }
            KeyCode::Right => {
                if self.cursor_pos < self.input.len() {
                    self.cursor_pos += 1;
                    self.send_input("\x1b[C");
                }
                true
            }
            KeyCode::Up => {
                self.send_input("\x1b[A");
                true
            }
            KeyCode::Down => {
                self.send_input("\x1b[B");
                true
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                self.send_input("\x1b[H");
                true
            }
            KeyCode::End => {
                self.cursor_pos = self.input.len();
                self.send_input("\x1b[F");
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
            KeyCode::Tab => {
                self.send_input("\t");
                self.input.insert(self.cursor_pos, '\t');
                self.cursor_pos += 1;
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
        while let Ok(_line) = self.output_rx.try_recv() {}

        let cols = self.pending_cols.get();
        let rows = self.pending_rows.get();
        if cols != self.last_cols.get() || rows != self.last_rows.get() {
            self.resize_pty(cols, rows);
            self.last_cols.set(cols);
            self.last_rows.set(rows);
        }
    }
}
