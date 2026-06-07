//! Loop System — Bootstrap and lifecycle for the 6-loop model
//
//! Wires all loops together through the Communication Loop's dispatch
//! pipeline: each loop sends through `dispatch_tx`, messages flow into
//! `MessageDispatch`, the Communication Loop delivers to loop inboxes,
//! and each loop processes and ticks.
//!
//! **Authority DAG:** Curation → Cybernetics → {Inference, Episodic, Semantic, Communication}

use crate::communication::CommunicationLoop;
use crate::communication::dispatch::MessageDispatch;
use hkask_cns::CyberneticsLoop;
use hkask_storage::lock_mutex;
use hkask_types::InfrastructureError;
use hkask_types::loops::HkaskLoop;
use hkask_types::loops::LoopId;
use hkask_types::loops::dispatch::LoopMessage;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

/// Adapter to share a CyberneticsLoop between the loop system and GovernedTool.
/// GovernedTool needs `Arc<RwLock<CyberneticsLoop>>`, but `register_loop` needs `Arc<dyn HkaskLoop>`.
/// This adapter bridges the gap.
pub struct CyberneticsLoopHandle(pub Arc<tokio::sync::RwLock<CyberneticsLoop>>);

#[async_trait::async_trait]
impl HkaskLoop for CyberneticsLoopHandle {
    fn id(&self) -> hkask_types::loops::LoopId {
        hkask_types::loops::LoopId::Cybernetics
    }

    async fn sense(&self) -> Vec<hkask_types::loops::Signal> {
        self.0.read().await.sense().await
    }

    async fn compute(
        &self,
        deviations: &[hkask_types::loops::Deviation],
    ) -> Vec<hkask_types::loops::LoopAction> {
        self.0.read().await.compute(deviations).await
    }

    async fn act(&self, actions: &[hkask_types::loops::LoopAction]) {
        self.0.read().await.act(actions).await
    }
}

// Per-loop default tick intervals.
//
// Each loop has a natural cadence based on its regulatory role.
// See [`INFERENCE_TICK_MS`], [`EPISODIC_SEMANTIC_TICK_SECS`], [`COMMUNICATION_TICK_MS`],
// [`CYBERNETICS_TICK_SECS`], and [`CURATION_TICK_SECS`] for the values.

/// Default tick interval for the Inference loop (500ms).
pub const INFERENCE_TICK_MS: u64 = 500;

/// Default tick interval for the Episodic and Semantic loops (5s).
pub const EPISODIC_SEMANTIC_TICK_SECS: u64 = 5;

/// Default tick interval for the Communication loop (100ms).
pub const COMMUNICATION_TICK_MS: u64 = 100;

/// Default tick interval for the Cybernetics loop (2s).
pub const CYBERNETICS_TICK_SECS: u64 = 2;

/// Default tick interval for the Snapshot loop (60s).
/// Snapshots are less frequent than Cybernetics sensing — they
/// check RetentionPolicy intervals (30min minimum) and only act
/// when a snapshot is due.
pub const SNAPSHOT_TICK_SECS: u64 = 60;

/// Default tick interval for the Curation loop (10s).
pub const CURATION_TICK_SECS: u64 = 10;

/// Fallback tick interval for unregistered loops (1s).
pub const DEFAULT_FALLBACK_TICK_SECS: u64 = 1;

pub fn default_tick_interval(loop_id: LoopId) -> Duration {
    match loop_id {
        LoopId::Inference => Duration::from_millis(INFERENCE_TICK_MS),
        LoopId::Episodic => Duration::from_secs(EPISODIC_SEMANTIC_TICK_SECS),
        LoopId::Semantic => Duration::from_secs(EPISODIC_SEMANTIC_TICK_SECS),
        LoopId::Communication => Duration::from_millis(COMMUNICATION_TICK_MS),
        LoopId::Cybernetics => Duration::from_secs(CYBERNETICS_TICK_SECS),
        LoopId::Snapshot => Duration::from_secs(SNAPSHOT_TICK_SECS),
        LoopId::Curation => Duration::from_secs(CURATION_TICK_SECS),
    }
}

