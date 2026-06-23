//! Workspace — split-tree layout, tab management, focus, and event routing.
//!
//! Each tab owns a `SplitNode` binary tree. Windows are stored directly
//! in leaf nodes. The workspace routes render, key events, focus, and
//! tick cycles through the active tab's tree.

use std::collections::HashMap;
use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use uuid::Uuid;

use crate::keybindings::{CHAT_BINDINGS, GLOBAL_BINDINGS};
use crate::repl_bridge::ReplBridge;
use crate::status_bar::StatusBar;
use crate::tab::Tab;
use crate::window::{Window, WindowId, WindowKind};
use crate::windows::chat::ChatWindow;
use crate::windows::logo::LogoWindow;
use crate::windows::sidebar::SidebarWindow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

pub enum SplitNode {
    Leaf(Box<dyn Window>),
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

impl std::fmt::Debug for SplitNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SplitNode::Leaf(w) => f.debug_tuple("Leaf").field(&w.id()).finish(),
            SplitNode::Horizontal { ratio, .. } => {
                f.debug_struct("H").field("ratio", ratio).finish()
            }
            SplitNode::Vertical { ratio, .. } => f.debug_struct("V").field("ratio", ratio).finish(),
        }
    }
}

impl SplitNode {
    fn collect_ids(&self, out: &mut Vec<WindowId>) {
        match self {
            SplitNode::Leaf(w) => out.push(w.id()),
            SplitNode::Horizontal { left, right, .. } => {
                left.collect_ids(out);
                right.collect_ids(out);
            }
            SplitNode::Vertical { top, bottom, .. } => {
                top.collect_ids(out);
                bottom.collect_ids(out);
            }
        }
    }

    fn window_ids(&self) -> Vec<WindowId> {
        let mut ids = Vec::new();
        self.collect_ids(&mut ids);
        ids
    }

    fn contains_window(&self, target: WindowId) -> bool {
        match self {
            SplitNode::Leaf(w) => w.id() == target,
            SplitNode::Horizontal { left, right, .. } => {
                left.contains_window(target) || right.contains_window(target)
            }
            SplitNode::Vertical { top, bottom, .. } => {
                top.contains_window(target) || bottom.contains_window(target)
            }
        }
    }

    fn find_leaf_mut(&mut self, target: WindowId) -> Option<&mut Box<dyn Window>> {
        match self {
            SplitNode::Leaf(w) if w.id() == target => Some(w),
            SplitNode::Horizontal { left, right, .. } => left
                .find_leaf_mut(target)
                .or_else(|| right.find_leaf_mut(target)),
            SplitNode::Vertical { top, bottom, .. } => top
                .find_leaf_mut(target)
                .or_else(|| bottom.find_leaf_mut(target)),
            _ => None,
        }
    }

