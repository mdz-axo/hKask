//! CompositeGasEstimator — Routes inference tools to InferenceGasEstimator,
//! all other tools to TableGasEstimator.

use crate::governed_tool::GasEstimator;
use crate::inference_estimator::InferenceGasEstimator;
use crate::table_gas_estimator::TableGasEstimator;
use serde_json::Value;

/// Composite gas estimator that routes inference tools to InferenceGasEstimator
/// and all other tools to TableGasEstimator.
///
/// This is the production estimator — it should be the default for all
/// GovernedTool instances. Inference calls (server == "hkask-mcp-inference")
/// use token-based estimation; everything else uses the per-server table.
pub struct CompositeGasEstimator {
    inference: InferenceGasEstimator,
    table: TableGasEstimator,
}

impl CompositeGasEstimator {
    /// Create a new CompositeGasEstimator with default table costs.
    pub fn new() -> Self {
        Self {
            inference: InferenceGasEstimator,
            table: TableGasEstimator::new(),
        }
    }

    /// The inference server identifier used for routing.
    pub const INFERENCE_SERVER: &'static str = "hkask-mcp-inference";
}

impl Default for CompositeGasEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl GasEstimator for CompositeGasEstimator {
    fn estimate_cost(&self, server: &str, tool: &str, args: &Value) -> u64 {
        if server == Self::INFERENCE_SERVER {
            self.inference.estimate_cost(server, tool, args)
        } else {
            self.table.estimate_cost(server, tool, args)
        }
    }
}
