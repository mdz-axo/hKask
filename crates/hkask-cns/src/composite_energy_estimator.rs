//! CompositeEnergyEstimator — Routes inference tools to InferenceEnergyEstimator,
//! all other tools to TableEnergyEstimator.

use crate::governed_tool::EnergyEstimator;
use crate::inference_estimator::InferenceEnergyEstimator;
use crate::table_energy_estimator::TableEnergyEstimator;
use serde_json::Value;

/// Composite gas estimator that routes inference tools to InferenceEnergyEstimator
/// and all other tools to TableEnergyEstimator.
///
/// This is the production estimator — it should be the default for all
/// GovernedTool instances. Inference calls use token-based estimation;
/// everything else uses the per-server table.
pub struct CompositeEnergyEstimator {
    inference: InferenceEnergyEstimator,
    table: TableEnergyEstimator,
}

impl CompositeEnergyEstimator {
    /// Create a new CompositeEnergyEstimator with default table costs.
    pub fn new() -> Self {
        Self {
            inference: InferenceEnergyEstimator,
            table: TableEnergyEstimator::new(),
        }
    }

    /// The inference routing key used for energy estimation.
    pub const INFERENCE_SERVER: &'static str = "hkask-mcp-inference";
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
