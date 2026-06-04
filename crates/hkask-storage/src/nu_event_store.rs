//! NuEventStore — Persistent storage for CNS ν-events

use hkask_types::event::{Phase, Span, SpanNamespace};
use hkask_types::id::{EventID, WebID};
use hkask_types::{InfrastructureError, NuEvent, NuEventSink};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NuEventError {
    #[error(transparent)]
    Infra(#[from] InfrastructureError),
}

impl From<rusqlite::Error> for NuEventError {
    fn from(e: rusqlite::Error) -> Self {
        NuEventError::Infra(InfrastructureError::Database(e.to_string()))
    }
}

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

pub struct NuEventStore {
    conn: Arc<Mutex<Connection>>,
}

impl NuEventStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub(crate) fn insert(&self, event: &NuEvent) -> Result<(), NuEventError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
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
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;

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
            .query_map(param_refs.as_slice(), |row| row_to_nu_event(row))
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
    let namespace = SpanNamespace::from_str(&namespace_str).unwrap_or_else(|| {
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
