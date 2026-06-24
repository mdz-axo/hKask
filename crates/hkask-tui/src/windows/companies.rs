//! Companies window — financial data and portfolio browser.
//!
//! `]` forward, `[` backward through Search→Profile→Financials→Portfolio→Chat.

use crate::bridges::CompaniesDataBridge;
use crate::mcp_tabbed::{McpChatState, McpTab, McpTabbedWindow};
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompanySection {
    Search,
    Profile,
    Financials,
    Portfolio,
}
impl CompanySection {
    fn next(self) -> Self {
        match self {
            Self::Search => Self::Profile,
            Self::Profile => Self::Financials,
            Self::Financials => Self::Portfolio,
            Self::Portfolio => Self::Search,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Search => Self::Portfolio,
            Self::Profile => Self::Search,
            Self::Financials => Self::Profile,
            Self::Portfolio => Self::Financials,
        }
    }
    fn title(&self) -> &str {
        match self {
            Self::Search => "Search",
            Self::Profile => "Profile",
            Self::Financials => "Financials",
            Self::Portfolio => "Portfolio",
        }
    }
}

pub struct CompaniesWindow {
    id: WindowId,
    section: CompanySection,
    active_tab: McpTab,
    chat_state: McpChatState,
    bridge: Arc<dyn ReplBridge>,
    companies: Option<Arc<dyn CompaniesDataBridge>>,
}

impl CompaniesWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: CompanySection::Search,
            active_tab: McpTab::Data,
            chat_state: McpChatState::new(),
            bridge,
            companies: None,
        }
    }
    pub fn with_companies_bridge(mut self, c: Arc<dyn CompaniesDataBridge>) -> Self {
        self.companies = Some(c);
        self
    }
}

impl Window for CompaniesWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        match self.active_tab {
            McpTab::Chat => "Companies Chat",
            McpTab::Data => "Companies",
        }
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Companies
    }
    fn render(&self, f: &mut Frame, area: Rect, _: bool) {
        match self.active_tab {
            McpTab::Chat => Self::default_render_chat_tab(&self.chat_state, "companies", f, area),
            McpTab::Data => self.render_data_tab(f, area),
        }
    }
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(']') => {
                match self.active_tab {
                    McpTab::Chat => {
                        self.active_tab = McpTab::Data;
                        self.section = CompanySection::Search;
                    }
                    McpTab::Data => {
                        self.section = self.section.next();
                        if self.section == CompanySection::Search {
                            self.active_tab = McpTab::Chat;
                        }
                    }
                }
                return true;
            }
            KeyCode::Char('[') => {
                match self.active_tab {
                    McpTab::Chat => {
                        self.active_tab = McpTab::Data;
                        self.section = CompanySection::Portfolio;
                    }
                    McpTab::Data => {
                        self.section = self.section.prev();
                        if self.section == CompanySection::Portfolio {
                            self.active_tab = McpTab::Chat;
                        }
                    }
                }
                return true;
            }
            _ => {}
        }
        match self.active_tab {
            McpTab::Chat => {
                if let Some(msg) = self.handle_chat_key(key) {
                    self.bridge
                        .start_scoped_inference(msg, self.mcp_server_name());
                    return true;
                }
                matches!(
                    key.code,
                    KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Enter | KeyCode::Esc
                )
            }
            McpTab::Data => false,
        }
    }
    fn tick(&mut self) {}
}

