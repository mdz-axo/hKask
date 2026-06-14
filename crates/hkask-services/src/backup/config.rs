//! Backup configuration — what to track, retention policy, auto-snapshot behavior.
//! # REQ: P1 (User Sovereignty) — user controls what is tracked and for how long.

use serde::{Deserialize, Serialize};

use super::scope::ArtifactType;

/// Configuration for the backup system.
///
/// Stored at `~/.config/hkask/backup.json`. Every field survives the
/// essentialist deletion test: delete it and backup behavior degrades.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Which artifact types to track (empty = track nothing).
    #[serde(default)]
    pub tracked_types: Vec<ArtifactType>,

    /// Retention policy (None = keep forever).
    #[serde(default)]
    pub retention: Option<RetentionPolicy>,

    /// Auto-snapshot on artifact mutation? (default: true)
    #[serde(default = "default_auto_snapshot")]
    pub auto_snapshot: bool,

    /// Verify integrity after each snapshot? (default: false — lazy)
    #[serde(default)]
    pub verify_after_snapshot: bool,
}

fn default_auto_snapshot() -> bool {
    true
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            tracked_types: Vec::new(),
            retention: None,
            auto_snapshot: true,
            verify_after_snapshot: false,
        }
    }
}

/// Retention policy for backup snapshots.
///
/// Controls when snapshots are eligible for pruning. Git's append-only
/// nature means pruning requires history rewriting — the policy defines
/// what SHOULD be kept, and the prune operation enforces it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Keep snapshots for this many seconds.
    pub max_age_secs: u64,

    /// Always keep at least this many recent snapshots (regardless of age).
    #[serde(default)]
    pub min_keep: usize,
}

impl RetentionPolicy {
    /// Create a retention policy from a human-readable duration string.
    ///
    /// Supported suffixes: `d` (days), `h` (hours), `m` (minutes).
    /// Example: `"30d"` → 30 days retention.
    pub fn from_duration_str(s: &str) -> Result<Self, String> {
        let (value, unit) = split_duration(s)?;
        let secs = match unit {
            "d" => value * 86400,
            "h" => value * 3600,
            "m" => value * 60,
            _ => return Err(format!("Unknown duration unit: '{unit}'")),
        };
        Ok(Self {
            max_age_secs: secs,
            min_keep: 1,
        })
    }
}

fn split_duration(s: &str) -> Result<(u64, &str), String> {
    let split_at = s
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| format!("Duration '{s}' has no unit suffix (d, h, m)"))?;
    let value: u64 = s[..split_at]
        .parse()
        .map_err(|e| format!("Invalid duration value in '{s}': {e}"))?;
    Ok((value, &s[split_at..]))
}

/// Path to the backup configuration file.
pub fn backup_config_path() -> std::path::PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("hkask").join("backup.json")
}

/// Load backup config from disk, falling back to defaults if the file
/// doesn't exist or is unreadable.
pub fn load_backup_config() -> BackupConfig {
    let path = backup_config_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => BackupConfig::default(),
    }
}

/// Persist backup config to disk.
pub fn save_backup_config(config: &BackupConfig) -> Result<(), std::io::Error> {
    let path = backup_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(&path, json)
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: BACKUP-CONFIG-001 — Default config tracks nothing, keeps forever
    #[test]
    fn default_config_tracks_nothing() {
        let config = BackupConfig::default();
        assert!(config.tracked_types.is_empty());
        assert!(config.retention.is_none());
        assert!(config.auto_snapshot);
        assert!(!config.verify_after_snapshot);
    }

    // REQ: BACKUP-CONFIG-002 — RetentionPolicy parses duration strings
    #[test]
    fn retention_policy_parses_durations() {
        let p = RetentionPolicy::from_duration_str("30d").unwrap();
        assert_eq!(p.max_age_secs, 30 * 86400);
        assert_eq!(p.min_keep, 1);

        let p = RetentionPolicy::from_duration_str("24h").unwrap();
        assert_eq!(p.max_age_secs, 24 * 3600);

        let p = RetentionPolicy::from_duration_str("60m").unwrap();
        assert_eq!(p.max_age_secs, 60 * 60);
    }

    // REQ: BACKUP-CONFIG-003 — Invalid duration strings error
    #[test]
    fn invalid_duration_errors() {
        assert!(RetentionPolicy::from_duration_str("abc").is_err());
        assert!(RetentionPolicy::from_duration_str("30x").is_err());
        assert!(RetentionPolicy::from_duration_str("").is_err());
    }
}
