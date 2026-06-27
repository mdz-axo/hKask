//! 7R7 Communication Listener — the seven passive receptor bots.
//!
//! The 7R7 framework observes hKask's internal state and external Matrix
//! communication through seven specialized receptors. Every receptor is a
//! **dumb pipe** — it observes a specific domain, emits CNS spans, and
//! **never classifies, escalates, moderates, or judges** content. Those
//! decisions belong to the agent layer (Curator + skills + templates).
//!
//! ## Receptor Inventory
//!
//! | # | Receptor | Role | Data Source | CNS Target |
//! |---|----------|------|-------------|------------|
//! | r7-1 | Observer | Matrix room messages | MatrixTransport | `cns.communication.message.observed` |
//! | r7-2 | Variety | System variety balance (Ashby) | CNS event store | `cns.variety.observed` |
//! | r7-3 | Algedonic | Pain/pleasure signal patterns | CNS event store | `cns.algedonic.observed` |
//! | r7-4 | Composer | Skill/template composition health | CNS event store | `cns.composer.observed` |
//! | r7-5 | Consolidator | Memory consolidation patterns | CNS event store | `cns.consolidation.observed` |
//! | r7-6 | Cybernetics | CNS meta-health (regulator of regulators) | CNS event store | `cns.cybernetics.observed` |
//! | r7-7 | Curator | Curator activity (observer of the decider) | CNS event store | `cns.curation.observed` |
//!
//! Architecture:
//!   Data sources → 7R7 receptor poll → CNS span emission → agent layer (Curator)
//!
//! The communication server is a dumb pipe. CNS observes. Agents decide.

use crate::matrix::MatrixTransport;
use hkask_types::WebID;
use hkask_types::event::{NuEvent, NuEventSink, Phase, Span};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock, watch};

// ── Receptor data abstraction ───────────────────────────────────────────────

/// Abstraction over CNS event storage for receptor observation.
///
/// Receptors query CNS events to observe system state. This trait decouples
/// them from the concrete `NuEventStore` in `hkask-storage`, keeping
/// `hkask-communication` free of storage dependencies.
///
/// Implemented by the service layer with `NuEventStore::query_algedonic()`
/// and related query methods.
pub trait ReceptorStore: Send + Sync {
    /// Query CNS events since a timestamp, filtered by span prefix.
    ///
    /// `span_prefix` matches against the span's `short_name()` — e.g.,
    /// `"communication."` matches `"communication.message.observed"`.
    ///
    /// expect: "Agents communicate through user-owned channels"
    /// pre:  since is a valid UTC timestamp
    /// pre:  span_prefix is non-empty
    /// pre:  limit > 0
    /// post: returns events ordered by timestamp ASC
    fn query_spans(
        &self,
        since: chrono::DateTime<chrono::Utc>,
        span_prefix: &str,
        limit: u64,
    ) -> Result<Vec<NuEvent>, Box<dyn std::error::Error + Send + Sync>>;
}

// ── 7R7 Listener (r7-1: Observer) ───────────────────────────────────────────

/// 7R7 communication listener — polls Matrix for messages, emits CNS spans.
///
/// This is a passive observer. It does not classify, escalate, or moderate.
/// Content decisions are made by the agent layer (Curator + skills + templates).
pub struct SevenR7Listener {
    /// Matrix transport for polling (Mutex-wrapped for shared &mut access).
    matrix: Arc<Mutex<MatrixTransport>>,
    /// Polling interval in seconds.
    poll_interval_secs: u64,
    /// CNS event sink for persisting observed messages as NuEvents.
    event_sink: Option<Arc<dyn NuEventSink>>,
    /// Whether the listener is active.
    active: RwLock<bool>,
    /// Cancellation channel — dropping the sender (via stop) signals the loop to exit.
    cancel_tx: RwLock<Option<watch::Sender<bool>>>,
}

impl SevenR7Listener {
    /// Create a new 7R7 listener.
    ///
    /// expect: "Agents communicate through user-owned channels"
    /// pre:  matrix is a valid MatrixTransport (authenticated)
    /// pre:  poll_interval_secs > 0
    /// post: returns SevenR7Listener with active=false
    pub fn new(matrix: Arc<Mutex<MatrixTransport>>, poll_interval_secs: u64) -> Self {
        Self {
            matrix,
            poll_interval_secs,
            event_sink: None,
            active: RwLock::new(false),
            cancel_tx: RwLock::new(None),
        }
    }

