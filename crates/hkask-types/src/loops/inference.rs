//! Loop 1: Inference — Capability handles
//!
//! The Inference loop governs the path from prompt to response:
//! prompt → context → model → response → parse → act
//!
//! Subloops:
//! - 1.1 Context Assembly (FILTER) — filter and compose context from episodic + semantic memory
//! - 1.2 Prompt Cache (CACHE) — hit? → return / miss → compute + store
//! - 1.3 Circuit Breaker (CIRCUIT) — fail → count → threshold → open → half-open → probe → close
//! - 1.4 Energy Budget (GUARD) — request → check remaining budget → allow or deny
//! - 1.5 Rate Limiting (GUARD) — request → check token bucket → allow or deny
//!
//! # Capability Discipline
//!
//! `InferenceHandle` can infer, read episodic + semantic memory, emit spans, check cache,
//! circuit-break, and rate-limit. It CANNOT write memory, reset alerts, process
//! sovereignty, or revoke capabilities.
//!
//! `EnergyBudgetHandle` can check remaining budget, request consumption, and get usage ratio.
//! It CANNOT set the cap, reset the budget, or change the alert threshold.
//!
//! It CANNOT resize the bucket, change the refill rate, or bypass limiting.

use crate::id::WebID;

// =============================================================================
// InferenceHandle — Loop 1 capability handle
// =============================================================================

/// Inference loop capability handle.
///
/// Provides read access to episodic and semantic memory, span emission, prompt cache,
/// circuit breaker status, energy budget checking, and rate limit checking.
///
/// # OCAP Boundaries (Hoare triples: requires → ensures)
///
/// - **CAN** infer (call model with assembled context)
/// - **CAN** read episodic memory (assemble episodic context for prompts)
/// - **CAN** read semantic memory (assemble semantic context for prompts)
/// - **CAN** emit CNS spans (observability)
/// - **CAN** check prompt cache (CACHE subloop)
/// - **CAN** check circuit breaker status (CIRCUIT subloop)
/// - **CAN** check energy budget (GUARD subloop)
/// - **CAN** check rate limit (GUARD subloop)
/// - **CANNOT** write memory (use `EpisodicWriteHandle` / `SemanticWriteHandle`)
/// - **CANNOT** reset alerts (use `CnsAdminHandle`)
/// - **CANNOT** process sovereignty (use `GovernanceHandle`)
/// - **CANNOT** revoke capabilities (use `GovernanceHandle`)
pub struct InferenceHandle {
    /// Agent whose inference this handle authorizes
    agent_webid: WebID,
    /// Remaining energy budget (tokens)
    energy_remaining: u64,
    /// Circuit breaker state: true = open (blocking), false = closed (allowing)
    circuit_open: bool,
}

impl InferenceHandle {
    /// Create a test handle with synthetic values.
    ///
    /// Production handles are created by the inference loop itself, not by external callers.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            agent_webid: WebID::new(),
            energy_remaining: u64::MAX,
            circuit_open: false,
        }
    }

    /// Create an inference handle for a specific agent.
    pub fn new(agent_webid: WebID, energy_remaining: u64, circuit_open: bool) -> Self {
        Self {
            agent_webid,
            energy_remaining,
            circuit_open,
        }
    }

    /// The agent this handle is scoped to.
    pub fn agent(&self) -> &WebID {
        &self.agent_webid
    }

    /// Check remaining energy budget.
    ///
    /// # Ensures
    /// Returns remaining token budget. Does not consume.
    pub fn energy_remaining(&self) -> u64 {
        self.energy_remaining
    }

    /// Check if circuit breaker is open (blocking calls).
    ///
    /// # Ensures
    /// Returns `true` if the circuit is open (calls should be blocked),
    /// `false` if closed (calls are allowed).
    pub fn is_circuit_open(&self) -> bool {
        self.circuit_open
    }

    /// Consume energy from the budget.
    ///
    /// # Requires
    /// - `amount` must not exceed remaining energy budget
    ///
    /// # Ensures
    /// - Decrements `energy_remaining` by `amount`
    /// - Returns `Ok(())` if sufficient budget, `Err` otherwise
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

