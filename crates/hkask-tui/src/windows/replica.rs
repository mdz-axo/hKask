//! Replica window — authorial style replicas: build, compare, generate.
//!
//! `]` forward, `[` backward through Replicas→Chat.

use crate::bridges::ReplicaDataBridge;
use crate::impl_mcp_tabbed;
use crate::mcp_tabbed::{McpChatState, McpTab, McpTabbedWindow};
use crate::repl_bridge::ReplBridge;
use crate::widgets::headers;
use crate::window::{Window, WindowId, WindowKind};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use std::sync::Arc;

pub struct ReplicaWindow {
    id: WindowId,
    active_tab: McpTab,
    chat_state: McpChatState,
    bridge: Arc<dyn ReplBridge>,
    replica: Option<Arc<dyn ReplicaDataBridge>>,
}

impl ReplicaWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            active_tab: McpTab::Data,
            chat_state: McpChatState::new(),
            bridge,
            replica: None,
        }
    }
    pub fn with_replica_bridge(mut self, r: Arc<dyn ReplicaDataBridge>) -> Self {
        self.replica = Some(r);
        self
    }
}

impl Window for ReplicaWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        match self.active_tab {
            McpTab::Chat => "Replica Chat",
            McpTab::Data => "Replica",
        }
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Replica
    }
    fn render(&self, f: &mut Frame, area: Rect, _: bool) {
        match self.active_tab {
            McpTab::Chat => <ReplicaWindow as McpTabbedWindow>::default_render_chat_tab(
                &self.chat_state,
                "replica",
                f,
                area,
            ),
            McpTab::Data => self.render_data_tab(f, area),
        }
    }
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(']') => {
                self.active_tab = McpTab::Chat;
                return true;
            }
            KeyCode::Char('[') => {
                self.active_tab = McpTab::Data;
                return true;
            }
            _ => {}
        }
        match self.active_tab {
            McpTab::Chat => {
                if let Some(msg) = self.handle_chat_key(key) {
                    let bridge = self.bridge.clone();
                    self.start_chat_request(bridge.as_ref(), msg);
                    return true;
                }
                matches!(
                    key.code,
                    KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Enter | KeyCode::Esc
                )
            }
            McpTab::Data => false,
        }
    }
    fn tick(&mut self) {
        let bridge = self.bridge.clone();
        self.poll_chat_request(bridge.as_ref());
    }
}

impl_mcp_tabbed!(ReplicaWindow, "replica", |this, f, area| {
    let mut lines = vec![headers::section("Replica ([ ] Chat/Data)"), Line::from("")];
    let rep_data: Vec<(String, usize, String)> = this
        .replica
        .as_ref()
        .map(|r| {
            r.list_replicas()
                .iter()
                .map(|r2| (r2.author.clone(), r2.centroid_count, r2.status.clone()))
                .collect()
        })
        .unwrap_or_default();
    if let Some(ref _r) = this.replica {
        lines.push(Line::from(format!("  {} replica(s)", rep_data.len())));
        for (author, centroid_count, status) in &rep_data {
            lines.push(Line::from(vec![
                Span::raw("  • "),
                Span::styled(author.clone(), Style::default().fg(Color::Green)),
                Span::raw(format!("  {} centroids  [{}]", centroid_count, status)),
            ]));
        }
    } else {
        lines.push(Line::from("  Use `kask mcp start replica` to enable."));
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
});
