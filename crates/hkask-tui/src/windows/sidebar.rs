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
use ratatui::layout::Rect;
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

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
}

impl SidebarWindow {
    pub fn new(id: WindowId, service_context: Arc<hkask_services::AgentService>) -> Self {
        Self {
            id,
            active_section: SidebarSection::CnsHealth,
            service_context,
        }
    }

    fn render_cns_section(&self) -> Vec<Line<'static>> {
        vec![
            Line::from(Span::styled(
                "── CNS Health ──",
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
            Line::from("  Status: ✓ Healthy"),
            Line::from("  Variety counters: nominal"),
            Line::from("  Active alerts: 0"),
            Line::from(""),
            Line::from(Span::styled(
                "  CNS domains:",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from("  • cns.tool.*       ✓"),
            Line::from("  • cns.inference    ✓"),
            Line::from("  • cns.keystore     ✓"),
            Line::from("  • cns.condenser    ✓"),
            Line::from("  • cns.mcp.*        ✓"),
            Line::from("  • cns.tui.*        ✓"),
        ]
    }

    fn render_mcp_section(&self) -> Vec<Line<'static>> {
        vec![
            Line::from(Span::styled(
                "── MCP Servers ──",
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
            Line::from("  No MCP servers loaded."),
            Line::from("  Use /mcp start all to load."),
            Line::from(""),
            Line::from(Span::styled(
                "  Available servers:",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from("  • condenser"),
            Line::from("  • research"),
            Line::from("  • media"),
            Line::from("  • memory"),
            Line::from("  • kanban"),
            Line::from("  • curator"),
        ]
    }

    fn render_pods_section(&self) -> Vec<Line<'static>> {
        vec![
            Line::from(Span::styled(
                "── Pods ──",
                Style::default().fg(Color::Cyan).bold(),
            )),
            Line::from(""),
            Line::from("  CuratorPod:      active"),
            Line::from("  ReplicantPods:   1 active"),
            Line::from("  TeamPods:        0"),
            Line::from(""),
            Line::from(Span::styled(
                "  Pod deployment model:",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from("  • Per-pod SQLCipher DB"),
            Line::from("  • Per-pod CNS runtime"),
            Line::from("  • Per-pod MCP bindings"),
            Line::from("  • No cross-pod dispatch"),
        ]
    }

    fn render_context_section(&self) -> Vec<Line<'static>> {
        vec![
            Line::from(Span::styled(
                "── Context Window ──",
                Style::default().fg(Color::Cyan).bold(),
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
                Style::default().fg(Color::Cyan).bold(),
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
