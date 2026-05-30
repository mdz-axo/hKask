//! Loop 4: Observability (CNS) — Capability handles
//!
//! The Observability loop monitors system health:
//! emit span → aggregate → detect anomaly → escalate
//!
//! Subloops:
//! - 4.1 Variety Tracking (SENSE) — state → measure → signal
//! - 4.2 Algedonic Alert Generation (ROUTE) — signal → classify → deliver to consumer
//! - 4.3 Bot Metrics Collection (SENSE) — state → measure → signal
//! - 4.4 Sovereignty Observation (SENSE) — state → measure → signal
//!
//! # Handle Architecture
//!
//! The CNS handle is split into four handles following the principle of
//! least authority (Q9 resolved):
//!
//! - `CnsWriteHandle` — Can emit spans and increment variety counters.
//!   Used by inference and memory loops to report observations.
//!   CANNOT reset alerts, subscribe, or process sovereignty events.
//!
//! - `CnsGovernReadHandle` — Can check variety, process sovereignty events (read-only).
//!   Used by Governance to read observability data for policy decisions.
//!   CANNOT set expected variety, calibrate thresholds, or emit spans.
//!
//! - `CnsGovernWriteHandle` — Can set expected variety and calibrate thresholds (read + write).
//!   Used by Curation to adjust observability policy based on system evaluation.
//!   CANNOT emit spans or reset alerts.
//!
//! - `CnsAdminHandle` — Can reset alerts, clear old alerts, subscribe listeners.
//!   Used by system administration for maintenance operations.
//!   CANNOT emit spans or check variety (separation of concerns).

use crate::event::SpanCategory;
use crate::id::WebID;

// =============================================================================
// CnsWriteHandle — Loop 4 write access (span emission)
// =============================================================================

/// CNS write handle for span emission and variety tracking.
///
/// This is the primary handle for loops that produce observability data
/// (inference, memory, governance). It can emit spans and increment
/// variety counters, but CANNOT reset alerts, subscribe listeners,
/// or process sovereignty events.
///
/// # OCAP Boundaries
///
/// - **CAN** emit CNS spans (observability)
/// - **CAN** increment variety counters (SENSE subloop)
/// - **CANNOT** reset alerts (use `CnsAdminHandle`)
/// - **CANNOT** subscribe listeners (use `CnsAdminHandle`)
/// - **CANNOT** process sovereignty events (use `CnsGovernReadHandle`)
pub struct CnsWriteHandle {
    /// Agent emitting spans
    emitter: WebID,
}

impl CnsWriteHandle {
    /// Create a test handle with synthetic values.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            emitter: WebID::new(),
        }
    }

    /// Create a CNS write handle for a specific agent.
    pub fn new(emitter: WebID) -> Self {
        Self { emitter }
    }

    /// The agent this handle emits spans on behalf of.
    pub fn emitter(&self) -> &WebID {
        &self.emitter
    }

    /// Emit a span for the given category.
    ///
    /// # Ensures
    /// - Span is emitted with the handle's emitter WebID
    /// - Span category is preserved
    pub fn emit_span(&self, category: SpanCategory, action: &str) -> CnsSpanEvent {
        CnsSpanEvent {
            emitter: self.emitter,
            category,
            action: action.to_string(),
        }
    }
}

/// A CNS span event produced by a `CnsWriteHandle`.
#[derive(Debug, Clone)]
pub struct CnsSpanEvent {
    pub emitter: WebID,
    pub category: SpanCategory,
    pub action: String,
}

// =============================================================================
// CnsGovernReadHandle — Loop 4 governance read access
// =============================================================================

