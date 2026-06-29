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
//! ```
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
mod window;
mod window_catalog;
mod workspace;

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
    ResearchDataBridge, SkillsDataBridge, TrainingDataBridge, WalletDataBridge,
};
pub use repl_bridge::{InferenceState, ReplBridge, TuiTurnResult};
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
        service_context: std::sync::Arc<hkask_services::AgentService>,
        bridge: std::sync::Arc<dyn ReplBridge>,
    ) -> anyhow::Result<Self> {
        let mut terminal = ratatui::init();
        terminal.clear()?;

        let workspace = Workspace::new(service_context, bridge);

        Ok(Self {
            terminal,
            workspace,
            layout_path: None,
            tick_rate: Duration::from_millis(16),
        })
    }

    pub fn with_wallet_bridge(mut self, wallet: std::sync::Arc<dyn WalletDataBridge>) -> Self {
        self.workspace.with_wallet_bridge(wallet);
        self
    }

    pub fn with_config_bridge(mut self, config: std::sync::Arc<dyn ConfigDataBridge>) -> Self {
        self.workspace.with_config_bridge(config);
        self
    }

    pub fn with_backup_bridge(mut self, backup: std::sync::Arc<dyn BackupDataBridge>) -> Self {
        self.workspace.with_backup_bridge(backup);
        self
    }

    pub fn with_registry_bridge(
        mut self,
        registry: std::sync::Arc<dyn RegistryDataBridge>,
    ) -> Self {
        self.workspace.with_registry_bridge(registry);
        self
    }

    pub fn with_memory_bridge(mut self, memory: std::sync::Arc<dyn MemoryDataBridge>) -> Self {
        self.workspace.with_memory_bridge(memory);
        self
    }

    pub fn with_kanban_bridge(mut self, kanban: std::sync::Arc<dyn KanbanDataBridge>) -> Self {
        self.workspace.with_kanban_bridge(kanban);
        self
    }

    pub fn with_matrix_bridge(mut self, matrix: std::sync::Arc<dyn MatrixDataBridge>) -> Self {
        self.workspace.with_matrix_bridge(matrix);
        self
    }

    pub fn with_media_bridge(mut self, media: std::sync::Arc<dyn MediaDataBridge>) -> Self {
        self.workspace.with_media_bridge(media);
        self
    }

    pub fn with_training_bridge(
        mut self,
        training: std::sync::Arc<dyn TrainingDataBridge>,
    ) -> Self {
        self.workspace.with_training_bridge(training);
        self
    }

    pub fn with_companies_bridge(
        mut self,
        companies: std::sync::Arc<dyn CompaniesDataBridge>,
    ) -> Self {
        self.workspace.with_companies_bridge(companies);
        self
    }

    pub fn with_research_bridge(
        mut self,
        research: std::sync::Arc<dyn ResearchDataBridge>,
    ) -> Self {
        self.workspace.with_research_bridge(research);
        self
    }
    pub fn with_docproc_bridge(mut self, docproc: std::sync::Arc<dyn DocprocDataBridge>) -> Self {
        self.workspace.with_docproc_bridge(docproc);
        self
    }
    pub fn with_replica_bridge(mut self, replica: std::sync::Arc<dyn ReplicaDataBridge>) -> Self {
        self.workspace.with_replica_bridge(replica);
        self
    }
    pub fn with_skills_bridge(mut self, skills: std::sync::Arc<dyn SkillsDataBridge>) -> Self {
        self.workspace.with_skills_bridge(skills);
        self
    }

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

        // Save layout on quit
        self.save_layout();
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
