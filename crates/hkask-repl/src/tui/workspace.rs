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

use crate::tui::bridges::{with_bridges, workspace_bridge_setter};
use crate::tui::repl_bridge::{ReplBridge, SessionBridge, SettingsBridge, SystemBridge};
use crate::tui::status_bar::StatusBar;
use crate::tui::tab::Tab;
use crate::tui::window::{Window, WindowId, WindowKind, WorkspaceAction};

pub enum SplitNode {
    Leaf(Option<Box<dyn Window>>),
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
            SplitNode::Leaf(Some(w)) => f.debug_tuple("Leaf").field(&w.id()).finish(),
            SplitNode::Leaf(None) => f.debug_tuple("Leaf").field(&"empty").finish(),
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
            SplitNode::Leaf(Some(w)) => out.push(w.id()),
            SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
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

    pub fn window_ids(&self) -> Vec<WindowId> {
        let mut ids = Vec::new();
        self.collect_ids(&mut ids);
        ids
    }

    pub fn window_kind(&self, target: WindowId) -> Option<crate::tui::window::WindowKind> {
        match self {
            SplitNode::Leaf(Some(w)) if w.id() == target => Some(w.kind()),
            SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
            SplitNode::Horizontal { left, right, .. } => left
                .window_kind(target)
                .or_else(|| right.window_kind(target)),
            SplitNode::Vertical { top, bottom, .. } => top
                .window_kind(target)
                .or_else(|| bottom.window_kind(target)),
            _ => None,
        }
    }

