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
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(clippy::needless_borrow)]

mod keybindings;
mod repl_bridge;
mod splash;
mod status_bar;
mod tab;
mod window;
mod workspace;

pub mod bridges;
pub mod widgets;
pub mod windows;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;
use std::io::Stdout;
use std::time::Duration;

use bridges::{
    BackupDataBridge, CompaniesDataBridge, ConfigDataBridge, KanbanDataBridge, MatrixDataBridge,
    MediaDataBridge, MemoryDataBridge, RegistryDataBridge, TrainingDataBridge, WalletDataBridge,
};
pub use repl_bridge::{InferenceState, ReplBridge, TurnResult};
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
    /// Whether the session should exit (set by quit keybinding or window close)
    should_quit: bool,
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
            should_quit: false,
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

    /// Run the main event loop. Blocks until the user quits.
    pub fn run(&mut self) -> anyhow::Result<()> {
        // Show splash screen first
        self.show_splash()?;

        while !self.should_quit {
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

    /// Route a key event: try the focused window first, then global bindings.
    fn handle_key(&mut self, key: KeyEvent) {
        // Global keybindings take precedence
        if self.handle_global_key(key) {
            return;
        }

        // Route to focused window
        self.workspace.handle_key(key);
    }

    /// Global keybindings that work regardless of which window is focused.
    fn handle_global_key(&mut self, key: KeyEvent) -> bool {
        use KeyCode::*;

        match (key.modifiers, key.code) {
            // Quit
            (KeyModifiers::CONTROL, Char('q')) => {
                self.should_quit = true;
                true
            }
            // New tab
            (KeyModifiers::CONTROL, Char('t')) => {
                self.workspace.new_tab();
                true
            }
            // Close focused window
            (KeyModifiers::CONTROL, Char('w')) => {
                self.workspace.close_tab();
                if self.workspace.is_empty() {
                    self.should_quit = true;
                }
                true
            }
            // Split horizontal (open sidebar or split)
            (modifiers, Char('h'))
                if modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::SHIFT)) =>
            {
                self.workspace.split_focused(SplitDirection::Horizontal);
                true
            }
            // Split vertical
            (modifiers, Char('j'))
                if modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::SHIFT)) =>
            {
                self.workspace.split_focused(SplitDirection::Vertical);
                true
            }
            // Navigation between windows
            (KeyModifiers::CONTROL, Char('k')) | (KeyModifiers::CONTROL, Up) => {
                self.workspace.focus_prev();
                true
            }
            (KeyModifiers::CONTROL, Char('j')) | (KeyModifiers::CONTROL, Down) => {
                self.workspace.focus_next();
                true
            }
            (KeyModifiers::CONTROL, Char('h')) | (KeyModifiers::CONTROL, Left) => {
                self.workspace.focus_prev();
                true
            }
            (KeyModifiers::CONTROL, Char('l')) | (KeyModifiers::CONTROL, Right) => {
                self.workspace.focus_next();
                true
            }
            // Increase/decrease split ratio
            (KeyModifiers::CONTROL, Char('=')) => {
                self.workspace.resize_focused(0.05);
                true
            }
            (KeyModifiers::CONTROL, Char('-')) => {
                self.workspace.resize_focused(-0.05);
                true
            }
            // Tab switching
            (KeyModifiers::CONTROL, Char(d @ '1'..='9')) => {
                let idx = (d as u8 - b'1') as usize;
                self.workspace.switch_tab(idx);
                true
            }
            // Command palette
            (KeyModifiers::CONTROL, Char('p')) => {
                self.workspace.open_command_palette();
                true
            }
            // Toggle sidebar
            (KeyModifiers::CONTROL, Char('b')) => {
                self.workspace.toggle_sidebar();
                true
            }
            // Help overlay
            (KeyModifiers::NONE, Char('?')) => {
                self.workspace.toggle_help();
                true
            }
            // New window (cycle through kinds)
            (KeyModifiers::CONTROL, Char('n')) => {
                self.workspace.open_next_window_kind();
                true
            }
            _ => false,
        }
    }
}

impl Drop for TuiSession {
    fn drop(&mut self) {
        ratatui::restore();
    }
}
