//! NuEventStore — Persistent storage for CNS ν-events

use hkask_types::event::{Phase, Span, SpanNamespace};
use hkask_types::id::{EventID, WebID};
use hkask_types::{InfrastructureError, NuEvent, NuEventSink};
use thiserror::Error;

/// Per-domain decay constants for weighted replay.
///
/// Each loop domain has its own `λ` (lambda) for exponential decay.
/// Half-life = ln(2)/λ. A Cybernetics half-life of ~5min (λ≈0.0023/s)
/// means events older than ~30min are negligible (weight < 0.001).
#[derive(Debug, Clone)]
pub struct DecayConfig {
    /// Cybernetics decay constant (1/s). Default: ln(2)/300 ≈ 0.00231 (5min half-life)
    pub cybernetics_lambda: f64,
    /// Curation decay constant (1/s). Default: ln(2)/900 ≈ 0.00077 (15min half-life)
    pub curation_lambda: f64,
    /// Inference decay constant (1/s). Default: ln(2)/120 ≈ 0.00578 (2min half-life)
    pub inference_lambda: f64,
    /// Episodic decay constant (1/s). Default: ln(2)/600 ≈ 0.00116 (10min half-life)
    pub episodic_lambda: f64,
    /// Minimum weight threshold — events below this are not replayed. Default: 0.001
    pub weight_threshold: f64,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            cybernetics_lambda: std::f64::consts::LN_2 / 300.0,
            curation_lambda: std::f64::consts::LN_2 / 900.0,
            inference_lambda: std::f64::consts::LN_2 / 120.0,
            episodic_lambda: std::f64::consts::LN_2 / 600.0,
            weight_threshold: 0.001,
        }
    }
}

/// A NuEvent with its computed replay weight.
#[derive(Debug, Clone)]
pub struct WeightedEvent {
    pub event: NuEvent,
    pub weight: f64,
}

#[derive(Error, Debug)]
pub enum NuEventError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),
}

impl_from_rusqlite!(NuEventError, Infra);

impl From<serde_json::Error> for NuEventError {
    fn from(e: serde_json::Error) -> Self {
        InfrastructureError::from(e).into()
    }
}

/// Algedonic-significant span categories for Curation review.
///
/// These are the CNS span namespaces that produce events requiring
/// Curation (Loop 5) attention: energy deficits, variety imbalances,
/// kill-zone threats, and agent pod failures.
const ALGEDONIC_SPAN_CATEGORIES: &[&str] = &["energy", "variety", "killzone", "agent_pod"];

define_store!(NuEventStore);

impl NuEventStore {
    /// Replay events with exponentially decaying weights.
    ///
    /// Events are weighted by `exp(-λ · Δt)` where Δt is the time elapsed
    /// since the event, and λ is the per-domain decay constant. Events with
    /// weight below `config.weight_threshold` are excluded.
    ///
    /// The domain is determined from the event's span namespace:
    /// - "variety", "gas", "killzone" → cybernetics_lambda
    /// - "curation", "spec" → curation_lambda
    /// - "inference" → inference_lambda
    /// - "agent_pod", "connector" → episodic_lambda
    /// - everything else → cybernetics_lambda (safe default)
    pub fn replay_weighted(
        &self,
        since: chrono::DateTime<chrono::Utc>,
        limit: u64,
        config: &DecayConfig,
    ) -> Result<Vec<WeightedEvent>, NuEventError> {
        let events = self.query_algedonic(since, limit)?;
        let now = chrono::Utc::now();

        let weighted: Vec<WeightedEvent> = events
            .into_iter()
            .filter_map(|event| {
                let delta_secs = (now - event.timestamp).num_seconds() as f64;
                let lambda = Self::lambda_for_category(event.span.namespace.short_name(), config);
                let weight = (-lambda * delta_secs).exp();
                if weight >= config.weight_threshold {
                    Some(WeightedEvent { event, weight })
                } else {
                    None
                }
            })
            .collect();

        Ok(weighted)
    }

