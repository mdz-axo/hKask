//! WalletEnergyEstimator — Composite estimator with rJoule conversion awareness.
//!
//! Wraps the existing `CompositeEnergyEstimator` and adds `gas_per_rjoule`
//! for use by `WalletBackedBudget`. The estimator itself produces gas costs;
//! conversion to rJoules happens at reserve/settle time in `WalletBackedBudget`.

use crate::composite_energy_estimator::CompositeEnergyEstimator;
use crate::governed_tool::EnergyEstimator;
use serde_json::Value;

/// Energy estimator that wraps `CompositeEnergyEstimator` and carries
/// the gas→rJoule conversion rate for wallet-backed budgets.
///
/// The estimator produces dimensionless gas costs (same as the standard
/// estimator). The `gas_per_rjoule` field is consumed by `WalletBackedBudget`
/// at reserve/settle time to convert gas to rJoules for wallet debiting.
pub struct WalletEnergyEstimator {
    /// The underlying composite estimator (inference → token-based, others → table).
    inner: CompositeEnergyEstimator,
    /// Conversion rate: how many gas units equal 1 rJoule.
    /// Default: 1000 gas = 1 rJ.
    pub gas_per_rjoule: u64,
}

impl WalletEnergyEstimator {
    /// Create a new WalletEnergyEstimator with the given conversion rate.
    pub fn new(gas_per_rjoule: u64) -> Self {
        Self {
            inner: CompositeEnergyEstimator::new(),
            gas_per_rjoule,
        }
    }
}

impl EnergyEstimator for WalletEnergyEstimator {
    fn estimate_cost(&self, server: &str, tool: &str, args: &Value) -> u64 {
        self.inner.estimate_cost(server, tool, args)
    }
}
