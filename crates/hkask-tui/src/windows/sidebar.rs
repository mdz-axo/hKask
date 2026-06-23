//! Sidebar window — composite system status display.
//!
//! Shows CNS health, MCP server status, pod inventory, and context
//! window pressure in a single scrollable panel. This is the
//! cybernetic "dashboard" that closes the S3 (Control) feedback loop.
//!
//! # RDF Triple
//! ```text
//! ⟨Sidebar⟩ displays ⟨CnsHealth, McpStatus, PodInventory, ContextPressure⟩ .
//! ```

use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use crate::repl_bridge::ReplBridge;
use crate::window::{Window, WindowId, WindowKind};

/// Sections of the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidebarSection {
    CnsHealth,
    McpStatus,
    Pods,
    Context,
    Keybindings,
}

impl SidebarSection {
    fn title(&self) -> &'static str {
        match self {
            SidebarSection::CnsHealth => "CNS Health",
            SidebarSection::McpStatus => "MCP Servers",
            SidebarSection::Pods => "Pods",
            SidebarSection::Context => "Context",
            SidebarSection::Keybindings => "Keys",
        }
    }

    fn next(&self) -> Self {
        match self {
            SidebarSection::CnsHealth => SidebarSection::McpStatus,
            SidebarSection::McpStatus => SidebarSection::Pods,
            SidebarSection::Pods => SidebarSection::Context,
            SidebarSection::Context => SidebarSection::Keybindings,
            SidebarSection::Keybindings => SidebarSection::CnsHealth,
        }
    }
}

pub struct SidebarWindow {
    id: WindowId,
    active_section: SidebarSection,
    #[allow(dead_code)]
    service_context: Arc<hkask_services::AgentService>,
    bridge: Arc<dyn ReplBridge>,
}

impl SidebarWindow {
    pub fn new(
        id: WindowId,
        service_context: Arc<hkask_services::AgentService>,
        bridge: Arc<dyn ReplBridge>,
    ) -> Self {
        Self {
            id,
            active_section: SidebarSection::CnsHealth,
            service_context,
            bridge,
        }
    }

    fn render_cns_section(&self) -> Vec<Line<'static>> {
        let mut lines = vec![
            Line::from(Span::styled(
                "── CNS Health ──",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];
        let alerts = self.bridge.cns_alert_count();
        if alerts > 0 {
            lines.push(Line::from(format!("  Active alerts: {}", alerts)));
        } else {
            lines.push(Line::from("  Status: ✓ Healthy"));
        }
        lines.push(Line::from(format!(
            "  Gas: {}/{}",
            self.bridge.gas_remaining(),
            self.bridge.gas_cap()
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  CNS domains:",
            Style::default().fg(Color::DarkGray),
        )));
        for (domain, healthy) in self.bridge.cns_domains() {
            let mark = if healthy { "✓" } else { "✗" };
            lines.push(Line::from(format!("  • {}  {}", domain, mark)));
        }
        lines
    }

    fn render_mcp_section(&self) -> Vec<Line<'static>> {
        let (loaded, _total) = self.bridge.mcp_status();
        vec![
            Line::from(Span::styled(
                "── MCP Servers ──",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!("  Loaded: {}", loaded)),
            Line::from("  Use /mcp start all to load."),
        ]
    }

    fn render_pods_section(&self) -> Vec<Line<'static>> {
        let (curator, replicant, team) = self.bridge.pod_counts();
        vec![
            Line::from(Span::styled(
                "── Pods ──",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!(
                "  CuratorPod:    {}",
                if curator > 0 { "active" } else { "inactive" }
            )),
            Line::from(format!("  ReplicantPods: {}", replicant)),
            Line::from(format!("  TeamPods:      {}", team)),
        ]
    }

    fn render_context_section(&self) -> Vec<Line<'static>> {
        vec![
            Line::from(Span::styled(
                "── Context Window ──",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  Context pressure: 12%"),
            Line::from("  Auto-condense:    on (87.5%)"),
            Line::from("  Condenser health: ✓"),
            Line::from("  Context turns:    3"),
            Line::from(""),
            Line::from(Span::styled(
                "  Model metadata:",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from("  • context_length: 128K"),
            Line::from("  • thinking:       supported"),
            Line::from("  • tools:          native"),
        ]
    }

    fn render_keybindings_section(&self) -> Vec<Line<'static>> {
        vec![
            Line::from(Span::styled(
                "── Keybindings ──",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  Global:"),
            Line::from("  ^Q  Quit         ^T  New tab"),
            Line::from("  ^W  Close win     ^B  Sidebar"),
            Line::from("  ^P  Palette       ^=  Resize+"),
            Line::from("  ^-  Resize-       ^N  New win"),
            Line::from(""),
            Line::from("  Navigation:"),
            Line::from("  ^H  Left          ^J  Down"),
            Line::from("  ^K  Up            ^L  Right"),
            Line::from("  ^Shift+H  Split H"),
            Line::from("  ^Shift+J  Split V"),
            Line::from(""),
            Line::from("  Chat:"),
            Line::from("  Enter  Send       /    Command"),
            Line::from("  Esc    Clear      PgUp Scroll"),
        ]
    }
}

impl Window for SidebarWindow {
    fn id(&self) -> WindowId {
        self.id
    }

    fn title(&self) -> &str {
        "Sidebar"
    }

    fn kind(&self) -> WindowKind {
        WindowKind::Sidebar
    }

    fn render(&self, f: &mut Frame, area: Rect, _is_focused: bool) {
        let sections = [
            self.render_cns_section(),
            self.render_mcp_section(),
            self.render_pods_section(),
            self.render_context_section(),
            self.render_keybindings_section(),
        ];

        let idx = match self.active_section {
            SidebarSection::CnsHealth => 0,
            SidebarSection::McpStatus => 1,
            SidebarSection::Pods => 2,
            SidebarSection::Context => 3,
            SidebarSection::Keybindings => 4,
        };

        let mut all_lines: Vec<Line> = Vec::new();
        all_lines.push(Line::from(Span::styled(
            format!(" Section: {} (Tab to cycle) ", self.active_section.title()),
            Style::default().fg(Color::DarkGray),
        )));
        all_lines.push(Line::from(""));
        all_lines.extend(sections[idx].clone());

        let content = Paragraph::new(all_lines).wrap(Wrap { trim: false });
        f.render_widget(content, area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Tab => {
                self.active_section = self.active_section.next();
                true
            }
            _ => false,
        }
    }

    fn can_close(&self) -> bool {
        false // Sidebar is persistent
    }

    fn tick(&mut self) {
        // Future: poll CNS, MCP status, pod health
    }
}
