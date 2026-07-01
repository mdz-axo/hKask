//! CNS Store SLO Provider — Bridges ν-event store to SLO evaluation
//!
//! Implements `SloDataProvider` by querying the real ν-event store.
//! Counts events matching a CNS namespace prefix and classifies them
//! as successful or failed based on observation content.

use crate::slo_manager::{SloDataPoint, SloDataProvider, SloManagerError};
use hkask_storage::{DecayConfig, NuEventStore};
use std::sync::Arc;

/// SLO data provider backed by the real NuEventStore.
///
/// Queries ν-events for the given span namespace and window,
/// and classifies events as successful or failed based on
/// the presence of error indicators in the observation data.
pub struct CnsStoreSloProvider {
    store: Arc<NuEventStore>,
}

impl CnsStoreSloProvider {
    /// Create a new provider backed by the given ν-event store.
    pub fn new(store: Arc<NuEventStore>) -> Self {
        Self { store }
    }
}

impl SloDataProvider for CnsStoreSloProvider {
    fn query(
        &self,
        span_namespace: &str,
        window_seconds: u64,
    ) -> Result<SloDataPoint, SloManagerError> {
        let since = chrono::Utc::now() - chrono::Duration::seconds(window_seconds as i64);
        let config = DecayConfig::default();

        let weighted = self
            .store
            .replay_weighted(since, 10_000, &config)
            .map_err(|e| SloManagerError::DataProvider(e.to_string()))?;

        let total = weighted.len() as u64;
        let matching: Vec<_> = weighted
            .into_iter()
            .filter(|we| we.event.span.namespace.as_str().starts_with(span_namespace))
            .collect();
        let total_ops = matching.len() as u64;
        let successful = matching
            .iter()
            .filter(|we| !is_error_event(&we.event.observation))
            .count() as u64;

        Ok(SloDataPoint {
            total_operations: total_ops,
            successful_operations: successful,
        })
    }
}

/// Heuristic: classify a ν-event observation as an error.
///
/// Checks for common error indicators in the observation JSON:
/// - "error" key present
/// - "status" key with "error" or "failed" value
/// - "outcome" key with "failure" value
fn is_error_event(observation: &serde_json::Value) -> bool {
    if let Some(obj) = observation.as_object() {
        if obj.contains_key("error") {
            return true;
        }
        if let Some(status) = obj.get("status").and_then(|v| v.as_str()) {
            if status == "error" || status == "failed" {
                return true;
            }
        }
        if let Some(outcome) = obj.get("outcome").and_then(|v| v.as_str()) {
            if outcome == "failure" {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_error_detects_error_key() {
        let obs = serde_json::json!({"error": "something went wrong"});
        assert!(is_error_event(&obs));
    }

    #[test]
    fn is_error_detects_failed_status() {
        let obs = serde_json::json!({"status": "failed"});
        assert!(is_error_event(&obs));
    }

    #[test]
    fn is_error_detects_error_status() {
        let obs = serde_json::json!({"status": "error"});
        assert!(is_error_event(&obs));
    }

    #[test]
    fn is_error_detects_failure_outcome() {
        let obs = serde_json::json!({"outcome": "failure"});
        assert!(is_error_event(&obs));
    }

    #[test]
    fn is_error_passes_clean_observation() {
        let obs = serde_json::json!({"result": "ok", "data": 42});
        assert!(!is_error_event(&obs));
    }

    #[test]
    fn is_error_passes_empty_object() {
        let obs = serde_json::json!({});
        assert!(!is_error_event(&obs));
    }

    #[test]
    fn is_error_passes_null() {
        assert!(!is_error_event(&serde_json::Value::Null));
    }
}
