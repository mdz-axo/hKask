//! NuEventStore — Persistent storage for CNS ν-events

use chrono::{DateTime, Utc};
use hkask_types::{EventID, NuEvent, Phase, Span, WebID};
use rusqlite::Connection;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum NuEventError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub struct NuEventStore {
    conn: Arc<Mutex<Connection>>,
}

impl NuEventStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn insert(&self, event: &NuEvent) -> Result<(), NuEventError> {
        let conn = self.conn.lock().unwrap();
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

    pub fn query_by_span(
        &self,
        span_category: &str,
        limit: usize,
    ) -> Result<Vec<NuEvent>, NuEventError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, observer_webid, span_category, span_path, phase, observation, regulation, outcome, recursion_depth, parent_event, visibility
             FROM nu_events
             WHERE span_category = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let events = stmt
            .query_map(rusqlite::params![span_category, limit as i64], |row| {
                Ok(NuEventRow {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    observer_webid: row.get(2)?,
                    span_category: row.get(3)?,
                    span_path: row.get(4)?,
                    phase: row.get(5)?,
                    observation: row.get(6)?,
                    regulation: row.get(7)?,
                    outcome: row.get(8)?,
                    recursion_depth: row.get(9)?,
                    parent_event: row.get(10)?,
                    visibility: row.get(11)?,
                })
            })?
            .filter_map(|r| r.ok())
            .filter_map(|row| row_to_event(row).ok())
            .collect();

        Ok(events)
    }

    pub fn query_by_observer(
        &self,
        observer: &WebID,
        limit: usize,
    ) -> Result<Vec<NuEvent>, NuEventError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, observer_webid, span_category, span_path, phase, observation, regulation, outcome, recursion_depth, parent_event, visibility
             FROM nu_events
             WHERE observer_webid = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let events = stmt
            .query_map(
                rusqlite::params![observer.0.to_string(), limit as i64],
                |row| {
                    Ok(NuEventRow {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        observer_webid: row.get(2)?,
                        span_category: row.get(3)?,
                        span_path: row.get(4)?,
                        phase: row.get(5)?,
                        observation: row.get(6)?,
                        regulation: row.get(7)?,
                        outcome: row.get(8)?,
                        recursion_depth: row.get(9)?,
                        parent_event: row.get(10)?,
                        visibility: row.get(11)?,
                    })
                },
            )?
            .filter_map(|r| r.ok())
            .filter_map(|row| row_to_event(row).ok())
            .collect();

        Ok(events)
    }

    pub fn query_recent(&self, limit: usize) -> Result<Vec<NuEvent>, NuEventError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, observer_webid, span_category, span_path, phase, observation, regulation, outcome, recursion_depth, parent_event, visibility
             FROM nu_events
             ORDER BY timestamp DESC
             LIMIT ?1",
        )?;

        let events = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok(NuEventRow {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    observer_webid: row.get(2)?,
                    span_category: row.get(3)?,
                    span_path: row.get(4)?,
                    phase: row.get(5)?,
                    observation: row.get(6)?,
                    regulation: row.get(7)?,
                    outcome: row.get(8)?,
                    recursion_depth: row.get(9)?,
                    parent_event: row.get(10)?,
                    visibility: row.get(11)?,
                })
            })?
            .filter_map(|r| r.ok())
            .filter_map(|row| row_to_event(row).ok())
            .collect();

        Ok(events)
    }

    pub fn prune_older_than(&self, cutoff: DateTime<Utc>) -> Result<usize, NuEventError> {
        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute(
            "DELETE FROM nu_events WHERE timestamp < ?1",
            rusqlite::params![cutoff.to_rfc3339()],
        )?;
        Ok(deleted)
    }

    pub fn count(&self) -> Result<usize, NuEventError> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM nu_events", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}

fn span_to_columns(span: &Span) -> (&'static str, &str) {
    match span {
        Span::Prompt(s) => ("prompt", s.as_str()),
        Span::Tool(s) => ("tool", s.as_str()),
        Span::AgentPod(s) => ("agent_pod", s.as_str()),
        Span::Connector(s) => ("connector", s.as_str()),
        Span::Pipeline(s) => ("pipeline", s.as_str()),
        Span::Energy(s) => ("energy", s.as_str()),
        Span::Review(s) => ("review", s.as_str()),
        Span::Sovereignty(s) => ("sovereignty", s.as_str()),
        Span::Goal(s) => ("goal", s.as_str()),
        Span::Spec(s) => ("spec", s.as_str()),
    }
}

