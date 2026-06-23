//! Backup window — snapshot management display.
//!
//! Shows backup snapshots, allows triggering backup operations
//! from the TUI. Delegates to hkask-services-backup through the bridge.

use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct BackupWindow {
    id: WindowId,
    #[allow(dead_code)]
    bridge: Arc<dyn ReplBridge>,
}

impl BackupWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self { id, bridge }
    }
}

impl Window for BackupWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "Backup"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Backup
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let lines = vec![
            Line::from(Span::styled(
                "── Backup Operations ──",
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
            Line::from("  Commands:"),
            Line::from("    /backup snapshot   — Create a new snapshot"),
            Line::from("    /backup restore    — Restore from snapshot"),
            Line::from("    /backup list       — List all snapshots"),
            Line::from("    /backup verify     — Verify backup integrity"),
            Line::from("    /backup prune      — Remove old snapshots"),
            Line::from(""),
            Line::from(Span::styled(
                "  Storage:",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from("    Location: ~/.config/hkask/backups/"),
            Line::from("    Format:   Encrypted SQLCipher (.db)"),
            Line::from(""),
            Line::from(Span::styled(
                "  Use `kask backup` CLI for full functionality.",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
    fn tick(&mut self) {}
}