    /// Attach a CNS event sink for persisting observed messages.
    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Start the polling loop.
    ///
    /// expect: "Agents communicate through user-owned channels"
    /// pre:  matrix transport is authenticated
    /// post: background polling task spawned
    /// post: idempotent — calling start() on already-active listener is no-op
    pub async fn start(&self) {
        let was_active = *self.active.read().await;
        if was_active {
            return;
        }
        *self.active.write().await = true;

        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        *self.cancel_tx.write().await = Some(cancel_tx);

        let matrix = Arc::clone(&self.matrix);
        let interval = self.poll_interval_secs;
        let event_sink = self.event_sink.clone();

        tokio::spawn(async move {
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(interval));
            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        let rooms = {
                            let transport = matrix.lock().await;
                            match transport.list_rooms().await {
                                Ok(r) => r,
                                Err(e) => {
                                    tracing::warn!(
                                        target: "cns.communication.listener",
                                        error = %e,
                                        "7R7 failed to list rooms"
                                    );
                                    continue;
                                }
                            }
                        };

                        for room in &rooms {
                            let room_id = room.room_id.as_str();
                            let transport = matrix.lock().await;
                            match transport.get_messages(&room.room_id, 10).await {
                                Ok(messages) => {
                                    for msg in &messages {
                                        tracing::info!(
                                            target: "cns.communication.message.observed",
                                            room_id = %room_id,
                                            sender = %msg.sender.as_str(),
                                            body_len = %msg.body.len(),
                                            "7R7 r7-1 observed message"
                                        );
                                        if let Some(ref sink) = event_sink {
                                            let span = Span::new(
                                                hkask_types::event::SpanNamespace::new("cns.communication.message"),
                                                "observed",
                                            );
                                            let event = NuEvent::new(
                                                WebID::from_persona(b"r7-1-observer"),
                                                span,
                                                Phase::Act,
                                                serde_json::json!({
                                                    "room_id": room_id,
                                                    "sender": msg.sender.as_str(),
                                                    "body": msg.body,
                                                    "timestamp": msg.timestamp,
                                                }),
                                                0,
                                            );
                                            if let Err(e) = sink.persist(&event) {
                                                tracing::warn!(
                                                    target: "cns.communication.listener",
                                                    error = %e,
                                                    "r7-1 failed to persist NuEvent"
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::debug!(
                                        target: "cns.communication.listener",
                                        room_id = %room_id,
                                        error = %e,
                                        "r7-1 failed to poll room"
                                    );
                                }
                            }
                        }
                    }
                    _ = cancel_rx.changed() => {
                        tracing::info!(target: "cns.communication.listener", "r7-1 listener stopped");
                        break;
                    }
                }
            }
        });

        tracing::info!(
            target: "cns.communication.listener.started",
            interval_secs = %interval,
            "r7-1 listener started"
        );
    }

    /// Stop the polling loop.
    ///
    /// expect: "Agents communicate through user-owned channels"
    /// post: active flag set to false
    /// post: idempotent — calling stop() on already-stopped listener is no-op
    pub async fn stop(&self) {
        *self.active.write().await = false;
        *self.cancel_tx.write().await = None;
        tracing::info!(target: "cns.communication.listener.stopped", "r7-1 listener stopped");
    }
}

// ── r7-2: Variety Receptor ──────────────────────────────────────────────────

/// Variety receptor — observes system variety balance (Ashby's Law).
///
/// Variety = number of distinguishable states. If unhandled variety exceeds
/// regulatory variety, the system becomes ungovernable. This receptor tracks
/// queue depths, alert counts, and variety EMA to detect variety deficits.
///
/// CNS target: `cns.variety.observed`
pub struct VarietyReceptor {
    /// CNS event sink for persisting observation spans.
    event_sink: Option<Arc<dyn NuEventSink>>,
    /// CNS event store abstraction for querying alert/queue state.
    store: Option<Arc<dyn ReceptorStore>>,
    /// Polling interval in seconds.
    poll_interval_secs: u64,
    active: RwLock<bool>,
    cancel_tx: RwLock<Option<watch::Sender<bool>>>,
}

