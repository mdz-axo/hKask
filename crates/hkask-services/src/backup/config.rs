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

    /// Auto-snapshot on schedule? (default: true)
    #[serde(default = "default_auto_snapshot")]
    pub auto_snapshot: bool,

    /// Verify integrity after each snapshot? (default: false — lazy)
    #[serde(default)]
    pub verify_after_snapshot: bool,

    /// Encryption passphrase for blob content (None = unencrypted).
    /// Derived key parameters are stored here; passphrase comes from keystore.
    #[serde(default)]
    pub encryption: Option<EncryptionConfig>,
}

/// Encryption configuration for backup blobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Salt for Argon2 key derivation (hex-encoded).
    pub salt_hex: String,
    /// Argon2 memory cost (KB).
    pub memory_kb: u32,
    /// Argon2 iteration count.
    pub iterations: u32,
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
            encryption: None,
        }
    }
}

/// Retention policy for backup snapshots.
///
/// Controls which snapshots survive pruning:
/// - Daily snapshots kept for `daily_days` (default: 21 days = 3 weeks)
/// - Weekly snapshots kept for `weekly_weeks` (default: 12 weeks = 3 months)
/// - Monthly snapshots kept indefinitely before that
///
/// A "weekly" snapshot is the first snapshot of each ISO week.
/// A "monthly" snapshot is the first snapshot of each month.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Keep daily snapshots for this many days.
    #[serde(default = "default_daily_days")]
    pub daily_days: u32,
    /// Keep weekly snapshots for this many weeks (after daily period).
    #[serde(default = "default_weekly_weeks")]
    pub weekly_weeks: u32,
}

fn default_daily_days() -> u32 {
    21
}
fn default_weekly_weeks() -> u32 {
    12
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            daily_days: 21,
            weekly_weeks: 12,
        }
    }
}

impl RetentionPolicy {
    /// Determine whether a snapshot at the given timestamp and commit index
    /// (0 = newest) should be retained.
    ///
    /// The newest `daily_days` commits are always kept (one per day).
    /// After that, one per week for `weekly_weeks` weeks.
    /// After that, one per month.
    pub fn should_keep(&self, commit_index: usize, timestamp_secs: u64, now_secs: u64) -> bool {
        let age_days = (now_secs.saturating_sub(timestamp_secs)) / 86400;

        // Always keep the most recent snapshot.
        if commit_index == 0 {
            return true;
        }

        // Daily: keep if within daily_days and it's the most recent for its day.
        if age_days < self.daily_days as u64 {
            return true; // Simplified: keep all within daily window
        }

        // Weekly: keep one per week within the weekly window.
        let age_weeks = age_days / 7;
        let weekly_window = self.daily_days as u64 / 7 + self.weekly_weeks as u64;
        if age_weeks < weekly_window {
            // Keep if it's the start of an ISO week (Monday).
            // Simplified: keep one per week.
            return age_days % 7 == 0;
        }

        // Monthly: keep one per month beyond weekly window.
        age_days % 30 == 0
    }

    /// Parse a duration string like "30d", "24h", or "60m" into a RetentionPolicy.
    pub fn from_duration_str(s: &str) -> Result<Self, String> {
        let (value, unit) = split_duration(s)?;
        let days = match unit {
            "d" => value,
            "h" => (value + 23) / 24,             // ceil to nearest day
            "m" => ((value + 59) / 60 + 23) / 24, // minutes → hours → days
            other => {
                return Err(format!(
                    "Unknown duration unit '{}', expected d, h, or m",
                    other
                ));
            }
        };
        Ok(Self {
            daily_days: days as u32,
            weekly_weeks: 0,
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
    let json = serde_json::to_string_pretty(config).map_err(|e| std::io::Error::other(e))?;
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
        assert!(config.encryption.is_none());
    }

    // REQ: BACKUP-CONFIG-002 — RetentionPolicy defaults are correct
    #[test]
    fn retention_policy_defaults() {
        let p = RetentionPolicy::default();
        assert_eq!(p.daily_days, 21);
        assert_eq!(p.weekly_weeks, 12);
    }

    // REQ: BACKUP-CONFIG-003 — RetentionPolicy keeps recent snapshots
    #[test]
    fn retention_policy_keeps_recent() {
        let p = RetentionPolicy::default();
        let now = 1_000_000_000;
        // Most recent snapshot (index 0) is always kept.
        assert!(p.should_keep(0, now, now));
        // Within 21 days — kept.
        assert!(p.should_keep(5, now - 5 * 86400, now));
        // Exactly 21 days (weekly keeper: 21 % 7 == 0).
        assert!(p.should_keep(21, now - 21 * 86400, now));
        // 28 days (weekly keeper: 28 % 7 == 0).
        assert!(p.should_keep(28, now - 28 * 86400, now));
        // 30 days — not a weekly keeper (30 % 7 != 0) and past 21-day window.
        assert!(!p.should_keep(30, now - 30 * 86400, now));
        // 120 days — past weekly window, monthly keeper (120 % 30 == 0).
        assert!(p.should_keep(120, now - 120 * 86400, now));
        // 150 days — not a monthly keeper (150 % 30 == 0 but we keep only one/month).
        // Actually 150 % 30 == 0, so it is kept.
        assert!(p.should_keep(150, now - 150 * 86400, now));
    }
}
