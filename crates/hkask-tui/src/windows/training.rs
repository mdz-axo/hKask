//! Training window — LoRA adapter management and training sessions.
//!
//! Displays adapters, deployment status, and session counts. Live data
//! from TrainingDataBridge / hkask-mcp-training.

use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::bridges::TrainingDataBridge;
use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct TrainingWindow {
    id: WindowId,
    #[allow(dead_code)]
    bridge: Arc<dyn ReplBridge>,
    training: Option<Arc<dyn TrainingDataBridge>>,
}

impl TrainingWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
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
        "Training"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Training
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let mut lines = vec![
            Line::from(Span::styled(
                "── Training ──",
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
                // Collect owned adapter data before pushing
                let adapter_data: Vec<(String, String, String, String, u64)> = adapters
                    .iter()
                    .map(|a| {
                        (
                            a.name.to_string(),
                            a.version.to_string(),
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
                // Collect owned deployment data before pushing
                let deployment_data: Vec<(String, String, String)> = deployments
                    .iter()
                    .map(|d| {
                        (
                            d.adapter_name.to_string(),
                            d.provider.clone(),
                            d.status.clone(),
                        )
                    })
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
            lines.push(Line::from("    Use /adapter deploy to load an adapter"));
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

    fn handle_key(&mut self, _key: KeyEvent) -> bool {
        false
    }
    fn tick(&mut self) {}
}
