//! CNS (Cybernetic Nervous System) types for hKask
//!
//! Namespace: cns.* (replaces okh.*)
//! Key spans: cns.tool.*, cns.prompt.*, cns.agent_pod.*, cns.connector.*, cns.template.*, cns.curation.*

use serde::{Deserialize, Serialize};

/// VarietyCounter — Tracks diversity in system behavior
///
/// Algedonic Alert: Variety deficit >100 → escalate to Curator/human
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VarietyCounter(pub u64);

impl VarietyCounter {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn increment(&mut self) {
        self.0 += 1;
    }

    pub fn decrement(&mut self) {
        if self.0 > 0 {
            self.0 -= 1;
        }
    }

    pub fn deficit(&self, target: u64) -> u64 {
        target.saturating_sub(self.0)
    }

    /// Default target variety level
    pub fn target() -> u64 {
        100
    }

    /// Check if variety deficit exceeds algedonic threshold
    /// Alert triggers when deficit > 100 (i.e., counter < 0 when target is 100)
    pub fn needs_alert(&self) -> bool {
        self.deficit(Self::target()) >= 100
    }
}

impl Default for VarietyCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for VarietyCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// AlgedonicAlert — Cybernetic alert when variety deficit exceeds threshold
///
/// Named after algedonic meter in Beer's viable system model.
/// Signals pain/pleasure balance in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgedonicAlert {
    /// Unique alert identifier
    pub id: u64,
    /// Current variety counter value
    pub current: u64,
    /// Threshold that triggered alert
    pub threshold: u64,
    /// Deficit amount
    pub deficit: u64,
    /// Whether alert has been escalated to Curator/human
    pub escalated: bool,
    /// Timestamp of alert
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Span where deficit was detected
    pub span: CnsSpan,
}

impl AlgedonicAlert {
    pub fn new(current: u64, threshold: u64, span: CnsSpan) -> Self {
        let deficit = threshold.saturating_sub(current);

        Self {
            id: Self::generate_id(),
            current,
            threshold,
            deficit,
            escalated: false,
            timestamp: chrono::Utc::now(),
            span,
        }
    }

    pub fn escalate(&mut self) {
        self.escalated = true;
    }

    fn generate_id() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

impl std::fmt::Display for AlgedonicAlert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AlgedonicAlert[deficit={}, span={}, escalated={}]",
            self.deficit, self.span, self.escalated
        )
    }
}

/// CnsSpan — Namespace for CNS monitoring spans
///
/// All CNS spans use cns.* prefix for observability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CnsSpan {
    /// Tool governance, invocation (cns.tool.*)
    Tool,
    /// Prompt render, validate, outcome (cns.prompt.*)
    Prompt,
    /// Agent pod lifecycle, delegation (cns.agent_pod.*)
    AgentPod,
    /// External I/O: LLM, embeddings (cns.connector.*)
    Connector,
    /// Template invocation, registry (cns.template.*)
    Template,
    /// Curation decisions, OCAP boundaries (cns.curation.*)
    Curation,
    /// Variety monitoring, algedonic alerts (cns.variety.*)
    Variety,
    /// Kill zone detection (cns.killzone.*)
    KillZone,
}

impl CnsSpan {
    /// Full span name with cns. prefix
    pub fn full_name(&self) -> String {
        match self {
            CnsSpan::Tool => "cns.tool".to_string(),
            CnsSpan::Prompt => "cns.prompt".to_string(),
            CnsSpan::AgentPod => "cns.agent_pod".to_string(),
            CnsSpan::Connector => "cns.connector".to_string(),
            CnsSpan::Template => "cns.template".to_string(),
            CnsSpan::Curation => "cns.curation".to_string(),
            CnsSpan::Variety => "cns.variety".to_string(),
            CnsSpan::KillZone => "cns.killzone".to_string(),
        }
    }
}

impl std::fmt::Display for CnsSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_name())
    }
}

