//! Shared settings path utility — single source of truth for the settings file
//! location used by CLI, API, and REPL surfaces. Magna Carta P3: all surfaces
//! read/write the same `~/.config/hkask/settings.json`.
//!
//! Also provides `HkaskSettings` for model defaults shared across all servers.

use serde::{Deserialize, Serialize};

/// Returns the canonical path to `~/.config/hkask/settings.json`,
/// creating the parent directory if needed.
pub fn settings_path() -> std::path::PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("hkask");
    let _ = std::fs::create_dir_all(&path);
    path.push("settings.json");
    path
}

/// System-wide model defaults persisted to `~/.config/hkask/settings.json`.
/// Shared across CLI, API, REPL, and all MCP servers.
/// Priority: env var > settings.json > hardcoded default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HkaskSettings {
    /// Default generation model for prose composition (replica_compose, spec_replica_rewrite).
    /// Override: `HKASK_REPLICA_MODEL` env var.
    #[serde(default = "default_generation_model")]
    pub generation_model: String,

    /// Default embedding model for vectorization.
    /// Override: `HKASK_EMBEDDING_MODEL` env var.
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,

    /// Default classifier model for section type and triple extraction.
    /// Override: `HKASK_CLASSIFIER_MODEL` env var.
    #[serde(default = "default_classifier_model")]
    pub classifier_model: String,

    /// Default OCR model for scanned PDF fallback.
    /// Override: `HKASK_OCR_MODEL` env var.
    #[serde(default = "default_ocr_model")]
    pub ocr_model: String,

    /// Communication server 7R7 moderation polling interval in seconds.
    /// How often the 7R7 bot checks Matrix rooms for content to moderate.
    /// Override: `HKASK_COMMUNICATION_POLL_INTERVAL_SECS` env var.
    #[serde(default = "default_poll_interval")]
    pub communication_poll_interval_secs: u64,
}

fn default_generation_model() -> String {
    "deepseek-v4-flash:cloud".to_string()
}

fn default_embedding_model() -> String {
    "DI/Qwen/Qwen3-Embedding-0.6B".to_string()
}

fn default_classifier_model() -> String {
    "google/gemma-4-26B-A4B-it".to_string()
}

fn default_ocr_model() -> String {
    "maternion/LightOnOCR-2:1b".to_string()
}

fn default_poll_interval() -> u64 {
    60
}

impl Default for HkaskSettings {
    fn default() -> Self {
        Self {
            generation_model: default_generation_model(),
            embedding_model: default_embedding_model(),
            classifier_model: default_classifier_model(),
            ocr_model: default_ocr_model(),
            communication_poll_interval_secs: default_poll_interval(),
        }
    }
}

impl HkaskSettings {
    /// Load settings from `~/.config/hkask/settings.json`.
    /// Falls back to defaults if the file doesn't exist or is unreadable.
    pub fn load() -> Self {
        let path = settings_path();
        match std::fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_else(|e| {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "Failed to parse settings.json — using defaults"
                );
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Resolve the effective model, preferring env var over settings over default.
    pub fn resolve_model(env_var: &str, settings_value: &str, default: &str) -> String {
        std::env::var(env_var)
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                if settings_value.is_empty() {
                    default.to_string()
                } else {
                    settings_value.to_string()
                }
            })
    }

    /// Resolve the generation model with env/settings/default priority.
    pub fn generation_model(&self) -> String {
        Self::resolve_model(
            "HKASK_REPLICA_MODEL",
            &self.generation_model,
            &default_generation_model(),
        )
    }

    /// Resolve the embedding model with env/settings/default priority.
    pub fn embedding_model(&self) -> String {
        Self::resolve_model(
            "HKASK_EMBEDDING_MODEL",
            &self.embedding_model,
            &default_embedding_model(),
        )
    }

    /// Resolve the classifier model with env/settings/default priority.
    pub fn classifier_model(&self) -> String {
        Self::resolve_model(
            "HKASK_CLASSIFIER_MODEL",
            &self.classifier_model,
            &default_classifier_model(),
        )
    }

    /// Resolve the OCR model with env/settings/default priority.
    pub fn ocr_model(&self) -> String {
        Self::resolve_model("HKASK_OCR_MODEL", &self.ocr_model, &default_ocr_model())
    }

    /// Save settings to `~/.config/hkask/settings.json`.
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = settings_path();
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)
    }
}
