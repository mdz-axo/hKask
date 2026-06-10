//! Shared settings path utility — single source of truth for the settings file
//! location used by CLI, API, and REPL surfaces. Magna Carta P3: all surfaces
//! read/write the same `~/.config/hkask/settings.json`.

/// Returns the canonical path to `~/.config/hkask/settings.json`,
/// creating the parent directory if needed.
pub fn settings_path() -> std::path::PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("hkask");
    let _ = std::fs::create_dir_all(&path);
    path.push("settings.json");
    path
}