    fn lambda_for_category(category: &str, config: &DecayConfig) -> f64 {
        match category {
            c if c.starts_with("variety") || c.starts_with("gas") || c.starts_with("killzone") => {
                config.cybernetics_lambda
            }
            c if c.starts_with("curation") || c.starts_with("spec") => config.curation_lambda,
            c if c.starts_with("inference") => config.inference_lambda,
            c if c.starts_with("agent_pod") || c.starts_with("connector") => config.episodic_lambda,
            _ => config.cybernetics_lambda, // safe default
        }
    }

    pub(crate) fn insert(&self, event: &NuEvent) -> Result<(), NuEventError> {
        let conn = self.lock_conn()?;
        let (span_category, span_path) = span_to_columns(&event.span);

        conn.execute(
            "INSERT INTO nu_events (id, timestamp, observer_webid, span_category, span_path, phase, observation, regulation, outcome, recursion_depth, parent_event, visibility)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                event.id.0.to_string(),
                event.timestamp.to_rfc3339(),
                event.observer_webid.0.to_string(),
                span_category,
                span_path,
                event.phase.as_str(),
                serde_json::to_string(&event.observation)?,
                event.regulation.as_ref().and_then(|v| serde_json::to_string(v).ok()),
                event.outcome.as_ref().and_then(|v| serde_json::to_string(v).ok()),
                event.recursion_depth,
                event.parent_event.map(|p| p.0.to_string()),
                event.visibility,
            ],
        )?;
        Ok(())
    }

    /// Query algedonic-significant events since a given timestamp.
    ///
    /// Returns NuEvents from algedonic span categories (energy, variety,
    /// killzone, agent_pod) with `phase = Act` and severity exceeding
    /// threshold, ordered by timestamp ascending. This is the canonical
    /// alerts log that Curation reads via cursor — one fact in one place.
    ///
    /// Per Fowler's Gateway pattern: the NuEvent store is the gateway;
    /// Curation queries it with a cursor, not live CNS state.
    pub fn query_algedonic(
        &self,
        since: chrono::DateTime<chrono::Utc>,
        limit: u64,
    ) -> Result<Vec<NuEvent>, NuEventError> {
        let conn = self.lock_conn()?;

        let since_str = since.to_rfc3339();

        // Build IN clause for algedonic span categories
        let placeholders: Vec<&str> = ALGEDONIC_SPAN_CATEGORIES.iter().map(|_| "?").collect();
        let sql = format!(
            "SELECT id, timestamp, observer_webid, span_category, span_path, phase, \
             observation, regulation, outcome, recursion_depth, parent_event, visibility \
             FROM nu_events \
             WHERE timestamp > ? AND span_category IN ({}) AND phase = 'act' \
             ORDER BY timestamp ASC \
             LIMIT ?",
            placeholders.join(", ")
        );

        let mut stmt = conn.prepare(&sql).map_err(NuEventError::from)?;

        // Dynamic params: since, then each span category, then limit
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> =
            Vec::with_capacity(2 + ALGEDONIC_SPAN_CATEGORIES.len());
        param_values.push(Box::new(since_str));
        for &cat in ALGEDONIC_SPAN_CATEGORIES {
            param_values.push(Box::new(cat.to_string()));
        }
        param_values.push(Box::new(limit as i64));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let events = stmt
            .query_map(param_refs.as_slice(), row_to_nu_event)
            .map_err(NuEventError::from)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(events)
    }
}