impl VarietyReceptor {
    /// expect: "The system provides cybernetic observability through CNS spans"
    /// pre:  poll_interval_secs > 0
    /// post: returns VarietyReceptor with active=false
    pub fn new(poll_interval_secs: u64) -> Self {
        Self {
            event_sink: None,
            store: None,
            poll_interval_secs,
            active: RwLock::new(false),
            cancel_tx: RwLock::new(None),
        }
    }

    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    pub fn with_store(mut self, store: Arc<dyn ReceptorStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Start the variety observation loop.
    ///
    /// expect: "The system provides cybernetic observability through CNS spans"
    /// pre:  store is attached (with_store called)
    /// post: background polling task spawned
    /// post: idempotent
    pub async fn start(&self) {
        let was_active = *self.active.read().await;
        if was_active {
            return;
        }
        *self.active.write().await = true;

        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        *self.cancel_tx.write().await = Some(cancel_tx);

        let interval = self.poll_interval_secs;
        let event_sink = self.event_sink.clone();
        let store = self.store.clone();

        tokio::spawn(async move {
            let mut last_seen = chrono::Utc::now();
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(interval));
            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        let since = last_seen;
                        let store = match &store {
                            Some(s) => s,
                            None => {
                                tracing::debug!(
                                    target: "cns.variety.receptor",
                                    "r7-2: no store attached — skipping cycle"
                                );
                                last_seen = chrono::Utc::now();
                                continue;
                            }
                        };

                        // Query recent algedonic spans to gauge system variety load
                        match store.query_spans(since, "algedonic.", 500) {
                            Ok(events) => {
                                let warning_count = events.iter()
                                    .filter(|e| {
                                        e.observation.get("severity")
                                            .and_then(|s| s.as_str())
                                            == Some("warning")
                                    })
                                    .count();
                                let critical_count = events.iter()
                                    .filter(|e| {
                                        e.observation.get("severity")
                                            .and_then(|s| s.as_str())
                                            == Some("critical")
                                    })
                                    .count();

                                let observation = serde_json::json!({
                                    "receptor": "r7-2-variety",
                                    "total_alerts_observed": events.len(),
                                    "warning_count": warning_count,
                                    "critical_count": critical_count,
                                    "window_secs": (chrono::Utc::now() - since).num_seconds(),
                                });

                                tracing::info!(
                                    target: "cns.variety.observed",
                                    alerts = %events.len(),
                                    warnings = %warning_count,
                                    criticals = %critical_count,
                                    "r7-2 variety observation cycle"
                                );

                                if let Some(ref sink) = event_sink {
                                    let span = Span::new(
                                        hkask_types::event::SpanNamespace::new("cns.variety"),
                                        "observed",
                                    );
                                    let event = NuEvent::new(
                                        WebID::from_persona(b"r7-2-variety"),
                                        span,
                                        Phase::Act,
                                        observation,
                                        0,
                                    );
                                    if let Err(e) = sink.persist(&event) {
                                        tracing::warn!(
                                            target: "cns.variety.receptor",
                                            error = %e,
                                            "r7-2 failed to persist NuEvent"
                                        );
                                    }
                                }

                                if let Some(latest) = events.last() {
                                    last_seen = latest.timestamp;
                                } else {
                                    last_seen = chrono::Utc::now();
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    target: "cns.variety.receptor",
                                    error = %e,
                                    "r7-2 failed to query spans"
                                );
                                last_seen = chrono::Utc::now();
                            }
                        }
                    }
                    _ = cancel_rx.changed() => {
                        tracing::info!(target: "cns.variety.receptor", "r7-2 variety receptor stopped");
                        break;
                    }
                }
            }
        });

        tracing::info!(
            target: "cns.variety.receptor.started",
            interval_secs = %interval,
            "r7-2 variety receptor started"
        );
    }

    /// Stop the observation loop.
    ///
    /// post: idempotent
    pub async fn stop(&self) {
        *self.active.write().await = false;
        *self.cancel_tx.write().await = None;
        tracing::info!(target: "cns.variety.receptor.stopped", "r7-2 variety receptor stopped");
    }
}

// ── r7-3: Algedonic Receptor ────────────────────────────────────────────────

/// Algedonic receptor — observes pain/pleasure signal patterns.
///
/// Algedonic = the system's feeling layer. Pain = warning + critical alerts.
/// Pleasure = resolved alerts, healthy state transitions. This receptor tracks
/// alert severity distributions and resolution rates.
///
/// CNS target: `cns.algedonic.observed`
pub struct AlgedonicReceptor {
    event_sink: Option<Arc<dyn NuEventSink>>,
    store: Option<Arc<dyn ReceptorStore>>,
    poll_interval_secs: u64,
    active: RwLock<bool>,
    cancel_tx: RwLock<Option<watch::Sender<bool>>>,
}

