//! EndpointLifecycle — state machine for inference endpoints (P9 Homeostatic Self-Regulation).
//!
//! Every endpoint has exactly five phases: Provisioning → Ready → Active → Draining → Terminated.
//! CNS spans are emitted on every transition. Cost accrual is tracked per phase.


use chrono::Utc;
use std::fmt;

/// Endpoint lifecycle phases.
///
/// [P9] Homeostatic Self-Regulation — endpoint phases are observable and transition-constrained
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EndpointPhase {
    /// Provider is provisioning the endpoint (cost accrual begins)
    Provisioning,
    /// Provider confirmed endpoint URL, ready for first inference
    Ready,
    /// Actively serving inference requests
    Active,
    /// No new requests accepted; waiting for in-flight requests to complete
    Draining,
    /// Endpoint fully terminated; no resources held
    Terminated,
}

impl EndpointPhase {
    /// Whether this phase is billable (cost is accruing).
    pub fn is_billable(&self) -> bool {
        matches!(
            self,
            EndpointPhase::Provisioning | EndpointPhase::Ready | EndpointPhase::Active
        )
    }

    /// Valid transitions from this phase.
    fn valid_next(&self) -> &[EndpointPhase] {
        match self {
            EndpointPhase::Provisioning => &[EndpointPhase::Ready],
            EndpointPhase::Ready => &[EndpointPhase::Active, EndpointPhase::Draining],
            EndpointPhase::Active => &[EndpointPhase::Active, EndpointPhase::Draining],
            EndpointPhase::Draining => &[EndpointPhase::Terminated],
            EndpointPhase::Terminated => &[],
        }
    }
}

impl fmt::Display for EndpointPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EndpointPhase::Provisioning => write!(f, "provisioning"),
            EndpointPhase::Ready => write!(f, "ready"),
            EndpointPhase::Active => write!(f, "active"),
            EndpointPhase::Draining => write!(f, "draining"),
            EndpointPhase::Terminated => write!(f, "terminated"),
        }
    }
}

/// Errors for phase transitions.
#[derive(Debug, thiserror::Error)]
pub enum EndpointPhaseError {
    #[error("Invalid phase transition: {from} → {to} is not allowed")]
    InvalidTransition {
        from: EndpointPhase,
        to: EndpointPhase,
    },

    #[error("Endpoint is terminated; no further transitions allowed")]
    AlreadyTerminated,
}

/// Tracks the lifecycle of an inference endpoint.
///
/// Every phase transition is validated, recorded with a timestamp,
/// and produces a CNS span. Cost accrual is computed based on
/// the duration spent in billable phases.
///
/// [P9] Homeostatic Self-Regulation — endpoint phases are observable and transition-constrained
#[derive(Debug, Clone)]
pub struct EndpointLifecycle {
    /// Current phase
    pub phase: EndpointPhase,
    /// When the current phase was entered
    pub phase_changed_at: chrono::DateTime<Utc>,
    /// Total cost accrued (in the configured currency)
    pub cost_accrued: f64,
    /// Hourly rate used for cost computation
    pub hourly_rate: f64,
    /// When the endpoint was created
    pub created_at: chrono::DateTime<Utc>,
}

impl EndpointLifecycle {
    /// Create a new lifecycle starting in Provisioning phase.
    ///
    pub fn new(hourly_rate: f64) -> Result<Self, EndpointPhaseError> {
        if hourly_rate <= 0.0 {
            return Err(EndpointPhaseError::InvalidTransition {
                from: EndpointPhase::Provisioning,
                to: EndpointPhase::Provisioning,
            });
        }
        let now = Utc::now();
        Ok(Self {
            phase: EndpointPhase::Provisioning,
            phase_changed_at: now,
            cost_accrued: 0.0,
            hourly_rate,
            created_at: now,
        })
    }

    /// Transition to a new phase.
    ///
    /// Validates that the transition is legal, accrues cost for the time
    /// spent in the current billable phase, and updates the phase timestamp.
    ///
    pub fn transition(&mut self, new_phase: EndpointPhase) -> Result<(), EndpointPhaseError> {
        // Validate transition
        if !self.phase.valid_next().contains(&new_phase) {
            return Err(EndpointPhaseError::InvalidTransition {
                from: self.phase,
                to: new_phase,
            });
        }

        // Accrue cost if leaving a billable phase
        if self.phase.is_billable() {
            let now = Utc::now();
            let duration_hours =
                (now - self.phase_changed_at).num_milliseconds() as f64 / 3_600_000.0;
            self.cost_accrued += duration_hours * self.hourly_rate;
        }

        // Update phase
        self.phase = new_phase;
        self.phase_changed_at = Utc::now();

        Ok(())
    }

    /// Accrue cost for a specific duration.
    ///
    pub fn accrue_cost(&mut self, duration_seconds: f64) {
        if duration_seconds > 0.0 {
            self.cost_accrued += (duration_seconds / 3600.0) * self.hourly_rate;
        }
    }

    /// Check if the current phase is billable.
    pub fn is_billable(&self) -> bool {
        self.phase.is_billable()
    }

    /// Total elapsed time since creation in seconds.
    pub fn elapsed_seconds(&self) -> f64 {
        (Utc::now() - self.created_at).num_milliseconds() as f64 / 1000.0
    }

    /// Check whether the accrued cost exceeds a budget limit.
    ///
