//! Layout persistence — save and restore TUI workspace layouts.
//!
//! Saves the split tree, window kinds, tab names, and active tab
//! to a JSON file per-agent. Loaded on TUI startup, saved on quit.
//! Associated with the replicant identity for privacy (P1).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::window::WindowKind;

/// Serializable representation of a window layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedLayout {
    pub version: u32,
    pub tabs: Vec<SavedTab>,
    pub active_tab: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedTab {
    pub name: String,
    pub root: SavedSplit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SavedSplit {
    Leaf(SavedLeaf),
    Horizontal {
        left: Box<SavedSplit>,
        right: Box<SavedSplit>,
        ratio: f32,
    },
    Vertical {
        top: Box<SavedSplit>,
        bottom: Box<SavedSplit>,
        ratio: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedLeaf {
    /// Serialized WindowKind variant name (e.g., "Chat", "Kanban")
    pub kind: String,
}

/// Convert a WindowKind to its serialized string.
pub fn kind_to_string(kind: WindowKind) -> String {
    kind.default_title().to_string()
}

/// Parse a serialized kind string back to WindowKind.
/// Returns Chat as a safe fallback for unknown kinds.
pub fn string_to_kind(s: &str) -> WindowKind {
    match s {
        "Chat" => WindowKind::Chat,
        "CNS Monitor" => WindowKind::CnsMonitor,
        "Backup" => WindowKind::Backup,
        "Registry" => WindowKind::Registry,
        "Pods" => WindowKind::Pods,
        "Kanban" => WindowKind::Kanban,
        "Wallet" => WindowKind::Wallet,
        "Memory" => WindowKind::Memory,
        "Companies" => WindowKind::Companies,
        "Matrix" => WindowKind::Matrix,
        "Configuration" => WindowKind::Configuration,
        "Sidebar" => WindowKind::Sidebar,
        "Curator" => WindowKind::Curator,
        "Terminal" => WindowKind::Terminal,
        "Editor" => WindowKind::Editor,
        "Training" => WindowKind::Training,
        "Media" => WindowKind::Media,
        "Skills" => WindowKind::Skills,
        "hKask" => WindowKind::Logo,
        _ => WindowKind::Chat,
    }
}

/// Path to the layout file for a given agent.
pub fn layout_path(agent_name: &str) -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("hkask")
        .join("agents")
        .join(sanitize(agent_name));
    base.join("tui_layout.json")
}

/// Sanitize an agent name for use in a directory path.
fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Load a saved layout from disk. Returns None if no saved layout exists.
pub fn load(path: &PathBuf) -> Option<SavedLayout> {
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// Save a layout to disk.
pub fn save(path: &PathBuf, layout: &SavedLayout) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(layout)?;
    fs::write(path, data)
}