impl AlgedonicReceptor {
    /// expect: "The system provides cybernetic observability through CNS spans"
    /// pre:  poll_interval_secs > 0
    /// post: returns AlgedonicReceptor with active=false
    pub fn new(poll_interval_secs: u64) -> Self {
        Self {
            event_sink: None,
            store: None,
            poll_interval_secs,
            active: RwLock::new(false),
            cancel_tx: RwLock::new(None),
        }
    }

    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    pub fn with_store(mut self, store: Arc<dyn ReceptorStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Start the algedonic observation loop.
    ///
    /// expect: "The system provides cybernetic observability through CNS spans"
    /// pre:  store is attached
    /// post: background polling task spawned
    /// post: idempotent
    pub async fn start(&self) {
        let was_active = *self.active.read().await;
        if was_active {
            return;
        }
        *self.active.write().await = true;

        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        *self.cancel_tx.write().await = Some(cancel_tx);

        let interval = self.poll_interval_secs;
        let event_sink = self.event_sink.clone();
        let store = self.store.clone();

        tokio::spawn(async move {
            let mut last_seen = chrono::Utc::now();
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(interval));
            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        let since = last_seen;
                        let store = match &store {
                            Some(s) => s,
                            None => {
                                tracing::debug!(
                                    target: "cns.algedonic.receptor",
                                    "r7-3: no store attached — skipping cycle"
                                );
                                last_seen = chrono::Utc::now();
                                continue;
                            }
                        };

                        // Query recent algedonic and circuit-breaker spans
                        let mut all_events = Vec::new();
                        for prefix in &["algedonic.", "circuit."] {
                            match store.query_spans(since, prefix, 500) {
                                Ok(events) => all_events.extend(events),
                                Err(e) => {
                                    tracing::warn!(
                                        target: "cns.algedonic.receptor",
                                        prefix = %prefix,
                                        error = %e,
                                        "r7-3 query failed"
                                    );
                                }
                            }
                        }

                        let warning_count = all_events.iter()
                            .filter(|e| {
                                e.observation.get("severity")
                                    .and_then(|s| s.as_str())
                                    == Some("warning")
                            })
                            .count();
                        let critical_count = all_events.iter()
                            .filter(|e| {
                                e.observation.get("severity")
                                    .and_then(|s| s.as_str())
                                    == Some("critical")
                            })
                            .count();
                        let resolved_count = all_events.iter()
                            .filter(|e| {
                                e.observation.get("severity")
                                    .and_then(|s| s.as_str())
                                    == Some("resolved")
                            })
                            .count();

                        let observation = serde_json::json!({
                            "receptor": "r7-3-algedonic",
                            "total_signals": all_events.len(),
                            "warning_count": warning_count,
                            "critical_count": critical_count,
                            "resolved_count": resolved_count,
                            "window_secs": (chrono::Utc::now() - since).num_seconds(),
                        });

                        tracing::info!(
                            target: "cns.algedonic.observed",
                            signals = %all_events.len(),
                            warnings = %warning_count,
                            criticals = %critical_count,
                            resolved = %resolved_count,
                            "r7-3 algedonic observation cycle"
                        );

                        if let Some(ref sink) = event_sink {
                            let span = Span::new(
                                hkask_types::event::SpanNamespace::new("cns.algedonic"),
                                "observed",
                            );
                            let event = NuEvent::new(
                                WebID::from_persona(b"r7-3-algedonic"),
                                span,
                                Phase::Act,
                                observation,
                                0,
                            );
                            if let Err(e) = sink.persist(&event) {
                                tracing::warn!(
                                    target: "cns.algedonic.receptor",
                                    error = %e,
                                    "r7-3 failed to persist NuEvent"
                                );
                            }
                        }

                        last_seen = match all_events.last() {
                            Some(e) => e.timestamp,
                            None => chrono::Utc::now(),
                        };
                    }
                    _ = cancel_rx.changed() => {
                        tracing::info!(target: "cns.algedonic.receptor", "r7-3 algedonic receptor stopped");
                        break;
                    }
                }
            }
        });

        tracing::info!(
            target: "cns.algedonic.receptor.started",
            interval_secs = %interval,
            "r7-3 algedonic receptor started"
        );
    }

    pub async fn stop(&self) {
        *self.active.write().await = false;
        *self.cancel_tx.write().await = None;
        tracing::info!(target: "cns.algedonic.receptor.stopped", "r7-3 algedonic receptor stopped");
    }
}

// ── r7-4: Composer Receptor ─────────────────────────────────────────────────

/// Composer receptor — observes skill/template/bundle composition health.
///
/// Tracks skill activations, template execution outcomes, contract violations,
/// and composition drift patterns. Watches for composition failures that
/// indicate degradation in the generative space (P3).
///
/// CNS target: `cns.composer.observed`
pub struct ComposerReceptor {
    event_sink: Option<Arc<dyn NuEventSink>>,
    store: Option<Arc<dyn ReceptorStore>>,
    poll_interval_secs: u64,
    active: RwLock<bool>,
    cancel_tx: RwLock<Option<watch::Sender<bool>>>,
}

impl ComposerReceptor {
    /// expect: "The system provides cybernetic observability through CNS spans"
    /// pre:  poll_interval_secs > 0
    /// post: returns ComposerReceptor with active=false
    pub fn new(poll_interval_secs: u64) -> Self {
        Self {
            event_sink: None,
            store: None,
            poll_interval_secs,
            active: RwLock::new(false),
            cancel_tx: RwLock::new(None),
        }
    }

    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    pub fn with_store(mut self, store: Arc<dyn ReceptorStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Start the composition observation loop.
    ///
    /// expect: "The system provides cybernetic observability through CNS spans"
    /// pre:  store is attached
    /// post: background polling task spawned
    /// post: idempotent
    pub async fn start(&self) {
        let was_active = *self.active.read().await;
        if was_active {
            return;
        }
        *self.active.write().await = true;

        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        *self.cancel_tx.write().await = Some(cancel_tx);

        let interval = self.poll_interval_secs;
        let event_sink = self.event_sink.clone();
        let store = self.store.clone();

        tokio::spawn(async move {
            let mut last_seen = chrono::Utc::now();
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(interval));
            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        let since = last_seen;
                        let store = match &store {
                            Some(s) => s,
                            None => {
                                tracing::debug!(
                                    target: "cns.composer.receptor",
                                    "r7-4: no store attached — skipping cycle"
                                );
                                last_seen = chrono::Utc::now();
                                continue;
                            }
                        };

                        // Query skill, template, and contract spans
                        let mut all_events = Vec::new();
                        for prefix in &["skill.", "spec.", "contract."] {
                            match store.query_spans(since, prefix, 500) {
                                Ok(events) => all_events.extend(events),
                                Err(e) => {
                                    tracing::warn!(
                                        target: "cns.composer.receptor",
                                        prefix = %prefix,
                                        error = %e,
                                        "r7-4 query failed"
                                    );
                                }
                            }
                        }

                        let skill_events = all_events.iter()
                            .filter(|e| e.span.namespace.short_name().starts_with("skill."))
                            .count();
                        let spec_events = all_events.iter()
                            .filter(|e| e.span.namespace.short_name().starts_with("spec."))
                            .count();
                        let contract_violations = all_events.iter()
                            .filter(|e| e.span.namespace.short_name().starts_with("contract."))
                            .count();

                        let observation = serde_json::json!({
                            "receptor": "r7-4-composer",
                            "total_composition_events": all_events.len(),
                            "skill_events": skill_events,
                            "spec_events": spec_events,
                            "contract_violations": contract_violations,
                            "window_secs": (chrono::Utc::now() - since).num_seconds(),
                        });

                        tracing::info!(
                            target: "cns.composer.observed",
                            total = %all_events.len(),
                            skills = %skill_events,
                            specs = %spec_events,
                            violations = %contract_violations,
                            "r7-4 composer observation cycle"
                        );

                        if let Some(ref sink) = event_sink {
                            let span = Span::new(
                                hkask_types::event::SpanNamespace::new("cns.composer"),
                                "observed",
                            );
                            let event = NuEvent::new(
                                WebID::from_persona(b"r7-4-composer"),
                                span,
                                Phase::Act,
                                observation,
                                0,
                            );
                            if let Err(e) = sink.persist(&event) {
                                tracing::warn!(
                                    target: "cns.composer.receptor",
                                    error = %e,
                                    "r7-4 failed to persist NuEvent"
                                );
                            }
                        }

                        last_seen = match all_events.last() {
                            Some(e) => e.timestamp,
                            None => chrono::Utc::now(),
                        };
                    }
                    _ = cancel_rx.changed() => {
                        tracing::info!(target: "cns.composer.receptor", "r7-4 composer receptor stopped");
                        break;
                    }
                }
            }
        });

        tracing::info!(
            target: "cns.composer.receptor.started",
            interval_secs = %interval,
            "r7-4 composer receptor started"
        );
    }

    pub async fn stop(&self) {
        *self.active.write().await = false;
        *self.cancel_tx.write().await = None;
        tracing::info!(target: "cns.composer.receptor.stopped", "r7-4 composer receptor stopped");
    }
}

