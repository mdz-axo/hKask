//! Training window — monitor training sessions, LoRA adapters, and artifacts.

use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct TrainingWindow {
    id: WindowId,
    #[allow(dead_code)]
    bridge: Arc<dyn ReplBridge>,
}

impl TrainingWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self { id, bridge }
    }
}

impl Window for TrainingWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "Training"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Training
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let lines = vec![
            Line::from(Span::styled(
                "── Training Sessions ──",
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
            Line::from("  Active Sessions: 0"),
            Line::from("  Completed:       0"),
            Line::from(""),
            Line::from(Span::styled(
                "  LoRA Adapters:",
                Style::default().fg(Color::Yellow),
            )),
            Line::from("    • None deployed"),
            Line::from("    Use /adapter deploy to load an adapter"),
            Line::from(""),
            Line::from(Span::styled(
                "  Training Artifacts:",
                Style::default().fg(Color::Yellow),
            )),
            Line::from("    • ~/.config/hkask/adapters/"),
            Line::from("    • ~/.config/hkask/sessions/"),
            Line::from(""),
            Line::from(Span::styled(
                "  Use `axolotl` CLI for fine-tuning, then deploy adapters via /adapter.",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
    fn tick(&mut self) {}
}
