//! NuEventStore — Persistent storage for CNS ν-events

use hkask_types::event::{Span, SpanCategory};
use hkask_types::{InfrastructureError, NuEvent, NuEventSink};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum NuEventError {
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

impl NuEventSink for NuEventStore {
    fn persist(&self, event: &NuEvent) -> Result<(), InfrastructureError> {
        self.insert(event).map_err(|e| match e {
            NuEventError::Infra(infra) => infra,
        })
    }
}
