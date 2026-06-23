//! Energy window — gas usage and energy budget analytics.

use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph, Wrap};

use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct EnergyWindow {
    id: WindowId,
    bridge: Arc<dyn ReplBridge>,
}

impl EnergyWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self { id, bridge }
    }
}

impl Window for EnergyWindow {
    fn id(&self) -> WindowId { self.id }
    fn title(&self) -> &str { "Energy" }
    fn kind(&self) -> WindowKind { WindowKind::Energy }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let remaining = self.bridge.gas_remaining();
        let cap = self.bridge.gas_cap();
        let ratio = if cap > 0 { (remaining as f64 / cap as f64).min(1.0) } else { 1.0 };

        let vert = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(8),
                ratatui::layout::Constraint::Min(1),
            ])
            .split(area);

        // Gas gauge
        let gauge = Gauge::default()
            .block(ratatui::widgets::Block::default().title(" Gas Budget ").borders(ratatui::widgets::Borders::ALL))
            .gauge_style(Style::default().fg(if ratio > 0.5 { Color::Green } else if ratio > 0.2 { Color::Yellow } else { Color::Red }))
            .ratio(ratio)
            .label(format!(" {} / {} ({:.0}%) ", remaining, cap, ratio * 100.0));
        f.render_widget(gauge, vert[0]);

        // Info text
        let lines = vec![
            Line::from(Span::styled("── Energy Budget ──", Style::default().fg(Color::Cyan).bold())),
            Line::from(""),
            Line::from(format!("  Gas remaining:  {}", remaining)),
            Line::from(format!("  Gas cap:        {}", cap)),
            Line::from(format!("  Replenish rate:  {} / tick", cap / 10)),
            Line::from(format!("  Alert threshold: 80% ({})", (cap as f64 * 0.8) as u64)),
            Line::from(""),
            Line::from(Span::styled("  Gas is the unit of action in configuration space (P5, P9).", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("  Every inference call and tool invocation costs gas.", Style::default().fg(Color::DarkGray))),
        ];
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), vert[1]);
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool { false }
    fn tick(&mut self) {}
}