/// Authority DAG tick order: meta-loops first, then domain loops.
/// Communication ticks independently as shared infrastructure.
/// Curation (5) → Cybernetics (6) → Snapshot (6b) → Inference (1) → Episodic (2a) → Semantic (2b)
/// No sideways edges. Authority flows downward.
pub const AUTHORITY_ORDER: [LoopId; 6] = [
    LoopId::Curation,
    LoopId::Cybernetics,
    LoopId::Snapshot,
    LoopId::Inference,
    LoopId::Episodic,
    LoopId::Semantic,
];

/// Loop System — manages the lifecycle and wiring of all loops (7 instances across the 6-loop model).
///
/// Provides:
/// - Channel creation and wiring for inter-loop communication
/// - Loop registration with inbox channel assignment
/// - Async task spawning for tick cycles and dispatch forwarding
/// - Graceful shutdown via cancellation token
pub struct LoopSystem {
    /// Shared priority-ordered message dispatch
    dispatch: Arc<MessageDispatch>,
    /// Communication loop that routes messages to target inboxes
    communication_loop: Arc<CommunicationLoop>,
    /// All registered loops keyed by LoopId
    loops: Arc<RwLock<HashMap<LoopId, Arc<dyn HkaskLoop>>>>,
    /// Per-loop inbox senders (CommunicationLoop delivers through these)
    inbox_senders: Arc<RwLock<HashMap<LoopId, tokio::sync::mpsc::UnboundedSender<LoopMessage>>>>,
    /// Per-loop inbox receivers (loops read from these in their tick)
    inbox_receivers:
        Arc<RwLock<HashMap<LoopId, tokio::sync::mpsc::UnboundedReceiver<LoopMessage>>>>,
    /// System-wide dispatch channel sender (given to loops for sending)
    dispatch_tx: tokio::sync::mpsc::UnboundedSender<LoopMessage>,
    /// System-wide dispatch channel receiver (forwarded to MessageDispatch)
    dispatch_rx: Mutex<tokio::sync::mpsc::UnboundedReceiver<LoopMessage>>,
    /// Cancellation token for graceful shutdown
    cancel: tokio_util::sync::CancellationToken,
    /// Per-loop tick intervals (keyed by LoopId)
    tick_intervals: HashMap<LoopId, Duration>,
}

use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

impl LoopSystem {
    /// Create a new LoopSystem with the shared MessageDispatch.
    ///
    /// The MessageDispatch is the priority queue that all loops send
    /// messages into (via `dispatch_tx`) and the Communication Loop
    /// reads from to deliver to target loop inboxes.
    pub fn new(dispatch: Arc<MessageDispatch>) -> Self {
        let (dispatch_tx, dispatch_rx) = tokio::sync::mpsc::unbounded_channel();

        let communication_loop = Arc::new(CommunicationLoop::new(Arc::clone(&dispatch)));

        Self {
            dispatch,
            communication_loop,
            loops: Arc::new(RwLock::new(HashMap::new())),
            inbox_senders: Arc::new(RwLock::new(HashMap::new())),
            inbox_receivers: Arc::new(RwLock::new(HashMap::new())),
            dispatch_tx,
            dispatch_rx: Mutex::new(dispatch_rx),
            cancel: CancellationToken::new(),
            tick_intervals: [
                LoopId::Inference,
                LoopId::Episodic,
                LoopId::Semantic,
                LoopId::Communication,
                LoopId::Cybernetics,
                LoopId::Snapshot,
                LoopId::Curation,
            ]
            .into_iter()
            .map(|id| (id, default_tick_interval(id)))
            .collect(),
        }
    }

