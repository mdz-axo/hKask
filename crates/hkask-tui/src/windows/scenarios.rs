//! Scenarios window — event trees, forecasts, calibration tracking.
//!
//! `]` forward, `[` backward through Pipeline→Calibration→Tree→Chat.
//! Tree section: `↑↓` navigate, `→` expand, `←` collapse, `Space` toggle,
//! `c` calibrate, `r` research selected event.

use crate::bridges::{EventNode, ScenariosDataBridge};
use crate::mcp_tabbed::{McpChatState, McpTab, McpTabbedWindow};
use crate::repl_bridge::ReplBridge;
use crate::widgets::headers;
use crate::window::{Window, WindowId, WindowKind};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::prelude::Stylize;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScenarioSection {
    Pipeline,
    Calibration,
    Tree,
}
impl ScenarioSection {
    fn next(self) -> Self {
        match self {
            Self::Pipeline => Self::Calibration,
            Self::Calibration => Self::Tree,
            Self::Tree => Self::Pipeline,
        }
    }
    fn prev(self) -> Self {
        match self {
            Self::Pipeline => Self::Tree,
            Self::Calibration => Self::Pipeline,
            Self::Tree => Self::Calibration,
        }
    }
    fn title(&self) -> &str {
        match self {
            Self::Pipeline => "Pipeline",
            Self::Calibration => "Calibration",
            Self::Tree => "Event Tree",
        }
    }
}

pub struct ScenariosWindow {
    id: WindowId,
    section: ScenarioSection,
    active_tab: McpTab,
    chat_state: McpChatState,
    bridge: Arc<dyn ReplBridge>,
    scenarios: Option<Arc<dyn ScenariosDataBridge>>,
    // Tree state
    expanded: HashSet<String>,
    cursor: usize,
    scroll: usize,
}

impl ScenariosWindow {
    pub fn new(id: WindowId, bridge: Arc<dyn ReplBridge>) -> Self {
        Self {
            id,
            section: ScenarioSection::Pipeline,
            active_tab: McpTab::Data,
            chat_state: McpChatState::new(),
            bridge,
            scenarios: None,
            expanded: HashSet::new(),
            cursor: 0,
            scroll: 0,
        }
    }
    pub fn with_scenarios_bridge(mut self, s: Arc<dyn ScenariosDataBridge>) -> Self {
        self.scenarios = Some(s);
        self
    }

    /// Build a flat list of visible nodes from the tree + expanded set.
    fn flatten_tree<'a>(
        nodes: &'a [EventNode],
        expanded: &HashSet<String>,
        depth: usize,
    ) -> Vec<(usize, &'a EventNode)> {
        let mut flat = Vec::new();
        for node in nodes {
            flat.push((depth, node));
            if expanded.contains(&node.id) && !node.children.is_empty() {
                flat.extend(Self::flatten_tree(&node.children, expanded, depth + 1));
            }
        }
        flat
    }
}

impl Window for ScenariosWindow {
    fn id(&self) -> WindowId {
        self.id
    }
    fn title(&self) -> &str {
        match self.active_tab {
            McpTab::Chat => "Scenarios Chat",
            McpTab::Data => "Scenarios",
        }
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Scenarios
    }
    fn render(&self, f: &mut Frame, area: Rect, _: bool) {
        match self.active_tab {
            McpTab::Chat => Self::default_render_chat_tab(&self.chat_state, "scenarios", f, area),
            McpTab::Data => self.render_data_tab(f, area),
        }
    }
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(']') => {
                match self.active_tab {
                    McpTab::Chat => {
                        self.active_tab = McpTab::Data;
                        self.section = ScenarioSection::Pipeline;
                    }
                    McpTab::Data => {
                        self.section = self.section.next();
                        if self.section == ScenarioSection::Pipeline {
                            self.active_tab = McpTab::Chat;
                        }
                    }
                }
                return true;
            }
            KeyCode::Char('[') => {
                match self.active_tab {
                    McpTab::Chat => {
                        self.active_tab = McpTab::Data;
                        self.section = ScenarioSection::Tree;
                    }
                    McpTab::Data => {
                        self.section = self.section.prev();
                        if self.section == ScenarioSection::Tree {
                            self.active_tab = McpTab::Chat;
                        }
                    }
                }
                return true;
            }
            _ => {}
        }
        match self.active_tab {
            McpTab::Chat => {
                if let Some(msg) = self.handle_chat_key(key) {
                    self.bridge
                        .start_scoped_inference(msg, self.mcp_server_name());
                    return true;
                }
                matches!(
                    key.code,
                    KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Enter | KeyCode::Esc
                )
            }
            McpTab::Data => self.handle_tree_key(key),
        }
    }
    fn tick(&mut self) {}
}

