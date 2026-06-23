//! Skills window — browse and manage the skill corpus.

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
}

impl SkillsWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: SkillSection::Installed,
            bridge,
        }
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
        match self.section {
            SkillSection::Installed => {
                lines.push(Line::from(
                    "  Installed skills from .agents/skills/ and registry/",
                ));
                lines.push(Line::from("  46 total templates available."));
                lines.push(Line::from("  Use /skill list to see installed skills."));
                lines.push(Line::from(
                    "  Use /skill status <name> for detailed status.",
                ));
            }
            SkillSection::Available => {
                lines.push(Line::from("  Skills available in the registry:"));

                lines.push(Line::from("    • coding-guidelines"));
                lines.push(Line::from("    • tdd"));
                lines.push(Line::from("    • diagnose"));
                lines.push(Line::from("    • deep-module"));
                lines.push(Line::from("    • essentialist"));
                lines.push(Line::from("    • pragmatic-semantics"));
                lines.push(Line::from("    • pragmatic-cybernetics"));
                lines.push(Line::from("    • ... and more"));
            }
            SkillSection::Active => {
                lines.push(Line::from("  Currently active skills (via /bundle apply):"));
                lines.push(Line::from("    • None active"));
                lines.push(Line::from(
                    "  Use /bundle compose to create a skill bundle.",
                ));
                lines.push(Line::from("  Use /bundle apply <id> to activate."));
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
