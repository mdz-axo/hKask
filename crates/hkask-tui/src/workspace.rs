//! Workspace — the window layout tree and focus manager.
//!
//! Modelled on Zed's workspace: a binary tree of splits hosts stateful
//! Window implementations. The workspace handles layout computation,
//! focus routing, split/resize operations, and tab management.
//!
//! # RDF Semantic Model (P8 Semantic Grounding)
//!
//! ```text
//! ⟨Workspace⟩ contains ⟨Tab⟩ .
//! ⟨Tab⟩ contains ⟨SplitNode⟩ .
//! ⟨SplitNode⟩ isA ⟨Leaf | Horizontal | Vertical⟩ .
//! ⟨Leaf⟩ hosts ⟨Window⟩ .
//! ⟨Window⟩ hasKind ⟨WindowKind⟩ .
//! ⟨Workspace⟩ hasFocus ⟨WindowId⟩ .
//! ⟨SplitNode⟩ hasRatio ⟨f32⟩ .
//! ```
//!
//! # CNS Spans (P9)
//!
//! - `cns.tui.workspace.window_opened { kind, id }`
//! - `cns.tui.workspace.window_closed { kind, id }`
//! - `cns.tui.workspace.focus_changed { from, to }`
//! - `cns.tui.workspace.tab_switched { from_idx, to_idx }`
//! - `cns.tui.workspace.split { direction }`

use std::collections::HashMap;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::status_bar::StatusBar;
use crate::tab::Tab;
use crate::window::{Window, WindowId, WindowKind};
use crate::windows::{ChatWindow, SidebarWindow};

/// Direction for splitting a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// A node in the binary split tree.
///
/// Leaves hold windows. Internal nodes split space between children.
#[derive(Debug)]
pub(crate) enum SplitNode {
    Leaf {
        window_id: WindowId,
    },
    Horizontal {
        left: Box<SplitNode>,
        right: Box<SplitNode>,
        ratio: f32,
    },
    Vertical {
        top: Box<SplitNode>,
        bottom: Box<SplitNode>,
        ratio: f32,
    },
}

impl SplitNode {
    fn window_ids(&self) -> Vec<WindowId> {
        match self {
            SplitNode::Leaf { window_id } => vec![*window_id],
            SplitNode::Horizontal { left, right, .. } => {
                let mut ids = left.window_ids();
                ids.extend(right.window_ids());
                ids
            }
            SplitNode::Vertical { top, bottom, .. } => {
                let mut ids = top.window_ids();
                ids.extend(bottom.window_ids());
                ids
            }
        }
    }

    fn contains_window(&self, target: WindowId) -> bool {
        match self {
            SplitNode::Leaf { window_id } => *window_id == target,
            SplitNode::Horizontal { left, right, .. } => {
                left.contains_window(target) || right.contains_window(target)
            }
            SplitNode::Vertical { top, bottom, .. } => {
                top.contains_window(target) || bottom.contains_window(target)
            }
        }
    }

    fn replace_with_single_leaf(&mut self, target: WindowId, replacement: WindowId) -> bool {
        if matches!(self, SplitNode::Leaf { window_id } if *window_id == target) {
            *self = SplitNode::Leaf {
                window_id: replacement,
            };
            return true;
        }
        match self {
            SplitNode::Horizontal { left, right, .. } => {
                if let SplitNode::Leaf { window_id } = left.as_ref() {
                    if *window_id == target {
                        *left = Box::new(SplitNode::Leaf {
                            window_id: replacement,
                        });
                        return true;
                    }
                }
                if let SplitNode::Leaf { window_id } = right.as_ref() {
                    if *window_id == target {
                        *right = Box::new(SplitNode::Leaf {
                            window_id: replacement,
                        });
                        return true;
                    }
                }
                left.replace_with_single_leaf(target, replacement)
                    || right.replace_with_single_leaf(target, replacement)
            }
            SplitNode::Vertical { top, bottom, .. } => {
                if let SplitNode::Leaf { window_id } = top.as_ref() {
                    if *window_id == target {
                        *top = Box::new(SplitNode::Leaf {
                            window_id: replacement,
                        });
                        return true;
                    }
                }
                if let SplitNode::Leaf { window_id } = bottom.as_ref() {
                    if *window_id == target {
                        *bottom = Box::new(SplitNode::Leaf {
                            window_id: replacement,
                        });
                        return true;
                    }
                }
                top.replace_with_single_leaf(target, replacement)
                    || bottom.replace_with_single_leaf(target, replacement)
            }
            _ => false,
        }
    }
}

