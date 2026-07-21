//! Backup window — snapshot management display.
//!
//! Shows backup snapshots and configuration. Read-only display —
//! destructive operations (snapshot, restore, prune) remain CLI-only.
//! Delegates to hkask-services-backup through the BackupDataBridge.

use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::bridges::BackupDataBridge;
use crate::bridges::backup::BackupSnapshot;
use crate::repl_bridge::ReplBridge;
use crate::widgets::headers;
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
        let mut lines = vec![headers::section("Backup Operations"), Line::from("")];

        match self.backup.as_ref().map(|backup| backup.snapshot()) {
            None => lines.push(Line::from(Span::styled(
                "  Status: unavailable — backup bridge not configured",
                Style::default().fg(Color::Yellow),
            ))),
            Some(BackupSnapshot::Unavailable { reason }) => lines.push(Line::from(Span::styled(
                format!("  Status: unavailable — {reason}"),
                Style::default().fg(Color::Yellow),
            ))),
            Some(BackupSnapshot::Failed { error }) => lines.push(Line::from(Span::styled(
                format!("  Status: failed — {error}"),
                Style::default().fg(Color::Red),
            ))),
            Some(BackupSnapshot::Ready(snapshot)) => {
                lines.push(Line::from(Span::styled(
                    "  Status:",
                    Style::default().fg(Color::Yellow),
                )));
                lines.push(Line::from(format!(
                    "    Snapshots:    {}",
                    snapshot.snapshot_count
                )));
                if let Some(ref last) = snapshot.last_snapshot {
                    lines.push(Line::from(format!(
                        "    Last:         {} — {} artifacts (trigger: {})",
                        last.timestamp, last.artifact_count, last.trigger
                    )));
                } else {
                    lines.push(Line::from("    Last:         none"));
                }
                let verify_color = if snapshot.verified {
                    Color::Green
                } else {
                    Color::Yellow
                };
                lines.push(Line::from(vec![
                    Span::raw("    Verified:     "),
                    Span::styled(
                        snapshot.verification_detail,
                        Style::default().fg(verify_color),
                    ),
                ]));
                lines.push(Line::from(""));

                let config = snapshot.config;
                lines.push(Line::from(Span::styled(
                    "  Configuration:",
                    Style::default().fg(Color::Yellow),
                )));
                lines.push(Line::from(format!(
                    "    Auto-Snapshot:        {}",
                    if config.auto_snapshot { "on" } else { "off" }
                )));
                lines.push(Line::from(format!(
                    "    Verify After Snapshot: {}",
                    if config.verify_after_snapshot {
                        "on"
                    } else {
                        "off"
                    }
                )));
                lines.push(Line::from(format!(
                    "    Encryption:           {}",
                    if config.encryption_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                )));
                lines.push(Line::from(format!(
                    "    Tracked Types:        {}",
                    config.tracked_types_count
                )));
                lines.push(Line::from(format!(
                    "    Retention:            {} daily / {} weekly",
                    config.retention_daily_days, config.retention_weekly_weeks
                )));
                lines.push(Line::from(""));
            }
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
