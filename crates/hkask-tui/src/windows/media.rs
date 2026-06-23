//! Media window — browse and manage gallery of media files (images, audio, video).

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
}

impl MediaWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: MediaSection::Gallery,
            bridge,
        }
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
        match self.section {
            MediaSection::Gallery => {
                lines.push(Line::from("  Gallery directory: ~/.config/hkask/gallery/"));
                lines.push(Line::from(
                    "  Supported formats: PNG, JPEG, GIF, WebP, WAV, MP3, MP4",
                ));
                lines.push(Line::from(
                    "  Use /listen to capture audio, /talk for TTS output.",
                ));
            }
            MediaSection::Collections => {
                lines.push(Line::from("  Collections group related media files."));
                lines.push(Line::from("  Create collections via the media MCP server."));
                lines.push(Line::from(
                    "  Use `kask mcp start media` to enable media tools.",
                ));
            }
            MediaSection::Recent => {
                lines.push(Line::from("  Recently generated images appear here."));
                lines.push(Line::from("  Recent audio recordings and transcripts."));
                lines.push(Line::from("  Recent TTS outputs from /talk sessions."));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Full media management via media MCP server and /listen /talk commands.",
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