    /// Customize the tick interval for a specific loop.
    ///
    /// Returns `Self` for chaining. If the loop ID doesn't yet have an
    /// entry (e.g. called before `register_loop`), the interval is stored
    /// and will be used when that loop's tick task starts.
    pub fn with_tick_interval(mut self, loop_id: LoopId, interval: Duration) -> Self {
        self.tick_intervals.insert(loop_id, interval);
        self
    }

    /// Register a loop with the system.
    ///
    /// Creates an inbox channel pair for the loop and registers the
    /// sender with the Communication Loop. The loop will receive
    /// messages targeted at its `LoopId` through the inbox receiver.
    ///
    /// Returns the dispatch sender that the loop (or its wrapper)
    /// should use to send `LoopMessage`s into the system.
    pub async fn register_loop(
        &self,
        loop_instance: Arc<dyn HkaskLoop>,
    ) -> tokio::sync::mpsc::UnboundedSender<LoopMessage> {
        let id = loop_instance.id();
        let worker_kind = loop_instance.worker_kind();
        let (inbox_tx, inbox_rx) = tokio::sync::mpsc::unbounded_channel::<LoopMessage>();

        // Register inbox sender with CommunicationLoop
        self.communication_loop
            .register_loop_inbox(id, inbox_tx.clone())
            .await;

        // If this loop is a worker, also register with worker routing
        if let Some(kind) = worker_kind {
            self.communication_loop
                .register_worker_inbox(kind, inbox_tx.clone())
                .await;
        }

        // Store loop and inbox
        {
            let mut loops = self.loops.write().await;
            loops.insert(id, loop_instance);
        }
        {
            let mut senders = self.inbox_senders.write().await;
            senders.insert(id, inbox_tx);
        }
        {
            let mut receivers = self.inbox_receivers.write().await;
            receivers.insert(id, inbox_rx);
        }

        info!(
            target: "loop_system",
            loop_id = %id,
            worker = worker_kind.is_some(),
            "Registered loop"
        );

        self.dispatch_tx.clone()
    }

    /// Get a clone of the system-wide dispatch sender.
    ///
    /// Use this to send `LoopMessage`s from outside the loop system
    /// (e.g., from the CLI/API composition root).
    pub fn dispatch_sender(&self) -> tokio::sync::mpsc::UnboundedSender<LoopMessage> {
        self.dispatch_tx.clone()
    }

    /// Get a clone of the Communication Loop's shared queue depth counter.
    ///
    /// CyberneticsLoop reads this counter to sense communication backpressure.
    /// Wire this before calling `start()` using
    /// `CyberneticsLoop::with_communication_queue_depth()`.
    pub fn communication_queue_depth_counter(
        &self,
    ) -> std::sync::Arc<std::sync::atomic::AtomicU64> {
        self.communication_loop.queue_depth_counter()
    }

    /// Get a reference to the MessageDispatch.
    pub fn dispatch(&self) -> &Arc<MessageDispatch> {
        &self.dispatch
    }

