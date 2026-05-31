//! Loop 1: Inference — Capability handles
//!
//! The Inference loop governs the path from prompt to response:
//! prompt → context → model → response → parse → act
//!
//! Subloops:
//! - 1.1 Context Assembly (FILTER)
//! - 1.2 Prompt Cache (CACHE)
//! - 1.3 Circuit Breaker (CIRCUIT)
//! - 1.4 Energy Budget (GUARD)

use crate::id::WebID;

/// Inference loop capability handle.
pub struct InferenceHandle {
    agent_webid: WebID,
    energy_remaining: u64,
    circuit_open: bool,
}

impl InferenceHandle {
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            agent_webid: WebID::new(),
            energy_remaining: u64::MAX,
            circuit_open: false,
        }
    }

    pub fn new(agent_webid: WebID, energy_remaining: u64, circuit_open: bool) -> Self {
        Self {
            agent_webid,
            energy_remaining,
            circuit_open,
        }
    }

    pub fn agent(&self) -> &WebID {
        &self.agent_webid
    }

    pub fn energy_remaining(&self) -> u64 {
        self.energy_remaining
    }

    pub fn is_circuit_open(&self) -> bool {
        self.circuit_open
    }

    pub fn consume_energy(&mut self, amount: u64) -> Result<(), InferenceBudgetExceeded> {
        if amount > self.energy_remaining {
            return Err(InferenceBudgetExceeded {
                requested: amount,
                remaining: self.energy_remaining,
            });
        }
        self.energy_remaining -= amount;
        Ok(())
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("energy budget exceeded: requested {requested}, remaining {remaining}")]
pub struct InferenceBudgetExceeded {
    pub requested: u64,
    pub remaining: u64,
}

/// Energy budget capability handle.
pub struct EnergyBudgetHandle {
    remaining: u64,
    cap: u64,
}

impl EnergyBudgetHandle {
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            remaining: u64::MAX,
            cap: u64::MAX,
        }
    }

    pub fn new(remaining: u64, cap: u64) -> Self {
        Self { remaining, cap }
    }

    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    pub fn cap(&self) -> u64 {
        self.cap
    }

    pub fn usage_ratio(&self) -> f64 {
        if self.cap == 0 {
            return 0.0;
        }
        self.remaining as f64 / self.cap as f64
    }

    pub fn consume(&mut self, amount: u64) -> Result<(), InferenceBudgetExceeded> {
        if amount > self.remaining {
            return Err(InferenceBudgetExceeded {
                requested: amount,
                remaining: self.remaining,
            });
        }
        self.remaining -= amount;
        Ok(())
    }
}