// ── r7-5: Consolidator Receptor ─────────────────────────────────────────────

/// Consolidator receptor — observes memory consolidation patterns.
///
/// Tracks memory encoding rates across episodic (PKO process) and semantic
/// (DC+BIBO state) domains. Watches for consolidation gaps — when one memory
/// axis is being under-encoded relative to the dual-axis mandate (P5.4).
///
/// CNS target: `cns.consolidation.observed`
pub struct ConsolidatorReceptor {
    event_sink: Option<Arc<dyn NuEventSink>>,
    store: Option<Arc<dyn ReceptorStore>>,
    poll_interval_secs: u64,
    active: RwLock<bool>,
    cancel_tx: RwLock<Option<watch::Sender<bool>>>,
}

impl ConsolidatorReceptor {
    /// expect: "The system provides cybernetic observability through CNS spans"
    /// pre:  poll_interval_secs > 0
    /// post: returns ConsolidatorReceptor with active=false
    pub fn new(poll_interval_secs: u64) -> Self {
        Self {
            event_sink: None,
            store: None,
            poll_interval_secs,
            active: RwLock::new(false),
            cancel_tx: RwLock::new(None),
        }
    }

    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    pub fn with_store(mut self, store: Arc<dyn ReceptorStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Start the consolidation observation loop.
    ///
    /// expect: "The system provides cybernetic observability through CNS spans"
    /// pre:  store is attached
    /// post: background polling task spawned
    /// post: idempotent
    pub async fn start(&self) {
        let was_active = *self.active.read().await;
        if was_active {
            return;
        }
        *self.active.write().await = true;

        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        *self.cancel_tx.write().await = Some(cancel_tx);

        let interval = self.poll_interval_secs;
        let event_sink = self.event_sink.clone();
        let store = self.store.clone();

        tokio::spawn(async move {
            let mut last_seen = chrono::Utc::now();
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(interval));
            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        let since = last_seen;
                        let store = match &store {
                            Some(s) => s,
                            None => {
                                tracing::debug!(
                                    target: "cns.consolidation.receptor",
                                    "r7-5: no store attached — skipping cycle"
                                );
                                last_seen = chrono::Utc::now();
                                continue;
                            }
                        };

                        // Query memory spans across both axes
                        let mut all_events = Vec::new();
                        for prefix in &["memory.", "consolidation."] {
                            match store.query_spans(since, prefix, 500) {
                                Ok(events) => all_events.extend(events),
                                Err(e) => {
                                    tracing::warn!(
                                        target: "cns.consolidation.receptor",
                                        prefix = %prefix,
                                        error = %e,
                                        "r7-5 query failed"
                                    );
                                }
                            }
                        }

                        let episodic_count = all_events.iter()
                            .filter(|e| {
                                e.observation.get("memory_domain")
                                    .and_then(|d| d.as_str())
                                    == Some("episodic")
                            })
                            .count();
                        let semantic_count = all_events.iter()
                            .filter(|e| {
                                e.observation.get("memory_domain")
                                    .and_then(|d| d.as_str())
                                    == Some("semantic")
                            })
                            .count();

                        let observation = serde_json::json!({
                            "receptor": "r7-5-consolidator",
                            "total_memory_events": all_events.len(),
                            "episodic_pko_count": episodic_count,
                            "semantic_dc_count": semantic_count,
                            "window_secs": (chrono::Utc::now() - since).num_seconds(),
                        });

                        tracing::info!(
                            target: "cns.consolidation.observed",
                            total = %all_events.len(),
                            episodic = %episodic_count,
                            semantic = %semantic_count,
                            "r7-5 consolidator observation cycle"
                        );

                        if let Some(ref sink) = event_sink {
                            let span = Span::new(
                                hkask_types::event::SpanNamespace::new("cns.consolidation"),
                                "observed",
                            );
                            let event = NuEvent::new(
                                WebID::from_persona(b"r7-5-consolidator"),
                                span,
                                Phase::Act,
                                observation,
                                0,
                            );
                            if let Err(e) = sink.persist(&event) {
                                tracing::warn!(
                                    target: "cns.consolidation.receptor",
                                    error = %e,
                                    "r7-5 failed to persist NuEvent"
                                );
                            }
                        }

                        last_seen = match all_events.last() {
                            Some(e) => e.timestamp,
                            None => chrono::Utc::now(),
                        };
                    }
                    _ = cancel_rx.changed() => {
                        tracing::info!(target: "cns.consolidation.receptor", "r7-5 consolidator receptor stopped");
                        break;
                    }
                }
            }
        });

        tracing::info!(
            target: "cns.consolidation.receptor.started",
            interval_secs = %interval,
            "r7-5 consolidator receptor started"
        );
    }

    pub async fn stop(&self) {
        *self.active.write().await = false;
        *self.cancel_tx.write().await = None;
        tracing::info!(target: "cns.consolidation.receptor.stopped", "r7-5 consolidator receptor stopped");
    }
}