impl ScenariosWindow {
    fn handle_tree_key(&mut self, key: KeyEvent) -> bool {
        if self.section != ScenarioSection::Tree {
            return false;
        }
        let sc = match self.scenarios.as_ref() {
            Some(s) => s,
            None => return false,
        };
        let tree = match sc.event_tree() {
            Some(t) => t,
            None => return false,
        };
        let flat = Self::flatten_tree(&tree.root_nodes, &self.expanded, 0);

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.cursor = self.cursor.saturating_sub(1);
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor + 1 < flat.len() {
                    self.cursor += 1;
                }
                true
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(' ') => {
                if let Some((_, node)) = flat.get(self.cursor) {
                    if !node.children.is_empty() {
                        if self.expanded.contains(&node.id) {
                            self.expanded.remove(&node.id);
                        } else {
                            self.expanded.insert(node.id.clone());
                        }
                    }
                }
                true
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some((_, node)) = flat.get(self.cursor) {
                    self.expanded.remove(&node.id);
                }
                true
            }
            KeyCode::Char('c') => {
                if let Some((_, node)) = flat.get(self.cursor) {
                    let msg = format!(
                        "Calibrate event '{}' via scenario_calibrate. Current probability: {:.0}%. Fermi sub-questions: {}. Question: {}",
                        node.name,
                        node.probability * 100.0,
                        node.sub_question_count,
                        node.question
                    );
                    self.bridge.start_scoped_inference(msg, "scenarios");
                }
                true
            }
            KeyCode::Char('r') => {
                if let Some((_, node)) = flat.get(self.cursor) {
                    let msg = format!(
                        "Research event '{}': {}. Search for recent developments, base rates, and evidence. Time horizon context: {}",
                        node.name, node.question, tree.time_horizon
                    );
                    self.bridge.start_scoped_inference(msg, "scenarios");
                }
                true
            }
            KeyCode::Char('e') => {
                // Expand all
                fn collect_ids(nodes: &[EventNode]) -> Vec<String> {
                    let mut ids = Vec::new();
                    for n in nodes {
                        ids.push(n.id.clone());
                        ids.extend(collect_ids(&n.children));
                    }
                    ids
                }
                for id in collect_ids(&tree.root_nodes) {
                    self.expanded.insert(id);
                }
                true
            }
            KeyCode::Char('w') => {
                // Collapse all
                self.expanded.clear();
                self.cursor = 0;
                true
            }
            _ => false,
        }
    }
}

