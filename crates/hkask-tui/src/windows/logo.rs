//! Logo window — persistent Kask logo in the top-left corner.
//!
//! Renders a downscaled version of the Kask vintage milk can logo using half-block
//! Unicode characters. The window is persistent and cannot
//! be closed — it anchors the workspace identity.

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::window::{Window, WindowId, WindowKind};

pub struct LogoWindow {
    id: WindowId,
}

impl LogoWindow {
    pub fn new(id: WindowId) -> Self {
        Self { id }
    }
}

impl Window for LogoWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "hKask"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Logo
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        // Guard: skip rendering on tiny allocations from deep splits.
        if area.width < 3 || area.height < 3 {
            return;
        }
        let lines = crate::splash::build_logo_window_lines();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(40, 42, 54)))
            .title(" hKask ");
        let inner = block.inner(area);
        f.render_widget(block, area);
        f.render_widget(Paragraph::new(lines), inner);
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
    fn tick(&mut self) {}
}