impl McpTabbedWindow for CompaniesWindow {
    fn active_tab(&self) -> McpTab {
        self.active_tab
    }
    fn set_active_tab(&mut self, tab: McpTab) {
        self.active_tab = tab;
    }
    fn chat_state_mut(&mut self) -> &mut McpChatState {
        &mut self.chat_state
    }
    fn mcp_server_name(&self) -> &str {
        "companies"
    }
    fn render_chat_tab(&self, f: &mut Frame, area: Rect) {
        Self::default_render_chat_tab(&self.chat_state, "companies", f, area);
    }
    fn render_data_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(Span::styled(
                format!(
                    "── Companies: {} ([ ] to navigate) ──",
                    self.section.title()
                ),
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];
        if let Some(ref comp) = self.companies {
            let results = comp
                .last_searched()
                .map(|q| comp.search(&q))
                .unwrap_or_default();
            let portfolios = comp.portfolio_list();
            let financials = comp.financials();
            match self.section {
                CompanySection::Search => {
                    if let Some(ref query) = comp.last_searched() {
                        lines.push(Line::from(format!("  Last search: {}", query)));
                        lines.push(Line::from(format!("  {} result(s)", results.len())));
                        for c in &results {
                            let symbol = c.symbol.clone();
                            let name = c.name.clone();
                            let mcap = c
                                .market_cap
                                .map(|m| {
                                    if m >= 1e12 {
                                        format!("${:.2}T", m / 1e12)
                                    } else if m >= 1e9 {
                                        format!("${:.2}B", m / 1e9)
                                    } else {
                                        format!("${:.0}M", m / 1e6)
                                    }
                                })
                                .unwrap_or_default();
                            lines.push(Line::from(vec![
                                Span::raw("  • "),
                                Span::styled(symbol, Style::default().fg(Color::Cyan)),
                                Span::raw("  "),
                                Span::styled(name, Style::default().fg(Color::White)),
                                Span::styled(
                                    format!("  {}", mcap),
                                    Style::default().fg(Color::Green),
                                ),
                            ]));
                            if let Some(ref ex) = c.exchange {
                                lines.push(Line::from(format!("    Exchange: {}", ex)));
                            }
                            if let Some(ref ind) = c.industry {
                                lines.push(Line::from(format!(
                                    "    Industry: {} / {}",
                                    ind,
                                    c.sector.as_deref().unwrap_or("-")
                                )));
                            }
                        }
                    } else {
                        lines.push(Line::from("  Use `kask mcp start companies` to enable."));
                    }
                }
                CompanySection::Profile => {
                    if let Some(ref query) = comp.last_searched() {
                        if let Some(ref c) = results.first() {
                            let symbol = c.symbol.clone();
                            let name = c.name.clone();
                            lines.push(Line::from(vec![
                                Span::styled(symbol, Style::default().fg(Color::Cyan).bold()),
                                Span::raw(" — "),
                                Span::styled(name, Style::default().fg(Color::White)),
                            ]));
                            lines.push(Line::from(format!(
                                "  Exchange: {}",
                                c.exchange.as_deref().unwrap_or("-")
                            )));
                            lines.push(Line::from(format!(
                                "  Industry: {}",
                                c.industry.as_deref().unwrap_or("-")
                            )));
                            lines.push(Line::from(format!(
                                "  Sector:   {}",
                                c.sector.as_deref().unwrap_or("-")
                            )));
                            if let Some(mc) = c.market_cap {
                                lines.push(Line::from(format!("  Mkt Cap:  ${:.2}T", mc / 1e12)));
                            }
                        } else {
                            lines.push(Line::from("  No results."));
                        }
                    } else {
                        lines.push(Line::from("  Search for a symbol to see its profile."));
                    }
                }
                CompanySection::Financials => {
                    if let Some(ref fin) = financials {
                        let symbol = fin.symbol.clone();
                        lines.push(Line::from(vec![
                            Span::styled(symbol, Style::default().fg(Color::Cyan).bold()),
                            Span::raw(" — Key Metrics"),
                        ]));
                        if let Some(p) = fin.price {
                            let ch = fin
                                .change_pct
                                .map(|c| format!(" ({:+.1}%)", c))
                                .unwrap_or_default();
                            let color = if fin.change_pct.unwrap_or(0.0) >= 0.0 {
                                Color::Green
                            } else {
                                Color::Red
                            };
                            lines.push(Line::from(vec![
                                Span::raw("  Price:     "),
                                Span::styled(
                                    format!("${:.2}{}", p, ch),
                                    Style::default().fg(color),
                                ),
                            ]));
                        }
                        if let Some(pe) = fin.pe_ratio {
                            lines.push(Line::from(format!("  P/E:       {:.1}", pe)));
                        }
                        if let Some(rg) = fin.revenue_growth {
                            lines.push(Line::from(format!("  Rev Growth: {:.1}%", rg)));
                        }
                    } else if comp.last_searched().is_some() {
                        lines.push(Line::from("  No financial data."));
                    } else {
                        lines.push(Line::from("  Requires FMP/EODHD keys."));
                    }
                }
                CompanySection::Portfolio => {
                    if portfolios.is_empty() {
                        lines.push(Line::from("  No portfolios."));
                    } else {
                        for p in &portfolios {
                            let name = p.name.clone();
                            lines.push(Line::from(vec![
                                Span::raw("  • "),
                                Span::styled(name, Style::default().fg(Color::Green)),
                                Span::raw(format!("  ({} holdings)", p.holdings)),
                            ]));
                        }
                    }
                }
            }
        } else {
            lines.push(Line::from("  No companies MCP server connected."));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Financial data via hkask-mcp-companies (FMP + EODHD).",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}
