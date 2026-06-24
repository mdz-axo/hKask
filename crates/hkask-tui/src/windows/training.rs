//! Training window — LoRA adapter management and training sessions.
//!
//! Displays adapters, deployment status, and session counts. Live data
//! from TrainingDataBridge / hkask-mcp-training.
//!
//! Adopts the MCP two-tab design (TUI_SPECIFICATION.md §3):
//! - Tab 1 (Chat): Focused chat scoped to the Training MCP server
//! - Tab 2 (Data): Adapters and deployments overview
//!
//! Tab key: toggles Chat ↔ Data directly (single data view).

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::bridges::TrainingDataBridge;
use crate::mcp_tabbed::{McpChatState, McpTab, McpTabbedWindow};
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct TrainingWindow {
    id: WindowId,
    active_tab: McpTab,
    chat_state: McpChatState,
    #[allow(dead_code)]
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

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        match self.active_tab {
            McpTab::Chat => {
                Self::default_render_chat_tab(&self.chat_state, "training", f, area);
            }
            McpTab::Data => self.render_data_tab(f, area),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Tab {
            self.active_tab = match self.active_tab {
                McpTab::Chat => McpTab::Data,
                McpTab::Data => McpTab::Chat,
            };
            return true;
        }

        match self.active_tab {
            McpTab::Chat => {
                if let Some(_msg) = self.handle_chat_key(key) {
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
    fn tick(&mut self) {}
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
        let mut lines = vec![
            Line::from(Span::styled(
                "── Training (Tab: Chat) ──",
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
        ];

        if let Some(ref t) = self.training {
            let adapters = t.adapter_list();
            let deployments = t.deployment_list();

            lines.push(Line::from(format!("  Sessions:    {}", t.session_count())));
            lines.push(Line::from(format!("  Adapters:    {}", t.adapter_count())));
            lines.push(Line::from(""));

            lines.push(Line::from(Span::styled(
                "  Adapters:",
                Style::default().fg(Color::Yellow),
            )));
            if adapters.is_empty() {
                lines.push(Line::from("    • None registered"));
            } else {
                let adapter_data: Vec<(String, String, String, String, u64)> = adapters
                    .iter()
                    .map(|a| {
                        (
                            a.name.clone(),
                            a.version.clone(),
                            a.base_model.clone(),
                            a.expertise.clone(),
                            a.size_bytes,
                        )
                    })
                    .collect();
                for (name, version, base_model, expertise, size_bytes) in &adapter_data {
                    lines.push(Line::from(vec![
                        Span::raw("    • "),
                        Span::styled(name.clone(), Style::default().fg(Color::Magenta)),
                        Span::styled(
                            format!("  v{}", version),
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                    lines.push(Line::from(format!(
                        "      {}  {}  {} MB",
                        base_model,
                        expertise,
                        size_bytes / 1_000_000
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
                let deployment_data: Vec<(String, String, String)> = deployments
                    .iter()
                    .map(|d| (d.adapter_name.clone(), d.provider.clone(), d.status.clone()))
                    .collect();
                for (name, provider, status) in &deployment_data {
                    let color = match status.as_str() {
                        "active" => Color::Green,
                        "provisioning" => Color::Yellow,
                        _ => Color::Red,
                    };
                    lines.push(Line::from(vec![
                        Span::raw("    • "),
                        Span::styled(name.clone(), Style::default().fg(Color::Cyan)),
                        Span::raw(format!("  via {:12}", provider)),
                        Span::styled(format!("  [{}]", status), Style::default().fg(color)),
                    ]));
                }
            }
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Use `kask adapter` CLI for deployment. 4 commands: list, deploy, status, teardown.",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            lines.push(Line::from("  Active Sessions: 0"));
            lines.push(Line::from("  Completed:       0"));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  LoRA Adapters:",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from("    • None deployed"));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Training Artifacts:",
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from("    • agents/{name}/adapters/"));
            lines.push(Line::from("    • agents/{name}/sessions/"));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Use `axolotl` CLI for fine-tuning, then deploy adapters via /adapter.",
                Style::default().fg(Color::DarkGray),
            )));
        }

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}