/// CnsEvent — Cybernetic audit trail event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CnsEvent {
    pub id: u64,
    pub span: CnsSpan,
    pub action: String,
    pub outcome: String,
    pub variety_before: Option<VarietyCounter>,
    pub variety_after: Option<VarietyCounter>,
    pub alert: Option<AlgedonicAlert>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl CnsEvent {
    pub fn new(span: CnsSpan, action: String, outcome: String) -> Self {
        Self {
            id: Self::generate_id(),
            span,
            action,
            outcome,
            variety_before: None,
            variety_after: None,
            alert: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_variety(mut self, before: VarietyCounter, after: VarietyCounter) -> Self {
        self.variety_before = Some(before);
        self.variety_after = Some(after);
        self
    }

    pub fn with_alert(mut self, alert: AlgedonicAlert) -> Self {
        self.alert = Some(alert);
        self
    }

    fn generate_id() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }
}

/// KillZoneState — Tracking state for catch-and-kill detection
///
/// Monitors VC investment patterns that indicate kill zone formation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillZoneState {
    /// Space/technology being monitored
    pub space_id: String,
    /// VC investment level (normalized 0.0-1.0)
    pub vc_investment: f32,
    /// Acquisition count in last N days
    pub acquisition_count: u32,
    /// Whether kill zone is detected
    pub kill_zone_detected: bool,
    /// Timestamp of last update
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl KillZoneState {
    pub fn new(space_id: String) -> Self {
        Self {
            space_id,
            vc_investment: 1.0,
            acquisition_count: 0,
            kill_zone_detected: false,
            last_updated: chrono::Utc::now(),
        }
    }

    /// Update VC investment level
    pub fn update_vc_investment(&mut self, level: f32) {
        self.vc_investment = level.clamp(0.0, 1.0);
        self.last_updated = chrono::Utc::now();

        // Kill zone detected if VC investment drops below 0.5 after major acquisition
        if self.vc_investment < 0.5 && self.acquisition_count > 0 {
            self.kill_zone_detected = true;
        }
    }

    /// Record acquisition event
    pub fn record_acquisition(&mut self) {
        self.acquisition_count += 1;
        self.last_updated = chrono::Utc::now();
    }

    /// Check if kill zone is active
    pub fn is_kill_zone(&self) -> bool {
        self.kill_zone_detected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variety_counter_increment() {
        let mut counter = VarietyCounter::new();
        assert_eq!(counter.0, 0);

        counter.increment();
        assert_eq!(counter.0, 1);
    }

    #[test]
    fn test_variety_counter_deficit() {
        let counter = VarietyCounter(50);
        assert_eq!(counter.deficit(100), 50);
        assert_eq!(counter.deficit(40), 0);
    }

    #[test]
    fn test_variety_counter_needs_alert() {
        let counter = VarietyCounter(0);
        assert!(counter.needs_alert());

        let counter = VarietyCounter(100);
        assert!(!counter.needs_alert());
    }

    #[test]
    fn test_algedonic_alert_new() {
        let alert = AlgedonicAlert::new(0, 100, CnsSpan::Variety);
        assert_eq!(alert.deficit, 100);
        assert!(!alert.escalated);
        assert_eq!(alert.span, CnsSpan::Variety);
    }

    #[test]
    fn test_algedonic_alert_escalate() {
        let mut alert = AlgedonicAlert::new(0, 100, CnsSpan::Variety);
        assert!(!alert.escalated);

        alert.escalate();
        assert!(alert.escalated);
    }

    #[test]
    fn test_cns_span_full_name() {
        assert_eq!(CnsSpan::Template.full_name(), "cns.template");
        assert_eq!(CnsSpan::Curation.full_name(), "cns.curation");
        assert_eq!(CnsSpan::KillZone.full_name(), "cns.killzone");
    }

    #[test]
    fn test_cns_event_new() {
        let event = CnsEvent::new(
            CnsSpan::Template,
            "invoke".to_string(),
            "success".to_string(),
        );
        assert_eq!(event.span, CnsSpan::Template);
        assert_eq!(event.action, "invoke");
        assert!(event.alert.is_none());
    }

    #[test]
    fn test_kill_zone_state_detection() {
        let mut state = KillZoneState::new("social_media".to_string());
        assert!(!state.is_kill_zone());

        state.record_acquisition();
        state.update_vc_investment(0.4);
        assert!(state.is_kill_zone());
    }

    #[test]
    fn test_kill_zone_state_safe() {
        let mut state = KillZoneState::new("open_source".to_string());
        state.update_vc_investment(0.8);
        assert!(!state.is_kill_zone());
    }
}
