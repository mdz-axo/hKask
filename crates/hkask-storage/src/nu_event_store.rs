//! NuEventStore — Persistent storage for CNS ν-events
use crate::{Store, now_rfc3339};
use hkask_rsolidity as rs;
use hkask_types::event::{Phase, Span, SpanCategory, SpanNamespace};
use hkask_types::id::{EventID, WebID};
use hkask_types::{InfrastructureError, NuEvent, NuEventSink, Visibility};
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
/// Algedonic-significant span categories for Curation review.
///
/// These are the CNS span namespaces that produce events requiring
/// Curation (Loop 5) attention: energy deficits, variety imbalances,
/// agent pod failures, and wallet key lifecycle events (exhaustion, expiry).
///
/// Matched against the stored `span_category` column (which holds the
/// full `short_name()` — e.g., `"wallet.key_expired"`).
const ALGEDONIC_SPAN_CATEGORIES: &[&str] = &[
    "gas",
    "variety",
    "agent_pod",
    "wallet.key_expired",
    "wallet.key_exhausted",
];
define_store!(NuEventStore);
impl NuEventStore {
    /// Replay events with exponentially decaying weights.
    ///
    /// Events are weighted by `exp(-λ · Δt)` where Δt is the time elapsed
    /// since the event, and λ is the per-domain decay constant. Events with
    /// weight below `config.weight_threshold` are excluded.
    ///
    /// The domain is determined from the event's span namespace:
    /// - "variety", "gas" → cybernetics_lambda
    /// - "curation", "spec" → curation_lambda
    /// - "inference" → inference_lambda
    /// - "agent_pod", "connector" → episodic_lambda
    /// - everything else → cybernetics_lambda (safe default)
    ///
    /// Replay events with temporal decay weighting.
    ///
    /// \[P3\] Motivating: Generative Space — replay events with temporal decay
    pub fn replay_weighted(
        &self,
        since: chrono::DateTime<chrono::Utc>,
        limit: u64,
        config: &DecayConfig,
    ) -> Result<Vec<WeightedEvent>, InfrastructureError> {
        let events = self.query_algedonic(since, limit)?;
        let now = chrono::Utc::now();
        let weighted: Vec<WeightedEvent> = events
            .into_iter()
            .filter_map(|event| {
                let delta_secs = (now - event.timestamp).num_seconds() as f64;
                // F-SYN-009: typed dispatch via SpanCategory.
                let lambda = Self::lambda_for(event.span.namespace.category(), config);
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
    /// Returns the decay constant `λ` for a `SpanCategory`.
    ///
    /// Unknown categories fall back to `cybernetics_lambda`.
    /// The fallback is explicit at the type level via `SpanCategory::Unknown`.
    /// Get the decay lambda for a span category.
    ///
    /// \[P3\] Motivating: Generative Space — get decay lambda for category
    pub fn lambda_for(category: SpanCategory, config: &DecayConfig) -> f64 {
        match category {
            SpanCategory::Cybernetics => config.cybernetics_lambda,
            SpanCategory::Curation => config.curation_lambda,
            SpanCategory::Inference => config.inference_lambda,
            SpanCategory::Episodic => config.episodic_lambda,
            SpanCategory::Wallet => config.cybernetics_lambda, // wallet ops are cybernetic (energy budget)
            SpanCategory::Unknown => config.cybernetics_lambda, // safe default
        }
    }
    pub(crate) fn insert(&self, event: &NuEvent) -> Result<(), InfrastructureError> {
        let conn = self.lock_conn()?;
        let (span_category, span_path) = span_to_columns(&event.span);
        conn.execute(
            "INSERT INTO nu_events (id, timestamp, observer_webid, span_category, span_path, phase, observation, regulation, outcome, recursion_depth, parent_event, visibility)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                event.id,
                event.timestamp.to_rfc3339(),
                event.observer_webid,
                span_category,
                span_path,
                event.phase.as_str(),
                serde_json::to_string(&event.observation)?,
                event.regulation.as_ref().and_then(|v| serde_json::to_string(v).ok()),
                event.outcome.as_ref().and_then(|v| serde_json::to_string(v).ok()),
                event.recursion_depth,
                event.parent_event,
                event.visibility,
            ],
        )?;
        Ok(())
    }
    /// Query algedonic-significant events since a given timestamp.
    ///
    /// Returns NuEvents from algedonic span categories (energy, variety,
    /// agent_pod) with `phase = Act` and severity exceeding
    /// threshold, ordered by timestamp ascending. This is the canonical
    /// alerts log that Curation reads via cursor — one fact in one place.
    ///
    /// Per Fowler's Gateway pattern: the NuEvent store is the gateway;
    /// Curation queries it with a cursor, not live CNS state.
    /// Persist a loop cursor value for crash recovery.
    ///
    /// Loop cursors (e.g., `curation_last_review_ms`) track the last-processed
    /// event timestamp. Persisting them ensures the system doesn't re-process
    /// all historical events after a restart.
    /// Persist a cursor value for event replay.
    ///
    /// \[P3\] Motivating: Generative Space — persist replay cursor
    pub fn persist_cursor(&self, key: &str, value: i64) -> Result<(), InfrastructureError> {
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO loop_cursors (key, value, updated_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![key, value, now_rfc3339()],
        )?;
        Ok(())
    }
    /// Load a persisted loop cursor value.
    ///
    /// Returns `Ok(None)` if no cursor has been persisted for the given key
    /// (e.g., first run after schema creation).
    /// Load a persisted cursor value.
    ///
    /// \[P3\] Motivating: Generative Space — load replay cursor
    pub fn load_cursor(&self, key: &str) -> Result<Option<i64>, InfrastructureError> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT value FROM loop_cursors WHERE key = ?1")?;
        let mut rows = stmt.query(rusqlite::params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }
    /// Query algedonic signals from the event store.
    ///
    /// \[P9\] Motivating: Homeostatic Self-Regulation — query algedonic signals
    pub fn query_algedonic(
        &self,
        since: chrono::DateTime<chrono::Utc>,
        limit: u64,
    ) -> Result<Vec<NuEvent>, InfrastructureError> {
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
        let mut stmt = conn.prepare(&sql)?;
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
        let events = collect_rows!(stmt, param_refs.as_slice(), row_to_nu_event);
        Ok(events)
    }
}
/// Reconstruct a NuEvent from a database row.
fn row_to_nu_event(row: &rusqlite::Row<'_>) -> Result<NuEvent, rusqlite::Error> {
    let id: EventID = row.get(0)?;
    let timestamp_str: String = row.get(1)?;
    let observer_webid: WebID = row.get(2)?;
    let span_category: String = row.get(3)?;
    let span_path: String = row.get(4)?;
    let phase_str: String = row.get(5)?;
    let observation_str: String = row.get(6)?;
    let regulation_str: Option<String> = row.get(7)?;
    let outcome_str: Option<String> = row.get(8)?;
    let recursion_depth: u8 = row.get(9)?;
    let parent_event: Option<EventID> = row.get(10)?;
    let visibility: Visibility = row.get(11)?;
    let visibility_str = visibility.as_str().to_string();
    let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
        })?
        .to_utc();
    // Reconstruct Span from stored category + path
    let namespace_str = format!("cns.{}", span_category);
    let namespace = SpanNamespace::parse(&namespace_str).unwrap_or_else(|| {
        // Fallback: use the stored category directly as a non-canonical namespace.
        // This shouldn't happen for canonical events, but provides graceful degradation.
        SpanNamespace::new("cns.gas") // safe default
    });
    // Extract the local path part after the namespace prefix.
    let ns_str = namespace.as_str();
    let local_path = if span_path.starts_with(ns_str)
        && span_path.len() > ns_str.len()
        && span_path.as_bytes().get(ns_str.len()) == Some(&b'.')
    {
        &span_path[ns_str.len() + 1..]
    } else {
        // Fallback: use the raw path without namespace stripping
        span_path.as_str()
    };
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
        visibility: visibility_str,
    })
}
fn span_to_columns(span: &Span) -> (&str, &str) {
    (span.namespace.short_name(), span.path.as_str())
}
impl NuEventSink for NuEventStore {
    fn persist(&self, event: &NuEvent) -> Result<(), InfrastructureError> {
        self.insert(event)
    }
}
#[cfg(test)]
mod tests {
    use hkask_types::event::{Span, SpanNamespace};
    //
    // Before fix, `span_path[namespace.as_str().len() + 1..]` was an unconditional
    // slice that panicked when span_path did not start with the namespace prefix
    // (e.g., when the fallback namespace "cns.gas" was used but span_path is just
    // "depleted").
    #[test]
    fn local_path_extraction_does_not_panic_on_short_span_path() {
        // Simulate a span_path that is shorter than the namespace prefix.
        let namespace = SpanNamespace::new("cns.gas");
        let ns_str = namespace.as_str();
        let span_path = "depleted"; // does NOT start with "cns.gas"
        let local_path = if span_path.starts_with(ns_str)
            && span_path.len() > ns_str.len()
            && span_path.as_bytes().get(ns_str.len()) == Some(&b'.')
        {
            &span_path[ns_str.len() + 1..]
        } else {
            span_path
        };
        assert_eq!(
            local_path, "depleted",
            "fallback should return raw path when prefix doesn't match"
        );
    }
    #[test]
    fn local_path_extraction_does_not_panic_on_exact_namespace_match() {
        let namespace = SpanNamespace::new("cns.gas");
        let ns_str = namespace.as_str();
        let span_path = "cns.gas"; // exactly the namespace, no local component
        let local_path = if span_path.starts_with(ns_str)
            && span_path.len() > ns_str.len()
            && span_path.as_bytes().get(ns_str.len()) == Some(&b'.')
        {
            &span_path[ns_str.len() + 1..]
        } else {
            span_path
        };
        assert_eq!(
            local_path, "cns.gas",
            "fallback should return raw path when no dot follows namespace"
        );
    }
    #[test]
    fn local_path_extraction_succeeds_on_well_formed_path() {
        let namespace = SpanNamespace::new("cns.gas");
        let ns_str = namespace.as_str();
        let span_path = "cns.gas.depleted";
        let local_path = if span_path.starts_with(ns_str)
            && span_path.len() > ns_str.len()
            && span_path.as_bytes().get(ns_str.len()) == Some(&b'.')
        {
            &span_path[ns_str.len() + 1..]
        } else {
            span_path
        };
        assert_eq!(local_path, "depleted");
        let _ = Span::new(namespace, local_path);
    }
}
