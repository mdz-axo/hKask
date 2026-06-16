//! CompositeEnergyEstimator — Routes inference tools to InferenceEnergyEstimator,
//! all other tools to TableEnergyEstimator.

use crate::governed_tool::EnergyEstimator;
use crate::inference_estimator::InferenceEnergyEstimator;
use crate::table_energy_estimator::TableEnergyEstimator;
use serde_json::Value;

/// Composite gas estimator that routes inference tools to InferenceEnergyEstimator
/// and all other tools to TableEnergyEstimator.
///
/// [NORMATIVE] This is the production estimator — it should be the default for all (P9 — Homeostatic Self-Regulation).
/// GovernedTool instances. Inference calls use token-based estimation;
/// everything else uses the per-server table.
pub struct CompositeEnergyEstimator {
    inference: InferenceEnergyEstimator,
    table: TableEnergyEstimator,
}

impl CompositeEnergyEstimator {
    /// Create a new CompositeEnergyEstimator with default table costs.
    ///
    /// REQ: P9-cns-est-composite-new
    /// [P9] Motivating: Homeostatic Self-Regulation — composite estimator enables feedback loops
    /// [P5] Constraining: Essentialism — minimal constructor, empty estimators
    /// post: returns CompositeEnergyEstimator with empty estimators
    pub fn new() -> Self {
        Self {
            inference: InferenceEnergyEstimator,
            table: TableEnergyEstimator::new(),
        }
    }

    /// The inference routing key used for energy estimation.
    ///
    /// Inference is no longer an MCP server — it's a direct internal
    /// call through `InferencePort`, not MCP dispatch. This key remains
    /// for energy estimation routing when a GovernedTool wraps an
    /// inference-like tool port.
    pub const INFERENCE_SERVER: &'static str = "inference";
}

impl Default for CompositeEnergyEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl EnergyEstimator for CompositeEnergyEstimator {
    fn estimate_cost(&self, server: &str, tool: &str, args: &Value) -> u64 {
        if server == Self::INFERENCE_SERVER {
            self.inference.estimate_cost(server, tool, args)
        } else {
            self.table.estimate_cost(server, tool, args)
        }
    }
}
