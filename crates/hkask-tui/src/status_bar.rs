//! Status bar — global system state display.
//!
//! Renders a single-line status bar showing model, energy budget,
//! CNS health, and context window pressure. This is a cybernetic
//! display surface (P9): it closes the "is the system healthy?"
//! feedback loop.

use std::collections::HashMap;

use ratatui::style::Color;
use ratatui::text::{Line, Span};

use crate::widgets::energy_gauge;
use crate::window::WindowId;

pub struct StatusBar {
    pub model: String,
    pub gas_remaining: u64,
    pub gas_cap: u64,
    pub cns_status: CnsStatus,
    pub context_pressure: f64,
    pub show_hints: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CnsStatus {
    Healthy,
    Warning(u32),
    Critical(u32),
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            model: String::new(),
            gas_remaining: 0,
            gas_cap: 10_000,
            cns_status: CnsStatus::Healthy,
            context_pressure: 0.0,
            show_hints: true,
        }
    }

    pub fn render(&self, focused: WindowId, titles: &HashMap<WindowId, String>) -> Line<'static> {
        self.build_line(focused, &|id| titles.get(&id).cloned())
    }

    fn build_line(
        &self,
        focused: WindowId,
        title_lookup: &dyn Fn(WindowId) -> Option<String>,
    ) -> Line<'static> {
        let mut spans: Vec<Span> = Vec::new();

        if !self.model.is_empty() {
            spans.push(Span::styled(
                format!(" Model: {} ", self.model),
                ratatui::style::Style::default().fg(Color::Cyan),
            ));
            spans.push(Span::raw("│"));
        }

        // Energy gauge — delegates to the shared widget (one source of truth)
        spans.extend(energy_gauge::render_gauge(self.gas_remaining, self.gas_cap).spans);
        spans.push(Span::raw("│"));

        match self.cns_status {
            CnsStatus::Healthy => {
                spans.push(Span::styled(
                    " CNS: ✓ ",
                    ratatui::style::Style::default().fg(Color::Green),
                ));
            }
            CnsStatus::Warning(n) => {
                spans.push(Span::styled(
                    format!(" CNS: ⚠ {} ", n),
                    ratatui::style::Style::default().fg(Color::Yellow),
                ));
            }
            CnsStatus::Critical(n) => {
                spans.push(Span::styled(
                    format!(" CNS: ✗ {} ", n),
                    ratatui::style::Style::default().fg(Color::Red),
                ));
            }
        }
        spans.push(Span::raw("│"));

        let ctx_pct = self.context_pressure * 100.0;
        let ctx_style = if ctx_pct > 87.5 {
            Color::Red
        } else if ctx_pct > 60.0 {
            Color::Yellow
        } else {
            Color::DarkGray
        };
        spans.push(Span::styled(
            format!(" ctx: {:.0}% ", ctx_pct),
            ratatui::style::Style::default().fg(ctx_style),
        ));

        if let Some(title) = title_lookup(focused) {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("[{}]", title),
                ratatui::style::Style::default().fg(Color::White),
            ));
        }

        if self.show_hints {
            spans.push(Span::styled(
                "  ^Q quit  ^T tab  ^P palette",
                ratatui::style::Style::default().fg(Color::DarkGray),
            ));
        }

        Line::from(spans)
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}
