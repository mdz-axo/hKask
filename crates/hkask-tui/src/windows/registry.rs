//! Registry window — browse templates, skills, styles, and bundles.

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
}

impl RegistryWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: RegistrySection::Templates,
            bridge,
        }
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
        let mut lines = vec![
            Line::from(Span::styled(
                format!("── Registry: {} (Tab to switch) ──", self.section.title()),
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];
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
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Use `kask list` CLI for full registry browsing.",
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
