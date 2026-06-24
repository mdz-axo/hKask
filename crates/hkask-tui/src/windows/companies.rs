//! Companies window — financial data and portfolio browser.
//!
//! Displays company profiles, key metrics, and portfolio data from
//! hkask-mcp-companies (FMP + EODHD dual-provider). Four tab-cycled
//! sections: Search, Profile, Financials, Portfolio.
//!
//! Adopts the MCP two-tab design (TUI_SPECIFICATION.md §3):
//! - Tab 1 (Chat): Focused chat scoped to the companies MCP server
//! - Tab 2 (Data): Search, Profile, Financials, Portfolio sections
//!
//! Tab key: cycles Search → Profile → Financials → Portfolio → Chat → Search.
//!
//! # Architecture
//! ⟨Companies⟩ surfaces ⟨CompanyProfile, FinancialMetrics, Portfolio⟩ .
//! ⟨Companies⟩ integratesWith ⟨hkask-mcp-companies⟩ .

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::bridges::CompaniesDataBridge;
use crate::mcp_tabbed::{McpChatState, McpTab, McpTabbedWindow};
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

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
    #[allow(dead_code)]
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

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        match self.active_tab {
            McpTab::Chat => {
                Self::default_render_chat_tab(&self.chat_state, "companies", f, area);
            }
            McpTab::Data => self.render_data_tab(f, area),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Tab {
            match self.active_tab {
                McpTab::Chat => {
                    self.active_tab = McpTab::Data;
                    self.section = CompanySection::Search;
                    return true;
                }
                McpTab::Data => {
                    self.section = self.section.next();
                    if self.section == CompanySection::Search {
                        self.active_tab = McpTab::Chat;
                    }
                    return true;
                }
            }
        }

        match self.active_tab {
            McpTab::Chat => {
                if let Some(_msg) = self.handle_chat_key(key) {
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
                    "── Companies: {} (Tab: next | Tab×4: Chat) ──",
                    self.section.title()
                ),
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];

        if let Some(ref comp) = self.companies {
            match self.section {
                CompanySection::Search => {
                    if let Some(ref query) = comp.last_searched() {
                        lines.push(Line::from(format!("  Last search: {}", query)));
                        let results = comp.search(query);
                        lines.push(Line::from(format!("  {} result(s)", results.len())));
                        lines.push(Line::from(""));
                        for c in &results {
                            let market_cap_str = c
                                .market_cap
                                .map(|m| {
                                    if m >= 1_000_000_000_000.0 {
                                        format!("${:.2}T", m / 1_000_000_000_000.0)
                                    } else if m >= 1_000_000_000.0 {
                                        format!("${:.2}B", m / 1_000_000_000.0)
                                    } else {
                                        format!("${:.0}M", m / 1_000_000.0)
                                    }
                                })
                                .unwrap_or_default();
                            lines.push(Line::from(vec![
                                Span::raw("  • "),
                                Span::styled(&c.symbol, Style::default().fg(Color::Cyan)),
                                Span::raw("  "),
                                Span::styled(&c.name, Style::default().fg(Color::White)),
                                Span::styled(
                                    format!("  {}", market_cap_str),
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
                        lines.push(Line::from("  Search via companies MCP tools."));
                        lines.push(Line::from("  Use `kask mcp start companies` to enable."));
                    }
                }
                CompanySection::Profile => {
                    if let Some(ref query) = comp.last_searched() {
                        let results = comp.search(query);
                        if let Some(ref c) = results.first() {
                            lines.push(Line::from(vec![
                                Span::styled(&c.symbol, Style::default().fg(Color::Cyan).bold()),
                                Span::raw(" — "),
                                Span::styled(&c.name, Style::default().fg(Color::White)),
                            ]));
                            lines.push(Line::from(format!(
                                "  Exchange:   {}",
                                c.exchange.as_deref().unwrap_or("-")
                            )));
                            lines.push(Line::from(format!(
                                "  Industry:   {}",
                                c.industry.as_deref().unwrap_or("-")
                            )));
                            lines.push(Line::from(format!(
                                "  Sector:     {}",
                                c.sector.as_deref().unwrap_or("-")
                            )));
                            if let Some(mc) = c.market_cap {
                                lines.push(Line::from(format!(
                                    "  Market Cap: ${:.2}T",
                                    mc / 1_000_000_000_000.0
                                )));
                            }
                            if let Some(ref desc) = c.description {
                                lines.push(Line::from(""));
                                lines.push(Line::from(format!("  {}", desc)));
                            }
                        } else {
                            lines.push(Line::from("  No company selected."));
                            lines.push(Line::from("  Search for a symbol to see its profile."));
                        }
                    } else {
                        lines.push(Line::from("  No company selected."));
                        lines.push(Line::from("  Search for a symbol to see its profile."));
                    }
                }
                CompanySection::Financials => {
                    if let Some(ref fin) = comp.financials() {
                        lines.push(Line::from(vec![
                            Span::styled(&fin.symbol, Style::default().fg(Color::Cyan).bold()),
                            Span::raw(" — Key Metrics"),
                        ]));
                        if let Some(price) = fin.price {
                            let change = fin
                                .change_pct
                                .map(|c| format!(" ({:+.1}%)", c))
                                .unwrap_or_default();
                            lines.push(Line::from(vec![
                                Span::raw("  Price:       "),
                                Span::styled(
                                    format!("${:.2}{}", price, change),
                                    Style::default().fg(if fin.change_pct.unwrap_or(0.0) >= 0.0 {
                                        Color::Green
                                    } else {
                                        Color::Red
                                    }),
                                ),
                            ]));
                        }
                        if let Some(pe) = fin.pe_ratio {
                            lines.push(Line::from(format!("  P/E Ratio:   {:.1}", pe)));
                        }
                        if let Some(rg) = fin.revenue_growth {
                            lines.push(Line::from(format!("  Rev Growth:  {:.1}%", rg)));
                        }
                    } else if comp.last_searched().is_some() {
                        lines.push(Line::from("  No financial data for this symbol."));
                    } else {
                        lines.push(Line::from("  Search for a symbol to see financials."));
                        lines.push(Line::from(
                            "  Requires hkask-mcp-companies with FMP/EODHD API keys.",
                        ));
                    }
                }
                CompanySection::Portfolio => {
                    let portfolios = comp.portfolio_list();
                    if portfolios.is_empty() {
                        lines.push(Line::from("  No portfolios."));
                        lines.push(Line::from(
                            "  Import via `companies ledger_import` MCP tool.",
                        ));
                    } else {
                        lines.push(Line::from(format!("  {} portfolio(s)", portfolios.len())));
                        lines.push(Line::from(""));
                        for p in &portfolios {
                            let created = p.created.as_deref().unwrap_or("-");
                            lines.push(Line::from(vec![
                                Span::raw("  • "),
                                Span::styled(&p.name, Style::default().fg(Color::Green)),
                                Span::raw(format!(
                                    "  ({} holding(s), created {})",
                                    p.holdings, created
                                )),
                            ]));
                        }
                    }
                }
            }
        } else {
            match self.section {
                CompanySection::Search => {
                    lines.push(Line::from("  Search via companies MCP tools."));
                    lines.push(Line::from("  Symbol search powered by FMP / EODHD."));
                    lines.push(Line::from("  Use `kask mcp start companies` to enable."));
                }
                CompanySection::Profile => {
                    lines.push(Line::from("  Company profiles from FMP / EODHD."));
                    lines.push(Line::from(
                        "  Name, exchange, industry, sector, market cap.",
                    ));
                }
                CompanySection::Financials => {
                    lines.push(Line::from("  Key metrics: price, P/E, revenue growth."));
                    lines.push(Line::from(
                        "  Also: income statement, balance sheet, cash flow.",
                    ));
                }
                CompanySection::Portfolio => {
                    lines.push(Line::from("  Portfolio management and returns."));
                    lines.push(Line::from(
                        "  Import ledgers, compare portfolios, compute TWR/IRR.",
                    ));
                }
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Financial data via hkask-mcp-companies (FMP + EODHD dual-provider).",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}
