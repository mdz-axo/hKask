//! Media window — image, audio, and video gallery browsing.
//!
//! Tab-cycled sections: Gallery, Collections, Recent. Live data from
//! MediaDataBridge / hkask-mcp-media GalleryStore.
//!
//! Adopts the MCP two-tab design (TUI_SPECIFICATION.md §3):
//! - Tab 1 (Chat): Focused chat scoped to the Media MCP server
//! - Tab 2 (Data): Gallery, Collections, Recent sections
//!
//! Tab key: cycles Gallery → Collections → Recent → Chat → Gallery.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::bridges::MediaDataBridge;
use crate::mcp_tabbed::{McpChatState, McpTab, McpTabbedWindow};
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

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
    #[allow(dead_code)]
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
    fn id(&self) -> WindowId { self.id }
    fn title(&self) -> &str {
        match self.active_tab {
            McpTab::Chat => "Media Chat",
            McpTab::Data => "Media",
        }
    }
    fn kind(&self) -> WindowKind { WindowKind::Media }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        match self.active_tab {
            McpTab::Chat => {
                Self::default_render_chat_tab(&self.chat_state, "media", f, area);
            }
            McpTab::Data => self.render_data_tab(f, area),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Tab {
            match self.active_tab {
                McpTab::Chat => {
                    self.active_tab = McpTab::Data;
                    self.section = MediaSection::Gallery;
                    return true;
                }
                McpTab::Data => {
                    self.section = self.section.next();
                    if self.section == MediaSection::Gallery {
                        self.active_tab = McpTab::Chat;
                    }
                    return true;
                }
            }
        }

        match self.active_tab {
            McpTab::Chat => {
                if let Some(_msg) = self.handle_chat_key(key) { return true; }
                matches!(key.code, KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Enter | KeyCode::Esc)
            }
            McpTab::Data => false,
        }
    }
    fn tick(&mut self) {}
}

impl McpTabbedWindow for MediaWindow {
    fn active_tab(&self) -> McpTab { self.active_tab }
    fn set_active_tab(&mut self, tab: McpTab) { self.active_tab = tab; }
    fn chat_state_mut(&mut self) -> &mut McpChatState { &mut self.chat_state }
    fn mcp_server_name(&self) -> &str { "media" }

    fn render_chat_tab(&self, f: &mut Frame, area: Rect) {
        Self::default_render_chat_tab(&self.chat_state, "media", f, area);
    }

    fn render_data_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![
            Line::from(Span::styled(
                format!("── Media: {} (Tab: next | Tab×3: Chat) ──", self.section.title()),
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];

        if let Some(ref m) = self.media {
            let gs = m.gallery_status();
            match self.section {
                MediaSection::Gallery => {
                    if !gs.active {
                        lines.push(Line::from("  No gallery active."));
                    } else {
                        lines.push(Line::from(format!("  Gallery: {}", gs.gallery_id.as_deref().unwrap_or("-"))));
                        lines.push(Line::from(format!("  Images: {}", gs.image_count)));
                        if let Some(ref root) = gs.root_path {
                            lines.push(Line::from(format!("  Root:   {}", root)));
                        }
                    }
                }
                MediaSection::Collections => {
                    let images = m.recent_images(12);
                    lines.push(Line::from(format!("  {} image(s) in gallery", images.len())));
                    lines.push(Line::from(""));
                    let image_data: Vec<(String, u32, u32, String)> = images.iter().map(|img| {
                        (img.path.clone(), img.width, img.height, img.format.clone())
                    }).collect();
                    for (path, width, height, format) in &image_data {
                        lines.push(Line::from(vec![
                            Span::raw("  📷 "),
                            Span::styled(path.clone(), Style::default().fg(Color::Cyan)),
                            Span::styled(format!("  {}×{}  {}", width, height, format), Style::default().fg(Color::DarkGray)),
                        ]));
                    }
                }
                MediaSection::Recent => {
                    let images = m.recent_images(8);
                    lines.push(Line::from(format!("  {} most recent image(s)", images.len())));
                    lines.push(Line::from(""));
                    let image_data: Vec<(String, u32, u32, String)> = images.iter().map(|img| {
                        (img.path.clone(), img.width, img.height, img.format.clone())
                    }).collect();
                    for (path, width, height, format) in &image_data {
                        lines.push(Line::from(vec![
                            Span::raw("  📷 "),
                            Span::styled(path.clone(), Style::default().fg(Color::Cyan)),
                            Span::styled(format!("  {}×{}  {}", width, height, format), Style::default().fg(Color::DarkGray)),
                        ]));
                    }
                }
            }
        } else {
            match self.section {
                MediaSection::Gallery => {
                    lines.push(Line::from("  Gallery organizes images for browsing and search."));
                    lines.push(Line::from("  Use gallery_organize to set up a gallery."));
                }
                MediaSection::Collections => {
                    lines.push(Line::from("  Browse images by tag, face, or timeline."));
                }
                MediaSection::Recent => {
                    lines.push(Line::from("  Most recently added images."));
                }
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Media MCP server: generate_image, describe_image, video_*, audio_*. 28 tools total.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}
