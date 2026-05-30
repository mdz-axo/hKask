//! CNS Sovereignty Observer — Data types for sovereignty event tracking
//!
//! Defines the event and state types used by `UnifiedVarietyTracker::process_sovereignty_event()`.
//! The standalone `SovereigntyObserver` struct has been removed — all sovereignty
//! monitoring is now handled by `UnifiedVarietyTracker`.

use hkask_types::{DataCategory, SovereigntyId, WebID};
use serde_json::Value;
use std::collections::HashMap;

/// Sovereignty event types monitored by CNS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SovereigntyEventType {
    /// Acquisition attempt detected
    AcquisitionAttempt,
    /// Kill zone alert triggered
    KillZoneAlert,
    /// Consent granted
    ConsentGranted,
    /// Consent revoked
    ConsentRevoked,
    /// Sovereignty boundary violation
    BoundaryViolation,
}

/// Sovereignty event record
#[derive(Debug, Clone)]
pub struct SovereigntyEvent {
    pub event_type: SovereigntyEventType,
    pub timestamp: std::time::Instant,
    pub webid: WebID,
    pub sovereignty_id: SovereigntyId,
    pub data_category: Option<DataCategory>,
    pub details: Value,
}

/// Sovereignty observer state
#[derive(Debug, Default, Clone)]
pub struct SovereigntyObserverState {
    /// Count of acquisition attempts per WebID
    pub acquisition_attempts: HashMap<WebID, u64>,
    /// Count of kill zone alerts per WebID
    pub kill_zone_alerts: HashMap<WebID, u64>,
    /// Count of boundary violations per WebID
    pub boundary_violations: HashMap<WebID, u64>,
    /// Total sovereignty events processed
    pub total_events: u64,
}