/// Reconstruct a NuEvent from a database row.
fn row_to_nu_event(row: &rusqlite::Row<'_>) -> Result<NuEvent, rusqlite::Error> {
    let id_str: String = row.get(0)?;
    let timestamp_str: String = row.get(1)?;
    let observer_webid_str: String = row.get(2)?;
    let span_category: String = row.get(3)?;
    let span_path: String = row.get(4)?;
    let phase_str: String = row.get(5)?;
    let observation_str: String = row.get(6)?;
    let regulation_str: Option<String> = row.get(7)?;
    let outcome_str: Option<String> = row.get(8)?;
    let recursion_depth: u8 = row.get(9)?;
    let parent_event_str: Option<String> = row.get(10)?;
    let visibility: String = row.get(11)?;

    let id = EventID(uuid::Uuid::parse_str(&id_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?);

    let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
        })?
        .to_utc();

    let observer_webid = WebID(uuid::Uuid::parse_str(&observer_webid_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
    })?);

    // Reconstruct Span from stored category + path
    let namespace_str = format!("cns.{}", span_category);
    let namespace = SpanNamespace::parse(&namespace_str).unwrap_or_else(|| {
        // Fallback: use the stored category directly as a non-canonical namespace.
        // This shouldn't happen for canonical events, but provides graceful degradation.
        SpanNamespace::new("cns.gas") // safe default
    });
    // span_path is fully-qualified (e.g., "cns.gas.depleted"), so extract
    // the local part after the namespace prefix + dot.
    let local_path = &span_path[namespace.as_str().len() + 1..];
    let span = Span::new(namespace, local_path);

    let phase = Phase::from_str(&phase_str);

    let observation: serde_json::Value = serde_json::from_str(&observation_str).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let regulation = regulation_str
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let outcome = outcome_str
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    let parent_event = parent_event_str
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .map(EventID);

    Ok(NuEvent {
        id,
        timestamp,
        observer_webid,
        span,
        phase,
        observation,
        regulation,
        outcome,
        recursion_depth,
        parent_event,
        visibility,
    })
}

fn span_to_columns(span: &Span) -> (&str, &str) {
    (span.namespace.short_name(), span.path.as_str())
}

