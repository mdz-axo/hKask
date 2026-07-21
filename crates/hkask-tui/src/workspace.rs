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

use crate::bridges::{
    BackupDataBridge, CompaniesDataBridge, ConfigDataBridge, DocprocDataBridge, KanbanDataBridge,
    MatrixDataBridge, MediaDataBridge, MemoryDataBridge, RegistryDataBridge, ReplicaDataBridge,
    ResearchDataBridge, ScenariosDataBridge, SkillsDataBridge, TrainingDataBridge,
    WalletDataBridge, with_bridges, workspace_bridge_setter,
};
use crate::keybindings::{CHAT_BINDINGS, GLOBAL_BINDINGS};
use crate::repl_bridge::{ReplBridge, SystemBridge};
use crate::status_bar::StatusBar;
use crate::tab::Tab;
use crate::widgets::headers;
use crate::window::{Window, WindowId, WindowKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

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

    pub fn window_kind(&self, target: WindowId) -> Option<crate::window::WindowKind> {
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

    fn find_kind(&self, kind: WindowKind) -> Option<WindowId> {
        match self {
            SplitNode::Leaf(Some(window)) if window.kind() == kind => Some(window.id()),
            SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
            SplitNode::Horizontal { left, right, .. } => {
                left.find_kind(kind).or_else(|| right.find_kind(kind))
            }
            SplitNode::Vertical { top, bottom, .. } => {
                top.find_kind(kind).or_else(|| bottom.find_kind(kind))
            }
            _ => None,
        }
    }

    fn can_close(&self, target: WindowId) -> Option<bool> {
        match self {
            SplitNode::Leaf(Some(window)) if window.id() == target => Some(window.can_close()),
            SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
            SplitNode::Horizontal { left, right, .. } => {
                left.can_close(target).or_else(|| right.can_close(target))
            }
            SplitNode::Vertical { top, bottom, .. } => {
                top.can_close(target).or_else(|| bottom.can_close(target))
            }
            _ => None,
        }
    }

    fn remove(self, target: WindowId) -> (Option<Self>, bool) {
        match self {
            SplitNode::Leaf(Some(window)) if window.id() == target => (None, true),
            SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
            SplitNode::Horizontal { left, right, ratio } => {
                let (left, removed) = left.remove(target);
                if removed {
                    return match left {
                        Some(left) => (
                            Some(SplitNode::Horizontal {
                                left: Box::new(left),
                                right,
                                ratio,
                            }),
                            true,
                        ),
                        None => (Some(*right), true),
                    };
                }
                let left = left.expect("unchanged branch remains present");
                let (right, removed) = right.remove(target);
                match right {
                    Some(right) => (
                        Some(SplitNode::Horizontal {
                            left: Box::new(left),
                            right: Box::new(right),
                            ratio,
                        }),
                        removed,
                    ),
                    None => (Some(left), true),
                }
            }
            SplitNode::Vertical { top, bottom, ratio } => {
                let (top, removed) = top.remove(target);
                if removed {
                    return match top {
                        Some(top) => (
                            Some(SplitNode::Vertical {
                                top: Box::new(top),
                                bottom,
                                ratio,
                            }),
                            true,
                        ),
                        None => (Some(*bottom), true),
                    };
                }
                let top = top.expect("unchanged branch remains present");
                let (bottom, removed) = bottom.remove(target);
                match bottom {
                    Some(bottom) => (
                        Some(SplitNode::Vertical {
                            top: Box::new(top),
                            bottom: Box::new(bottom),
                            ratio,
                        }),
                        removed,
                    ),
                    None => (Some(top), true),
                }
            }
            leaf => (Some(leaf), false),
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
            SplitNode::Leaf(window)
                if window.as_ref().map(|w| w.id() == target).unwrap_or(false) =>
            {
                let existing = match window.take() {
                    Some(w) => w,
                    None => return false,
                };
                *self = match direction {
                    SplitDirection::Horizontal => SplitNode::Horizontal {
                        left: Box::new(SplitNode::Leaf(Some(existing))),
                        right: Box::new(SplitNode::Leaf(Some(new_widget))),
                        ratio,
                    },
                    SplitDirection::Vertical => SplitNode::Vertical {
                        top: Box::new(SplitNode::Leaf(Some(existing))),
                        bottom: Box::new(SplitNode::Leaf(Some(new_widget))),
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
    /// Whether the session should quit (set by Ctrl+Q).
    pub should_quit: bool,
    system_bridge: Arc<dyn SystemBridge>,
    repl_bridge: Arc<dyn ReplBridge>,
    bridges: WorkspaceBridges,
    status_bar: StatusBar,
    help_visible: bool,
    pub palette_open: bool,
    palette_prev_focus: Option<WindowId>,
    pub(crate) command_palette: crate::command_palette::CommandPalette,
}

#[derive(Default)]
struct WorkspaceBridges {
    wallet_bridge: Option<Arc<dyn WalletDataBridge>>,
    config_bridge: Option<Arc<dyn ConfigDataBridge>>,
    backup_bridge: Option<Arc<dyn BackupDataBridge>>,
    registry_bridge: Option<Arc<dyn RegistryDataBridge>>,
    memory_bridge: Option<Arc<dyn MemoryDataBridge>>,
    kanban_bridge: Option<Arc<dyn KanbanDataBridge>>,
    matrix_bridge: Option<Arc<dyn MatrixDataBridge>>,
    media_bridge: Option<Arc<dyn MediaDataBridge>>,
    training_bridge: Option<Arc<dyn TrainingDataBridge>>,
    companies_bridge: Option<Arc<dyn CompaniesDataBridge>>,
    research_bridge: Option<Arc<dyn ResearchDataBridge>>,
    docproc_bridge: Option<Arc<dyn DocprocDataBridge>>,
    replica_bridge: Option<Arc<dyn ReplicaDataBridge>>,
    skills_bridge: Option<Arc<dyn SkillsDataBridge>>,
    scenarios_bridge: Option<Arc<dyn ScenariosDataBridge>>,
}

impl WorkspaceBridges {
    fn to_window_bridges(
        &self,
        system_bridge: Arc<dyn SystemBridge>,
        repl_bridge: Arc<dyn ReplBridge>,
    ) -> crate::window_catalog::WindowBridges {
        crate::window_catalog::WindowBridges {
            system_bridge,
            repl_bridge,
            wallet_bridge: self.wallet_bridge.clone(),
            config_bridge: self.config_bridge.clone(),
            backup_bridge: self.backup_bridge.clone(),
            registry_bridge: self.registry_bridge.clone(),
            memory_bridge: self.memory_bridge.clone(),
            kanban_bridge: self.kanban_bridge.clone(),
            matrix_bridge: self.matrix_bridge.clone(),
            media_bridge: self.media_bridge.clone(),
            training_bridge: self.training_bridge.clone(),
            companies_bridge: self.companies_bridge.clone(),
            research_bridge: self.research_bridge.clone(),
            docproc_bridge: self.docproc_bridge.clone(),
            replica_bridge: self.replica_bridge.clone(),
            skills_bridge: self.skills_bridge.clone(),
            scenarios_bridge: self.scenarios_bridge.clone(),
        }
    }
}

impl Workspace {
    pub fn new(system: Arc<dyn SystemBridge>, repl: Arc<dyn ReplBridge>) -> Self {
        let model = system.model_name().to_string();
        let chat_id = WindowId(Uuid::new_v4());
        let bridges = WorkspaceBridges::default();
        let factory_ctx = bridges.to_window_bridges(system.clone(), repl.clone());
        let chat = crate::window_catalog::create_window(WindowKind::Chat, chat_id, &factory_ctx);

        // Logo window — persistent top-left anchor
        let logo_id = WindowId(Uuid::new_v4());
        let logo = crate::window_catalog::create_window(WindowKind::Logo, logo_id, &factory_ctx);

        // Default layout: Logo + Chat (left 65%) | Curator (right 35%)
        //   Left pane: vertical split — Logo (25%) + Chat (75%)
        //   Right pane: Curator (100%)
        let left = Box::new(SplitNode::Vertical {
            top: Box::new(SplitNode::Leaf(Some(logo))),
            bottom: Box::new(SplitNode::Leaf(Some(chat))),
            ratio: 0.25,
        });
        let curator_id = WindowId(Uuid::new_v4());
        let curator =
            crate::window_catalog::create_window(WindowKind::Curator, curator_id, &factory_ctx);
        let root = SplitNode::Horizontal {
            left,
            right: Box::new(SplitNode::Leaf(Some(curator))),
            ratio: 0.65,
        };
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
            help_visible: false,
            palette_open: false,
            palette_prev_focus: None,
            command_palette: crate::command_palette::CommandPalette::new(),
        }
    }

    with_bridges!(workspace_bridge_setter;
        wallet_bridge, WalletDataBridge, with_wallet_bridge;
        config_bridge, ConfigDataBridge, with_config_bridge;
        backup_bridge, BackupDataBridge, with_backup_bridge;
        registry_bridge, RegistryDataBridge, with_registry_bridge;
        memory_bridge, MemoryDataBridge, with_memory_bridge;
        kanban_bridge, KanbanDataBridge, with_kanban_bridge;
        matrix_bridge, MatrixDataBridge, with_matrix_bridge;
        media_bridge, MediaDataBridge, with_media_bridge;
        training_bridge, TrainingDataBridge, with_training_bridge;
        companies_bridge, CompaniesDataBridge, with_companies_bridge;
        research_bridge, ResearchDataBridge, with_research_bridge;
        docproc_bridge, DocprocDataBridge, with_docproc_bridge;
        replica_bridge, ReplicaDataBridge, with_replica_bridge;
        skills_bridge, SkillsDataBridge, with_skills_bridge;
        scenarios_bridge, ScenariosDataBridge, with_scenarios_bridge
    );

    /// Create a minimal workspace for testing — no AgentService, single Chat window.
    /// Uses the provided bridge for all window data. No logo, no curator, no splits.
    pub fn new_test(system: Arc<dyn SystemBridge>, repl: Arc<dyn ReplBridge>) -> Self {
        let model = system.model_name().to_string();
        let chat_id = WindowId(Uuid::new_v4());
        let bridges = WorkspaceBridges::default();
        let factory_ctx = bridges.to_window_bridges(system.clone(), repl.clone());
        let chat = crate::window_catalog::create_window(WindowKind::Chat, chat_id, &factory_ctx);
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
            help_visible: false,
            palette_open: false,
            palette_prev_focus: None,
            command_palette: crate::command_palette::CommandPalette::new(),
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
        if self.help_visible {
            self.render_help_overlay(f, content_area);
        }
        if self.palette_open {
            self.command_palette.render(f, content_area);
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

    /// Handle global keybindings. Returns true if the event was consumed.
    /// These are keys that work regardless of which window is focused.
    pub fn handle_global_key(&mut self, key: KeyEvent) -> bool {
        use KeyCode::*;
        use crossterm::event::{KeyCode, KeyModifiers};

        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, Char('q')) => {
                self.should_quit = true;
                true
            }
            (KeyModifiers::CONTROL, Char('t')) => {
                self.new_tab();
                true
            }
            (KeyModifiers::CONTROL, Char('w')) => {
                self.close_focused_window();
                true
            }
            (modifiers, Char('h'))
                if modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::SHIFT)) =>
            {
                self.split_focused(SplitDirection::Horizontal);
                true
            }
            (modifiers, Char('j'))
                if modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::SHIFT)) =>
            {
                self.split_focused(SplitDirection::Vertical);
                true
            }
            (KeyModifiers::CONTROL, Char('k')) | (KeyModifiers::CONTROL, Up) => {
                self.focus_prev();
                true
            }
            (KeyModifiers::CONTROL, Char('j')) | (KeyModifiers::CONTROL, Down) => {
                self.focus_next();
                true
            }
            (KeyModifiers::CONTROL, Char('h')) | (KeyModifiers::CONTROL, Left) => {
                self.focus_prev();
                true
            }
            (KeyModifiers::CONTROL, Char('l')) | (KeyModifiers::CONTROL, Right) => {
                self.focus_next();
                true
            }
            (KeyModifiers::CONTROL, Char('=')) => {
                self.resize_focused(0.05);
                true
            }
            (KeyModifiers::CONTROL, Char('-')) => {
                self.resize_focused(-0.05);
                true
            }
            (KeyModifiers::CONTROL, Char(d @ '1'..='9')) => {
                let idx = (d as u8 - b'1') as usize;
                self.switch_tab(idx);
                true
            }
            (KeyModifiers::CONTROL, Char('p')) => {
                self.open_command_palette();
                true
            }

            (KeyModifiers::NONE, Char('?')) => {
                self.toggle_help();
                true
            }
            (KeyModifiers::CONTROL, Char('n')) => {
                self.new_chat_window();
                true
            }
            (KeyModifiers::NONE, KeyCode::Tab) => {
                if let Some(focus) = self.focused_window {
                    let kind = self.root().window_kind(focus);
                    // Terminal and MCP-tabbed windows use Tab internally;
                    // determined by WindowKind::uses_internal_tab() from META.
                    if kind.is_some_and(|k| k.uses_internal_tab()) {
                        return false; // let focused window handle Tab
                    }
                }
                self.focus_next();
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

    // ── Split / Resize ───────────────────────────────────────────────

    pub fn split_focused(&mut self, direction: SplitDirection) {
        let Some(focused) = self.focused_window else {
            return;
        };
        let new_id = WindowId(Uuid::new_v4());
        let new_kind = match direction {
            SplitDirection::Horizontal => WindowKind::Chat,
            SplitDirection::Vertical => WindowKind::Chat,
        };
        let new_win = self.create_window_of_kind(new_kind, new_id);
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
        let chat = self.create_window_of_kind(WindowKind::Chat, chat_id);
        let root = SplitNode::Leaf(Some(chat));
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

    pub fn toggle_help(&mut self) {
        self.help_visible = !self.help_visible;
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

    pub fn close_focused_window(&mut self) {
        let Some(target) = self.focused_window else {
            return;
        };
        if self.window_count() <= 1 || self.root().can_close(target) != Some(true) {
            return;
        }

        let root = std::mem::replace(self.root_mut(), SplitNode::Leaf(None));
        let (root, removed) = root.remove(target);
        *self.root_mut() = root.expect("closing one of multiple windows preserves a root");
        if removed {
            self.focused_window = self.root().window_ids().first().copied();
        }
    }

    /// Open a new Chat window as a vertical split from the focused window.
    pub fn new_chat_window(&mut self) {
        self.open_window_kind(WindowKind::Chat);
    }

    pub fn open_command_palette(&mut self) {
        if self.palette_open {
            self.palette_open = false;
            if let Some(prev) = self.palette_prev_focus.take() {
                self.focus_window(prev);
            }
        } else {
            self.palette_open = true;
            self.palette_prev_focus = self.focused_window;
            self.command_palette.reset();
        }
    }

    /// Handle a key event while the command palette is open.
    /// Returns true if the event was consumed.
    pub fn handle_palette_key(&mut self, key: KeyEvent) -> bool {
        if !self.palette_open {
            return false;
        }
        use crossterm::event::{KeyCode, KeyModifiers};
        // Toggle dismiss
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('p') {
            self.palette_open = false;
            if let Some(prev) = self.palette_prev_focus.take() {
                self.focus_window(prev);
            }
            return true;
        }
        if let Some(action) = self.command_palette.handle_key(key) {
            match action {
                crate::command_palette::PaletteAction::Close => {
                    self.palette_open = false;
                    if let Some(prev) = self.palette_prev_focus.take() {
                        self.focus_window(prev);
                    }
                    return true;
                }
                crate::command_palette::PaletteAction::Open(kind) => {
                    self.palette_open = false;
                    self.palette_prev_focus.take();
                    self.open_window_kind(kind);
                    return true;
                }
            }
        }
        // Navigation/typing consumed by palette
        true
    }
    /// Open a window of the given kind at the currently focused split.
    pub fn open_window_kind(&mut self, kind: WindowKind) {
        if !kind.allows_multiple() {
            if let Some((tab_index, window_id)) = self
                .tabs
                .iter()
                .enumerate()
                .find_map(|(index, tab)| tab.root.find_kind(kind).map(|id| (index, id)))
            {
                self.active_tab = tab_index;
                self.focus_window(window_id);
                return;
            }
        }

        let new_id = WindowId(uuid::Uuid::new_v4());
        let new_win = self.create_window_of_kind(kind, new_id);
        let focused = self.focused_window;
        if self.root_mut().replace_leaf_with_split(
            focused.unwrap_or(WindowId(uuid::Uuid::nil())),
            new_win,
            SplitDirection::Vertical,
            0.6,
        ) {
            self.focused_window = Some(new_id);
        }
    }

    /// Create a window of the given kind without adding it to the tree.
    fn create_window_of_kind(&self, kind: WindowKind, id: WindowId) -> Box<dyn Window> {
        let ctx = self
            .bridges
            .to_window_bridges(self.system_bridge.clone(), self.repl_bridge.clone());
        crate::window_catalog::create_window(kind, id, &ctx)
    }

    fn render_help_overlay(&self, f: &mut Frame, area: Rect) {
        let ow = area.width.min(60);
        let oh = area.height.min(20);
        // Guard: zero-size area on tiny terminals.
        if ow == 0 || oh == 0 {
            return;
        }
        let mut lines: Vec<ratatui::text::Line> = Vec::new();
        lines.push(headers::section_with_color(
            "Keybindings (? to close)",
            Color::Yellow,
        ));
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
        let ox = area.x + (area.width.saturating_sub(ow)) / 2;
        let oy = area.y + (area.height.saturating_sub(oh)) / 2;
        f.render_widget(overlay, Rect::new(ox, oy, ow, oh));
    }

    // ── Tick ─────────────────────────────────────────────────────────

    // ── Layout persistence ────────────────────────────────────────────

    /// Extract the current layout into a serializable form.
    pub fn extract_layout(&self) -> crate::layout::SavedLayout {
        let tabs: Vec<crate::layout::SavedTab> = self
            .tabs
            .iter()
            .map(|tab| crate::layout::SavedTab {
                name: tab.name.clone(),
                root: Self::extract_split(&tab.root),
            })
            .collect();
        crate::layout::SavedLayout {
            version: 1,
            tabs,
            active_tab: self.active_tab,
        }
    }

    fn extract_split(node: &SplitNode) -> crate::layout::SavedSplit {
        match node {
            SplitNode::Leaf(Some(window)) => {
                crate::layout::SavedSplit::Leaf(crate::layout::SavedLeaf {
                    kind: crate::layout::kind_to_string(window.kind()),
                })
            }
            SplitNode::Leaf(None) => unreachable!("Leaf must contain a window"),
            SplitNode::Horizontal { left, right, ratio } => crate::layout::SavedSplit::Horizontal {
                left: Box::new(Self::extract_split(left)),
                right: Box::new(Self::extract_split(right)),
                ratio: *ratio,
            },
            SplitNode::Vertical { top, bottom, ratio } => crate::layout::SavedSplit::Vertical {
                top: Box::new(Self::extract_split(top)),
                bottom: Box::new(Self::extract_split(bottom)),
                ratio: *ratio,
            },
        }
    }

    /// Build a window from a saved leaf kind.
    fn build_window(&self, kind: crate::layout::SavedLeaf) -> Box<dyn Window> {
        let wk = crate::layout::string_to_kind(&kind.kind);
        let new_id = WindowId(uuid::Uuid::new_v4());
        self.create_window_of_kind(wk, new_id)
    }

    fn restore_split(&self, saved: &crate::layout::SavedSplit) -> SplitNode {
        match saved {
            crate::layout::SavedSplit::Leaf(leaf) => {
                SplitNode::Leaf(Some(self.build_window(leaf.clone())))
            }
            crate::layout::SavedSplit::Horizontal { left, right, ratio } => SplitNode::Horizontal {
                left: Box::new(self.restore_split(left)),
                right: Box::new(self.restore_split(right)),
                ratio: *ratio,
            },
            crate::layout::SavedSplit::Vertical { top, bottom, ratio } => SplitNode::Vertical {
                top: Box::new(self.restore_split(top)),
                bottom: Box::new(self.restore_split(bottom)),
                ratio: *ratio,
            },
        }
    }

    /// Restore the workspace from a saved layout.
    /// Replaces all tabs and windows with those from the saved layout.
    pub fn restore_layout(&mut self, layout: &crate::layout::SavedLayout) {
        if !layout.is_valid() {
            return;
        }

        self.tabs.clear();
        for saved_tab in &layout.tabs {
            let root = self.restore_split(&saved_tab.root);
            self.tabs
                .push(crate::tab::Tab::new(saved_tab.name.clone(), root));
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
        self.status_bar.gas_remaining = self.system_bridge.gas_remaining();
        self.status_bar.gas_cap = self.system_bridge.gas_cap();
        let alerts = self.system_bridge.cns_alert_count();
        self.status_bar.cns_status = if alerts >= 5 {
            crate::status_bar::CnsStatus::Critical(alerts)
        } else if alerts > 0 {
            crate::status_bar::CnsStatus::Warning(alerts)
        } else {
            crate::status_bar::CnsStatus::Healthy
        };
        self.status_bar.context_pressure = self.system_bridge.context_pressure();
        self.status_bar.model = self.system_bridge.model_name().to_string();
    }
}
