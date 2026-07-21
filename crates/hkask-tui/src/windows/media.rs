//! Media window — gallery browsing. Gallery, Collections, Recent.
//!
//! `]` forward, `[` backward through sections + Chat tab.

use crate::bridges::MediaDataBridge;
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
enum MediaSection {
    Gallery,
    Collections,
    Recent,
}
impl MediaSection {
    fn next(self) -> Self {
        match self {
            Self::Gallery => Self::Collections,
            Self::Collections => Self::Recent,
            Self::Recent => Self::Gallery,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Gallery => Self::Recent,
            Self::Collections => Self::Gallery,
            Self::Recent => Self::Collections,
        }
    }
    fn title(&self) -> &str {
        match self {
            Self::Gallery => "Gallery",
            Self::Collections => "Collections",
            Self::Recent => "Recent",
        }
    }
}

pub struct MediaWindow {
    id: WindowId,
    section: MediaSection,
    active_tab: McpTab,
    chat_state: McpChatState,
    bridge: Arc<dyn ReplBridge>,
    media: Option<Arc<dyn MediaDataBridge>>,
}

impl MediaWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: MediaSection::Gallery,
            active_tab: McpTab::Data,
            chat_state: McpChatState::new(),
            bridge,
            media: None,
        }
    }
    pub fn with_media_bridge(mut self, m: Arc<dyn MediaDataBridge>) -> Self {
        self.media = Some(m);
        self
    }
}

impl Window for MediaWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        match self.active_tab {
            McpTab::Chat => "Media Chat",
            McpTab::Data => "Media",
        }
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Media
    }
    fn render(&self, f: &mut Frame, area: Rect, _: bool) {
        match self.active_tab {
            McpTab::Chat => <MediaWindow as McpTabbedWindow>::default_render_chat_tab(
                &self.chat_state,
                "media",
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
                        self.section = MediaSection::Gallery;
                    }
                    McpTab::Data => {
                        self.section = self.section.next();
                        if self.section == MediaSection::Gallery {
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
                        self.section = MediaSection::Recent;
                    }
                    McpTab::Data => {
                        self.section = self.section.prev();
                        if self.section == MediaSection::Recent {
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

impl_mcp_tabbed!(MediaWindow, "media", |this, f, area| {
    let mut lines = vec![
        headers::section(format!("Media: {} ([ ] to navigate)", this.section.title())),
        Line::from(""),
    ];
    if let Some(ref m) = this.media {
        let gs = m.gallery_status();
        let images = m.recent_images(12);
        match this.section {
            MediaSection::Gallery => {
                if !gs.active {
                    lines.push(Line::from("  No gallery active."));
                } else {
                    lines.push(Line::from(format!(
                        "  Gallery: {}  Images: {}",
                        gs.gallery_id.as_deref().unwrap_or("-"),
                        gs.image_count
                    )));
                }
            }
            MediaSection::Collections => {
                lines.push(Line::from(format!("  {} image(s)", images.len())));
                for img in &images {
                    let path = img.path.clone();
                    lines.push(Line::from(vec![
                        Span::raw("  📷 "),
                        Span::styled(path, Style::default().fg(Color::Cyan)),
                        Span::styled(
                            format!("  {}×{}  {}", img.width, img.height, img.format),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }
            }
            MediaSection::Recent => {
                lines.push(Line::from(format!("  {} recent image(s)", images.len())));
                for img in &images {
                    let path = img.path.clone();
                    lines.push(Line::from(vec![
                        Span::raw("  📷 "),
                        Span::styled(path, Style::default().fg(Color::Cyan)),
                        Span::styled(
                            format!("  {}×{}  {}", img.width, img.height, img.format),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }
            }
        }
    } else {
        lines.push(Line::from("  No media MCP server connected."));
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
});