    fn render(&self, f: &mut Frame, area: Rect, focused_id: Option<WindowId>) {
        match self {
            SplitNode::Leaf(w) => {
                let focused = focused_id == Some(w.id());
                let bs = if focused {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let block = Block::default()
                    .title(w.title())
                    .borders(Borders::ALL)
                    .border_style(bs);
                let inner = block.inner(area);
                f.render_widget(block, area);
                w.render(f, inner, focused);
            }
            SplitNode::Horizontal { left, right, ratio } => {
                let left_w = ((area.width as f32) * ratio).round() as u16;
                let right_w = area.width.saturating_sub(left_w);
                left.render(
                    f,
                    Rect::new(area.x, area.y, left_w, area.height),
                    focused_id,
                );
                right.render(
                    f,
                    Rect::new(area.x + left_w, area.y, right_w, area.height),
                    focused_id,
                );
            }
            SplitNode::Vertical { top, bottom, ratio } => {
                let top_h = ((area.height as f32) * ratio).round() as u16;
                let bottom_h = area.height.saturating_sub(top_h);
                top.render(f, Rect::new(area.x, area.y, area.width, top_h), focused_id);
                bottom.render(
                    f,
                    Rect::new(area.x, area.y + top_h, area.width, bottom_h),
                    focused_id,
                );
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent, focused_id: Option<WindowId>) -> bool {
        match self {
            SplitNode::Leaf(w) => focused_id == Some(w.id()) && w.handle_key(key),
            SplitNode::Horizontal { left, right, .. } => {
                left.handle_key(key, focused_id) || right.handle_key(key, focused_id)
            }
            SplitNode::Vertical { top, bottom, .. } => {
                top.handle_key(key, focused_id) || bottom.handle_key(key, focused_id)
            }
        }
    }

    fn tick(&mut self) {
        match self {
            SplitNode::Leaf(w) => w.tick(),
            SplitNode::Horizontal { left, right, .. } => {
                left.tick();
                right.tick();
            }
            SplitNode::Vertical { top, bottom, .. } => {
                top.tick();
                bottom.tick();
            }
        }
    }

    fn titles(&self, out: &mut HashMap<WindowId, String>) {
        match self {
            SplitNode::Leaf(w) => {
                out.insert(w.id(), w.title().to_string());
            }
            SplitNode::Horizontal { left, right, .. } => {
                left.titles(out);
                right.titles(out);
            }
            SplitNode::Vertical { top, bottom, .. } => {
                top.titles(out);
                bottom.titles(out);
            }
        }
    }

    /// Replace the leaf containing `target` with a split.
    /// The replacement widget `new_widget` is consumed only if the target is found.
    fn replace_leaf_with_split(
        &mut self,
        target: WindowId,
        new_widget: Box<dyn Window>,
        direction: SplitDirection,
        ratio: f32,
    ) -> bool {
        match self {
            SplitNode::Leaf(w) if w.id() == target => {
                // Take the existing leaf out
                let dummy = SplitNode::Leaf(Box::new(DummyWindow(target)));
                let old = std::mem::replace(self, dummy);
                let existing = if let SplitNode::Leaf(w) = old {
                    w
                } else {
                    return false;
                };

                *self = match direction {
                    SplitDirection::Horizontal => SplitNode::Horizontal {
                        left: Box::new(SplitNode::Leaf(existing)),
                        right: Box::new(SplitNode::Leaf(new_widget)),
                        ratio,
                    },
                    SplitDirection::Vertical => SplitNode::Vertical {
                        top: Box::new(SplitNode::Leaf(existing)),
                        bottom: Box::new(SplitNode::Leaf(new_widget)),
                        ratio,
                    },
                };
                true
            }
            SplitNode::Horizontal { left, right, .. } => {
                // Only recurse — if target found, widget consumed inside
                if left.contains_window(target) {
                    left.replace_leaf_with_split(target, new_widget, direction, ratio)
                } else if right.contains_window(target) {
                    right.replace_leaf_with_split(target, new_widget, direction, ratio)
                } else {
                    false
                }
            }
            SplitNode::Vertical { top, bottom, .. } => {
                if top.contains_window(target) {
                    top.replace_leaf_with_split(target, new_widget, direction, ratio)
                } else if bottom.contains_window(target) {
                    bottom.replace_leaf_with_split(target, new_widget, direction, ratio)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

pub struct Workspace {
    tabs: Vec<Tab>,
    active_tab: usize,
    focused_window: Option<WindowId>,
    service_context: Arc<hkask_services::AgentService>,
    bridge: Arc<dyn ReplBridge>,
    status_bar: StatusBar,
    sidebar_open: bool,
    help_visible: bool,
    _palette_prev_focus: Option<WindowId>,
}

impl Workspace {
    pub fn new(
        service_context: Arc<hkask_services::AgentService>,
        bridge: Arc<dyn ReplBridge>,
    ) -> Self {
        let agent = bridge.agent_name().to_string();
        let model = bridge.model_name().to_string();
        let chat_id = WindowId(Uuid::new_v4());
        let chat = ChatWindow::new(
            chat_id,
            &agent,
            &model,
            service_context.clone(),
            bridge.clone(),
        );
        let root = SplitNode::Leaf(Box::new(chat));
        let tab = Tab::new("Chat".to_string(), root);

        let mut status_bar = StatusBar::new();
        status_bar.model = model;
        status_bar.gas_remaining = bridge.gas_remaining();
        status_bar.gas_cap = bridge.gas_cap();

        Self {
            tabs: vec![tab],
            active_tab: 0,
            focused_window: Some(chat_id),
            service_context,
            bridge,
            status_bar,
            sidebar_open: false,
            help_visible: false,
            _palette_prev_focus: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty() || self.tabs[self.active_tab].root.window_ids().is_empty()
    }

    fn root(&self) -> &SplitNode {
        &self.tabs[self.active_tab].root
    }
    fn root_mut(&mut self) -> &mut SplitNode {
        &mut self.tabs[self.active_tab].root
    }

    // ── Rendering ────────────────────────────────────────────────────

    pub fn render(&self, f: &mut Frame) {
        let area = f.area();
        let tab_h = if self.tabs.len() > 1 { 1u16 } else { 0u16 };
        let status_h = 1u16;
        let content_h = area.height.saturating_sub(tab_h).saturating_sub(status_h);

        let mut y = area.y;
        if tab_h > 0 {
            self.render_tab_bar(f, Rect::new(area.x, y, area.width, tab_h));
            y += tab_h;
        }
        let content_area = Rect::new(area.x, y, area.width, content_h);
        y += content_h;
        let status_area = Rect::new(area.x, y, area.width, status_h);

        self.root().render(f, content_area, self.focused_window);
        if self.help_visible {
            self.render_help_overlay(f, content_area);
        }
        self.render_status(f, status_area);
    }

    fn render_tab_bar(&self, f: &mut Frame, area: Rect) {
        let parts: Vec<String> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                if i == self.active_tab {
                    format!(" [{}] ", tab.name)
                } else {
                    format!("  {}  ", tab.name)
                }
            })
            .collect();
        let bar = Paragraph::new(parts.join(""))
            .style(Style::default().fg(Color::White).bg(Color::DarkGray));
        f.render_widget(bar, area);
    }

    fn render_status(&self, f: &mut Frame, area: Rect) {
        // Update status from bridge each frame
        let mut titles = HashMap::new();
        self.root().titles(&mut titles);

        let bar_text = self.status_bar.render(
            self.focused_window.unwrap_or(WindowId(Uuid::nil())),
            &titles,
        );
        let bar = Paragraph::new(bar_text)
            .style(Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 40)));
        f.render_widget(bar, area);
    }

    // ── Events ───────────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: KeyEvent) {
        let focused = self.focused_window;
        self.root_mut().handle_key(key, focused);
    }

    // ── Focus ────────────────────────────────────────────────────────

    pub fn focus_next(&mut self) {
        let ids = self.root().window_ids();
        if let Some(ref fw) = self.focused_window {
            if let Some(pos) = ids.iter().position(|id| id == fw) {
                self.focus_window(ids[(pos + 1) % ids.len()]);
            }
        } else if let Some(&first) = ids.first() {
            self.focus_window(first);
        }
    }

    pub fn focus_prev(&mut self) {
        let ids = self.root().window_ids();
        if let Some(ref fw) = self.focused_window {
            if let Some(pos) = ids.iter().position(|id| id == fw) {
                let prev = if pos == 0 { ids.len() - 1 } else { pos - 1 };
                self.focus_window(ids[prev]);
            }
        } else if let Some(&first) = ids.first() {
            self.focus_window(first);
        }
    }

    fn focus_window(&mut self, id: WindowId) {
        if self.focused_window == Some(id) {
            return;
        }
        if let Some(prev) = self.focused_window {
            if let Some(w) = self.root_mut().find_leaf_mut(prev) {
                w.on_blur();
            }
        }
        self.focused_window = Some(id);
        if let Some(w) = self.root_mut().find_leaf_mut(id) {
            w.on_focus();
        }
    }

    // ── Split / Resize ───────────────────────────────────────────────

    pub fn split_focused(&mut self, direction: SplitDirection) {
        let Some(focused) = self.focused_window else {
            return;
        };
        let new_id = WindowId(Uuid::new_v4());
        let new_kind = match direction {
            SplitDirection::Horizontal => WindowKind::Sidebar,
            SplitDirection::Vertical => WindowKind::Chat,
        };
        let new_win: Box<dyn Window> = match new_kind {
            WindowKind::Sidebar => Box::new(SidebarWindow::new(
                new_id,
                self.service_context.clone(),
                self.bridge.clone(),
            )),
            _ => Box::new(ChatWindow::new(
                new_id,
                self.bridge.agent_name(),
                self.bridge.model_name(),
                self.service_context.clone(),
                self.bridge.clone(),
            )),
        };
        // focused is a Copy type, safe to use after root_mut()
        let ok = self
            .root_mut()
            .replace_leaf_with_split(focused, new_win, direction, 0.7);
        if ok {
            self.focused_window = Some(new_id);
        }
    }

    pub fn resize_focused(&mut self, delta: f32) {
        let Some(focused) = self.focused_window else {
            return;
        };
        Self::adjust_ratio(self.root_mut(), focused, delta);
    }

    fn adjust_ratio(node: &mut SplitNode, target: WindowId, delta: f32) -> bool {
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
                Self::adjust_ratio(left, target, delta) || Self::adjust_ratio(right, target, delta)
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
                Self::adjust_ratio(top, target, delta) || Self::adjust_ratio(bottom, target, delta)
            }
            _ => false,
        }
    }

    // ── Tabs ─────────────────────────────────────────────────────────

    pub fn new_tab(&mut self) {
        let chat_id = WindowId(Uuid::new_v4());
        let chat = ChatWindow::new(
            chat_id,
            self.bridge.agent_name(),
            self.bridge.model_name(),
            self.service_context.clone(),
            self.bridge.clone(),
        );
        let root = SplitNode::Leaf(Box::new(chat));
        let tab = Tab::new(format!("Tab {}", self.tabs.len() + 1), root);
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        self.focus_window(chat_id);
    }

    pub fn switch_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() && idx != self.active_tab {
            self.active_tab = idx;
            if let Some(&first_id) = self.root().window_ids().first() {
                self.focus_window(first_id);
            }
        }
    }

    // ── Sidebar ──────────────────────────────────────────────────────

    pub fn toggle_help(&mut self) {
        self.help_visible = !self.help_visible;
    }

    pub fn toggle_sidebar(&mut self) {
        if self.sidebar_open {
            self.sidebar_open = false;
        } else {
            self.split_focused(SplitDirection::Horizontal);
            self.sidebar_open = true;
            self.focus_prev();
        }
    }

    pub fn close_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.tabs.remove(self.active_tab);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len().saturating_sub(1);
            }
            if let Some(&first_id) = self.root().window_ids().first() {
                self.focus_window(first_id);
            }
        }
    }

    pub fn open_command_palette(&mut self) {
        self._palette_prev_focus = self.focused_window;
    }

    fn render_help_overlay(&self, f: &mut Frame, area: Rect) {
        let mut lines: Vec<ratatui::text::Line> = Vec::new();
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            "── Keybindings (? to close) ──",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        lines.push(ratatui::text::Line::from(""));
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            "Global:",
            Style::default().fg(Color::Cyan),
        )));
        for (key, desc) in GLOBAL_BINDINGS {
            lines.push(ratatui::text::Line::from(format!("  {:20} {}", key, desc)));
        }
        lines.push(ratatui::text::Line::from(""));
        lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
            "Chat:",
            Style::default().fg(Color::Cyan),
        )));
        for (key, desc) in CHAT_BINDINGS {
            lines.push(ratatui::text::Line::from(format!("  {:20} {}", key, desc)));
        }
        let overlay = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Help ")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().bg(Color::Rgb(20, 20, 30)));
        // Center the overlay
        let ow = area.width.min(60);
        let oh = area.height.min(20);
        let ox = area.x + (area.width.saturating_sub(ow)) / 2;
        let oy = area.y + (area.height.saturating_sub(oh)) / 2;
        f.render_widget(overlay, Rect::new(ox, oy, ow, oh));
    }

    // ── Tick ─────────────────────────────────────────────────────────

    pub fn tick(&mut self) {
        self.root_mut().tick();
        self.status_bar.gas_remaining = self.bridge.gas_remaining();
        self.status_bar.gas_cap = self.bridge.gas_cap();
        let alerts = self.bridge.cns_alert_count();
        self.status_bar.cns_status = if alerts >= 5 {
            crate::status_bar::CnsStatus::Critical(alerts)
        } else if alerts > 0 {
            crate::status_bar::CnsStatus::Warning(alerts)
        } else {
            crate::status_bar::CnsStatus::Healthy
        };
        self.status_bar.context_pressure = self.bridge.context_pressure();
        self.status_bar.model = self.bridge.model_name().to_string();
    }
}

/// Minimal window used only as a temporary placeholder during split operations.
struct DummyWindow(WindowId);
impl Window for DummyWindow {
    fn id(&self) -> WindowId {
        self.0
    }
    fn title(&self) -> &str {
        ""
    }
    fn kind(&self) -> WindowKind {
        WindowKind::Chat
    }
    fn render(&self, _: &mut Frame, _: Rect, _: bool) {}
    fn handle_key(&mut self, _: KeyEvent) -> bool {
        false
    }
}
