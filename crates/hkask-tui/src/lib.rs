//! hKask TUI — Terminal UI workspace for the hKask agent platform.
//!
//! Provides a multi-window, split-pane terminal interface modelled on
//! Zed's workspace architecture: a binary tree of splits hosts stateful
//! Window implementations, with keyboard-driven focus, resize, and tab
//! management. Each window renders a subsystem (chat, CNS monitor,
//! backup, registry, matrix, pods, kanban) as a ratatui widget.
//!
//! # Architecture
//!
//! ```text
//! TuiSession
//!   └── Workspace
//!         ├── Tab bar
//!         ├── SplitNode tree (binary splits)
//!         │     ├── Leaf: Window
//!         │     ├── Horizontal { left, right, ratio }
//!         │     └── Vertical   { top, bottom, ratio }
//!         └── StatusBar (global)
//! ```rust,no_run
//!
//! # Entry Point
//!
//! ```ignore
//! let session = TuiSession::new(state, repl_settings)?;
//! session.run()?;
//! ```

#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::new_without_default)]
#![allow(clippy::useless_format)]
#![allow(clippy::needless_borrow)]

mod keybindings;
mod repl_bridge;
mod splash;
mod status_bar;
mod tab;
mod text_cursor;
mod window;
mod window_catalog;
mod workspace;

#[cfg(test)]
mod render_guards_tests;
#[cfg(test)]
mod test_util;

pub mod bridges;
pub mod command_palette;
pub mod layout;
pub mod mcp_tabbed;
pub mod widgets;
pub mod windows;

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;
use std::io::Stdout;
use std::time::Duration;

use bridges::{
    BackupDataBridge, CompaniesDataBridge, ConfigDataBridge, DocprocDataBridge, KanbanDataBridge,
    MatrixDataBridge, MediaDataBridge, MemoryDataBridge, RegistryDataBridge, ReplicaDataBridge,
    ResearchDataBridge, ScenariosDataBridge, SkillsDataBridge, TrainingDataBridge,
    WalletDataBridge, tui_bridge_setter, with_bridges,
};
pub use repl_bridge::{
    InferenceRequestId, InferenceState, ModelSwitchResult, ReplBridge, SessionBridge,
    SettingsBridge, SystemBridge, TuiModelInfo, TuiTurnResult,
};
pub use splash::SplashScreen;
pub use window::{Window, WindowId, WindowKind};
pub use workspace::{SplitDirection, Workspace};

/// Top-level TUI session — owns the terminal, workspace, and event loop.
///
/// Constructed with the shared service context from `hkask-services`
/// and user-configurable settings. The session takes over the terminal
/// (raw mode + alternate screen) and restores it on drop.
pub struct TuiSession {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    workspace: Workspace,
    layout_path: Option<std::path::PathBuf>,
    /// Tick rate for event polling (ms)
    tick_rate: Duration,
}

impl TuiSession {
    /// Create a new TUI session with the given service context.
    ///
    /// Initializes the terminal in raw mode + alternate screen,
    /// builds the workspace with a default layout (chat window).
    pub fn new(
        system: std::sync::Arc<dyn SystemBridge>,
        repl: std::sync::Arc<dyn ReplBridge>,
    ) -> anyhow::Result<Self> {
        let mut terminal = ratatui::init();
        terminal.clear()?;

        let workspace = Workspace::new(system, repl);

        Ok(Self {
            terminal,
            workspace,
            layout_path: None,
            tick_rate: Duration::from_millis(16),
        })
    }

    with_bridges!(tui_bridge_setter;
        settings_bridge, SettingsBridge, with_settings_bridge;
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

    /// Run the main event loop. Blocks until the user quits.
    /// Set the layout path for per-agent workspace persistence.
    pub fn with_layout_path(mut self, path: std::path::PathBuf) -> Self {
        self.layout_path = Some(path);
        self
    }

    fn restore_layout(&mut self) {
        if let Some(ref path) = self.layout_path {
            if let Some(layout) = crate::layout::load(path) {
                self.workspace.restore_layout(&layout);
            }
        }
    }

    fn save_layout(&self) {
        if let Some(ref path) = self.layout_path {
            let layout = self.workspace.extract_layout();
            let _ = crate::layout::save(path, &layout);
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        // Show splash screen first
        self.show_splash()?;

        // Restore saved layout if available
        self.restore_layout();

        while !self.workspace.should_quit {
            // Render current frame
            self.terminal.draw(|f| self.workspace.render(f))?;

            // Poll for events
            if event::poll(self.tick_rate)? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.handle_key(key);
                    }
                    Event::Resize(_, _) => {
                        // ratatui handles resize on next draw automatically
                    }
                    _ => {}
                }
            }

            // Tick workspace for background updates (CNS polling, etc.)
            self.workspace.tick();
        }

        // Save layout on quit
        self.save_layout();
        Ok(())
    }

    /// Display the Kask logo splash screen before entering the main workspace.
    fn show_splash(&mut self) -> anyhow::Result<()> {
        let mut splash = SplashScreen::new();

        loop {
            self.terminal.draw(|f| splash.render(f))?;

            // Check for early dismissal via key press
            if splash.check_early_dismiss() {
                break;
            }

            // Auto-dismiss after duration
            if splash.should_dismiss() {
                break;
            }

            std::thread::sleep(Duration::from_millis(16));
        }

        Ok(())
    }

    /// Route a key event: palette (if open), then global bindings, then focused window.
    fn handle_key(&mut self, key: KeyEvent) {
        // Command palette interception — palette eats all keys when open
        if self.workspace.palette_open {
            self.workspace.handle_palette_key(key);
            return;
        }

        // Global keybindings take precedence
        if self.workspace.handle_global_key(key) {
            return;
        }

        // Route to focused window
        self.workspace.handle_key(key);
    }
}

impl Drop for TuiSession {
    fn drop(&mut self) {
        ratatui::restore();
    }
}
