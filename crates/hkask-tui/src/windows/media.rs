//! Media window — image, audio, and video gallery browsing.
//!
//! Tab-cycled sections: Gallery, Collections, Recent. Live data from
//! MediaDataBridge / hkask-mcp-media GalleryStore.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::bridges::MediaDataBridge;
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
    #[allow(dead_code)]
    bridge: Arc<dyn ReplBridge>,
    media: Option<Arc<dyn MediaDataBridge>>,
}

impl MediaWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: MediaSection::Gallery,
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
        "Media"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Media
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let mut lines = vec![
            Line::from(Span::styled(
                format!("── Media: {} (Tab to switch) ──", self.section.title()),
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
                        lines.push(Line::from("  Use gallery_organize to set up a gallery."));
                    } else {
                        lines.push(Line::from(format!(
                            "  Gallery: {}",
                            gs.gallery_id.as_deref().unwrap_or("-")
                        )));
                        lines.push(Line::from(format!("  Images: {}", gs.image_count)));
                        if let Some(ref root) = gs.root_path {
                            lines.push(Line::from(format!("  Root:   {}", root)));
                        }
                        lines.push(Line::from(""));
                        lines.push(Line::from(
                            "  Tools: gallery_search, gallery_find_similar, gallery_timeline",
                        ));
                    }
                }
                MediaSection::Collections => {
                    let images = m.recent_images(12);
                    lines.push(Line::from(format!(
                        "  {} image(s) in gallery",
                        images.len()
                    )));
                    lines.push(Line::from(""));
                    for img in &images {
                        let tags = if img.tags.is_empty() {
                            String::new()
                        } else {
                            format!("  [{}]", img.tags.join(", "))
                        };
                        lines.push(Line::from(format!(
                            "  [{}] {}  {}×{} {}",
                            img.index, img.path, img.width, img.height, tags
                        )));
                    }
                }
                MediaSection::Recent => {
                    let images = m.recent_images(8);
                    lines.push(Line::from(format!(
                        "  {} most recent image(s)",
                        images.len()
                    )));
                    lines.push(Line::from(""));
                    // Collect owned image data before pushing
                    let image_data: Vec<(String, u32, u32, String)> = images
                        .iter()
                        .map(|img| {
                            (
                                img.path.to_string(),
                                img.width,
                                img.height,
                                img.format.clone(),
                            )
                        })
                        .collect();
                    for (path, width, height, format) in &image_data {
                        lines.push(Line::from(vec![
                            Span::raw("  📷 "),
                            Span::styled(path.clone(), Style::default().fg(Color::Cyan)),
                            Span::styled(
                                format!("  {}×{}  {}", width, height, format),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]));
                    }
                }
            }
        } else {
            match self.section {
                MediaSection::Gallery => {
                    lines.push(Line::from(
                        "  Gallery organizes images for browsing and search.",
                    ));
                    lines.push(Line::from("  Use gallery_organize to set up a gallery."));
                    lines.push(Line::from(
                        "  Auto-analyzes faces, objects, colors, composition.",
                    ));
                }
                MediaSection::Collections => {
                    lines.push(Line::from("  Browse images by tag, face, or timeline."));
                    lines.push(Line::from("  Use gallery_search to find tagged images."));
                    lines.push(Line::from("  Use gallery_timeline for EXIF-date browsing."));
                }
                MediaSection::Recent => {
                    lines.push(Line::from("  Most recently added images."));
                    lines.push(Line::from("  Use gallery_refresh to rescan."));
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