impl McpTabbedWindow for ScenariosWindow {
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
        "scenarios"
    }
    fn render_chat_tab(&self, f: &mut Frame, area: Rect) {
        Self::default_render_chat_tab(&self.chat_state, "scenarios", f, area);
    }
    fn render_data_tab(&self, f: &mut Frame, area: Rect) {
        let mut lines = vec![
            headers::section(format!(
                "Scenarios: {}  ([ ] to navigate)",
                self.section.title()
            )),
            Line::from(""),
        ];
        match self.scenarios.as_ref() {
            Some(sc) => match self.section {
                ScenarioSection::Pipeline => {
                    if let Some(state) = sc.pipeline_state() {
                        lines.push(Line::from(vec![
                            Span::raw("  Forecasts: "),
                            Span::styled(
                                format!("{}", state.forecast_count),
                                Style::default().fg(Color::Cyan).bold(),
                            ),
                            Span::raw(format!(
                                "  ({} resolved, {} pending)",
                                state.resolved_count, state.pending_count
                            )),
                        ]));
                        if let Some(brier) = state.overall_brier {
                            let (color, label) = if brier < 0.05 {
                                (Color::Green, "excellent")
                            } else if brier < 0.10 {
                                (Color::Green, "good")
                            } else if brier < 0.20 {
                                (Color::Yellow, "fair")
                            } else if brier < 0.33 {
                                (Color::Yellow, "poor")
                            } else {
                                (Color::Red, "needs work")
                            };
                            lines.push(Line::from(vec![
                                Span::raw("  Brier:   "),
                                Span::styled(
                                    format!("{:.4} ({})", brier, label),
                                    Style::default().fg(color),
                                ),
                            ]));
                        }
                        lines.push(Line::from(""));
                        lines.push(Line::from(Span::styled(
                            "  Recent Forecasts",
                            Style::default().fg(Color::White).bold(),
                        )));
                        for fc in &state.recent_forecasts {
                            let oc = match fc.outcome {
                                Some(true) => (Color::Green, " \u{2713}"),
                                Some(false) => (Color::Red, " \u{2717}"),
                                None => (Color::DarkGray, " \u{2026}"),
                            };
                            lines.push(Line::from(vec![
                                Span::raw("  \u{2022} "),
                                Span::styled(
                                    fc.event_name.clone(),
                                    Style::default().fg(Color::Cyan),
                                ),
                                Span::raw(format!("  {:.0}%", fc.probability * 100.0)),
                                Span::styled(oc.1, Style::default().fg(oc.0)),
                            ]));
                        }
                    } else {
                        lines.push(Line::from(
                            "  No scenario data. Use `kask mcp start scenarios` to enable.",
                        ));
                    }
                }
                ScenarioSection::Calibration => {
                    if let Some(cal) = sc.calibration() {
                        lines.push(Line::from(vec![
                            Span::raw("  Resolved: "),
                            Span::styled(
                                format!("{}", cal.resolved_forecasts),
                                Style::default().fg(Color::Cyan),
                            ),
                            Span::raw(format!(" / {} total", cal.total_forecasts)),
                        ]));
                        if let Some(brier) = cal.overall_brier {
                            lines.push(Line::from(vec![
                                Span::raw("  Brier:    "),
                                Span::styled(
                                    format!("{:.4}", brier),
                                    Style::default().fg(if brier < 0.10 {
                                        Color::Green
                                    } else {
                                        Color::Yellow
                                    }),
                                ),
                            ]));
                        }
                        if let Some(oc) = cal.overconfidence_score {
                            let (color, desc) = if oc.abs() < 0.05 {
                                (Color::Green, "well calibrated")
                            } else if oc > 0.0 {
                                (Color::Red, "overconfident")
                            } else {
                                (Color::Yellow, "underconfident")
                            };
                            lines.push(Line::from(vec![
                                Span::raw("  Bias:     "),
                                Span::styled(
                                    format!("{:+.3} ({})", oc, desc),
                                    Style::default().fg(color),
                                ),
                            ]));
                        }
                        lines.push(Line::from(vec![
                            Span::raw("  Verdict:  "),
                            Span::styled(
                                cal.interpretation.clone(),
                                Style::default().fg(Color::White),
                            ),
                        ]));
                    } else {
                        lines.push(Line::from("  No calibration data."));
                    }
                }
                ScenarioSection::Tree => {
                    if let Some(tree) = sc.event_tree() {
                        let flat = Self::flatten_tree(&tree.root_nodes, &self.expanded, 0);
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(
                                tree.subject.clone(),
                                Style::default().fg(Color::Cyan).bold(),
                            ),
                            Span::raw(format!("  |  {}  |  Joint: ", tree.time_horizon)),
                            Span::styled(
                                format!("{:.1}%", tree.joint_probability * 100.0),
                                Style::default().fg(Color::Green),
                            ),
                        ]));
                        lines.push(Line::from(Span::styled(
                            "  \u{2191}\u{2193}nav \u{2192}expand \u{2190}collapse Space c calibrate r research e all w collapse",
                            Style::default().fg(Color::DarkGray),
                        )));
                        lines.push(Line::from(""));
                        if flat.is_empty() {
                            lines.push(Line::from("  No events in tree."));
                        } else {
                            let vis_h = area.height.saturating_sub(8) as usize;
                            let start = self.scroll.min(flat.len().saturating_sub(1));
                            let end = (start + vis_h.max(1)).min(flat.len());
                            for (i, (depth, node)) in
                                flat.iter().enumerate().skip(start).take(end - start)
                            {
                                let indent = "  ".repeat(*depth);
                                let marker = if node.children.is_empty() {
                                    "  "
                                } else if self.expanded.contains(&node.id) {
                                    "[-]"
                                } else {
                                    "[+]"
                                };
                                let is_cursor = i == self.cursor;
                                let cm = if is_cursor { ">" } else { " " };
                                let ns = if is_cursor {
                                    Style::default().fg(Color::Cyan).bold()
                                } else {
                                    Style::default().fg(Color::Cyan)
                                };
                                let tc = match node.certainty_tier.as_str() {
                                    "proximate" => Color::Green,
                                    "probable" => Color::Yellow,
                                    _ => Color::Red,
                                };
                                lines.push(Line::from(vec![
                                    Span::raw(format!("{}{} {} ", cm, indent, marker)),
                                    Span::styled(node.name.clone(), ns),
                                    Span::raw("  "),
                                    Span::styled(
                                        format!("{:.0}%", node.probability * 100.0),
                                        Style::default().fg(tc),
                                    ),
                                    Span::raw(format!("  {}", node.certainty_tier)),
                                ]));
                            }
                        }
                    } else {
                        lines.push(Line::from("  No active event tree. Build via scenario_frame \u{2192} brainstorm \u{2192} quantify."));
                    }
                }
            },
            None => {
                lines.push(Line::from("  No scenarios MCP server connected."));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Tetlock + Schwartz + Chermack via hkask-mcp-scenarios.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
    }
}
