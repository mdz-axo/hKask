//! Kanban window — task coordination via the `kanban` MCP server.
//!
//! Provides a dedicated pane where the user can interact with their
//! kanban boards and tasks. Queries are scoped to the `kanban` MCP
//! server's tools (board create/list, task create/list/move/verify, etc.)

use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::tui::repl_bridge::ReplBridge;
use crate::tui::window::{Window, WindowId, WindowKind, WorkspaceAction};
use crate::tui::windows::mcp_scoped::McpScopedState;

pub struct KanbanWindow {
    state: McpScopedState,
}

impl KanbanWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        let state = McpScopedState::new(
            id,
            WindowKind::Kanban,
            "Kanban",
            "kanban",
            bridge,
            "Kanban — task coordination. Type a query (e.g., 'list my boards', \
             'create a task: fix bug in auth module on the main board'). \
             Type /help for commands.",
        );
        Self { state }
    }
}

impl Window for KanbanWindow {
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