// ── r7-6: Cybernetics Receptor ──────────────────────────────────────────────

/// Cybernetics receptor — meta-observer of the CNS itself.
///
/// The regulator's regulator. Observes whether the CNS is healthy: circuit
/// breaker states, energy budget balance, self-healing operations, and
/// overall CNS health scores. Detects when the regulatory system itself
/// needs attention (regression of regulation).
///
/// CNS target: `cns.cybernetics.observed`
pub struct CyberneticsReceptor {
    event_sink: Option<Arc<dyn NuEventSink>>,
    store: Option<Arc<dyn ReceptorStore>>,
    poll_interval_secs: u64,
    active: RwLock<bool>,
    cancel_tx: RwLock<Option<watch::Sender<bool>>>,
}

impl CyberneticsReceptor {
    /// expect: "The system provides cybernetic observability through CNS spans"
    /// pre:  poll_interval_secs > 0
    /// post: returns CyberneticsReceptor with active=false
    pub fn new(poll_interval_secs: u64) -> Self {
        Self {
            event_sink: None,
            store: None,
            poll_interval_secs,
            active: RwLock::new(false),
            cancel_tx: RwLock::new(None),
        }
    }

    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    pub fn with_store(mut self, store: Arc<dyn ReceptorStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Start the cybernetics observation loop.
    ///
    /// expect: "The system provides cybernetic observability through CNS spans"
    /// pre:  store is attached
    /// post: background polling task spawned
    /// post: idempotent
    pub async fn start(&self) {
        let was_active = *self.active.read().await;
        if was_active {
            return;
        }
        *self.active.write().await = true;

        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        *self.cancel_tx.write().await = Some(cancel_tx);

        let interval = self.poll_interval_secs;
        let event_sink = self.event_sink.clone();
        let store = self.store.clone();

        tokio::spawn(async move {
            let mut last_seen = chrono::Utc::now();
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(interval));
            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        let since = last_seen;
                        let store = match &store {
                            Some(s) => s,
                            None => {
                                tracing::debug!(
                                    target: "cns.cybernetics.receptor",
                                    "r7-6: no store attached — skipping cycle"
                                );
                                last_seen = chrono::Utc::now();
                                continue;
                            }
                        };

                        // Query CNS infrastructure spans — circuit breakers, self-heal, energy
                        let mut all_events = Vec::new();
                        for prefix in &["circuit.", "heal.", "energy.", "cns."] {
                            match store.query_spans(since, prefix, 500) {
                                Ok(events) => all_events.extend(events),
                                Err(e) => {
                                    tracing::warn!(
                                        target: "cns.cybernetics.receptor",
                                        prefix = %prefix,
                                        error = %e,
                                        "r7-6 query failed"
                                    );
                                }
                            }
                        }

                        let circuit_breaker_events = all_events.iter()
                            .filter(|e| e.span.namespace.short_name().starts_with("circuit."))
                            .count();
                        let self_heal_events = all_events.iter()
                            .filter(|e| e.span.namespace.short_name().starts_with("heal."))
                            .count();
                        let energy_events = all_events.iter()
                            .filter(|e| e.span.namespace.short_name().starts_with("energy."))
                            .count();

                        let observation = serde_json::json!({
                            "receptor": "r7-6-cybernetics",
                            "total_cns_infra_events": all_events.len(),
                            "circuit_breaker_events": circuit_breaker_events,
                            "self_heal_events": self_heal_events,
                            "energy_events": energy_events,
                            "window_secs": (chrono::Utc::now() - since).num_seconds(),
                        });

                        tracing::info!(
                            target: "cns.cybernetics.observed",
                            total = %all_events.len(),
                            circuit_breakers = %circuit_breaker_events,
                            self_heals = %self_heal_events,
                            energy = %energy_events,
                            "r7-6 cybernetics observation cycle"
                        );

                        if let Some(ref sink) = event_sink {
                            let span = Span::new(
                                hkask_types::event::SpanNamespace::new("cns.cybernetics"),
                                "observed",
                            );
                            let event = NuEvent::new(
                                WebID::from_persona(b"r7-6-cybernetics"),
                                span,
                                Phase::Act,
                                observation,
                                0,
                            );
                            if let Err(e) = sink.persist(&event) {
                                tracing::warn!(
                                    target: "cns.cybernetics.receptor",
                                    error = %e,
                                    "r7-6 failed to persist NuEvent"
                                );
                            }
                        }

                        last_seen = match all_events.last() {
                            Some(e) => e.timestamp,
                            None => chrono::Utc::now(),
                        };
                    }
                    _ = cancel_rx.changed() => {
                        tracing::info!(target: "cns.cybernetics.receptor", "r7-6 cybernetics receptor stopped");
                        break;
                    }
                }
            }
        });

        tracing::info!(
            target: "cns.cybernetics.receptor.started",
            interval_secs = %interval,
            "r7-6 cybernetics receptor started"
        );
    }

    pub async fn stop(&self) {
        *self.active.write().await = false;
        *self.cancel_tx.write().await = None;
        tracing::info!(target: "cns.cybernetics.receptor.stopped", "r7-6 cybernetics receptor stopped");
    }
}

