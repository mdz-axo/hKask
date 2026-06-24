//! Research window — web search, RSS feeds, content extraction.
//!
//! `]` forward, `[` backward through Search→Feeds→Extract→Chat.

use crate::bridges::ResearchDataBridge;
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
enum ResearchSection {
    Search,
    Feeds,
    Extract,
}
impl ResearchSection {
    fn next(self) -> Self {
        match self {
            Self::Search => Self::Feeds,
            Self::Feeds => Self::Extract,
            Self::Extract => Self::Search,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Search => Self::Extract,
            Self::Feeds => Self::Search,
            Self::Extract => Self::Feeds,
        }
    }
    fn title(&self) -> &str {
        match self {
            Self::Search => "Search",
            Self::Feeds => "Feeds",
            Self::Extract => "Extract",
        }
    }
}

pub struct ResearchWindow {
    id: WindowId,
    section: ResearchSection,
    active_tab: McpTab,
    chat_state: McpChatState,
    bridge: Arc<dyn ReplBridge>,
    research: Option<Arc<dyn ResearchDataBridge>>,
}

impl ResearchWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: ResearchSection::Search,
            active_tab: McpTab::Data,
            chat_state: McpChatState::new(),
            bridge,
            research: None,
        }
    }
    pub fn with_research_bridge(mut self, r: Arc<dyn ResearchDataBridge>) -> Self {
        self.research = Some(r);
        self
    }
}

impl Window for ResearchWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        match self.active_tab {
            McpTab::Chat => "Research Chat",
            McpTab::Data => "Research",
        }
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Research
    }
    fn render(&self, f: &mut Frame, area: Rect, _: bool) {
        match self.active_tab {
            McpTab::Chat => Self::default_render_chat_tab(&self.chat_state, "research", f, area),
            McpTab::Data => self.render_data_tab(f, area),
        }
    }
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(']') => {
                match self.active_tab {
                    McpTab::Chat => {
                        self.active_tab = McpTab::Data;
                        self.section = ResearchSection::Search;
                    }
                    McpTab::Data => {
                        self.section = self.section.next();
                        if self.section == ResearchSection::Search {
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
                        self.section = ResearchSection::Extract;
                    }
                    McpTab::Data => {
                        self.section = self.section.prev();
                        if self.section == ResearchSection::Extract {
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

impl McpTabbedWindow for ResearchWindow {
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
        "research"
    }
    fn render_chat_tab(&self, f: &mut Frame, area: Rect) {
        Self::default_render_chat_tab(&self.chat_state, "research", f, area);
    }
    fn render_data_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(Span::styled(
                format!("── Research: {} ([ ] to navigate) ──", self.section.title()),
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];
        let feed_data: Vec<(String, usize)> = self
            .research
            .as_ref()
            .map(|r| {
                r.feed_list()
                    .iter()
                    .map(|f| (f.title.clone(), f.unread))
                    .collect()
            })
            .unwrap_or_default();
        if let Some(ref r) = self.research {
            match self.section {
                ResearchSection::Search => {
                    if let Some(ref q) = r.last_query() {
                        lines.push(Line::from(format!("  Query: {}", q)));
                    }
                    let search_results = r.search("");
                    for result in &search_results {
                        lines.push(Line::from(vec![
                            Span::raw("  • "),
                            Span::styled(result.title.clone(), Style::default().fg(Color::Green)),
                            Span::raw(format!("  {}", result.url)),
                        ]));
                        lines.push(Line::from(format!("    {}", result.snippet)));
                    }
                }
                ResearchSection::Feeds => {
                    lines.push(Line::from(format!("  {} feed(s)", feed_data.len())));
                    for (title, count) in &feed_data {
                        let unread_str = if *count > 0 {
                            format!(" ({} unread)", count)
                        } else {
                            String::new()
                        };
                        lines.push(Line::from(vec![
                            Span::raw("  📡 "),
                            Span::styled(title.as_str(), Style::default().fg(Color::Cyan)),
                            Span::styled(unread_str, Style::default().fg(Color::Yellow)),
                        ]));
                    }
                }
                ResearchSection::Extract => {
                    lines.push(Line::from("  Extract content from URLs into markdown."));
                }
            }
        } else {
            lines.push(Line::from("  Use `kask mcp start research` to enable."));
        }
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}
