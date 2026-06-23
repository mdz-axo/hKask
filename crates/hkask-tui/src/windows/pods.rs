//! Pods window — displays pod deployment status.
//!
//! Shows CuratorPod, ReplicantPods, and TeamPods with their
//! deployment status, storage paths, and CNS runtime state.

use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

pub struct PodsWindow {
    id: WindowId,
    bridge: Arc<dyn ReplBridge>,
}

impl PodsWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self { id, bridge }
    }
}

impl Window for PodsWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        "Pods"
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Pods
    }

    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
        let (curator, replicant, team) = self.bridge.pod_counts();
        let lines = vec![
            Line::from(Span::styled(
                "── Pod Deployment Status ──",
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Tier 1 — CuratorPod:",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(format!(
                "  Status: {}",
                if curator > 0 {
                    "✓ Active"
                } else {
                    "✗ Inactive"
                }
            )),
            Line::from("  Storage: ~/.config/hkask/agents/curator/pod.db"),
            Line::from("  Role:   SemanticIndex owner, CNS coordination"),
            Line::from(""),
            Line::from(Span::styled(
                "Tier 2 — TeamPods:",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(format!("  Count:  {}", team)),
            Line::from("  Storage: ~/.config/hkask/agents/team.{name}/pod.db"),
            Line::from("  Role:   Shared bot episodic storage"),
            Line::from(""),
            Line::from(Span::styled(
                "Tier 3 — ReplicantPods:",
                Style::default().fg(Color::Yellow),
            )),
            Line::from(format!("  Count:  {}", replicant)),
            Line::from("  Storage: ~/.config/hkask/agents/replicant.{name}/pod.db"),
            Line::from("  Role:   Human+replicant pair, private episodic"),
            Line::from(""),
            Line::from(Span::styled(
                "  Model: Per-pod SQLCipher, no cross-pod access (P4.1, P11.1)",
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
