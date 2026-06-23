//! Status bar — global system state display.
//!
//! Renders a single-line status bar showing model, energy budget,
//! CNS health, and context window pressure. This is a cybernetic
//! display surface (P9): it closes the "is the system healthy?"
//! feedback loop.
//!
//! # RDF Triple
//! ```text
//! ⟨StatusBar⟩ displays ⟨ModelMeta, EnergyBudget, CnsHealth, ContextPressure⟩ .
//! ```

use std::collections::HashMap;

use ratatui::style::Color;
use ratatui::text::{Line, Span};

use crate::window::{Window, WindowId};

/// Global status bar — computed state displayed every frame.
pub struct StatusBar {
    /// Current model name
    pub model: String,
    /// Gas remaining / gas cap
    pub gas_remaining: u64,
    pub gas_cap: u64,
    /// CNS health summary
    pub cns_status: CnsStatus,
    /// Context window pressure (0.0–1.0)
    pub context_pressure: f64,
    /// Show keybinding hints
    pub show_hints: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CnsStatus {
    /// All CNS domains healthy
    Healthy,
    /// Warning threshold crossed in ≥1 domain
    Warning(u32),
    /// Critical threshold crossed in ≥1 domain
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

    /// Update from the active windows.
    pub fn tick(&mut self) {
        // Status bar state will be populated from CNS and inference loop
        // in a future iteration. For now, these are placeholders.
    }

    /// Render the status bar as styled Spans.
    pub fn render(
        &self,
        focused: WindowId,
        windows: &HashMap<WindowId, Box<dyn Window>>,
    ) -> Line<'static> {
        let mut spans: Vec<Span> = Vec::new();

        // Model indicator
        if !self.model.is_empty() {
            spans.push(Span::styled(
                format!(" Model: {} ", self.model),
                ratatui::style::Style::default().fg(Color::Cyan),
            ));
            spans.push(Span::raw("│"));
        }

        // Energy gauge
        let gas_pct = if self.gas_cap > 0 {
            (self.gas_remaining as f64 / self.gas_cap as f64) * 100.0
        } else {
            100.0
        };
        let gas_style = if gas_pct < 20.0 {
            Color::Red
        } else if gas_pct < 50.0 {
            Color::Yellow
        } else {
            Color::Green
        };
        let bar_width = 10;
        let filled = ((gas_pct / 100.0) * bar_width as f64) as usize;
        let empty = bar_width - filled;
        spans.push(Span::raw(" Gas: "));
        spans.push(Span::styled(
            "█".repeat(filled),
            ratatui::style::Style::default().fg(gas_style),
        ));
        spans.push(Span::styled(
            "░".repeat(empty),
            ratatui::style::Style::default().fg(Color::DarkGray),
        ));
        spans.push(Span::styled(
            format!(" {:.0}% ", gas_pct),
            ratatui::style::Style::default().fg(gas_style),
        ));
        spans.push(Span::raw("│"));

        // CNS health indicator
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

        // Context pressure
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

        // Focused window title (right-aligned via padding)
        if let Some(win) = windows.get(&focused) {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("[{}]", win.title()),
                ratatui::style::Style::default().fg(Color::White),
            ));
        }

        // Keybinding hints
        if self.show_hints {
            spans.push(Span::styled(
                "  ^Q quit  ^T tab  ^W close  ^B sidebar  ^P palette",
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
