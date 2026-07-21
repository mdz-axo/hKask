//! Pods window — displays pod deployment status.
//!
//! Shows CuratorPod and UserPod with their deployment status, storage
//! paths, and CNS runtime state. One user = one userpod (no TeamPods).

use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::repl_bridge::ReplBridge;
use crate::widgets::headers;
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
        let Some((curator, userpod)) = self.bridge.pod_counts() else {
            let lines = vec![
                headers::section("Pod Deployment Status"),
                Line::from(""),
                Line::from(Span::styled(
                    "Pod registry unavailable — scan failed",
                    Style::default().fg(Color::Red),
                )),
            ];
            f.render_widget(Paragraph::new(lines), area);
            return;
        };
        let lines = vec![
            headers::section("Pod Deployment Status"),
            Line::from(""),
            Line::from(Span::styled(
                "CuratorPod:",
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
            Line::from("  Storage: ~/.local/share/hkask/curator/pod.db"),
            Line::from("  Role:   SemanticIndex owner, CNS coordination"),
            Line::from(""),
            Line::from(Span::styled("UserPod:", Style::default().fg(Color::Yellow))),
            Line::from(format!(
                "  Status: {}",
                if userpod > 0 {
                    "✓ Active"
                } else {
                    "✗ Inactive"
                }
            )),
            Line::from("  Storage: ~/.local/share/hkask/userpods/{name}/pod.db"),
            Line::from("  Role:   Human+userpod pair, private episodic"),
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
