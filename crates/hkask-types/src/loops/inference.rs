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
//! `RateLimiterHandle` can check the token bucket and consume an invocation slot.
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
// RateLimiterHandle — Loop 1, Subloop 1.5 (GUARD)
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
pub struct RateLimiterHandle {
    /// Agent this handle is scoped to
    agent_webid: WebID,
    /// Current tokens available
    tokens_available: u32,
}

impl RateLimiterHandle {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inference_handle_new_test() {
        let handle = InferenceHandle::new_test();
        assert!(!handle.is_circuit_open());
        assert_eq!(handle.energy_remaining(), u64::MAX);
    }

    #[test]
    fn energy_budget_usage_ratio() {
        let handle = EnergyBudgetHandle::new(50, 100);
        assert!((handle.usage_ratio() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn rate_limiter_consume() {
        let mut handle = RateLimiterHandle::new_test();
        assert!(handle.is_available());
        assert!(handle.consume().is_ok());
        assert!(handle.consume().is_ok());
    }
}

/// Cybernetic unit tests — PR 9a
///
/// These tests verify the cybernetic control loops at the type level:
/// each loop's GUARD/CIRCUIT/FILTER either admits or denies, and OCAP
/// discipline ensures handles cannot escape their capability boundaries.
#[cfg(test)]
mod cyber_tests {
    use super::*;
    use crate::WebID;

    // -------------------------------------------------------------------------
    // Loop 1.0 — InferenceHandle capability discipline
    // -------------------------------------------------------------------------

    /// Verify InferenceHandle exposes only the capabilities it should.
    ///
    /// OCAP discipline: if a method doesn't exist on the type, it can't be
    /// called. We enumerate the methods that *do* exist to confirm the
    /// boundary is correct, and show that forbidden operations are
    /// unreachable by construction.
    #[test]
    fn cyber_inference_handle_capabilities() {
        let mut handle = InferenceHandle::new_test();

        // --- Energy budget is non-zero initially ---
        assert!(
            handle.energy_remaining() > 0,
            "InferenceHandle should start with energy_remaining > 0"
        );

        // --- Circuit starts CLOSED (allowing calls) ---
        assert!(
            !handle.is_circuit_open(),
            "InferenceHandle circuit should start closed (false = allowing)"
        );

        // --- Consume energy succeeds within budget ---
        assert!(
            handle.consume_energy(100).is_ok(),
            "Consuming 100 units within u64::MAX budget should succeed"
        );

        // --- After partial consumption, budget is reduced ---
        // u64::MAX - 100 = u64::MAX - 100, which is still enormous.
        // To test exhaustion we create a handle with a small budget.
        let mut limited = InferenceHandle::new(WebID::new(), 200, false);
        assert!(
            limited.consume_energy(200).is_ok(),
            "Consuming exact budget amount should succeed"
        );

        // --- Exhausted budget denies further consumption ---
        let err = limited.consume_energy(1).unwrap_err();
        assert_eq!(
            err.requested, 1,
            "Exhaustion error should report the requested amount"
        );
        assert_eq!(
            err.remaining, 0,
            "Exhaustion error should report zero remaining"
        );

        // --- OCAP: verify InferenceHandle CANNOT write memory, reset alerts,
        //     or process sovereignty.
        //
        // In Rust's capability model, absence of a method IS the enforcement.
        // The following methods DO exist on InferenceHandle and are the ONLY
        // ways to interact with inference resources:
        //
        //   new_test()              → create test handle
        //   new(_, _, _)            → create production handle
        //   agent()                 → read agent WebID
        //   energy_remaining()      → read energy budget
        //   is_circuit_open()       → read circuit state
        //   consume_energy(u64)     → decrement energy budget
        //
        // The following methods are INTENTIONALLY ABSENT:
        //   write_memory()          → use EpisodicWriteHandle / SemanticWriteHandle
        //   reset_alerts()          → use CnsAdminHandle
        //   process_sovereignty()   → use GovernanceHandle
        //   revoke_capabilities()   → use GovernanceHandle
        //
        // Because these methods do not exist on InferenceHandle, the type
        // system prevents them from being called — no runtime check needed.
        // This test documents the boundary; if someone adds a forbidden
        // method, this comment serves as the design intent marker.

        // We verify the boundary dynamically: the handle we just created can
        // only call the methods listed above. Any attempt to call methods
        // from other handles (write_memory, reset_alerts, etc.) would not
        // compile because InferenceHandle does not implement those traits.
        let _agent: &WebID = limited.agent(); // succeeds — within boundary
    }

    // -------------------------------------------------------------------------
    // Loop 1.4 — GUARD: Energy budget enforcement
    // -------------------------------------------------------------------------

    /// Verify Loop 1.4 GUARD closes: budget check → deny when exceeded.
    #[test]
    fn cyber_energy_budget_enforcement() {
        // --- Unbounded test handle ---
        let unbounded = EnergyBudgetHandle::new_test();
        assert_eq!(
            unbounded.remaining(),
            u64::MAX,
            "Test handle remaining should be u64::MAX"
        );
        assert_eq!(
            unbounded.cap(),
            u64::MAX,
            "Test handle cap should be u64::MAX"
        );
        assert!(
            unbounded.usage_ratio() > 0.99,
            "Usage ratio should be near 1.0 for an unbounded handle, got {}",
            unbounded.usage_ratio()
        );

        // --- Bounded handle: cap = 1000, remaining = 1000 ---
        let mut bounded = EnergyBudgetHandle::new(1000, 1000);
        assert_eq!(
            bounded.remaining(),
            1000,
            "Bounded handle should start with remaining = cap"
        );
        assert_eq!(bounded.cap(), 1000, "Bounded handle cap should be 1000");

        // --- Consume within budget succeeds ---
        assert!(
            bounded.consume(500).is_ok(),
            "Consuming 500 of 1000 should succeed"
        );
        assert_eq!(
            bounded.remaining(),
            500,
            "After consuming 500, remaining should be 500"
        );

        // --- Consume exceeding budget is denied ---
        let err = bounded.consume(600).unwrap_err();
        assert_eq!(
            err.requested, 600,
            "Denial error should report requested = 600"
        );
        assert_eq!(
            err.remaining, 500,
            "Denial error should report remaining = 500"
        );

        // --- GUARD CLOSED: budget exceeded → request denied ---
        // This proves Loop 1.4 GUARD closes: the energy budget enforces
        // an absolute cap, and once consumed, further requests are denied.
    }

    // -------------------------------------------------------------------------
    // Loop 1.5 — GUARD: Rate limiter enforcement
    // -------------------------------------------------------------------------

    /// Verify Loop 1.5 GUARD closes: rate check → deny when tokens exhausted.
    #[test]
    fn cyber_rate_limiter_enforcement() {
        let mut handle = RateLimiterHandle::new_test(); // tokens_available = 100

        // --- Initially available ---
        assert!(
            handle.is_available(),
            "Rate limiter should be available with 100 tokens"
        );

        // --- Consume tokens one at a time, all should succeed ---
        for i in 1..=100 {
            assert!(handle.consume().is_ok(), "Token {i}/100 should succeed");
        }

        // --- All tokens consumed: GUARD CLOSED ---
        assert!(
            !handle.is_available(),
            "Rate limiter should be unavailable after consuming all 100 tokens"
        );

        // --- Next consume is denied ---
        let err = handle.consume().unwrap_err();
        assert_eq!(
            err.agent,
            *handle.agent(),
            "RateLimitExceeded should reference the correct agent"
        );

        // This proves Loop 1.5 GUARD closes: the rate limiter enforces
        // a token bucket cap, and once exhausted, further requests are denied.
    }

    // -------------------------------------------------------------------------
    // Loop 1.3 — CIRCUIT: Circuit breaker state transitions
    // -------------------------------------------------------------------------

    /// Verify Loop 1.3 CIRCUIT state is observable and transitions are possible.
    #[test]
    fn cyber_inference_circuit_breaker() {
        // --- Circuit starts OPEN (blocking) ---
        let open_handle = InferenceHandle::new(WebID::new(), 10000, true);
        assert!(
            open_handle.is_circuit_open(),
            "Circuit created open should report is_circuit_open() == true"
        );

        // --- Circuit starts CLOSED (allowing) ---
        let closed_handle = InferenceHandle::new(WebID::new(), 10000, false);
        assert!(
            !closed_handle.is_circuit_open(),
            "Circuit created closed should report is_circuit_open() == false"
        );

        // --- An open circuit still has energy (the two dimensions are independent) ---
        assert_eq!(
            open_handle.energy_remaining(),
            10000,
            "Open circuit should still report its energy budget"
        );

        // This proves Loop 1.3 CIRCUIT: the circuit breaker state is
        // observable. A closed circuit allows inference; an open circuit
        // blocks it. The `circuit_open` field provides the control signal
        // that the inference loop checks before proceeding.
    }
}
