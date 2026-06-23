//! Companies window — organization and entity data.
//!
//! Search, profile, people, and relationship tabs. Live data from
//! CompaniesDataBridge (hkask-mcp-companies / Firecrawl). Deferred
//! integration pending Companies MCP server availability.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::bridges::CompaniesDataBridge;
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompanySection {
    Search,
    Profile,
    People,
    Relations,
}

impl CompanySection {
    fn next(self) -> Self {
        match self {
            Self::Search => Self::Profile,
            Self::Profile => Self::People,
            Self::People => Self::Relations,
            Self::Relations => Self::Search,
        }
    }
    fn title(&self) -> &str {
        match self {
            Self::Search => "Search",
            Self::Profile => "Profile",
            Self::People => "People",
            Self::Relations => "Relations",
        }
    }
}

pub struct CompaniesWindow {
    id: WindowId,
    section: CompanySection,
    #[allow(dead_code)]
    bridge: Arc<dyn ReplBridge>,
    companies: Option<Arc<dyn CompaniesDataBridge>>,
}

impl CompaniesWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: CompanySection::Search,
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
        "Companies"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Companies
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let mut lines = vec![
            Line::from(Span::styled(
                format!("── Companies: {} (Tab to switch) ──", self.section.title()),
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
                            let name = c.name.to_string();
                            let domain = c.domain.clone();
                            let info = format!(
                                "{}  {}  {}",
                                c.industry.as_deref().unwrap_or(""),
                                c.size.as_deref().unwrap_or(""),
                                c.location.as_deref().unwrap_or(""),
                            );
                            lines.push(Line::from(vec![
                                Span::raw("  • "),
                                Span::styled(name, Style::default().fg(Color::Cyan)),
                                Span::styled(format!("  ({})", domain), Style::default().fg(Color::DarkGray)),
                            ]));
                            lines.push(Line::from(format!("    {}", info)));
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
                            lines.push(Line::from(format!("  Name:     {}", c.name)));
                            lines.push(Line::from(format!("  Domain:   {}", c.domain)));
                            lines.push(Line::from(format!(
                                "  Industry: {}",
                                c.industry.as_deref().unwrap_or("-")
                            )));
                            lines.push(Line::from(format!(
                                "  Size:     {}",
                                c.size.as_deref().unwrap_or("-")
                            )));
                            lines.push(Line::from(format!(
                                "  Location: {}",
                                c.location.as_deref().unwrap_or("-")
                            )));
                        } else {
                            lines.push(Line::from("  No company selected."));
                        }
                    } else {
                        lines.push(Line::from("  No company selected."));
                        lines.push(Line::from("  Search for a company to see its profile."));
                    }
                }
                CompanySection::People => {
                    let people = comp.people();
                    if people.is_empty() {
                        lines.push(Line::from("  No people data."));
                    } else {
                        for p in &people {
                            let name = p.name.to_string();
                            let role = p.role.as_deref().unwrap_or("-");
                            let company = p.company.clone();
                            lines.push(Line::from(vec![
                                Span::raw("  • "),
                                Span::styled(name, Style::default().fg(Color::Green)),
                                Span::raw(format!("  {}  @{}", role, company)),
                            ]));
                        }
                    }
                }
                CompanySection::Relations => {
                    lines.push(Line::from("  Entity relationships from company graph."));
                    lines.push(Line::from("  Subsidiaries, parents, competitors, partners."));
                }
            }
        } else {
            match self.section {
                CompanySection::Search => {
                    lines.push(Line::from("  Search via companies MCP tools."));
                    lines.push(Line::from("  Powered by hkask-mcp-companies + Firecrawl."));
                    lines.push(Line::from("  Use `kask mcp start companies` to enable."));
                }
                CompanySection::Profile => {
                    lines.push(Line::from("  Detailed company profiles from Firecrawl."));
                    lines.push(Line::from("  Name, domain, industry, size, funding."));
                }
                CompanySection::People => {
                    lines.push(Line::from("  Key people and contacts from the company."));
                    lines.push(Line::from("  Roles, contact info, social profiles."));
                }
                CompanySection::Relations => {
                    lines.push(Line::from("  Entity relationships: subsidiaries, parents."));
                }
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Companies data via hkask-mcp-companies + Firecrawl integration (deferred).",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Tab {
            self.section = self.section.next();
            true
        } else {
            false
        }
    }
    fn tick(&mut self) {}
}
