//! Well — Gas/rJoule source for the hKask installation.
//!
//! A Well produces gas and rJoule on a schedule. One default Well per installation.
//! Wells are the sole source of new gas/rJoule entering the system.
//! Agents draw from Wells to fill their wallets.

use crate::GasError;
use crate::energy::GasCost;
use serde::{Deserialize, Serialize};

/// Configuration for a gas/rJoule Well.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WellConfig {
    /// Unique well identifier within this installation
    pub well_id: String,
    /// Gas produced per replenishment cycle
    pub gas_rate: GasCost,
    /// rJoule produced per replenishment cycle
    pub rjoule_rate: u64,
}

/// Unique identifier for a Well.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WellID(pub u64);

/// Current state of a Well.
#[derive(Debug, Clone)]
pub struct Well {
    pub config: WellConfig,
    /// Current available gas in the Well
    pub gas_available: GasCost,
    /// Current available rJoule in the Well
    pub rjoule_available: u64,
}

impl Well {
    pub fn new(config: WellConfig) -> Self {
        Self {
            config,
            gas_available: GasCost(0),
            rjoule_available: 0,
        }
    }

    /// Replenish the Well — adds gas and rJoule at the configured rates.
    pub fn replenish(&mut self) {
        self.gas_available = GasCost(self.gas_available.0.saturating_add(self.config.gas_rate.0));
        self.rjoule_available = self
            .rjoule_available
            .saturating_add(self.config.rjoule_rate);
    }

    /// Check whether the Well can supply the requested amounts.
    pub fn can_supply(&self, gas: GasCost, rjoule: u64) -> bool {
        self.gas_available.0 >= gas.0 && self.rjoule_available >= rjoule
    }

    /// Draw gas and rJoule from the Well. Returns amounts actually drawn.
    pub fn draw(&mut self, gas: GasCost, rjoule: u64) -> (GasCost, u64) {
        let gas_drawn = gas.0.min(self.gas_available.0);
        let rjoule_drawn = rjoule.min(self.rjoule_available);
        self.gas_available = GasCost(self.gas_available.0.saturating_sub(gas_drawn));
        self.rjoule_available = self.rjoule_available.saturating_sub(rjoule_drawn);
        (GasCost(gas_drawn), rjoule_drawn)
    }

    pub fn is_exhausted(&self) -> bool {
        self.gas_available.0 == 0 && self.rjoule_available == 0
    }
}

/// Manages Wells — creation, replenishment, drawing.
pub struct WellManager {
    wells: std::collections::HashMap<WellID, Well>,
    next_id: u64,
    /// ID of the default Well (agents auto-draw from this one)
    default_well: Option<WellID>,
}

impl WellManager {
    pub fn new() -> Self {
        Self {
            wells: std::collections::HashMap::new(),
            next_id: 1,
            default_well: None,
        }
    }

    /// Create a new Well. If it's the first Well, it becomes the default.
    pub fn create_well(&mut self, config: WellConfig) -> (WellID, bool) {
        let id = WellID(self.next_id);
        self.next_id += 1;
        let is_default = self.default_well.is_none();
        if is_default {
            self.default_well = Some(id);
        }
        self.wells.insert(id, Well::new(config));
        (id, is_default)
    }

    /// Replenish all Wells.
    pub fn replenish_all(&mut self) {
        for well in self.wells.values_mut() {
            well.replenish();
        }
    }

    /// Draw from a specific Well.
    pub fn draw(
        &mut self,
        well_id: WellID,
        gas: GasCost,
        rjoule: u64,
    ) -> Result<(GasCost, u64), GasError> {
        let well = self
            .wells
            .get_mut(&well_id)
            .ok_or_else(|| GasError::Persistence(format!("Well not found: {:?}", well_id)))?;
        let (gas_drawn, rjoule_drawn) = well.draw(gas, rjoule);
        Ok((gas_drawn, rjoule_drawn))
    }

    /// Draw from the default Well.
    pub fn draw_from_default(
        &mut self,
        gas: GasCost,
        rjoule: u64,
    ) -> Result<(GasCost, u64), GasError> {
        let default_id = self
            .default_well
            .ok_or_else(|| GasError::Persistence("No default Well configured".into()))?;
        self.draw(default_id, gas, rjoule)
    }

    /// Check if the default Well is exhausted.
    pub fn default_well_exhausted(&self) -> bool {
        self.default_well
            .and_then(|id| self.wells.get(&id))
            .map(|w| w.is_exhausted())
            .unwrap_or(false)
    }

    /// Get the default Well ID.
    pub fn default_well_id(&self) -> Option<WellID> {
        self.default_well
    }
}

impl Default for WellManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_replenish_adds_gas_and_rjoule() {
        let config = WellConfig {
            well_id: "test".into(),
            gas_rate: GasCost(100),
            rjoule_rate: 50,
        };
        let mut well = Well::new(config);
        assert_eq!(well.gas_available.0, 0);
        well.replenish();
        assert_eq!(well.gas_available.0, 100);
        assert_eq!(well.rjoule_available, 50);
    }

    #[test]
    fn well_draw_respects_available() {
        let config = WellConfig {
            well_id: "test".into(),
            gas_rate: GasCost(1000),
            rjoule_rate: 500,
        };
        let mut well = Well::new(config);
        well.replenish();
        let (gas, rj) = well.draw(GasCost(50), 30);
        assert_eq!(gas.0, 50);
        assert_eq!(rj, 30);
        assert_eq!(well.gas_available.0, 950);
    }

    #[test]
    fn well_draw_partial_when_insufficient() {
        let config = WellConfig {
            well_id: "test".into(),
            gas_rate: GasCost(10),
            rjoule_rate: 0,
        };
        let mut well = Well::new(config);
        well.replenish();
        let (gas, rj) = well.draw(GasCost(100), 0);
        assert_eq!(gas.0, 10); // only 10 available
        assert_eq!(rj, 0);
        assert_eq!(well.gas_available.0, 0);
    }

    #[test]
    fn well_exhausted_when_both_zero() {
        let config = WellConfig {
            well_id: "test".into(),
            gas_rate: GasCost(0),
            rjoule_rate: 0,
        };
        let well = Well::new(config);
        assert!(well.is_exhausted());
    }

    #[test]
    fn first_well_becomes_default() {
        let mut mgr = WellManager::new();
        let (id, is_default) = mgr.create_well(WellConfig {
            well_id: "primary".into(),
            gas_rate: GasCost(1000),
            rjoule_rate: 100,
        });
        assert!(is_default);
        assert_eq!(mgr.default_well, Some(id));
    }
}
