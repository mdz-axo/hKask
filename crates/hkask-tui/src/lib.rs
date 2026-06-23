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

mod keybindings;
mod repl_bridge;
mod status_bar;
mod tab;
mod window;
mod workspace;

pub mod widgets;
pub mod windows;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;
use std::io::Stdout;
use std::time::Duration;

pub use repl_bridge::{InferenceState, ReplBridge, TurnResult};
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

    /// Run the main event loop. Blocks until the user quits.
    pub fn run(&mut self) -> anyhow::Result<()> {
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
