//! Settings window — display inference and system settings.
//!
//! Live data items (gas, context pressure, MCP, agent, model) come from
//! ReplBridge. Explicit settings (temperature, top-p, tool loop, etc.)
//! come from ConfigDataBridge.

use std::sync::Arc;

use crate::widgets::headers;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::bridges::ConfigDataBridge;
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct ConfigurationWindow {
    id: WindowId,
    bridge: Arc<dyn ReplBridge>,
    config: Option<Arc<dyn ConfigDataBridge>>,
}

impl ConfigurationWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            bridge,
            config: None,
        }
    }

    pub fn with_config_bridge(mut self, config: Arc<dyn ConfigDataBridge>) -> Self {
        self.config = Some(config);
        self
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

        let temp;
        let top_p;
        let max_tokens;
        let tool_limit;
        let ctx_turns;
        let gas_heuristic;
        let auto_condense;

        if let Some(ref cfg) = self.config {
            let snap = cfg.config_snapshot();
            temp = format!("{:.2}", snap.temperature);
            top_p = format!("{:.2}", snap.top_p);
            max_tokens = format!("{}", snap.max_tokens);
            tool_limit = format!("{}", snap.tool_loop_limit);
            ctx_turns = format!("{}", snap.context_turns);
            gas_heuristic = format!("{}", snap.gas_heuristic);
            auto_condense = if snap.auto_condense {
                "on (87.5%)"
            } else {
                "off"
            };
        } else {
            temp = "0.70".into();
            top_p = "0.90".into();
            max_tokens = "512".into();
            tool_limit = "21".into();
            ctx_turns = "3".into();
            gas_heuristic = "500".into();
            auto_condense = "on (87.5%)";
        }

        let mut lines = vec![headers::section("Configuration"), Line::from("")];
        lines.extend([
            Line::from("  Inference:"),
            Line::from(format!("    Model:        {}", self.bridge.model_name())),
            Line::from(format!("    Temperature:  {}", temp)),
            Line::from(format!("    Top-P:        {}", top_p)),
            Line::from(format!("    Max Tokens:   {}", max_tokens)),
            Line::from(""),
            Line::from("  Tool Loop:"),
            Line::from(format!("    Tool Limit:   {}", tool_limit)),
            Line::from(format!("    Context Turns: {}", ctx_turns)),
            Line::from(format!("    Auto-Condense: {}", auto_condense)),
            Line::from(""),
            Line::from("  Energy:"),
            Line::from(format!("    Gas Heuristic: {}", gas_heuristic)),
            Line::from(format!("    Gas Cap:       {} (current: {})", cap, gas)),
            Line::from(format!("    Context Used:  {:.0}%", ctx * 100.0)),
            Line::from(""),
            Line::from("  System:"),
            Line::from(format!("    MCP Servers:   {}/{}", mcp_loaded, mcp_total)),
            Line::from(format!("    Agent:         {}", self.bridge.userpod_name())),
            Line::from(""),
            Line::from(Span::styled(
                "  Use `kask settings` CLI or /repl command to change values.",
                Style::default().fg(Color::DarkGray),
            )),
        ]);
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
    fn tick(&mut self) {}
}
