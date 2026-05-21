//! Sovereignty Observer — Processes CNS sovereignty events

use crate::algedonic::AlgedonicManager;
use hkask_types::{CnsEvent, CnsSpan, WebID};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SovereigntyObserverError {
    #[error("Invalid event: {0}")]
    InvalidEvent(String),
}

pub type SovereigntyObserverResult<T> = Result<T, SovereigntyObserverError>;

/// Sovereignty observer — Processes CNS sovereignty events
pub struct SovereigntyObserver {
    #[allow(dead_code)]
    algedonic_manager: AlgedonicManager,
    #[allow(dead_code)]
    curator_webid: WebID,
}

impl SovereigntyObserver {
    /// Create new sovereignty observer
    pub fn new(algedonic_manager: AlgedonicManager, curator_webid: WebID) -> Self {
        Self {
            algedonic_manager,
            curator_webid,
        }
    }

    /// Process sovereignty event
    pub fn on_event(&self, event: &CnsEvent) -> SovereigntyObserverResult<()> {
        match &event.span {
            CnsSpan::Sovereignty => {
                if event.action.contains("alert.killzone") {
                    self.handle_killzone_alert(event)?;
                } else if event.action.contains("acquisition_attempt") {
                    self.handle_acquisition_attempt(event)?;
                } else if event.action.contains("consent") {
                    self.handle_consent_change(event)?;
                }
            }
            _ => {
                // Not a sovereignty event, ignore
            }
        }
        Ok(())
    }

    fn handle_killzone_alert(&self, event: &CnsEvent) -> SovereigntyObserverResult<()> {
        tracing::warn!(
            target: "cns.sovereignty",
            event = ?event.id,
            action = %event.action,
            outcome = %event.outcome,
            "Kill zone alert triggered"
        );
        Ok(())
    }

    fn handle_acquisition_attempt(&self, event: &CnsEvent) -> SovereigntyObserverResult<()> {
        tracing::info!(
            target: "cns.sovereignty",
            event = ?event.id,
            "Acquisition attempt detected"
        );
        Ok(())
    }

    fn handle_consent_change(&self, event: &CnsEvent) -> SovereigntyObserverResult<()> {
        tracing::info!(
            target: "cns.sovereignty",
            event = ?event.id,
            action = %event.action,
            "Consent change recorded"
        );
        Ok(())
    }

    /// Check if sovereignty state requires immediate attention
    pub fn requires_immediate_attention(&self, event: &CnsEvent) -> bool {
        matches!(&event.span, CnsSpan::Sovereignty) && event.action.contains("alert.killzone")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::CnsSpan;

    #[test]
    fn test_sovereignty_observer_creation() {
        let algedonic_manager = AlgedonicManager::new(100);
        let curator_webid = WebID::new();
        let observer = SovereigntyObserver::new(algedonic_manager, curator_webid);

        assert!(true);
    }

    #[test]
    fn test_non_sovereignty_event_ignored() {
        let algedonic_manager = AlgedonicManager::new(100);
        let curator_webid = WebID::new();
        let observer = SovereigntyObserver::new(algedonic_manager, curator_webid);

        let event = CnsEvent::new(
            CnsSpan::Tool,
            "test_action".to_string(),
            "test_outcome".to_string(),
        );

        assert!(observer.on_event(&event).is_ok());
    }
}
