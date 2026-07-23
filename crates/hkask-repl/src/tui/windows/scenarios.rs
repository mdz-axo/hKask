//! Scenarios window — scenario planning and forecasting via the `scenarios` MCP server.
//!
//! Provides a dedicated pane where the user can frame scenarios, run
//! the scenario pipeline, build events, quantify drivers, calibrate
//! forecasts, and synthesize dragonfly-eye conclusions. Queries are
//! scoped to the `scenarios` MCP server's tools.

use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::tui::repl_bridge::{ReplBridge, ToolInvokeBridge};
use crate::tui::window::{Window, WindowId, WindowKind, WorkspaceAction};
use crate::tui::windows::mcp_scoped::McpScopedState;

pub struct ScenariosWindow {
    state: McpScopedState,
}

impl ScenariosWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        let state = McpScopedState::new(
            id,
            WindowKind::Scenarios,
            "Scenarios",
            "scenarios",
            bridge,
            "Scenarios — planning and forecasting. Type a tool name (e.g., 'scenario_status')\n             or a natural language query. Type /help for commands.",
        );
        Self { state }
    }

    pub fn with_tool_invoke_bridge(mut self, bridge: Arc<dyn ToolInvokeBridge>) -> Self {
        self.state = self.state.with_tool_invoke_bridge(bridge);
        self
    }
}

impl Window for ScenariosWindow {
    fn id(&self) -> WindowId {
        self.state.id
    }

    fn title(&self) -> &str {
        &self.state.title
    }

    fn kind(&self) -> WindowKind {
        self.state.kind
    }

    fn render(&self, f: &mut Frame, area: Rect, is_focused: bool) {
        self.state.render(f, area, is_focused);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        self.state.handle_key(key)
    }

    fn tick(&mut self) {
        self.state.tick();
    }

    fn drain_actions(&mut self) -> Vec<WorkspaceAction> {
        self.state.drain_actions()
    }
}
