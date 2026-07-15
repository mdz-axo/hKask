//! Scenario data bridge — event trees, forecasts, calibration state.

use serde::{Deserialize, Serialize};

/// Summary of a scenario forecast for display in the TUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioForecastSummary {
    pub forecast_id: String,
    pub event_id: String,
    pub event_name: String,
    pub subject: String,
    pub probability: f64,
    pub created_at: String,
    pub outcome: Option<bool>,
}

/// Pipeline state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioPipelineState {
    pub forecast_count: usize,
    pub resolved_count: usize,
    pub pending_count: usize,
    pub overall_brier: Option<f64>,
    pub recent_forecasts: Vec<ScenarioForecastSummary>,
}

/// Calibration curve summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationSummary {
    pub total_forecasts: usize,
    pub resolved_forecasts: usize,
    pub overall_brier: Option<f64>,
    pub overconfidence_score: Option<f64>,
    pub interpretation: String,
}

/// A node in the event tree with dependency children.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventNode {
    pub id: String,
    pub name: String,
    pub question: String,
    pub probability: f64,
    pub certainty_tier: String,
    pub basis: Option<String>,
    pub marginal_probability: Option<f64>,
    pub parent_ids: Vec<String>,
    pub children: Vec<EventNode>,
    pub sub_question_count: usize,
    pub has_base_rate: bool,
    pub brier_score: Option<f64>,
}

/// Full event tree with resolved nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTreeDetail {
    pub subject: String,
    pub time_horizon: String,
    pub event_count: usize,
    pub all_events_probability: f64,
    pub root_nodes: Vec<EventNode>,
}

/// Bridge trait for scenario data.
pub trait ScenariosDataBridge: Send + Sync {
    /// Current pipeline state.
    fn pipeline_state(&self) -> Option<ScenarioPipelineState>;
    /// Calibration curve.
    fn calibration(&self) -> Option<CalibrationSummary>;
    /// Full event tree with resolved nodes and children.
    fn event_tree(&self) -> Option<EventTreeDetail>;
}

/// Mock bridge for testing and disconnected state.
pub struct MockScenariosBridge {
    pub pipeline: Option<ScenarioPipelineState>,
    pub calibration: Option<CalibrationSummary>,
    pub tree: Option<EventTreeDetail>,
}

impl ScenariosDataBridge for MockScenariosBridge {
    fn pipeline_state(&self) -> Option<ScenarioPipelineState> {
        self.pipeline.clone()
    }
    fn calibration(&self) -> Option<CalibrationSummary> {
        self.calibration.clone()
    }
    fn event_tree(&self) -> Option<EventTreeDetail> {
        self.tree.clone()
    }
}
