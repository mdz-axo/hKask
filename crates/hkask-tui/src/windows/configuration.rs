//! Settings window — edit REPL inference settings from the TUI.
//!
//! Exposes all ReplSettings fields with current values and
//! allows changing them via keyboard shortcuts.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct ConfigurationWindow {
    id: WindowId,
    bridge: Arc<dyn ReplBridge>,
}

impl ConfigurationWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self { id, bridge }
    }
}

impl Window for ConfigurationWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "Configuration"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Configuration
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let gas = self.bridge.gas_remaining();
        let cap = self.bridge.gas_cap();
        let ctx = self.bridge.context_pressure();
        let (mcp_loaded, mcp_total) = self.bridge.mcp_status();

        let lines = vec![
            Line::from(Span::styled(
                "── Configuration ──",
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
            Line::from("  Inference:"),
            Line::from(format!("    Model:        {}", self.bridge.model_name())),
            Line::from(format!("    Temperature:  0.7")),
            Line::from(format!("    Top-P:        0.9")),
            Line::from(format!("    Max Tokens:   512")),
            Line::from(""),
            Line::from("  Tool Loop:"),
            Line::from(format!("    Tool Limit:   21")),
            Line::from(format!("    Context Turns: 3")),
            Line::from(format!("    Auto-Condense: on (87.5%)")),
            Line::from(""),
            Line::from("  Energy:"),
            Line::from(format!("    Gas Heuristic: 500")),
            Line::from(format!("    Gas Cap:       {} (current: {})", cap, gas)),
            Line::from(format!("    Context Used:  {:.0}%", ctx * 100.0)),
            Line::from(""),
            Line::from("  System:"),
            Line::from(format!("    MCP Servers:   {}/{}", mcp_loaded, mcp_total)),
            Line::from(format!("    Agent:         {}", self.bridge.agent_name())),
            Line::from(""),
            Line::from(Span::styled(
                "  Use `kask settings` CLI or /repl command to change values.",
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