pub struct Workspace {
    service_context: std::sync::Arc<hkask_services::AgentService>,
    agent_name: String,
    current_model: String,
    tabs: Vec<Tab>,
    active_tab: usize,
    focused: WindowId,
    windows: HashMap<WindowId, Box<dyn Window>>,
    status: StatusBar,
    sidebar_open: bool,
    sidebar_id: Option<WindowId>,
    palette_prev_focus: Option<WindowId>,
}

impl Workspace {
    pub fn new(
        service_context: std::sync::Arc<hkask_services::AgentService>,
        agent_name: String,
        current_model: String,
    ) -> Self {
        let mut windows: HashMap<WindowId, Box<dyn Window>> = HashMap::new();

        let chat_id = WindowId(uuid::Uuid::new_v4());
        let chat = ChatWindow::new(
            chat_id,
            &agent_name,
            &current_model,
            service_context.clone(),
        );
        windows.insert(chat_id, Box::new(chat));

        let root = SplitNode::Leaf { window_id: chat_id };
        let tab = Tab::new("Main".to_string(), root);

        let mut ws = Self {
            service_context,
            agent_name,
            current_model,
            tabs: vec![tab],
            active_tab: 0,
            focused: chat_id,
            windows,
            status: StatusBar::new(),
            sidebar_open: false,
            sidebar_id: None,
            palette_prev_focus: None,
        };

        if let Some(w) = ws.windows.get_mut(&chat_id) {
            w.on_focus();
        }

        ws
    }

    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    fn root(&self) -> &SplitNode {
        &self.tabs[self.active_tab].root
    }

    fn root_mut(&mut self) -> &mut SplitNode {
        &mut self.tabs[self.active_tab].root
    }

    // ── Rendering ────────────────────────────────────────────────────────

    pub fn render(&self, f: &mut Frame) {
        let area = f.area();
        let vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(area);

        self.render_tab_bar(f, vert[0]);
        self.render_content(f, vert[1]);
        self.render_status_bar(f, vert[2]);
    }

    fn render_tab_bar(&self, f: &mut Frame, area: Rect) {
        let mut parts: Vec<String> = Vec::new();
        for (i, tab) in self.tabs.iter().enumerate() {
            if i == self.active_tab {
                parts.push(format!(" [{}] ", tab.name));
            } else {
                parts.push(format!("  {}  ", tab.name));
            }
        }
        let bar_text = parts.join("");
        let bar =
            Paragraph::new(bar_text).style(Style::default().fg(Color::White).bg(Color::DarkGray));
        f.render_widget(bar, area);
    }

    fn render_content(&self, f: &mut Frame, area: Rect) {
        self.render_node(f, self.root(), area);
    }

