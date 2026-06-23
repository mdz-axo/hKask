//! Companies window — organization and entity data.
//!
//! Displays company profiles, contacts, and entity relationships.
//! Integrates with hkask-mcp-companies for company data lookup.
//!
//! # Architecture
//! ⟨Companies⟩ displays ⟨Profiles, Contacts, Relationships⟩ .
//! ⟨Companies⟩ integratesWith ⟨hkask-mcp-companies⟩ .
//!
//! # MCP Two-Tab Design (future)
//! Tab 1: Companies chat — focused chat using companies MCP tools
//! Tab 2: Data view — structured company profiles and relationships

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

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
}

impl CompaniesWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: CompanySection::Search,
            bridge,
        }
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
        match self.section {
            CompanySection::Search => {
                lines.push(Line::from(
                    "  Search for companies by name, domain, or industry.",
                ));
                lines.push(Line::from("  Powered by hkask-mcp-companies MCP server."));
                lines.push(Line::from("  Use `kask mcp start companies` to enable."));
            }
            CompanySection::Profile => {
                lines.push(Line::from("  Detailed company profile:"));
                lines.push(Line::from("    • Name, domain, industry"));
                lines.push(Line::from("    • Size, funding, location"));
                lines.push(Line::from("    • Description and tags"));
            }
            CompanySection::People => {
                lines.push(Line::from("  Key people associated with the company."));
                lines.push(Line::from("  Roles, contact info, social profiles."));
            }
            CompanySection::Relations => {
                lines.push(Line::from("  Entity relationships:"));
                lines.push(Line::from("    • Subsidiaries and parents"));
                lines.push(Line::from("    • Competitors and partners"));
                lines.push(Line::from("    • Investment relationships"));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Companies data via hkask-mcp-companies + Firecrawl integration.",
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
