//! Set-points and configuration for the Cybernetics Loop.
//!
//! Homeostatic set-points define the reference values against which sensed
//! signals are compared. When a signal deviates beyond its set-point,
//! the loop produces an efferent action.

use hkask_types::cns::QueueDepth;

/// Default minimum energy budget remaining ratio (20%).
///
/// When gas remaining drops below this ratio, the Cybernetics Loop produces
/// a throttle action to reduce consumption.
pub const DEFAULT_ENERGY_MIN_REMAINING_RATIO: f64 = 0.2;

/// Default maximum variety deficit before escalation (100).
///
/// When variety deficit exceeds this value, an algedonic alert is triggered.
pub const DEFAULT_VARIETY_MAX_DEFICIT: f64 = 100.0;

/// Default maximum error rate (30%).
///
/// When the error rate exceeds this ratio, the Cybernetics Loop produces
/// a calibration action.
pub const DEFAULT_ERROR_RATE_MAX: f64 = 0.3;

/// Default maximum connector latency in seconds.
///
/// When connector latency exceeds this threshold, the Cybernetics Loop
/// produces a throttle action.
pub const DEFAULT_CONNECTOR_LATENCY_MAX_SECS: f64 = 30.0;

/// Default communication queue depth threshold for backpressure regulation.
///
/// When the Communication Loop's queue depth exceeds this value,
/// the Cybernetics Loop produces a Throttle(Communication) action.
pub const DEFAULT_COMMUNICATION_BACKPRESSURE_THRESHOLD: QueueDepth =
    QueueDepth::DEFAULT_BACKPRESSURE;

/// Default minimum seam coverage ratio before alert.
///
/// When per-crate coverage drops below its previous snapshot value,
/// R7.3 fires an algedonic alert. Default: 0.0 (alert on ANY regression —
/// [NORMATIVE] coverage should never go down). (P9 — Homeostatic Self-Regulation).
pub const DEFAULT_SEAM_COVERAGE_MIN: f64 = 0.0;

/// Default maximum number of regulation iterations per cycle.
///
/// Prevents unbounded cascading in the compute→act pipeline.
pub const DEFAULT_MAX_ITERATIONS: u32 = 100;

/// Homeostatic set-points for the Cybernetics Loop.
///
/// These define the reference values against which sensed signals
/// are compared. When a signal deviates beyond its set-point,
/// the loop produces an efferent action.
#[derive(Debug, Clone)]
pub struct SetPoints {
    /// Minimum energy budget remaining ratio (0.0-1.0). Default: 0.2 (20% remaining)
    pub gas_min_remaining: f64,
    /// Maximum variety deficit before escalation. Default: 100
    pub variety_max_deficit: f64,
    /// Maximum error rate (0.0-1.0). Default: 0.3 (30% errors)
    pub error_rate_max: f64,
    /// Maximum connector latency in seconds. Default: 30.0
    pub connector_latency_max_secs: f64,
    /// Communication queue depth threshold for backpressure regulation.
    /// When the Communication Loop's queue depth exceeds this value,
    /// CyberneticsLoop produces a Throttle(Communication) action.
    /// Default: 100 messages
    pub communication_backpressure_threshold: QueueDepth,
    /// Minimum seam coverage ratio per crate before R7.3 alert.
    /// When per-crate coverage drops below its previous snapshot,
    /// an algedonic alert fires. Default: 0.0 (any regression alerts).
    pub seam_coverage_min: f64,
}

/// Configurable thresholds for Curation decisions (spec coherence, drift).
/// Loaded from YAML via `HKASK_CNS_CONFIG` (same pattern as `SetPointsConfig`).
///
/// Type definition lives in `hkask_types::curation`; YAML loading was
/// in the now-deleted `hkask_cli::curation_config` (duplicate, zero callers).
pub use hkask_types::curation::CurationThresholdConfig;

/// YAML-configurable set-points. Fields are Optional so partial configs work.
/// Missing fields fall back to the `SetPoints::default()` values.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SetPointsConfig {
    pub gas_min_remaining: Option<f64>,
    pub variety_max_deficit: Option<f64>,
    pub error_rate_max: Option<f64>,
    pub connector_latency_max_secs: Option<f64>,
    pub communication_backpressure_threshold: Option<QueueDepth>,
    pub seam_coverage_min: Option<f64>,
}

impl SetPointsConfig {
    /// Load set-points from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }

    /// Load set-points from a YAML file.
    pub fn load_from_file(path: &str) -> Result<Self, std::io::Error> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_yaml(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl Default for SetPoints {
    fn default() -> Self {
        Self {
            gas_min_remaining: DEFAULT_ENERGY_MIN_REMAINING_RATIO,
            variety_max_deficit: DEFAULT_VARIETY_MAX_DEFICIT,
            error_rate_max: DEFAULT_ERROR_RATE_MAX,
            connector_latency_max_secs: DEFAULT_CONNECTOR_LATENCY_MAX_SECS,
            communication_backpressure_threshold: DEFAULT_COMMUNICATION_BACKPRESSURE_THRESHOLD,
            seam_coverage_min: DEFAULT_SEAM_COVERAGE_MIN,
        }
    }
}

impl SetPoints {
    /// Create SetPoints from a config, using defaults for missing fields.
    pub fn from_config(config: &SetPointsConfig) -> Self {
        let defaults = SetPoints::default();
        Self {
            gas_min_remaining: config
                .gas_min_remaining
                .unwrap_or(defaults.gas_min_remaining),
            variety_max_deficit: config
                .variety_max_deficit
                .unwrap_or(defaults.variety_max_deficit),
            error_rate_max: config.error_rate_max.unwrap_or(defaults.error_rate_max),
            connector_latency_max_secs: config
                .connector_latency_max_secs
                .unwrap_or(defaults.connector_latency_max_secs),
            communication_backpressure_threshold: config
                .communication_backpressure_threshold
                .unwrap_or(defaults.communication_backpressure_threshold),
            seam_coverage_min: config
                .seam_coverage_min
                .unwrap_or(defaults.seam_coverage_min),
        }
    }
}

/// Load set-points from `HKASK_CNS_CONFIG` env var, falling back to defaults.
///
/// If `HKASK_CNS_CONFIG` is set, reads the YAML file at that path.
/// If unset or the file doesn't exist, returns default set-points.
pub fn load_set_points() -> SetPoints {
    match std::env::var("HKASK_CNS_CONFIG") {
        Ok(path) => match SetPointsConfig::load_from_file(&path) {
            Ok(config) => {
                tracing::info!(
                    target: "cns.config()",
                    path = %path,
                    "Loaded CNS set-points from config file"
                );
                SetPoints::from_config(&config)
            }
            Err(e) => {
                tracing::warn!(
                    target: "cns.config()",
                    path = %path,
                    error = %e,
                    "Failed to load CNS config file, using defaults"
                );
                SetPoints::default()
            }
        },
        Err(_) => SetPoints::default(),
    }
}
