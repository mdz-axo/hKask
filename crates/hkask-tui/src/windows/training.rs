//! Training window — LoRA adapter management and training sessions.
//!
//! `]` switches to Chat, `[` switches to Data.

use crate::bridges::TrainingDataBridge;
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

pub struct TrainingWindow {
    id: WindowId,
    active_tab: McpTab,
    chat_state: McpChatState,
    bridge: Arc<dyn ReplBridge>,
    training: Option<Arc<dyn TrainingDataBridge>>,
}

impl TrainingWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            active_tab: McpTab::Data,
            chat_state: McpChatState::new(),
            bridge,
            training: None,
        }
    }
    pub fn with_training_bridge(mut self, t: Arc<dyn TrainingDataBridge>) -> Self {
        self.training = Some(t);
        self
    }
}

impl Window for TrainingWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        match self.active_tab {
            McpTab::Chat => "Training Chat",
            McpTab::Data => "Training",
        }
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Training
    }
    fn render(&self, f: &mut Frame, area: Rect, _: bool) {
        match self.active_tab {
            McpTab::Chat => Self::default_render_chat_tab(&self.chat_state, "training", f, area),
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

impl McpTabbedWindow for TrainingWindow {
    fn active_tab(&self) -> McpTab {
        self.active_tab
    }
    fn set_active_tab(&mut self, tab: McpTab) {
        self.active_tab = tab;
    }
    fn chat_state_mut(&mut self) -> &mut McpChatState {
        &mut self.chat_state
    }
    fn mcp_server_name(&self) -> &str {
        "training"
    }
    fn render_chat_tab(&self, f: &mut Frame, area: Rect) {
        Self::default_render_chat_tab(&self.chat_state, "training", f, area);
    }
    fn render_data_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![headers::section("Training ([ ] Chat/Data)"), Line::from("")];
        if let Some(ref t) = self.training {
            let adapters = t.adapter_list();
            let deployments = t.deployment_list();
            lines.push(Line::from(format!(
                "  Sessions: {}   Adapters: {}",
                t.session_count(),
                t.adapter_count()
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Adapters:",
                Style::default().fg(Color::Yellow),
            )));
            if adapters.is_empty() {
                lines.push(Line::from("    • None registered"));
            } else {
                for a in &adapters {
                    let name = a.name.clone();
                    let version = a.version.clone();
                    lines.push(Line::from(vec![
                        Span::raw("    • "),
                        Span::styled(name, Style::default().fg(Color::Magenta)),
                        Span::styled(
                            format!("  v{}", version),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                    lines.push(Line::from(format!(
                        "      {}  {}  {} MB",
                        a.base_model,
                        a.expertise,
                        a.size_bytes / 1_000_000
                    )));
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Deployments:",
                Style::default().fg(Color::Yellow),
            )));
            if deployments.is_empty() {
                lines.push(Line::from("    • No active deployments"));
            } else {
                for d in &deployments {
                    let name = d.adapter_name.clone();
                    let color = match d.status.as_str() {
                        "active" => Color::Green,
                        "provisioning" => Color::Yellow,
                        _ => Color::Red,
                    };
                    lines.push(Line::from(vec![
                        Span::raw("    • "),
                        Span::styled(name, Style::default().fg(Color::Cyan)),
                        Span::raw(format!("  via {:12}", d.provider)),
                        Span::styled(format!("  [{}]", d.status), Style::default().fg(color)),
                    ]));
                }
            }
        }
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}
