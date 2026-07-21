//! Memory window — browse agent memories. Episodic, Semantic, Triples, Consolidation.
//!
//! `]` forward, `[` backward through sections + Chat tab.

use crate::bridges::MemoryDataBridge;
use crate::impl_mcp_tabbed;
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
enum MemorySection {
    Episodic,
    Semantic,
    Triples,
    Consolidation,
}
impl MemorySection {
    fn next(self) -> Self {
        match self {
            Self::Episodic => Self::Semantic,
            Self::Semantic => Self::Triples,
            Self::Triples => Self::Consolidation,
            Self::Consolidation => Self::Episodic,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Episodic => Self::Consolidation,
            Self::Semantic => Self::Episodic,
            Self::Triples => Self::Semantic,
            Self::Consolidation => Self::Triples,
        }
    }
    fn title(&self) -> &str {
        match self {
            Self::Episodic => "Episodic",
            Self::Semantic => "Semantic",
            Self::Triples => "Triples",
            Self::Consolidation => "Consolidation",
        }
    }
}

pub struct MemoryWindow {
    id: WindowId,
    section: MemorySection,
    active_tab: McpTab,
    chat_state: McpChatState,
    bridge: Arc<dyn ReplBridge>,
    memory: Option<Arc<dyn MemoryDataBridge>>,
}

impl MemoryWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: MemorySection::Episodic,
            active_tab: McpTab::Data,
            chat_state: McpChatState::new(),
            bridge,
            memory: None,
        }
    }
    pub fn with_memory_bridge(mut self, m: Arc<dyn MemoryDataBridge>) -> Self {
        self.memory = Some(m);
        self
    }
}

impl Window for MemoryWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        match self.active_tab {
            McpTab::Chat => "Memory Chat",
            McpTab::Data => "Memory",
        }
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Memory
    }
    fn render(&self, f: &mut Frame, area: Rect, _: bool) {
        match self.active_tab {
            McpTab::Chat => <MemoryWindow as McpTabbedWindow>::default_render_chat_tab(
                &self.chat_state,
                "memory",
                f,
                area,
            ),
            McpTab::Data => self.render_data_tab(f, area),
        }
    }
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(']') => {
                match self.active_tab {
                    McpTab::Chat => {
                        self.active_tab = McpTab::Data;
                        self.section = MemorySection::Episodic;
                    }
                    McpTab::Data => {
                        self.section = self.section.next();
                        if self.section == MemorySection::Episodic {
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
                        self.section = MemorySection::Consolidation;
                    }
                    McpTab::Data => {
                        self.section = self.section.prev();
                        if self.section == MemorySection::Consolidation {
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

impl_mcp_tabbed!(MemoryWindow, "memory", |this, f, area| {
    let mut lines = vec![
        headers::section(format!(
            "Memory: {} ([ ] to navigate)",
            this.section.title()
        )),
        Line::from(""),
    ];
    if let Some(ref mem) = this.memory {
        let summary = mem.memory_summary();
        let episodic = mem.recent_episodic(15);
        let semantic = mem.recent_semantic(15);
        let cs = mem.consolidation_status();
        match this.section {
            MemorySection::Episodic => {
                lines.push(Line::from(format!(
                    "  Episodic: {} / {} ({:.0}%)",
                    summary.episodic_count,
                    summary.episodic_budget,
                    if summary.episodic_budget > 0 {
                        (summary.episodic_count as f64 / summary.episodic_budget as f64) * 100.0
                    } else {
                        0.0
                    }
                )));
                for t in &episodic {
                    let entity = format!("{}", t.entity);
                    let attr = format!("{}", t.attribute);
                    let val = format!("{}", t.value);
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(entity, Style::default().fg(Color::Cyan)),
                        Span::raw(" · "),
                        Span::styled(attr, Style::default().fg(Color::Yellow)),
                        Span::raw(" = "),
                        Span::styled(val, Style::default().fg(Color::White)),
                    ]));
                }
            }
            MemorySection::Semantic => {
                lines.push(Line::from(format!(
                    "  Semantic: {} (low: {})",
                    summary.semantic_count, summary.semantic_low_confidence
                )));
                for t in &semantic {
                    let entity = format!("{}", t.entity);
                    let attr = format!("{}", t.attribute);
                    let val = format!("{}", t.value);
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(entity, Style::default().fg(Color::Green)),
                        Span::raw(" · "),
                        Span::styled(attr, Style::default().fg(Color::Yellow)),
                        Span::raw(" = "),
                        Span::styled(val, Style::default().fg(Color::White)),
                    ]));
                }
            }
            MemorySection::Triples => {
                lines.push(Line::from(format!(
                    "  Episodic: {}   Semantic: {}",
                    summary.episodic_count, summary.semantic_count
                )));
                lines.push(Line::from(
                    "  Each h_mem: entity, attribute, value, confidence, visibility, owner WebID",
                ));
            }
            MemorySection::Consolidation => {
                lines.push(Line::from(format!(
                    "  Candidates: {}   Semantic: {}   Low-conf: {}",
                    cs.candidate_count, cs.semantic_count, cs.low_confidence_count
                )));
            }
        }
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
});
