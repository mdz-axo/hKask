//! Wallet window — gas, rJoule, and energy budget management.
//!
//! Shows gas budget, rJoule balance, and transaction history.
//! Gas comes from ReplBridge (InferenceLoop); rJoule and transactions
//! come from WalletDataBridge (WalletService).
//!
//! # Architecture
//! ⟨Wallet⟩ displays ⟨GasBudget, RJouleBalance, Transactions⟩ .
//! ⟨Wallet⟩ integratesWith ⟨hkask-wallet, hkask-ledger⟩ .

use std::sync::Arc;

use crate::widgets::headers;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph, Wrap};

use crate::bridges::WalletDataBridge;
use crate::bridges::wallet::WalletSnapshot;
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct WalletWindow {
    id: WindowId,
    bridge: Arc<dyn ReplBridge>,
    wallet: Option<Arc<dyn WalletDataBridge>>,
}

impl WalletWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            bridge,
            wallet: None,
        }
    }

    pub fn with_wallet_bridge(mut self, wallet: Arc<dyn WalletDataBridge>) -> Self {
        self.wallet = Some(wallet);
        self
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

        // Guard: gauge needs at least 8 rows (5 for gauge + 3 for details).
        // On tiny windows, render a compact single-line summary instead.
        if area.height < 8 {
            let summary = Line::from(Span::styled(
                format!(" Gas: {:.0}% ({} / {} rJ) ", ratio * 100.0, remaining, cap),
                Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::Rgb(30, 30, 40)),
            ));
            f.render_widget(Paragraph::new(summary), area);
            return;
        }

        let vert = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(5),
                ratatui::layout::Constraint::Min(1),
            ])
            .split(area);

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

        let mut lines = vec![headers::section("Wallet"), Line::from("")];
        let wallet_snapshot = self.wallet.as_ref().map(|wallet| wallet.snapshot(10));

        // ── rJoule balance section ──
        lines.push(Line::from(Span::styled(
            "  rJoule Balance:",
            Style::default().fg(Color::Yellow),
        )));
        match wallet_snapshot.as_ref() {
            Some(WalletSnapshot::Ready(wallet)) => {
                lines.push(Line::from(format!("    Balance:  {} rJ", wallet.rjoules)));
                lines.push(Line::from(format!(
                    "    USD:      {:.6} ({} µUSDC)",
                    wallet.usdc_micro as f64 / 1_000_000.0,
                    wallet.usdc_micro
                )));
                lines.push(Line::from(format!(
                    "    Gas Equiv: {} gas ({} gas/rJ)",
                    wallet.gas_equivalent, wallet.gas_per_rjoule
                )));
            }
            Some(WalletSnapshot::Unavailable { reason }) => lines.push(Line::from(Span::styled(
                format!("    Unavailable: {reason}"),
                Style::default().fg(Color::Yellow),
            ))),
            Some(WalletSnapshot::Failed { error }) => lines.push(Line::from(Span::styled(
                format!("    Failed: {error}"),
                Style::default().fg(Color::Red),
            ))),
            None => lines.push(Line::from(Span::styled(
                "    Unavailable: wallet bridge not configured",
                Style::default().fg(Color::Yellow),
            ))),
        }
        lines.push(Line::from(""));

        // ── Gas budget section ──
        lines.push(Line::from(Span::styled(
            "  Gas Budget:",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(format!("    Remaining: {}", remaining)));
        lines.push(Line::from(format!("    Cap:       {}", cap)));
        let rate = match wallet_snapshot.as_ref() {
            Some(WalletSnapshot::Ready(wallet)) => wallet.gas_per_rjoule,
            _ => 0,
        };
        lines.push(Line::from(format!("    Rate:      {} gas / rJ", rate)));
        lines.push(Line::from(""));

        // ── Transaction history section ──
        lines.push(Line::from(Span::styled(
            "  Transactions:",
            Style::default().fg(Color::Yellow),
        )));
        if let Some(WalletSnapshot::Ready(wallet)) = wallet_snapshot.as_ref() {
            if wallet.transactions.is_empty() {
                lines.push(Line::from(format!(
                    "    No transactions yet ({} total).",
                    wallet.transaction_count
                )));
            } else {
                for tx in &wallet.transactions {
                    let sign = if tx.rjoules_delta >= 0 { "+" } else { "" };
                    let color = if tx.rjoules_delta >= 0 {
                        Color::Green
                    } else {
                        Color::Red
                    };
                    let detail = tx
                        .detail
                        .as_deref()
                        .map(|d| format!(" — {}", d))
                        .unwrap_or_default();
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled(tx.tx_type.clone(), Style::default().fg(Color::DarkGray)),
                        Span::raw(" "),
                        Span::styled(
                            format!("{}{} rJ", sign, tx.rjoules_delta),
                            Style::default().fg(color),
                        ),
                        Span::raw(format!("  → {} rJ{}", tx.balance_after, detail)),
                    ]));
                }
            }
        } else {
            lines.push(Line::from("    Transactions unavailable."));
        }
        lines.push(Line::from(""));

        lines.push(Line::from(Span::styled(
            "  Use `kask wallet` CLI for deposits, withdrawals, and API key management.",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::styled(
            "  rJoule is the native unit of energy in hKask's economy (P5, P9).",
            Style::default().fg(Color::DarkGray),
        )));

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), vert[1]);
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
    fn tick(&mut self) {}
}
