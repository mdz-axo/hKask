//! Docproc window — document processing: chunk, QA, RDF, embeddings.
//!
//! `]` forward, `[` backward through Chunks→QA→Index→Chat.

use crate::bridges::DocprocDataBridge;
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
enum DocprocSection {
    Chunks,
    QA,
    Index,
}
impl DocprocSection {
    fn next(self) -> Self {
        match self {
            Self::Chunks => Self::QA,
            Self::QA => Self::Index,
            Self::Index => Self::Chunks,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Chunks => Self::Index,
            Self::QA => Self::Chunks,
            Self::Index => Self::QA,
        }
    }
    fn title(&self) -> &str {
        match self {
            Self::Chunks => "Chunks",
            Self::QA => "QA",
            Self::Index => "Index",
        }
    }
}

pub struct DocprocWindow {
    id: WindowId,
    section: DocprocSection,
    active_tab: McpTab,
    chat_state: McpChatState,
    bridge: Arc<dyn ReplBridge>,
    docproc: Option<Arc<dyn DocprocDataBridge>>,
}

impl DocprocWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: DocprocSection::Chunks,
            active_tab: McpTab::Data,
            chat_state: McpChatState::new(),
            bridge,
            docproc: None,
        }
    }
    pub fn with_docproc_bridge(mut self, d: Arc<dyn DocprocDataBridge>) -> Self {
        self.docproc = Some(d);
        self
    }
}

impl Window for DocprocWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        match self.active_tab {
            McpTab::Chat => "Docproc Chat",
            McpTab::Data => "Docproc",
        }
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Docproc
    }
    fn render(&self, f: &mut Frame, area: Rect, _: bool) {
        match self.active_tab {
            McpTab::Chat => <DocprocWindow as McpTabbedWindow>::default_render_chat_tab(
                &self.chat_state,
                "docproc",
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
                        self.section = DocprocSection::Chunks;
                    }
                    McpTab::Data => {
                        self.section = self.section.next();
                        if self.section == DocprocSection::Chunks {
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
                        self.section = DocprocSection::Index;
                    }
                    McpTab::Data => {
                        self.section = self.section.prev();
                        if self.section == DocprocSection::Index {
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

impl_mcp_tabbed!(DocprocWindow, "docproc", |this, f, area| {
    let mut lines = vec![
        headers::section(format!(
            "Docproc: {} ([ ] to navigate)",
            this.section.title()
        )),
        Line::from(""),
    ];
    let chunk_data: Vec<(usize, usize, String)> = this
        .docproc
        .as_ref()
        .map(|d| {
            d.chunk_list()
                .iter()
                .map(|c| (c.index, c.token_count, c.preview.clone()))
                .collect()
        })
        .unwrap_or_default();
    let _qa_data: Vec<(String, String, String)> = this
        .docproc
        .as_ref()
        .map(|d| {
            d.qa_list()
                .iter()
                .map(|q| (q.question.clone(), q.answer.clone(), q.level.clone()))
                .collect()
        })
        .unwrap_or_default();
    let (indexed, total) = this
        .docproc
        .as_ref()
        .map(|d| d.index_status())
        .unwrap_or((0, 0));
    if let Some(ref d) = this.docproc {
        match this.section {
            DocprocSection::Chunks => {
                lines.push(Line::from(format!("  {} chunk(s)", chunk_data.len())));
                for (idx, tokens, preview) in &chunk_data {
                    lines.push(Line::from(vec![
                        Span::raw(format!("  [{}] ", idx)),
                        Span::styled(
                            format!("{} tokens", tokens),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::raw(" — "),
                        Span::styled(preview.as_str(), Style::default().fg(Color::DarkGray)),
                    ]));
                }
            }
            DocprocSection::QA => {
                let qas = d.qa_list();
                lines.push(Line::from(format!("  {} QA pair(s)", qas.len())));
                for q in &qas {
                    lines.push(Line::from(vec![
                        Span::raw("  Q: "),
                        Span::styled(q.question.clone(), Style::default().fg(Color::Cyan)),
                        Span::raw(format!("  [{}]", q.level)),
                    ]));
                    lines.push(Line::from(format!("  A: {}", q.answer)));
                }
            }
            DocprocSection::Index => {
                lines.push(Line::from(format!("  Indexed: {} / {}", indexed, total)));
            }
        }
    } else {
        lines.push(Line::from("  Use `kask mcp start docproc` to enable."));
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
});
