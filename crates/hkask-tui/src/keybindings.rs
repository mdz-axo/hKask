//! Keybinding definitions for the TUI.
//!
//! Maps key combinations to actions. Provides help text for
//! the keybinding hint bar and the `/help` command.

/// Keybinding reference — used for the help display and hint bar.
pub const GLOBAL_BINDINGS: &[(&str, &str)] = &[
    ("Ctrl+Q", "Quit"),
    ("Ctrl+N", "New Chat"),
    ("Ctrl+T", "New tab"),
    ("Ctrl+W", "Close window"),
    ("Ctrl+B", "Toggle sidebar"),
    ("Ctrl+P", "Command palette"),
    ("Tab", "Next window"),
    ("Ctrl+H/J/K/L", "Navigate focus"),
    ("Ctrl+Shift+H", "Split horizontal"),
    ("Ctrl+Shift+J", "Split vertical"),
    ("Ctrl+=", "Increase split"),
    ("Ctrl+-", "Decrease split"),
    ("Ctrl+1-9", "Switch tab"),
];

pub const CHAT_BINDINGS: &[(&str, &str)] = &[
    ("Enter", "Send message"),
    ("/", "Slash command"),
    ("Ctrl+R", "Search history"),
    ("PageUp/PageDown", "Scroll history"),
    ("Esc", "Clear input / cancel"),
    ("[ / ]", "Previous/next section"),
];