// ── r7-7: Curator Receptor ──────────────────────────────────────────────────

/// Curator receptor — observes Curator activity.
///
/// The observer of the decider. Tracks metacognition loop iterations,
/// CAT engagement decisions (speak/silent ratio), template execution
/// outcomes, and directives issued. Detects when the Curator itself
/// is stalled, over-engaged, or producing anomalous patterns.
///
/// CNS target: `cns.curation.observed`
pub struct CuratorReceptor {
    event_sink: Option<Arc<dyn NuEventSink>>,
    store: Option<Arc<dyn ReceptorStore>>,
    poll_interval_secs: u64,
    active: RwLock<bool>,
    cancel_tx: RwLock<Option<watch::Sender<bool>>>,
}

impl CuratorReceptor {
    /// expect: "The system provides cybernetic observability through CNS spans"
    /// pre:  poll_interval_secs > 0
    /// post: returns CuratorReceptor with active=false
    pub fn new(poll_interval_secs: u64) -> Self {
        Self {
            event_sink: None,
            store: None,
            poll_interval_secs,
            active: RwLock::new(false),
            cancel_tx: RwLock::new(None),
        }
    }

    pub fn with_event_sink(mut self, sink: Arc<dyn NuEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    pub fn with_store(mut self, store: Arc<dyn ReceptorStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Start the curation observation loop.
    ///
    /// expect: "The system provides cybernetic observability through CNS spans"
    /// pre:  store is attached
    /// post: background polling task spawned
    /// post: idempotent
    pub async fn start(&self) {
        let was_active = *self.active.read().await;
        if was_active {
            return;
        }
        *self.active.write().await = true;

        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        *self.cancel_tx.write().await = Some(cancel_tx);

        let interval = self.poll_interval_secs;
        let event_sink = self.event_sink.clone();
        let store = self.store.clone();

        tokio::spawn(async move {
            let mut last_seen = chrono::Utc::now();
            let mut timer = tokio::time::interval(std::time::Duration::from_secs(interval));
            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        let since = last_seen;
                        let store = match &store {
                            Some(s) => s,
                            None => {
                                tracing::debug!(
                                    target: "cns.curation.receptor",
                                    "r7-7: no store attached — skipping cycle"
                                );
                                last_seen = chrono::Utc::now();
                                continue;
                            }
                        };

                        // Query curation spans
                        match store.query_spans(since, "curation.", 500) {
                            Ok(events) => {
                                let metacognition_cycles = events.iter()
                                    .filter(|e| {
                                        e.observation.get("phase")
                                            .and_then(|p| p.as_str())
                                            == Some("metacognition")
                                    })
                                    .count();
                                let cat_decisions = events.iter()
                                    .filter(|e| {
                                        e.observation.get("cat_decision")
                                            .is_some()
                                    })
                                    .count();
                                let directives = events.iter()
                                    .filter(|e| {
                                        e.observation.get("directive")
                                            .is_some()
                                    })
                                    .count();

                                let observation = serde_json::json!({
                                    "receptor": "r7-7-curator",
                                    "total_curation_events": events.len(),
                                    "metacognition_cycles": metacognition_cycles,
                                    "cat_engagement_decisions": cat_decisions,
                                    "directives_issued": directives,
                                    "window_secs": (chrono::Utc::now() - since).num_seconds(),
                                });

                                tracing::info!(
                                    target: "cns.curation.observed",
                                    total = %events.len(),
                                    metacognition = %metacognition_cycles,
                                    cat = %cat_decisions,
                                    directives = %directives,
                                    "r7-7 curator observation cycle"
                                );

                                if let Some(ref sink) = event_sink {
                                    let span = Span::new(
                                        hkask_types::event::SpanNamespace::new("cns.curation"),
                                        "observed",
                                    );
                                    let event = NuEvent::new(
                                        WebID::from_persona(b"r7-7-curator"),
                                        span,
                                        Phase::Act,
                                        observation,
                                        0,
                                    );
                                    if let Err(e) = sink.persist(&event) {
                                        tracing::warn!(
                                            target: "cns.curation.receptor",
                                            error = %e,
                                            "r7-7 failed to persist NuEvent"
                                        );
                                    }
                                }

                                last_seen = match events.last() {
                                    Some(e) => e.timestamp,
                                    None => chrono::Utc::now(),
                                };
                            }
                            Err(e) => {
                                tracing::warn!(
                                    target: "cns.curation.receptor",
                                    error = %e,
                                    "r7-7 failed to query spans"
                                );
                                last_seen = chrono::Utc::now();
                            }
                        }
                    }
                    _ = cancel_rx.changed() => {
                        tracing::info!(target: "cns.curation.receptor", "r7-7 curator receptor stopped");
                        break;
                    }
                }
            }
        });