    pub fn contains_window(&self, target: WindowId) -> bool {
        match self {
            SplitNode::Leaf(Some(w)) => w.id() == target,
            SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
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
            SplitNode::Leaf(Some(w)) if w.id() == target => Some(w),
            SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
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
            SplitNode::Leaf(Some(w)) => {
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
            SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
            SplitNode::Horizontal { left, right, ratio } => {
                let left_w = ((area.width as f32) * ratio).round() as u16;
                let right_w = area.width.saturating_sub(left_w);
                // Guard: tiny areas with extreme ratios can produce zero-width panes.
                if left_w < 2 || right_w < 2 {
                    return;
                }
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
                // Guard: tiny areas with extreme ratios can produce zero-height panes.
                if top_h < 2 || bottom_h < 2 {
                    return;
                }
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
            SplitNode::Leaf(Some(w)) => focused_id == Some(w.id()) && w.handle_key(key),
            SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
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
            SplitNode::Leaf(Some(w)) => w.tick(),
            SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
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
            SplitNode::Leaf(Some(w)) => {
                out.insert(w.id(), w.title().to_string());
            }
            SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
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
}

pub struct Workspace {
    tabs: Vec<Tab>,
    active_tab: usize,
    focused_window: Option<WindowId>,
    /// Whether the session should quit (set by Ctrl+Q).
    pub should_quit: bool,
    system_bridge: Arc<dyn SystemBridge>,
    repl_bridge: Arc<dyn ReplBridge>,
    bridges: WorkspaceBridges,
    status_bar: StatusBar,
}

#[derive(Default)]
struct WorkspaceBridges {
    settings_bridge: Option<Arc<dyn SettingsBridge>>,
    session_bridge: Option<Arc<dyn SessionBridge>>,
}

impl WorkspaceBridges {
    fn to_window_bridges(
        &self,
        system_bridge: Arc<dyn SystemBridge>,
        repl_bridge: Arc<dyn ReplBridge>,
    ) -> crate::tui::window_catalog::WindowBridges {
        crate::tui::window_catalog::WindowBridges {
            system_bridge,
            repl_bridge,
            settings_bridge: self.settings_bridge.clone(),
            session_bridge: self.session_bridge.clone(),
        }
    }
}

impl Workspace {
    pub fn new(system: Arc<dyn SystemBridge>, repl: Arc<dyn ReplBridge>) -> Self {
        let model = system.model_name().to_string();
        let chat_id = WindowId(Uuid::new_v4());
        let bridges = WorkspaceBridges::default();
        let factory_ctx = bridges.to_window_bridges(system.clone(), repl.clone());
        let chat =
            crate::tui::window_catalog::create_window(WindowKind::Chat, chat_id, &factory_ctx);

        // Default layout: single Chat window filling the workspace.
        // The Chat window defaults to Curator mode (P12.1 dual-presence),
        // so the user interacts with the Curator daemon by default and
        // opens other windows via slash commands or the command palette.
        let root = SplitNode::Leaf(Some(chat));
        let tab = Tab::new("Chat".to_string(), root);

        let mut status_bar = StatusBar::new();
        status_bar.model = model;
        status_bar.gas_remaining = system.gas_remaining();
        status_bar.gas_cap = system.gas_cap();

        Self {
            tabs: vec![tab],
            active_tab: 0,
            focused_window: Some(chat_id),
            should_quit: false,
            system_bridge: system,
            repl_bridge: repl,
            bridges,
            status_bar,
        }
    }

    with_bridges!(workspace_bridge_setter;
        settings_bridge, SettingsBridge, with_settings_bridge;
        session_bridge, SessionBridge, with_session_bridge
    );

    /// Create a minimal workspace for testing — single Chat window.
    /// Uses the provided bridge for all window data. No splits.
    pub fn new_test(system: Arc<dyn SystemBridge>, repl: Arc<dyn ReplBridge>) -> Self {
        let model = system.model_name().to_string();
        let chat_id = WindowId(Uuid::new_v4());
        let bridges = WorkspaceBridges::default();
        let factory_ctx = bridges.to_window_bridges(system.clone(), repl.clone());
        let chat =
            crate::tui::window_catalog::create_window(WindowKind::Chat, chat_id, &factory_ctx);
        let root = SplitNode::Leaf(Some(chat));
        let tab = Tab::new("Test".to_string(), root);

        let mut status_bar = StatusBar::new();
        status_bar.model = model;
        status_bar.gas_remaining = system.gas_remaining();
        status_bar.gas_cap = system.gas_cap();

        Self {
            tabs: vec![tab],
            active_tab: 0,
            focused_window: Some(chat_id),
            should_quit: false,
            system_bridge: system,
            repl_bridge: repl,
            bridges,
            status_bar,
        }
    }

    /// Get the currently focused window ID.
    pub fn focused_window(&self) -> Option<WindowId> {
        self.focused_window
    }

    /// Number of windows in the active tab's split tree.
    pub fn window_count(&self) -> usize {
        self.root().window_ids().len()
    }

    /// Number of open tabs.
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// Index of the currently active tab.
    pub fn active_tab_index(&self) -> usize {
        self.active_tab
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty() || self.tabs[self.active_tab].root.window_ids().is_empty()
    }

    pub fn root(&self) -> &SplitNode {
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

        // Guard: tiny terminals may have zero content height after subtracting
        // tab + status bars. Rendering into zero area panics ratatui.
        if content_h == 0 {
            return;
        }

        let mut y = area.y;
        if tab_h > 0 {
            self.render_tab_bar(f, Rect::new(area.x, y, area.width, tab_h));
            y += tab_h;
        }
        let content_area = Rect::new(area.x, y, area.width, content_h);
        y += content_h;
        let status_area = Rect::new(area.x, y, area.width, status_h);

        self.root().render(f, content_area, self.focused_window);
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

    /// Handle global keybindings. Returns true if the event was consumed.
    /// These are keys that work regardless of which window is focused.
    pub fn handle_global_key(&mut self, key: KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
                self.should_quit = true;
                true
            }
            _ => false,
        }
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

    /// Create a window of the given kind without adding it to the tree.
    fn create_window_of_kind(&self, kind: WindowKind, id: WindowId) -> Box<dyn Window> {
        let ctx = self
            .bridges
            .to_window_bridges(self.system_bridge.clone(), self.repl_bridge.clone());
        crate::tui::window_catalog::create_window(kind, id, &ctx)
    }

    fn focus_window(&mut self, id: WindowId) {
        if self.focused_window == Some(id) {
            return;
        }
        if let Some(prev) = self.focused_window
            && let Some(w) = self.root_mut().find_leaf_mut(prev)
        {
            w.on_blur();
        }
        self.focused_window = Some(id);
        if let Some(w) = self.root_mut().find_leaf_mut(id) {
            w.on_focus();
        }
    }

    // ── Tick ─────────────────────────────────────────────────────────

    // ── Layout persistence ────────────────────────────────────────────

    /// Extract the current layout into a serializable form.
    pub fn extract_layout(&self) -> crate::tui::layout::SavedLayout {
        let tabs: Vec<crate::tui::layout::SavedTab> = self
            .tabs
            .iter()
            .map(|tab| crate::tui::layout::SavedTab {
                name: tab.name.clone(),
                root: Self::extract_split(&tab.root),
            })
            .collect();
        crate::tui::layout::SavedLayout {
            version: 1,
            tabs,
            active_tab: self.active_tab,
        }
    }

    fn extract_split(node: &SplitNode) -> crate::tui::layout::SavedSplit {
        match node {
            SplitNode::Leaf(Some(window)) => {
                crate::tui::layout::SavedSplit::Leaf(crate::tui::layout::SavedLeaf {
                    kind: crate::tui::layout::kind_to_string(window.kind()),
                })
            }
            SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
            SplitNode::Horizontal { left, right, ratio } => {
                crate::tui::layout::SavedSplit::Horizontal {
                    left: Box::new(Self::extract_split(left)),
                    right: Box::new(Self::extract_split(right)),
                    ratio: *ratio,
                }
            }
            SplitNode::Vertical { top, bottom, ratio } => {
                crate::tui::layout::SavedSplit::Vertical {
                    top: Box::new(Self::extract_split(top)),
                    bottom: Box::new(Self::extract_split(bottom)),
                    ratio: *ratio,
                }
            }
        }
    }

    /// Build a window from a saved leaf kind.
    fn build_window(&self, kind: crate::tui::layout::SavedLeaf) -> Box<dyn Window> {
        let wk = crate::tui::layout::string_to_kind(&kind.kind);
        let new_id = WindowId(uuid::Uuid::new_v4());
        self.create_window_of_kind(wk, new_id)
    }

    fn restore_split(&self, saved: &crate::tui::layout::SavedSplit) -> SplitNode {
        match saved {
            crate::tui::layout::SavedSplit::Leaf(leaf) => {
                SplitNode::Leaf(Some(self.build_window(leaf.clone())))
            }
            crate::tui::layout::SavedSplit::Horizontal { left, right, ratio } => {
                SplitNode::Horizontal {
                    left: Box::new(self.restore_split(left)),
                    right: Box::new(self.restore_split(right)),
                    ratio: *ratio,
                }
            }
            crate::tui::layout::SavedSplit::Vertical { top, bottom, ratio } => {
                SplitNode::Vertical {
                    top: Box::new(self.restore_split(top)),
                    bottom: Box::new(self.restore_split(bottom)),
                    ratio: *ratio,
                }
            }
        }
    }

    /// Restore the workspace from a saved layout.
    /// Replaces all tabs and windows with those from the saved layout.
    pub fn restore_layout(&mut self, layout: &crate::tui::layout::SavedLayout) {
        if !layout.is_valid() {
            return;
        }

        self.tabs.clear();
        for saved_tab in &layout.tabs {
            let root = self.restore_split(&saved_tab.root);
            self.tabs
                .push(crate::tui::tab::Tab::new(saved_tab.name.clone(), root));
        }
        if layout.active_tab < self.tabs.len() {
            self.active_tab = layout.active_tab;
        } else if !self.tabs.is_empty() {
            self.active_tab = 0;
        }
        if let Some(&first) = self.root().window_ids().first() {
            self.focused_window = Some(first);
        }
    }

    pub fn tick(&mut self) {
        self.root_mut().tick();
        // Drain window actions (only Quit now — window management removed).
        let actions = collect_actions(self.root_mut());
        for action in actions {
            match action {
                WorkspaceAction::Quit => self.should_quit = true,
            }
        }
        self.status_bar.gas_remaining = self.system_bridge.gas_remaining();
        self.status_bar.gas_cap = self.system_bridge.gas_cap();
        let alerts = self.system_bridge.reg_alert_count();
        self.status_bar.reg_status = if alerts >= 5 {
            crate::tui::status_bar::RegStatus::Critical(alerts)
        } else if alerts > 0 {
            crate::tui::status_bar::RegStatus::Warning(alerts)
        } else {
            crate::tui::status_bar::RegStatus::Healthy
        };
        self.status_bar.context_pressure = self.system_bridge.context_pressure();
        self.status_bar.model = self.system_bridge.model_name().to_string();
    }
}

/// Recursively collect `WorkspaceAction`s from all windows in the tree.
fn collect_actions(node: &mut SplitNode) -> Vec<WorkspaceAction> {
    let mut actions = Vec::new();
    match node {
        SplitNode::Leaf(Some(w)) => {
            if let Some(action) = w.drain_action() {
                actions.push(action);
            }
        }
        SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
        SplitNode::Horizontal { left, right, .. } => {
            actions.extend(collect_actions(left));
            actions.extend(collect_actions(right));
        }
        SplitNode::Vertical { top, bottom, .. } => {
            actions.extend(collect_actions(top));
            actions.extend(collect_actions(bottom));
        }
    }
    actions
}
