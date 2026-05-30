//! NuEventStore — Persistent storage for CNS ν-events

use chrono::{DateTime, Utc};
use hkask_types::event::{Span, SpanCategory};
use hkask_types::{
    EventID, InfrastructureError, NuEvent, NuEventSink, NuEventSinkError, Phase, WebID,
};
use rusqlite::Connection;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

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

pub struct NuEventStore {
    conn: Arc<Mutex<Connection>>,
}

impl NuEventStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn insert(&self, event: &NuEvent) -> Result<(), NuEventError> {
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

    pub fn query_by_span(
        &self,
        span_category: &str,
        limit: usize,
    ) -> Result<Vec<NuEvent>, NuEventError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
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
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
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
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
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
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let deleted = conn.execute(
            "DELETE FROM nu_events WHERE timestamp < ?1",
            rusqlite::params![cutoff.to_rfc3339()],
        )?;
        Ok(deleted)
    }

    pub fn count(&self) -> Result<usize, NuEventError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| InfrastructureError::LockPoisoned)?;
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM nu_events", [], |row| row.get(0))?;
        Ok(count as usize)
    }
}

fn span_to_columns(span: &Span) -> (&'static str, &str) {
    let category_str = match span.category {
        SpanCategory::Prompt => "prompt",
        SpanCategory::Tool => "tool",
        SpanCategory::AgentPod => "agent_pod",
        SpanCategory::Connector => "connector",
        SpanCategory::Pipeline => "pipeline",
        SpanCategory::Energy => "energy",
        SpanCategory::Review => "review",
        SpanCategory::Template => "template",
        SpanCategory::Curation => "curation",
        SpanCategory::Variety => "variety",
        SpanCategory::KillZone => "killzone",
        SpanCategory::Sovereignty => "sovereignty",
        SpanCategory::Goal => "goal",
        SpanCategory::Spec => "spec",
    };
    (category_str, span.path.as_str())
}

fn span_from_columns(category: &str, path: &str) -> Span {
    let cat = SpanCategory::parse_str(category).unwrap_or(SpanCategory::Tool);
    Span {
        category: cat,
        path: path.to_string(),
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

impl NuEventSink for NuEventStore {
    fn persist(&self, event: &NuEvent) -> Result<(), NuEventSinkError> {
        self.insert(event).map_err(|e| match e {
            NuEventError::Infra(InfrastructureError::Database(msg)) => {
                NuEventSinkError::Database(msg)
            }
            NuEventError::Infra(InfrastructureError::Serialization(msg)) => {
                NuEventSinkError::Serialization(msg)
            }
            NuEventError::Infra(InfrastructureError::LockPoisoned) => {
                NuEventSinkError::Database("lock poisoned".to_string())
            }
            NuEventError::Infra(InfrastructureError::NotFound(msg)) => {
                NuEventSinkError::Database(msg)
            }
            NuEventError::Infra(InfrastructureError::Io(msg)) => NuEventSinkError::Database(msg),
        })
    }
}