/// Error returned when energy budget is exceeded.
#[derive(Debug, Clone, thiserror::Error)]
#[error("energy budget exceeded: requested {requested}, remaining {remaining}")]
pub struct InferenceBudgetExceeded {
    pub requested: u64,
    pub remaining: u64,
}

// =============================================================================
// EnergyBudgetHandle — Loop 1, Subloop 1.4 (GUARD)
// =============================================================================

/// Energy budget capability handle.
///
/// Provides read-only access to energy budget status. The budget cap is set
/// by Curation (via `CnsGovernWriteHandle`) and enforced by the Inference loop.
///
/// # OCAP Boundaries
///
/// - **CAN** check remaining budget
/// - **CAN** request consumption (decrement)
/// - **CAN** get usage ratio (remaining / cap)
/// - **CANNOT** set the cap (use `CnsGovernWriteHandle`)
/// - **CANNOT** reset the budget (use `CnsAdminHandle`)
/// - **CANNOT** change the alert threshold (use `CnsGovernWriteHandle`)
pub struct EnergyBudgetHandle {
    /// Current energy remaining
    remaining: u64,
    /// Maximum energy cap
    cap: u64,
}

impl EnergyBudgetHandle {
    /// Create a test handle with synthetic values.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            remaining: u64::MAX,
            cap: u64::MAX,
        }
    }

    /// Create an energy budget handle.
    pub fn new(remaining: u64, cap: u64) -> Self {
        Self { remaining, cap }
    }

    /// Check remaining energy budget.
    pub fn remaining(&self) -> u64 {
        self.remaining
    }

    /// Get the energy cap.
    pub fn cap(&self) -> u64 {
        self.cap
    }

    /// Get the usage ratio (remaining / cap).
    ///
    /// Returns 0.0 if cap is 0 (defensive; should not happen in practice).
    pub fn usage_ratio(&self) -> f64 {
        if self.cap == 0 {
            return 0.0;
        }
        self.remaining as f64 / self.cap as f64
    }

    /// Request consumption of energy units.
    ///
    /// # Requires
    /// - `amount` must not exceed remaining budget
    ///
    /// # Ensures
    /// - Decrements remaining by `amount`
    /// - Returns `Ok(())` if sufficient, `Err` otherwise
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

// =============================================================================
// =============================================================================

/// Rate limiter capability handle.
///
/// Provides access to the rate limiter's current state. The bucket size and
/// refill rate are set by Governance and Curation, not by this handle.
///
/// # OCAP Boundaries
///
/// - **CAN** check token bucket availability
/// - **CAN** consume an invocation slot (decrement)
/// - **CANNOT** resize the bucket (use Governance configuration)
/// - **CANNOT** change the refill rate (use Governance configuration)
/// - **CANNOT** bypass limiting (no mechanism exists)
    /// Agent this handle is scoped to
    agent_webid: WebID,
    /// Current tokens available
    tokens_available: u32,
}

    /// Create a test handle with synthetic values.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            agent_webid: WebID::new(),
            tokens_available: 100,
        }
    }

    /// Create a rate limiter handle for a specific agent.
    pub fn new(agent_webid: WebID, tokens_available: u32) -> Self {
        Self {
            agent_webid,
            tokens_available,
        }
    }

    /// The agent this handle is scoped to.
    pub fn agent(&self) -> &WebID {
        &self.agent_webid
    }

    /// Check if a slot is available.
    pub fn is_available(&self) -> bool {
        self.tokens_available > 0
    }

    /// Get the number of tokens available.
    pub fn tokens_available(&self) -> u32 {
        self.tokens_available
    }

    /// Consume an invocation slot.
    ///
    /// # Requires
    /// - At least one token must be available
    ///
    /// # Ensures
    /// - Decrements `tokens_available` by 1
    /// - Returns `Ok(())` if available, `Err` otherwise
    pub fn consume(&mut self) -> Result<(), RateLimitExceeded> {
        if self.tokens_available == 0 {
            return Err(RateLimitExceeded {
                agent: self.agent_webid,
            });
        }
        self.tokens_available -= 1;
        Ok(())
    }
}

/// Error returned when rate limit is exceeded.
#[derive(Debug, Clone, thiserror::Error)]
#[error("rate limit exceeded for agent {agent}")]
pub struct RateLimitExceeded {
    pub agent: WebID,
}

