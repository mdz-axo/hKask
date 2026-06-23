//! Wallet window — gas, rJoule, and energy budget management.
//!
//! Shows gas budget, rJoule balance, and transaction history.
//! Will support buying rJoule, viewing deposits/withdrawals, and
//! API key management via hkask-wallet integration.
//!
//! # Architecture
//! ⟨Wallet⟩ displays ⟨GasBudget, RJouleBalance, Transactions⟩ .
//! ⟨Wallet⟩ integratesWith ⟨hkask-wallet, hkask-ledger⟩ .

use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph, Wrap};

use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct WalletWindow {
    id: WindowId,
    bridge: Arc<dyn ReplBridge>,
}

impl WalletWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self { id, bridge }
    }
}

impl Window for WalletWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "Wallet"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Wallet
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let remaining = self.bridge.gas_remaining();
        let cap = self.bridge.gas_cap();
        let ratio = if cap > 0 {
            (remaining as f64 / cap as f64).min(1.0)
        } else {
            1.0
        };

        let vert = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(5),
                ratatui::layout::Constraint::Min(1),
            ])
            .split(area);

        // Gas gauge
        let gauge = Gauge::default()
            .block(
                ratatui::widgets::Block::default()
                    .title(" Gas Budget ")
                    .borders(ratatui::widgets::Borders::ALL),
            )
            .gauge_style(Style::default().fg(if ratio > 0.5 {
                Color::Green
            } else if ratio > 0.2 {
                Color::Yellow
            } else {
                Color::Red
            }))
            .ratio(ratio)
            .label(format!(" {} / {} ({:.0}%) ", remaining, cap, ratio * 100.0));
        f.render_widget(gauge, vert[0]);

        let lines = vec![
            Line::from(Span::styled(
                "── Wallet ──",
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  rJoule Balance:",
                Style::default().fg(Color::Yellow),
            )),
            Line::from("    Balance:  0 rJ"),
            Line::from("    Reserved: 0 rJ (gas holds)"),
            Line::from(""),
            Line::from(Span::styled(
                "  Gas Budget:",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(format!("    Remaining: {}", remaining)),
            Line::from(format!("    Cap:       {}", cap)),
            Line::from(format!("    Rate:      {} / tick", cap / 10)),
            Line::from(""),
            Line::from(Span::styled(
                "  Transactions:",
                Style::default().fg(Color::Yellow),
            )),
            Line::from("    No recent transactions."),
            Line::from(""),
            Line::from(Span::styled(
                "  Use `kask wallet` CLI for deposits, withdrawals, and API key management.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "  rJoule is the native unit of energy in hKask's economy (P5, P9).",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), vert[1]);
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
    fn tick(&mut self) {}
}
