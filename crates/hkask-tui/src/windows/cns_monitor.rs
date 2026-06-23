//! CNS Monitor window — live cybernetic nervous system display.
//!
//! Shows variety counters, algedonic alerts, and domain health
//! in a dedicated window. Complements the sidebar summary.

use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct CnsMonitorWindow {
    id: WindowId,
    bridge: Arc<dyn ReplBridge>,
}

impl CnsMonitorWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self { id, bridge }
    }
}

impl Window for CnsMonitorWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "CNS Monitor"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::CnsMonitor
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let mut lines: Vec<Line> = Vec::new();
        lines.push(Line::from(Span::styled(
            "── CNS Health ──",
            Style::default().fg(Color::Cyan).bold(),
        )));
        lines.push(Line::from(""));

        let alerts = self.bridge.cns_alert_count();
        if alerts > 0 {
            lines.push(Line::from(Span::styled(
                format!("Active alerts: {}", alerts),
                Style::default().fg(Color::Red),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "Status: ✓ All domains healthy",
                Style::default().fg(Color::Green),
            )));
        }
        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            "Domain Status:",
            Style::default().fg(Color::DarkGray),
        )));
        for (domain, healthy) in self.bridge.cns_domains() {
            let (mark, color) = if healthy {
                ("✓", Color::Green)
            } else {
                ("✗", Color::Red)
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(mark, Style::default().fg(color)),
                Span::raw(format!("  {}", domain)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Gas Budget:",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(format!(
            "  Remaining: {} / {} ({:.0}%)",
            self.bridge.gas_remaining(),
            self.bridge.gas_cap(),
            if self.bridge.gas_cap() > 0 {
                (self.bridge.gas_remaining() as f64 / self.bridge.gas_cap() as f64) * 100.0
            } else {
                100.0
            }
        )));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Context Window:",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(format!(
            "  Pressure: {:.0}%",
            self.bridge.context_pressure() * 100.0
        )));

        let (loaded, total_mcp) = self.bridge.mcp_status();
        let (curator, replicant, team) = self.bridge.pod_counts();
        lines.push(Line::from(""));
        lines.push(Line::from(format!(
            "MCP Servers: {}/{} loaded",
            loaded, total_mcp
        )));
        lines.push(Line::from(format!(
            "Pods: {} curator, {} replicant, {} team",
            curator, replicant, team
        )));

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
    fn tick(&mut self) {}
}