    fn render_node(&self, f: &mut Frame, node: &SplitNode, area: Rect) {
        match node {
            SplitNode::Leaf { window_id } => {
                if let Some(window) = self.windows.get(window_id) {
                    let is_focused = *window_id == self.focused;
                    let border_style = if is_focused {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let block = Block::default()
                        .title(window.title())
                        .borders(Borders::ALL)
                        .border_style(border_style);
                    let inner = block.inner(area);
                    f.render_widget(block, area);
                    window.render(f, inner, is_focused);
                }
            }
            SplitNode::Horizontal { left, right, ratio } => {
                let split = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Ratio((*ratio * 100.0) as u32, 100),
                        Constraint::Ratio(((1.0 - *ratio) * 100.0) as u32, 100),
                    ])
                    .split(area);
                self.render_node(f, left, split[0]);
                self.render_node(f, right, split[1]);
            }
            SplitNode::Vertical { top, bottom, ratio } => {
                let split = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Ratio((*ratio * 100.0) as u32, 100),
                        Constraint::Ratio(((1.0 - *ratio) * 100.0) as u32, 100),
                    ])
                    .split(area);
                self.render_node(f, top, split[0]);
                self.render_node(f, bottom, split[1]);
            }
        }
    }

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let bar_text = self.status.render(self.focused, &self.windows);
        let bar = Paragraph::new(bar_text)
            .style(Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 40)));
        f.render_widget(bar, area);
    }

    // ── Event Handling ───────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: KeyEvent) {
        if let Some(window) = self.windows.get_mut(&self.focused) {
            window.handle_key(key);
        }
    }

    // ── Window Management ────────────────────────────────────────────────

    pub fn open_window(&mut self, kind: WindowKind) -> WindowId {
        let id = WindowId(uuid::Uuid::new_v4());

        let window: Box<dyn Window> = match kind {
            WindowKind::Chat => Box::new(ChatWindow::new(
                id,
                &self.agent_name,
                &self.current_model,
                self.service_context.clone(),
            )),
            WindowKind::Sidebar => Box::new(SidebarWindow::new(id, self.service_context.clone())),
            _ => {
                use crossterm::event::KeyEvent as _;
                struct StubWindow {
                    id: WindowId,
                    kind: WindowKind,
                }
                impl Window for StubWindow {
                    fn id(&self) -> WindowId {
                        self.id
                    }
                    fn title(&self) -> &str {
                        self.kind.default_title()
                    }
                    fn kind(&self) -> WindowKind {
                        self.kind
                    }
                    fn render(&self, f: &mut Frame, area: Rect, _focused: bool) {
                        f.render_widget(
                            Paragraph::new(format!("{} — coming soon", self.kind.default_title())),
                            area,
                        );
                    }
                    fn handle_key(&mut self, _key: KeyEvent) -> bool {
                        false
                    }
                }
                Box::new(StubWindow { id, kind })
            }
        };

        tracing::info!(target: "cns.tui.workspace", operation = "window_opened", kind = ?kind, id = %id.0, "CNS");
        self.windows.insert(id, window);
        self.focus_window(id);
        id
    }

    pub fn split_focused(&mut self, direction: SplitDirection) {
        if self.sidebar_open {
            return;
        }

        let new_kind = match direction {
            SplitDirection::Horizontal => WindowKind::Sidebar,
            SplitDirection::Vertical => WindowKind::Chat,
        };

        let new_id = self.open_window(new_kind);
        let new_leaf = SplitNode::Leaf { window_id: new_id };
        let ratio = 0.7;
        let focused = self.focused;
        let root = self.root_mut();
        Self::replace_leaf_with_split(root, focused, &new_leaf, direction, ratio);
        self.focused = new_id;

        tracing::info!(target: "cns.tui.workspace", operation = "split", direction = ?direction, "CNS");
    }

    fn replace_leaf_with_split(
        node: &mut SplitNode,
        target: WindowId,
        new_leaf: &SplitNode,
        direction: SplitDirection,
        ratio: f32,
    ) -> bool {
        match node {
            SplitNode::Leaf { window_id } if *window_id == target => {
                let existing = SplitNode::Leaf { window_id: target };
                let new = SplitNode::Leaf {
                    window_id: if let SplitNode::Leaf { window_id } = new_leaf {
                        *window_id
                    } else {
                        return false;
                    },
                };
                *node = match direction {
                    SplitDirection::Horizontal => SplitNode::Horizontal {
                        left: Box::new(existing),
                        right: Box::new(new),
                        ratio,
                    },
                    SplitDirection::Vertical => SplitNode::Vertical {
                        top: Box::new(existing),
                        bottom: Box::new(new),
                        ratio,
                    },
                };
                true
            }
            SplitNode::Horizontal { left, right, .. } => {
                Self::replace_leaf_with_split(left, target, new_leaf, direction, ratio)
                    || Self::replace_leaf_with_split(right, target, new_leaf, direction, ratio)
            }
            SplitNode::Vertical { top, bottom, .. } => {
                Self::replace_leaf_with_split(top, target, new_leaf, direction, ratio)
                    || Self::replace_leaf_with_split(bottom, target, new_leaf, direction, ratio)
            }
            _ => false,
        }
    }

    pub fn close_focused_window(&mut self) {
        let focused = self.focused;
        if let Some(window) = self.windows.get(&focused) {
            if !window.can_close() {
                return;
            }
        }
        let sibling_id = self.remove_window_from_tree(focused);
        self.windows.remove(&focused);
        tracing::info!(target: "cns.tui.workspace", operation = "window_closed", id = %focused.0, "CNS");

        if let Some(sid) = sibling_id {
            self.focus_window(sid);
        } else if let Some(&first_id) = self.root().window_ids().first() {
            self.focus_window(first_id);
        }
    }

    fn remove_window_from_tree(&mut self, target: WindowId) -> Option<WindowId> {
        let ids = self.root().window_ids();
        if ids.len() <= 1 {
            return None;
        }
        let sibling = ids.iter().find(|&&id| id != target).copied();
        if let Some(sid) = sibling {
            self.root_mut().replace_with_single_leaf(target, sid);
        }
        sibling
    }

    pub fn focus_window(&mut self, id: WindowId) {
        if self.focused == id {
            return;
        }
        let prev = self.focused;
        if let Some(w) = self.windows.get_mut(&prev) {
            w.on_blur();
        }
        self.focused = id;
        if let Some(w) = self.windows.get_mut(&id) {
            w.on_focus();
        }
        tracing::info!(target: "cns.tui.workspace", operation = "focus_changed", from = %prev.0, to = %id.0, "CNS");
    }

    pub fn focus_next(&mut self) {
        let ids = self.root().window_ids();
        if let Some(pos) = ids.iter().position(|&id| id == self.focused) {
            let next = (pos + 1) % ids.len();
            self.focus_window(ids[next]);
        }
    }

    pub fn focus_prev(&mut self) {
        let ids = self.root().window_ids();
        if let Some(pos) = ids.iter().position(|&id| id == self.focused) {
            let prev = if pos == 0 { ids.len() - 1 } else { pos - 1 };
            self.focus_window(ids[prev]);
        }
    }

    pub fn resize_focused(&mut self, delta: f32) {
        let focused = self.focused;
        let root = self.root_mut();
        Self::adjust_split_ratio(root, focused, delta);
    }

    fn adjust_split_ratio(node: &mut SplitNode, target: WindowId, delta: f32) -> bool {
        match node {
            SplitNode::Horizontal { left, right, ratio } => {
                if left.contains_window(target) {
                    *ratio = (*ratio + delta).clamp(0.1, 0.9);
                    return true;
                }
                if right.contains_window(target) {
                    *ratio = (*ratio - delta).clamp(0.1, 0.9);
                    return true;
                }
                Self::adjust_split_ratio(left, target, delta)
                    || Self::adjust_split_ratio(right, target, delta)
            }
            SplitNode::Vertical { top, bottom, ratio } => {
                if top.contains_window(target) {
                    *ratio = (*ratio + delta).clamp(0.1, 0.9);
                    return true;
                }
                if bottom.contains_window(target) {
                    *ratio = (*ratio - delta).clamp(0.1, 0.9);
                    return true;
                }
                Self::adjust_split_ratio(top, target, delta)
                    || Self::adjust_split_ratio(bottom, target, delta)
            }
            _ => false,
        }
    }

    // ── Tab Management ───────────────────────────────────────────────────

    pub fn new_tab(&mut self) {
        let chat_id = WindowId(uuid::Uuid::new_v4());
        let chat = ChatWindow::new(
            chat_id,
            &self.agent_name,
            &self.current_model,
            self.service_context.clone(),
        );
        self.windows.insert(chat_id, Box::new(chat));

        let root = SplitNode::Leaf { window_id: chat_id };
        let tab = Tab::new(format!("Tab {}", self.tabs.len() + 1), root);
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        self.focus_window(chat_id);

        tracing::info!(target: "cns.tui.workspace", operation = "tab_switched", to_idx = self.active_tab, "CNS");
    }

    pub fn switch_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() && idx != self.active_tab {
            let _prev = self.active_tab;
            self.active_tab = idx;
            if let Some(&first_id) = self.root().window_ids().first() {
                self.focus_window(first_id);
            }
            tracing::info!(target: "cns.tui.workspace", operation = "tab_switched", from_idx = _prev, to_idx = idx, "CNS");
        }
    }

    // ── Sidebar ──────────────────────────────────────────────────────────

    pub fn toggle_sidebar(&mut self) {
        if self.sidebar_open {
            if let Some(sid) = self.sidebar_id.take() {
                self.close_window_by_id(sid);
            }
            self.sidebar_open = false;
        } else {
            self.split_focused(SplitDirection::Horizontal);
            self.sidebar_open = true;
            self.sidebar_id = Some(self.focused);
            self.focus_prev();
        }
    }

    fn close_window_by_id(&mut self, id: WindowId) {
        let prev_focus = self.focused;
        self.focused = id;
        self.close_focused_window();
        if self.windows.contains_key(&prev_focus) {
            self.focus_window(prev_focus);
        }
    }

    // ── Command Palette ──────────────────────────────────────────────────

    pub fn open_command_palette(&mut self) {
        self.palette_prev_focus = Some(self.focused);
        tracing::info!(target: "cns.tui.workspace", operation = "command_palette_opened", "CNS");
    }

    // ── Tick ─────────────────────────────────────────────────────────────

    pub fn tick(&mut self) {
        self.status.tick();
    }
}
