//! Skills window — browse and manage the skill corpus.
//!
//! Live data from RegistryDataBridge (SqliteRegistry). Tab-cycled
//! sections: Installed, Available, Active.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::bridges::RegistryDataBridge;
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkillSection {
    Installed,
    Available,
    Active,
}

impl SkillSection {
    fn next(self) -> Self {
        match self {
            Self::Installed => Self::Available,
            Self::Available => Self::Active,
            Self::Active => Self::Installed,
        }
    }
    fn title(&self) -> &str {
        match self {
            Self::Installed => "Installed",
            Self::Available => "Available",
            Self::Active => "Active",
        }
    }
}

pub struct SkillsWindow {
    id: WindowId,
    section: SkillSection,
    #[allow(dead_code)]
    bridge: Arc<dyn ReplBridge>,
    registry: Option<Arc<dyn RegistryDataBridge>>,
}

impl SkillsWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: SkillSection::Installed,
            bridge,
            registry: None,
        }
    }

    pub fn with_registry_bridge(mut self, reg: Arc<dyn RegistryDataBridge>) -> Self {
        self.registry = Some(reg);
        self
    }
}

impl Window for SkillsWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "Skills"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Skills
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let mut lines = vec![
            Line::from(Span::styled(
                format!("── Skills: {} (Tab to switch) ──", self.section.title()),
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];

        if let Some(ref reg) = self.registry {
            match self.section {
                SkillSection::Installed => {
                    let skills = reg.list_skills();
                    lines.push(Line::from(format!("  {} skill(s) installed", skills.len())));
                    lines.push(Line::from(""));
                    if skills.is_empty() {
                        lines.push(Line::from("  No skills installed."));
                    } else {
                        for s in &skills {
                            let name = s.name.to_string();
                            let domain = s.domain.to_string();
                            let desc: String = s
                                .description
                                .as_deref()
                                .map(|d| format!(" — {}", d))
                                .unwrap_or_default();
                            lines.push(Line::from(vec![
                                Span::raw("  • "),
                                Span::styled(name, Style::default().fg(Color::Magenta)),
                                Span::raw(format!("  [{}]", domain)),
                                Span::styled(desc, Style::default().fg(Color::DarkGray)),
                            ]));
                        }
                    }
                }
                SkillSection::Available => {
                    let templates = reg.list_templates();
                    lines.push(Line::from(format!(
                        "  {} template(s) available",
                        templates.len()
                    )));
                    lines.push(Line::from(""));
                    if templates.is_empty() {
                        lines.push(Line::from("  No templates available."));
                    } else {
                        for t in templates.iter().take(30) {
                            let name = t.name.to_string();
                            let desc: String = t
                                .description
                                .as_deref()
                                .map(|d| format!(" — {}", d))
                                .unwrap_or_default();
                            lines.push(Line::from(vec![
                                Span::raw("  • "),
                                Span::styled(name, Style::default().fg(Color::Cyan)),
                                Span::styled(desc, Style::default().fg(Color::DarkGray)),
                            ]));
                        }
                        if templates.len() > 30 {
                            lines.push(Line::from(format!(
                                "  ... and {} more",
                                templates.len() - 30
                            )));
                        }
                    }
                }
                SkillSection::Active => {
                    let bundles = reg.list_bundles();
                    lines.push(Line::from(format!(
                        "  {} bundle(s) available",
                        bundles.len()
                    )));
                    lines.push(Line::from(""));
                    if bundles.is_empty() {
                        lines.push(Line::from("  No active bundles."));
                    } else {
                        for b in &bundles {
                            let name = b.name.to_string();
                            let version = b.version.to_string();
                            let desc: String = b
                                .description
                                .as_deref()
                                .map(|d| format!(" — {}", d))
                                .unwrap_or_default();
                            lines.push(Line::from(vec![
                                Span::raw("  • "),
                                Span::styled(name, Style::default().fg(Color::Green)),
                                Span::raw(format!("  v{}", version)),
                                Span::styled(desc, Style::default().fg(Color::DarkGray)),
                            ]));
                            lines.push(Line::from(format!("     {} skill(s)", b.skill_count)));
                        }
                    }
                }
            }
        } else {
            match self.section {
                SkillSection::Installed => {
                    lines.push(Line::from("  Installed skills from .agents/skills/"));
                    lines.push(Line::from("  Use /skill list to see installed skills."));
                }
                SkillSection::Available => {
                    lines.push(Line::from("  Skills available in the registry:"));
                    lines.push(Line::from("    • coding-guidelines"));
                    lines.push(Line::from("    • tdd"));
                    lines.push(Line::from("    • diagnose"));
                }
                SkillSection::Active => {
                    lines.push(Line::from("  Currently active skills:"));
                    lines.push(Line::from("    • None active"));
                }
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Skills are PDCA FlowDef loops with quality threshold + energy budget.",
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
