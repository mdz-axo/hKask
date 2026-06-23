//! Backup window — snapshot management display.
//!
//! Shows backup snapshots and configuration. Read-only display —
//! destructive operations (snapshot, restore, prune) remain CLI-only.
//! Delegates to hkask-services-backup through the BackupDataBridge.

use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::bridges::BackupDataBridge;
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct BackupWindow {
    id: WindowId,
    #[allow(dead_code)]
    bridge: Arc<dyn ReplBridge>,
    backup: Option<Arc<dyn BackupDataBridge>>,
}

impl BackupWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            bridge,
            backup: None,
        }
    }

    pub fn with_backup_bridge(mut self, backup: Arc<dyn BackupDataBridge>) -> Self {
        self.backup = Some(backup);
        self
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
        let mut lines = vec![
            Line::from(Span::styled(
                "── Backup Operations ──",
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];

        if let Some(ref bk) = self.backup {
            let cfg = bk.config();
            let (verified, verify_msg) = bk.verify_status();

            lines.push(Line::from(Span::styled(
                "  Status:",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(format!(
                "    Snapshots:    {}",
                bk.snapshot_count()
            )));
            if let Some(ref snap) = bk.last_snapshot() {
                lines.push(Line::from(format!(
                    "    Last:         {} — {} artifacts (trigger: {})",
                    snap.timestamp, snap.artifact_count, snap.trigger
                )));
            } else {
                lines.push(Line::from("    Last:         none"));
            }
            let verify_color = if verified {
                Color::Green
            } else {
                Color::Yellow
            };
            lines.push(Line::from(vec![
                Span::raw("    Verified:     "),
                Span::styled(verify_msg, Style::default().fg(verify_color)),
            ]));
            lines.push(Line::from(""));

            lines.push(Line::from(Span::styled(
                "  Configuration:",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(format!(
                "    Auto-Snapshot:        {}",
                if cfg.auto_snapshot { "on" } else { "off" }
            )));
            lines.push(Line::from(format!(
                "    Verify After Snapshot: {}",
                if cfg.verify_after_snapshot {
                    "on"
                } else {
                    "off"
                }
            )));
            lines.push(Line::from(format!(
                "    Encryption:           {}",
                if cfg.encryption_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            )));
            lines.push(Line::from(format!(
                "    Tracked Types:        {}",
                cfg.tracked_types_count
            )));
            lines.push(Line::from(format!(
                "    Retention:            {} daily / {} weekly",
                cfg.retention_daily_days, cfg.retention_weekly_weeks
            )));
            lines.push(Line::from(""));
        }

        lines.push(Line::from("  Commands:"));
        lines.push(Line::from("    /backup snapshot   — Create a new snapshot"));
        lines.push(Line::from("    /backup restore    — Restore from snapshot"));
        lines.push(Line::from("    /backup list       — List all snapshots"));
        lines.push(Line::from(
            "    /backup verify     — Verify backup integrity",
        ));
        lines.push(Line::from("    /backup prune      — Remove old snapshots"));
        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            "  Storage:",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from("    Location: ~/.config/hkask/backups/"));
        lines.push(Line::from("    Format:   Encrypted SQLCipher (.db)"));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Use `kask backup` CLI for full functionality.",
            Style::default().fg(Color::DarkGray),
        )));

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
    fn tick(&mut self) {}
}
