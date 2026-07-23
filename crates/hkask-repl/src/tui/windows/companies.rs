//! Companies window — financial data via the `companies` MCP server.
//!
//! Provides a dedicated pane where the user can query company profiles,
//! stock quotes, financial statements, key metrics, and historical prices.
//! Queries are scoped to the `companies` MCP server's tools.

use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::tui::repl_bridge::{ReplBridge, ToolInvokeBridge};
use crate::tui::window::{Window, WindowId, WindowKind, WorkspaceAction};
use crate::tui::windows::mcp_scoped::McpScopedState;

pub struct CompaniesWindow {
    state: McpScopedState,
}

impl CompaniesWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        let state = McpScopedState::new(
            id,
            WindowKind::Companies,
            "Companies",
            "companies",
            bridge,
            "Companies — financial data. Type a tool name (e.g., 'company_profile symbol=AAPL')\n             or a natural language query. Type /help for commands.",
        );
        Self { state }
    }

    pub fn with_tool_invoke_bridge(mut self, bridge: Arc<dyn ToolInvokeBridge>) -> Self {
        self.state = self.state.with_tool_invoke_bridge(bridge);
        self
    }
}

impl Window for CompaniesWindow {
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