        tracing::info!(
            target: "cns.curation.receptor.started",
            interval_secs = %interval,
            "r7-7 curator receptor started"
        );
    }

    pub async fn stop(&self) {
        *self.active.write().await = false;
        *self.cancel_tx.write().await = None;
        tracing::info!(target: "cns.curation.receptor.stopped", "r7-7 curator receptor stopped");
    }
}

// ── Module tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::WebID;

    /// Dummy NuEventSink for testing — records persisted events.
    struct DummySink {
        events: std::sync::Mutex<Vec<NuEvent>>,
    }

    impl DummySink {
        fn new() -> Self {
            Self {
                events: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn into_events(self) -> Vec<NuEvent> {
            self.events.into_inner().unwrap()
        }
    }

    impl NuEventSink for DummySink {
        fn persist(&self, event: &NuEvent) -> Result<(), hkask_types::InfrastructureError> {
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }
    }

    /// Dummy ReceptorStore for testing — returns empty events.
    struct DummyStore;

    impl ReceptorStore for DummyStore {
        fn query_spans(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
            _span_prefix: &str,
            _limit: u64,
        ) -> Result<Vec<NuEvent>, Box<dyn std::error::Error + Send + Sync>> {
            Ok(vec![])
        }
    }

    #[test]
    fn receptor_webids_are_deterministic() {
        let a = WebID::from_persona(b"r7-2-variety");
        let b = WebID::from_persona(b"r7-2-variety");
        assert_eq!(a, b);
    }

    #[test]
    fn receptor_webids_are_distinct() {
        let r7_1 = WebID::from_persona(b"r7-1-observer");
        let r7_2 = WebID::from_persona(b"r7-2-variety");
        let r7_3 = WebID::from_persona(b"r7-3-algedonic");
        let r7_4 = WebID::from_persona(b"r7-4-composer");
        let r7_5 = WebID::from_persona(b"r7-5-consolidator");
        let r7_6 = WebID::from_persona(b"r7-6-cybernetics");
        let r7_7 = WebID::from_persona(b"r7-7-curator");

        let all = [&r7_1, &r7_2, &r7_3, &r7_4, &r7_5, &r7_6, &r7_7];
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(all[i], all[j], "receptor WebIDs must be distinct");
            }
        }
    }

    #[test]
    fn receptor_start_stop_is_idempotent() {
        // Verify that start/stop on a receptor without store doesn't panic
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let receptor = VarietyReceptor::new(60);
            receptor.start().await; // no store — shouldn't panic
            receptor.start().await; // double start — idempotent
            receptor.stop().await;
            receptor.stop().await; // double stop — idempotent
        });
    }

    #[test]
    fn receptor_with_sink_emits_events() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let sink = Arc::new(DummySink::new());
            let store = Arc::new(DummyStore);
            let receptor = VarietyReceptor::new(60)
                .with_event_sink(sink.clone())
                .with_store(store);

            receptor.start().await;
            // Let one cycle complete
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            receptor.stop().await;
            // Drop the receptor so its spawned task can release the Arc reference
            drop(receptor);
            // Let the spawned task exit
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;

            let events = Arc::try_unwrap(sink)
                .unwrap_or_else(|_| panic!("sink still referenced"))
                .into_events();
            // With empty store, the background task may not have emitted
            // if it couldn't find events — that's fine; the structure is verified
            // The background task emits a NuEvent through the sink.
            // We verify the receptor's structure works; the exact span
            // format is type-checked at compile time via Span/SpanNamespace.
            assert!(
                !events.is_empty(),
                "receptor should emit at least one observation"
            );
        });
    }
}