fn span_from_columns(category: &str, path: &str) -> Span {
    match category {
        "prompt" => Span::Prompt(path.to_string()),
        "tool" => Span::Tool(path.to_string()),
        "agent_pod" => Span::AgentPod(path.to_string()),
        "connector" => Span::Connector(path.to_string()),
        "pipeline" => Span::Pipeline(path.to_string()),
        "energy" => Span::Energy(path.to_string()),
        "review" => Span::Review(path.to_string()),
        "sovereignty" => Span::Sovereignty(path.to_string()),
        "goal" => Span::Goal(path.to_string()),
        "spec" => Span::Spec(path.to_string()),
        _ => Span::Tool(path.to_string()),
    }
}

fn row_to_event(row: NuEventRow) -> Result<NuEvent, NuEventError> {
    let id = EventID(Uuid::parse_str(&row.id).unwrap_or_else(|_| Uuid::new_v4()));
    let timestamp = DateTime::parse_from_rfc3339(&row.timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let observer_webid =
        WebID(Uuid::parse_str(&row.observer_webid).unwrap_or_else(|_| Uuid::new_v4()));
    let span = span_from_columns(&row.span_category, &row.span_path);
    let phase = match row.phase.as_str() {
        "regulate" => Phase::Regulate,
        "outcome" => Phase::Outcome,
        _ => Phase::Observe,
    };
    let observation: Value = serde_json::from_str(&row.observation)?;
    let regulation: Option<Value> = row.regulation.and_then(|s| serde_json::from_str(&s).ok());
    let outcome: Option<Value> = row.outcome.and_then(|s| serde_json::from_str(&s).ok());
    let parent_event = row
        .parent_event
        .and_then(|s| Uuid::parse_str(&s).ok())
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
        recursion_depth: row.recursion_depth,
        parent_event,
        visibility: row.visibility,
    })
}

struct NuEventRow {
    id: String,
    timestamp: String,
    observer_webid: String,
    span_category: String,
    span_path: String,
    phase: String,
    observation: String,
    regulation: Option<String>,
    outcome: Option<String>,
    recursion_depth: u8,
    parent_event: Option<String>,
    visibility: String,
}

impl hkask_types::NuEventSink for NuEventStore {
    fn persist(&self, event: &NuEvent) -> Result<(), String> {
        self.insert(event).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::Span;

    fn test_store() -> NuEventStore {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS nu_events (id TEXT PRIMARY KEY, timestamp TEXT NOT NULL, observer_webid TEXT NOT NULL, span_category TEXT NOT NULL, span_path TEXT NOT NULL, phase TEXT NOT NULL, observation TEXT NOT NULL, regulation TEXT, outcome TEXT, recursion_depth INTEGER NOT NULL, parent_event TEXT, visibility TEXT NOT NULL DEFAULT 'private');",
        ).unwrap();
        NuEventStore::new(Arc::new(Mutex::new(conn)))
    }

    #[test]
    fn test_insert_and_query() {
        let store = test_store();
        let event = NuEvent::new(
            WebID::new(),
            Span::tool("test"),
            Phase::Observe,
            serde_json::json!({"test": true}),
            0,
        );

        store.insert(&event).unwrap();
        let results = store.query_recent(10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].span.as_str(), event.span.as_str());
    }

    #[test]
    fn test_prune_older_than() {
        let store = test_store();
        let old_event = NuEvent::new(
            WebID::new(),
            Span::tool("old"),
            Phase::Observe,
            serde_json::json!({"old": true}),
            0,
        );
        store.insert(&old_event).unwrap();

        let cutoff = Utc::now() + chrono::Duration::hours(1);
        let deleted = store.prune_older_than(cutoff).unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_query_by_span() {
        let store = test_store();
        let e1 = NuEvent::new(
            WebID::new(),
            Span::tool("a"),
            Phase::Observe,
            serde_json::json!({}),
            0,
        );
        let e2 = NuEvent::new(
            WebID::new(),
            Span::prompt("b"),
            Phase::Observe,
            serde_json::json!({}),
            0,
        );
        let e3 = NuEvent::new(
            WebID::new(),
            Span::tool("c"),
            Phase::Observe,
            serde_json::json!({}),
            0,
        );
        store.insert(&e1).unwrap();
        store.insert(&e2).unwrap();
        store.insert(&e3).unwrap();

        let tool_events = store.query_by_span("tool", 10).unwrap();
        assert_eq!(tool_events.len(), 2);
    }
}