    /// Get the cancellation token for external cancellation.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Start all loop tasks.
    ///
    /// Spawns three categories of async tasks:
    /// 1. **Dispatch forwarder** — reads from `dispatch_rx` and enqueues
    ///    into `MessageDispatch` (the priority queue)
    /// 2. **Communication Loop tick** — dequeues from `MessageDispatch`
    ///    and delivers to target loop inboxes
    /// 3. **Per-loop tick** — each registered loop runs its
    ///    `sense → compare → compute → act` cycle on a timer
    pub async fn start(&self) -> Result<(), InfrastructureError> {
        let cancel = self.cancel.clone();

        // 1. Dispatch forwarder: dispatch_rx → MessageDispatch
        {
            let dispatch = Arc::clone(&self.dispatch);
            let mut rx_guard = lock_mutex(&self.dispatch_rx)?;
            let mut rx = std::mem::replace(&mut *rx_guard, {
                // Replace with a closed channel so nobody can use it
                let (_, dead_rx) = tokio::sync::mpsc::unbounded_channel();
                dead_rx
            });

            tokio::spawn(async move {
                info!(target: "loop_system", "Dispatch forwarder started");
                loop {
                    tokio::select! {
                        msg = rx.recv() => {
                            match msg {
                                Some(msg) => {
                                    dispatch.send(msg).await;
                                }
                                None => {
                                    info!(target: "loop_system", "Dispatch channel closed");
                                    break;
                                }
                            }
                        }
                        _ = cancel.cancelled() => {
                            info!(target: "loop_system", "Dispatch forwarder cancelled");
                            break;
                        }
                    }
                }
            });
        }

        // 2. Communication Loop tick
        {
            let comm = Arc::clone(&self.communication_loop);
            let tick_interval = self
                .tick_intervals
                .get(&LoopId::Communication)
                .copied()
                .unwrap_or(Duration::from_millis(COMMUNICATION_TICK_MS));
            let cancel = self.cancel.clone();

            tokio::spawn(async move {
                info!(target: "loop_system", "Communication Loop tick started");
                let mut interval = tokio::time::interval(tick_interval);
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            comm.tick().await;
                        }
                        _ = cancel.cancelled() => {
                            info!(target: "loop_system", "Communication Loop tick cancelled");
                            break;
                        }
                    }
                }
            });
        }

        // 3. Per-loop tick tasks
        let loops_map = self.loops.read().await.clone();
        for (id, loop_instance) in loops_map {
            let cancel = self.cancel.clone();
            let tick_interval = self
                .tick_intervals
                .get(&id)
                .copied()
                .unwrap_or(Duration::from_secs(DEFAULT_FALLBACK_TICK_SECS));

            tokio::spawn(async move {
                info!(
                    target: "loop_system",
                    loop_id = %id,
                    tick_interval_ms = tick_interval.as_millis() as u64,
                    "Loop tick task started"
                );
                let mut interval = tokio::time::interval(tick_interval);
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            loop_instance.tick().await;
                        }
                        _ = cancel.cancelled() => {
                            info!(
                                target: "loop_system",
                                loop_id = %id,
                                "Loop tick task cancelled"
                            );
                            break;
                        }
                    }
                }
            });
        }

        info!(
            target: "loop_system",
            "LoopSystem started with per-loop tick intervals"
        );

        Ok(())
    }

    /// Run a single regulation cycle across all loops in authority order.
    ///
    /// Authority DAG: Curation → Cybernetics → {Inference, Episodic, Semantic, Communication}
    ///
    /// This runs all loops synchronously in a single call, which is useful for:
    /// - Testing (deterministic order)
    /// - Single-threaded operation
    ///   Full regulation cycle: tick all registered loops in authority order.
    ///
    /// Authority DAG: Curation → Cybernetics → {Inference, Episodic, Semantic, Communication}
    /// Meta-loops tick first so their regulatory actions take effect before domain loops sense.
    pub async fn tick(&self) {
        for loop_id in AUTHORITY_ORDER {
            let loops = self.loops.read().await;
            if let Some(loop_instance) = loops.get(&loop_id) {
                loop_instance.tick().await;
            }
        }
    }

    /// Run multiple regulation cycles.
    ///
    /// Useful for testing convergence: run until the system stabilizes
    /// or a maximum number of ticks is reached.
    pub async fn tick_n(&self, max_ticks: usize) {
        for i in 0..max_ticks {
            self.tick().await;
            tracing::debug!(
                target: "loop_system",
                tick = i + 1,
                max = max_ticks,
                "Tick cycle completed"
            );
        }
    }

    /// Signal all loop tasks to stop.
    pub fn shutdown(&self) {
        info!(target: "loop_system", "LoopSystem shutting down");
        self.cancel.cancel();
    }

    /// Check how many loops are registered.
    pub async fn registered_count(&self) -> usize {
        self.loops.read().await.len()
    }

    /// Get the IDs of all registered loops.
    pub async fn registered_loop_ids(&self) -> Vec<LoopId> {
        self.loops.read().await.keys().copied().collect()
    }
}