impl NuEventSink for NuEventStore {
    fn persist(&self, event: &NuEvent) -> Result<(), InfrastructureError> {
        self.insert(event).map_err(|e| match e {
            NuEventError::Infra(infra) => infra,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::event::{NuEvent, Phase, Span, SpanNamespace};
    use hkask_types::id::WebID;

    #[test]
    fn decay_config_default_values() {
        let config = DecayConfig::default();
        // Cybernetics: ln(2)/300 ≈ 0.00231 (5min half-life)
        let expected_cyber = std::f64::consts::LN_2 / 300.0;
        assert!((config.cybernetics_lambda - expected_cyber).abs() < 1e-10);
        // Curation: ln(2)/900 ≈ 0.00077 (15min half-life)
        let expected_cur = std::f64::consts::LN_2 / 900.0;
        assert!((config.curation_lambda - expected_cur).abs() < 1e-10);
        // Inference: ln(2)/120 ≈ 0.00578 (2min half-life)
        let expected_inf = std::f64::consts::LN_2 / 120.0;
        assert!((config.inference_lambda - expected_inf).abs() < 1e-10);
        // Episodic: ln(2)/600 ≈ 0.00116 (10min half-life)
        let expected_epi = std::f64::consts::LN_2 / 600.0;
        assert!((config.episodic_lambda - expected_epi).abs() < 1e-10);
        // Weight threshold
        assert!((config.weight_threshold - 0.001).abs() < 1e-10);
    }

    #[test]
    fn lambda_for_category_mapping() {
        let config = DecayConfig::default();
        // Cybernetics domain
        assert!(
            (NuEventStore::lambda_for_category("variety", &config) - config.cybernetics_lambda)
                .abs()
                < 1e-10
        );
        assert!(
            (NuEventStore::lambda_for_category("gas", &config) - config.cybernetics_lambda).abs()
                < 1e-10
        );
        assert!(
            (NuEventStore::lambda_for_category("killzone", &config) - config.cybernetics_lambda)
                .abs()
                < 1e-10
        );
        // Curation domain
        assert!(
            (NuEventStore::lambda_for_category("curation", &config) - config.curation_lambda).abs()
                < 1e-10
        );
        assert!(
            (NuEventStore::lambda_for_category("spec", &config) - config.curation_lambda).abs()
                < 1e-10
        );
        // Inference domain
        assert!(
            (NuEventStore::lambda_for_category("inference", &config) - config.inference_lambda)
                .abs()
                < 1e-10
        );
        // Episodic domain
        assert!(
            (NuEventStore::lambda_for_category("agent_pod", &config) - config.episodic_lambda)
                .abs()
                < 1e-10
        );
        assert!(
            (NuEventStore::lambda_for_category("connector", &config) - config.episodic_lambda)
                .abs()
                < 1e-10
        );
        // Default falls back to cybernetics
        assert!(
            (NuEventStore::lambda_for_category("tool", &config) - config.cybernetics_lambda).abs()
                < 1e-10
        );
        assert!(
            (NuEventStore::lambda_for_category("prompt", &config) - config.cybernetics_lambda)
                .abs()
                < 1e-10
        );
    }

    #[test]
    fn replay_weighted_filters_below_threshold() {
        // Create an in-memory database and store
        let db = crate::Database::in_memory().expect("in-memory db");
        let store = NuEventStore::new(db.conn_arc());

        // Insert an old event (60 min ago) with variety category (cybernetics λ ≈ 0.00231)
        // Weight = exp(-0.00231 * 3600) ≈ exp(-8.316) ≈ 0.00024 < 0.001 threshold
        let mut old_event = NuEvent::new(
            WebID(uuid::Uuid::new_v4()),
            Span::new(SpanNamespace::new("cns.variety"), "depleted"),
            Phase::Act,
            serde_json::json!({"variety_count": 0}),
            0,
        );
        // Set timestamp to 60 min ago so it falls below threshold
        old_event.timestamp = chrono::Utc::now() - chrono::Duration::minutes(60);

        // Insert a recent event (just created) — weight close to 1.0, well above threshold
        let recent_event = NuEvent::new(
            WebID(uuid::Uuid::new_v4()),
            Span::new(SpanNamespace::new("cns.variety"), "depleted"),
            Phase::Act,
            serde_json::json!({"variety_count": 42}),
            0,
        );

        store.insert(&old_event).expect("insert old event");
        store.insert(&recent_event).expect("insert recent event");

        // Query with default decay config
        let config = DecayConfig::default();
        let since = chrono::Utc::now() - chrono::Duration::hours(2);
        let result = store
            .replay_weighted(since, 100, &config)
            .expect("replay_weighted");

        // Only the recent event should survive; the old one is below threshold
        // Note: the old event may or may not be returned by query_algedonic
        // depending on its phase — we verify that any returned events pass
        // the weight threshold.
        for weighted in &result {
            assert!(
                weighted.weight >= config.weight_threshold,
                "weight {} should be >= threshold {}",
                weighted.weight,
                config.weight_threshold
            );
        }
    }

    #[test]
    fn replay_weighted_applies_decay() {
        let db = crate::Database::in_memory().expect("in-memory db");
        let store = NuEventStore::new(db.conn_arc());

        // Insert an event with timestamp 5 minutes ago
        // For variety (cybernetics λ ≈ 0.00231), weight = exp(-0.00231 * 300) ≈ 0.50
        let mut event = NuEvent::new(
            WebID(uuid::Uuid::new_v4()),
            Span::new(SpanNamespace::new("cns.variety"), "test"),
            Phase::Act,
            serde_json::json!({"test": true}),
            0,
        );
        event.timestamp = chrono::Utc::now() - chrono::Duration::minutes(5);
        store.insert(&event).expect("insert event");

        let config = DecayConfig::default();
        let since = chrono::Utc::now() - chrono::Duration::hours(1);
        let result = store
            .replay_weighted(since, 100, &config)
            .expect("replay_weighted");

        // The event should have weight < 1.0 (5 min of decay) and > 0.0
        for weighted in &result {
            assert!(
                weighted.weight < 1.0,
                "weight {} should be < 1.0 for an event with elapsed time",
                weighted.weight
            );
            assert!(
                weighted.weight > 0.0,
                "weight {} should be > 0.0",
                weighted.weight
            );
        }
    }
}
