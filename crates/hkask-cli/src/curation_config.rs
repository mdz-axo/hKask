//! Curation config loading — I/O layer for CurationThresholdConfig
//!
//! These loader functions read YAML config files and env vars to populate
//! `CurationThresholdConfig`. They were relocated from `hkask-cns` because
//! config loading is I/O, not cybernetic regulation (Loop 6).
//!
//! The data type (`CurationThresholdConfig`) lives in `hkask_types::curation`.

use hkask_types::curation::CurationThresholdConfig;

/// Load curation thresholds from a YAML string.
pub fn curation_threshold_from_yaml(
    yaml: &str,
) -> Result<CurationThresholdConfig, serde_yaml::Error> {
    serde_yaml::from_str(yaml)
}

/// Load curation thresholds from a YAML file.
pub fn curation_threshold_from_file(path: &str) -> Result<CurationThresholdConfig, std::io::Error> {
    let contents = std::fs::read_to_string(path)?;
    curation_threshold_from_yaml(&contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Load curation thresholds from `HKASK_CNS_CONFIG` env var, falling back to defaults.
///
/// If `HKASK_CNS_CONFIG` is set, reads the YAML file at that path.
/// If unset or the file doesn't exist, returns default thresholds.
pub fn load_curation_thresholds() -> CurationThresholdConfig {
    match std::env::var("HKASK_CNS_CONFIG") {
        Ok(path) => match curation_threshold_from_file(&path) {
            Ok(config) => {
                tracing::info!(
                    target: "cns.config",
                    path = %path,
                    coherence_threshold = config.coherence_threshold,
                    drift_threshold = config.drift_threshold,
                    "Loaded CNS curation thresholds from config file"
                );
                config
            }
            Err(e) => {
                tracing::warn!(
                    target: "cns.config",
                    path = %path,
                    error = %e,
                    "Failed to load CNS config file for curation thresholds, using defaults"
                );
                CurationThresholdConfig::default()
            }
        },
        Err(_) => {
            let defaults = CurationThresholdConfig::default();
            tracing::info!(
                target: "cns.config",
                coherence_threshold = defaults.coherence_threshold,
                drift_threshold = defaults.drift_threshold,
                "HKASK_CNS_CONFIG not set, using default curation thresholds"
            );
            defaults
        }
    }
}
