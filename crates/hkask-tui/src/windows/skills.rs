//! Skills window — skill corpus browser, template executor, and editor.
//!
//! `]` forward, `[` backward through Browse→Execute→Active→Chat.

use crate::bridges::{RegistryDataBridge, SkillsDataBridge};
use crate::mcp_tabbed::{McpChatState, McpTab, McpTabbedWindow};
use crate::repl_bridge::ReplBridge;
use crate::widgets::headers;
use crate::window::{Window, WindowId, WindowKind};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkillSection {
    Browse,
    Execute,
    Active,
}
impl SkillSection {
    fn next(self) -> Self {
        match self {
            Self::Browse => Self::Execute,
            Self::Execute => Self::Active,
            Self::Active => Self::Browse,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Browse => Self::Active,
            Self::Execute => Self::Browse,
            Self::Active => Self::Execute,
        }
    }
    fn title(&self) -> &str {
        match self {
            Self::Browse => "Browse",
            Self::Execute => "Execute",
            Self::Active => "Active",
        }
    }
}

pub struct SkillsWindow {
    id: WindowId,
    section: SkillSection,
    active_tab: McpTab,
    chat_state: McpChatState,
    bridge: Arc<dyn ReplBridge>,
    registry: Option<Arc<dyn RegistryDataBridge>>,
    skills: Option<Arc<dyn SkillsDataBridge>>,
}

impl SkillsWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: SkillSection::Browse,
            active_tab: McpTab::Data,
            chat_state: McpChatState::new(),
            bridge,
            registry: None,
            skills: None,
        }
    }
    pub fn with_registry_bridge(mut self, r: Arc<dyn RegistryDataBridge>) -> Self {
        self.registry = Some(r);
        self
    }
    pub fn with_skills_bridge(mut self, s: Arc<dyn SkillsDataBridge>) -> Self {
        self.skills = Some(s);
        self
    }
}

impl Window for SkillsWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        match self.active_tab {
            McpTab::Chat => "Skills Chat",
            McpTab::Data => "Skills",
        }
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Skills
    }
    fn render(&self, f: &mut Frame, area: Rect, _: bool) {
        match self.active_tab {
            McpTab::Chat => Self::default_render_chat_tab(&self.chat_state, "skill", f, area),
            McpTab::Data => self.render_data_tab(f, area),
        }
    }
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(']') => {
                match self.active_tab {
                    McpTab::Chat => {
                        self.active_tab = McpTab::Data;
                        self.section = SkillSection::Browse;
                    }
                    McpTab::Data => {
                        self.section = self.section.next();
                        if self.section == SkillSection::Browse {
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
                        self.section = SkillSection::Active;
                    }
                    McpTab::Data => {
                        self.section = self.section.prev();
                        if self.section == SkillSection::Active {
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
                    let bridge = self.bridge.clone();
                    self.start_chat_request(bridge.as_ref(), msg);
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
    fn tick(&mut self) {
        let bridge = self.bridge.clone();
        self.poll_chat_request(bridge.as_ref());
    }
}

impl McpTabbedWindow for SkillsWindow {
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
        "skill"
    }
    fn render_chat_tab(&self, f: &mut Frame, area: Rect) {
        Self::default_render_chat_tab(&self.chat_state, "skill", f, area);
    }
    fn render_data_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![
            headers::section(format!(
                "Skills: {} ([ ] to navigate)",
                self.section.title()
            )),
            Line::from(""),
        ];
        match self.section {
            SkillSection::Browse => {
                if let Some(ref sk) = self.skills {
                    let list = sk.skill_list();
                    lines.push(Line::from(format!("  {} skill(s) registered", list.len())));
                    let skill_items: Vec<(String, String)> = list
                        .iter()
                        .map(|s| (s.id.clone(), s.description.clone()))
                        .collect();
                    for (id, desc) in &skill_items {
                        lines.push(Line::from(vec![
                            Span::raw("  • "),
                            Span::styled(id.clone(), Style::default().fg(Color::Green)),
                            Span::raw(" — "),
                            Span::styled(desc.clone(), Style::default().fg(Color::DarkGray)),
                        ]));
                    }
                } else if let Some(ref r) = self.registry {
                    lines.push(Line::from(format!(
                        "  Templates: {}   Bundles: {}",
                        r.template_count(),
                        r.bundle_count()
                    )));
                    let tmpl_ids: Vec<String> =
                        r.list_templates().iter().map(|t| t.id.clone()).collect();
                    for id in &tmpl_ids {
                        lines.push(Line::from(vec![
                            Span::raw("  • "),
                            Span::styled(id.clone(), Style::default().fg(Color::Green)),
                        ]));
                    }
                } else {
                    lines.push(Line::from("  Use `kask mcp start skill` to enable."));
                }
            }
            SkillSection::Execute => {
                lines.push(Line::from(
                    "  Execute a skill template with context variables.",
                ));
                lines.push(Line::from(
                    "  Use the Chat tab to run: skill_execute <id> <context_json>",
                ));
                if let Some(ref sk) = self.skills {
                    lines.push(Line::from(format!(
                        "  {} skill(s) available for execution",
                        sk.skill_count()
                    )));
                }
            }
            SkillSection::Active => {
                if let Some(ref r) = self.registry {
                    let bundles = r.list_bundles();
                    for b in &bundles {
                        lines.push(Line::from(vec![
                            Span::raw("  • "),
                            Span::styled(b.name.clone(), Style::default().fg(Color::Magenta)),
                            Span::raw(format!(" v{}  ({} skills)", b.version, b.skill_count)),
                        ]));
                    }
                } else {
                    lines.push(Line::from("  No active bundles."));
                }
            }
        }
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}