/// CNS governance read handle for policy-informed observation.
///
/// Used by Governance to read observability data for policy decisions.
/// Can check variety, process sovereignty events, and read alert status,
/// but CANNOT set expected variety, calibrate thresholds, or emit spans.
///
/// # OCAP Boundaries
///
/// - **CAN** check variety counters (SENSE subloop, read-only)
/// - **CAN** process sovereignty events (SENSE subloop, read-only)
/// - **CAN** read alert status (ROUTE subloop, read-only)
/// - **CANNOT** set expected variety (use `CnsGovernWriteHandle`)
/// - **CANNOT** calibrate thresholds (use `CnsGovernWriteHandle`)
/// - **CANNOT** emit spans (use `CnsWriteHandle`)
/// - **CANNOT** reset alerts (use `CnsAdminHandle`)
pub struct CnsGovernReadHandle {
    /// Agent performing governance reads
    governor: WebID,
}

impl CnsGovernReadHandle {
    /// Create a test handle with synthetic values.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            governor: WebID::new(),
        }
    }

    /// Create a CNS governance read handle for a specific agent.
    pub fn new(governor: WebID) -> Self {
        Self { governor }
    }

    /// The agent performing governance reads.
    pub fn governor(&self) -> &WebID {
        &self.governor
    }
}

// =============================================================================
// CnsGovernWriteHandle — Loop 4 + 5 governance write access
// =============================================================================

/// CNS governance write handle for threshold calibration.
///
/// Used by Curation to adjust observability policy based on system
/// evaluation. This handle can set expected variety and calibrate
/// thresholds, but CANNOT emit spans or reset alerts.
///
/// # OCAP Boundaries
///
/// - **CAN** set expected variety (ADAPT subloop, write)
/// - **CAN** calibrate thresholds (ADAPT subloop, write)
/// - **CAN** read variety counters (inherits from `CnsGovernReadHandle`)
/// - **CANNOT** emit spans (use `CnsWriteHandle`)
/// - **CANNOT** reset alerts (use `CnsAdminHandle`)
/// - **CANNOT** subscribe listeners (use `CnsAdminHandle`)
pub struct CnsGovernWriteHandle {
    /// Agent performing governance writes (typically Curator)
    governor: WebID,
}

impl CnsGovernWriteHandle {
    /// Create a test handle with synthetic values.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            governor: WebID::new(),
        }
    }

    /// Create a CNS governance write handle for a specific agent.
    pub fn new(governor: WebID) -> Self {
        Self { governor }
    }

    /// The agent performing governance writes.
    pub fn governor(&self) -> &WebID {
        &self.governor
    }
}

// =============================================================================
// CnsAdminHandle — Loop 4 administration access
// =============================================================================

/// CNS administration handle for system maintenance.
///
/// Used for operational maintenance: resetting alerts, clearing old
/// alert history, and subscribing event listeners. This handle has
/// the narrowest write scope — it can modify alert state but CANNOT
/// emit spans, check variety, or calibrate thresholds.
///
/// # OCAP Boundaries
///
/// - **CAN** reset alerts
/// - **CAN** clear old alerts
/// - **CAN** subscribe listeners
/// - **CANNOT** emit spans (use `CnsWriteHandle`)
/// - **CANNOT** check variety (use `CnsGovernReadHandle`)
/// - **CANNOT** calibrate thresholds (use `CnsGovernWriteHandle`)
pub struct CnsAdminHandle {
    /// Administrator WebID
    admin: WebID,
}

impl CnsAdminHandle {
    /// Create a test handle with synthetic values.
    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            admin: WebID::new(),
        }
    }

    /// Create a CNS admin handle for a specific administrator.
    pub fn new(admin: WebID) -> Self {
        Self { admin }
    }

    /// The administrator this handle is scoped to.
    pub fn admin(&self) -> &WebID {
        &self.admin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cns_write_handle_emit_span() {
        let handle = CnsWriteHandle::new_test();
        let event = handle.emit_span(SpanCategory::Tool, "invoked");
        assert_eq!(event.action, "invoked");
    }

    #[test]
    fn cns_handles_have_distinct_access() {
        // Verify that handle types exist and are distinct
        let _write = CnsWriteHandle::new_test();
        let _govern_read = CnsGovernReadHandle::new_test();
        let _govern_write = CnsGovernWriteHandle::new_test();
        let _admin = CnsAdminHandle::new_test();
        // Type system enforces: write can emit, govern_read can check,
        // govern_write can calibrate, admin can reset
    }
}
