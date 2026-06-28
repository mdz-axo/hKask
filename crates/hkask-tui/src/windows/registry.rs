//! Registry window — browse templates, skills, styles, and bundles.
//!
//! Live data from RegistryDataBridge (SqliteRegistry). Tab-cycled
//! sections: Templates, Skills, Styles, Bundles.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use crate::widgets::headers;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::bridges::RegistryDataBridge;
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegistrySection {
    Templates,
    Skills,
    Styles,
    Bundles,
}

impl RegistrySection {
    fn next(self) -> Self {
        match self {
            Self::Templates => Self::Skills,
            Self::Skills => Self::Styles,
            Self::Styles => Self::Bundles,
            Self::Bundles => Self::Templates,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Templates => Self::Bundles,
            Self::Skills => Self::Templates,
            Self::Styles => Self::Skills,
            Self::Bundles => Self::Styles,
        }
    }
    fn title(&self) -> &str {
        match self {
            Self::Templates => "Templates",
            Self::Skills => "Skills",
            Self::Styles => "Styles",
            Self::Bundles => "Bundles",
        }
    }
}

pub struct RegistryWindow {
    id: WindowId,
    section: RegistrySection,
    #[allow(dead_code)]
    bridge: Arc<dyn ReplBridge>,
    registry: Option<Arc<dyn RegistryDataBridge>>,
}

impl RegistryWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: RegistrySection::Templates,
            bridge,
            registry: None,
        }
    }

    pub fn with_registry_bridge(mut self, reg: Arc<dyn RegistryDataBridge>) -> Self {
        self.registry = Some(reg);
        self
    }
}

impl Window for RegistryWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "Registry"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Registry
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let mut lines = vec![headers::section(format!(
            "Registry: {} (Tab to switch)",
            self.section.title()
        ))];
            Line::from(""),
        ];

        if let Some(ref reg) = self.registry {
            match self.section {
                RegistrySection::Templates => {
                    let templates = reg.list_templates();
                    lines.push(Line::from(format!(
                        "  {} template(s) registered",
                        reg.template_count()
                    )));
                    lines.push(Line::from(""));
                    for t in templates.iter().take(20) {
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
                    if reg.template_count() > 20 {
                        lines.push(Line::from(format!(
                            "  ... and {} more",
                            reg.template_count() - 20
                        )));
                    }
                }
                RegistrySection::Skills => {
                    let skills = reg.list_skills();
                    lines.push(Line::from(format!(
                        "  {} skill(s) registered",
                        reg.skill_count()
                    )));
                    lines.push(Line::from(""));
                    for s in skills.iter().take(20) {
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
                RegistrySection::Styles => {
                    lines.push(Line::from("  Styles are prose composition templates."));
                    lines.push(Line::from("  Use /style compose <name> to apply a style."));
                    lines.push(Line::from("  Use /style list to browse available styles."));
                }
                RegistrySection::Bundles => {
                    let bundles = reg.list_bundles();
                    lines.push(Line::from(format!(
                        "  {} bundle(s) registered",
                        reg.bundle_count()
                    )));
                    lines.push(Line::from(""));
                    for b in bundles.iter() {
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
        } else {
            match self.section {
                RegistrySection::Templates => {
                    lines.push(Line::from(
                        "  Templates are WordAct/FlowDef/KnowAct manifests.",
                    ));
                    lines.push(Line::from(
                        "  Use /template list to browse available templates.",
                    ));
                    lines.push(Line::from(
                        "  Use /template run <id> to execute a template.",
                    ));
                }
                RegistrySection::Skills => {
                    lines.push(Line::from("  Skills are iterative PDCA FlowDef loops."));
                    lines.push(Line::from("  Use /skill list to see installed skills."));
                    lines.push(Line::from(
                        "  Use /skill status <name> for detailed status.",
                    ));
                }
                RegistrySection::Styles => {
                    lines.push(Line::from("  Styles are prose composition templates."));
                    lines.push(Line::from("  Use /style compose <name> to apply a style."));
                    lines.push(Line::from("  Use /style list to browse available styles."));
                }
                RegistrySection::Bundles => {
                    lines.push(Line::from("  Bundles compose multiple skills together."));
                    lines.push(Line::from("  Use /bundle compose to create a bundle."));
                    lines.push(Line::from("  Use /bundle apply <id> to activate a bundle."));
                }
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Use `kask list` CLI for full registry browsing.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Char('[') {
            self.section = self.section.prev();
            true
        } else if key.code == KeyCode::Char(']') {
            self.section = self.section.next();
            true
        } else {
            false
        }
    }
    fn tick(&mut self) {}
}
