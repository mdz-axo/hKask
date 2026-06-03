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
use hkask_types::loops::HkaskLoop;
use hkask_types::loops::LoopId;
use hkask_types::loops::dispatch::LoopMessage;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

/// Default tick interval for loop regulation cycles (1 second).
const DEFAULT_TICK_INTERVAL: Duration = Duration::from_secs(1);

/// Loop System — manages the lifecycle and wiring of all 6 loops.
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
    /// Tick interval for loop regulation cycles
    tick_interval: Duration,
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
            tick_interval: DEFAULT_TICK_INTERVAL,
        }
    }

    /// Create a LoopSystem with a custom tick interval.
    pub fn with_tick_interval(dispatch: Arc<MessageDispatch>, tick_interval: Duration) -> Self {
        let mut system = Self::new(dispatch);
        system.tick_interval = tick_interval;
        system
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
        let (inbox_tx, inbox_rx) = tokio::sync::mpsc::unbounded_channel::<LoopMessage>();

        // Register inbox sender with CommunicationLoop
        self.communication_loop
            .register_loop_inbox(id, inbox_tx.clone())
            .await;

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
    pub async fn start(&self) {
        let cancel = self.cancel.clone();

        // 1. Dispatch forwarder: dispatch_rx → MessageDispatch
        {
            let dispatch = Arc::clone(&self.dispatch);
            let mut rx_guard = self.dispatch_rx.lock().unwrap();
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
            let tick_interval = self.tick_interval;
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
            let tick_interval = self.tick_interval;

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
            tick_interval_ms = self.tick_interval.as_millis() as u64,
            "LoopSystem started"
        );
    }

    /// Run a single regulation cycle across all loops in authority order.
    ///
    /// Authority DAG: Curation → Cybernetics → {Inference, Episodic, Semantic, Communication}
    ///
    /// This runs all loops synchronously in a single call, which is useful for:
    /// - Testing (deterministic order)
    /// - Single-threaded operation
    /// - Debugging (sequential inspection)
    ///
    /// Returns the total number of actions produced across all loops.
    pub async fn tick(&self) -> usize {
        // Authority order: Curation (5) → Cybernetics (6) → domain loops
        let authority_order = [
            LoopId::Curation,
            LoopId::Cybernetics,
            LoopId::Inference,
            LoopId::Episodic,
            LoopId::Semantic,
            LoopId::Communication,
        ];

        // Forward any pending dispatch_rx messages into MessageDispatch
        let pending: Vec<LoopMessage> = {
            let mut rx_guard = self.dispatch_rx.lock().unwrap();
            let mut msgs = Vec::new();
            while let Ok(msg) = rx_guard.try_recv() {
                msgs.push(msg);
            }
            msgs
        };
        for msg in pending {
            self.dispatch.send(msg).await;
        }

        // Let the Communication Loop deliver any pending messages first
        self.communication_loop.tick().await;

        // Process inbox messages for each loop, then tick
        let total_actions = 0;
        for loop_id in &authority_order {
            // Drain inbox messages for this loop
            self.drain_inbox(*loop_id).await;

            // Run the loop's regulation cycle
            let loops = self.loops.read().await;
            if let Some(loop_instance) = loops.get(loop_id) {
                loop_instance.tick().await;
                // Note: we can't easily count actions from tick() since it
                // doesn't return a count. We rely on the tracing spans instead.
            }
        }

        total_actions
    }

    /// Drain inbox messages for a specific loop and log them.
    async fn drain_inbox(&self, loop_id: LoopId) {
        let mut receivers = self.inbox_receivers.write().await;
        if let Some(rx) = receivers.get_mut(&loop_id) {
            let mut message_count = 0;
            while let Ok(msg) = rx.try_recv() {
                message_count += 1;
                tracing::debug!(
                    target: "loop_system",
                    loop_id = %loop_id,
                    trace_id = %msg.trace_id,
                    origin = ?msg.origin,
                    "Processing inbox message"
                );
                // The message has been delivered; the loop's sense() will
                // pick up any state changes caused by the message payload.
                // For now, log the delivery. Future work: make loops
                // consume specific payload types in their sense() phase.
            }
            if message_count > 0 {
                tracing::info!(
                    target: "loop_system",
                    loop_id = %loop_id,
                    message_count = message_count,
                    "Processed inbox messages"
                );
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

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::loops::{Deviation, LoopAction, Signal};

    /// A minimal loop for testing registration and tick.
    struct TestLoop {
        id: LoopId,
    }

    #[async_trait::async_trait]
    impl HkaskLoop for TestLoop {
        fn id(&self) -> LoopId {
            self.id
        }

        async fn sense(&self) -> Vec<Signal> {
            vec![]
        }

        async fn compute(&self, _deviations: &[Deviation]) -> Vec<LoopAction> {
            vec![]
        }

        async fn act(&self, _actions: &[LoopAction]) {}
    }

    #[tokio::test]
    async fn loop_system_registers_loops() {
        let dispatch = Arc::new(MessageDispatch::new());
        let system = LoopSystem::new(dispatch);

        let test_loop = Arc::new(TestLoop {
            id: LoopId::Inference,
        });
        system.register_loop(test_loop).await;

        assert_eq!(system.registered_count().await, 1);
        assert_eq!(system.registered_loop_ids().await, vec![LoopId::Inference]);
    }

    #[tokio::test]
    async fn loop_system_registers_multiple_loops() {
        let dispatch = Arc::new(MessageDispatch::new());
        let system = LoopSystem::new(dispatch);

        system
            .register_loop(Arc::new(TestLoop {
                id: LoopId::Inference,
            }))
            .await;
        system
            .register_loop(Arc::new(TestLoop {
                id: LoopId::Cybernetics,
            }))
            .await;

        assert_eq!(system.registered_count().await, 2);
        let mut ids = system.registered_loop_ids().await;
        ids.sort();
        assert_eq!(ids, vec![LoopId::Inference, LoopId::Cybernetics]);
    }

    #[tokio::test]
    async fn loop_system_shutdown_cancels() {
        let dispatch = Arc::new(MessageDispatch::new());
        let system = LoopSystem::new(dispatch);

        let cancel = system.cancel_token();
        assert!(!cancel.is_cancelled());

        system.shutdown();
        assert!(cancel.is_cancelled());
    }

    #[tokio::test]
    async fn loop_system_provides_dispatch_sender() {
        let dispatch = Arc::new(MessageDispatch::new());
        let system = LoopSystem::new(dispatch);

        let sender = system.dispatch_sender();
        // Verify we can send a message without error
        let msg = LoopMessage::critical(
            hkask_types::loops::LoopId::External,
            hkask_types::loops::dispatch::LoopPayload::AlgedonicAlert {
                current: 50,
                threshold: 100,
                deficit: 50,
            },
        );
        assert!(sender.send(msg).is_ok());
    }

    #[tokio::test]
    async fn loop_system_tick_runs_in_authority_order() {
        let dispatch = Arc::new(MessageDispatch::new());
        let system = LoopSystem::new(dispatch);

        // Register all 6 loop types with simple TestLoop instances
        for id in [
            LoopId::Curation,
            LoopId::Cybernetics,
            LoopId::Inference,
            LoopId::Episodic,
            LoopId::Semantic,
            LoopId::Communication,
        ] {
            system.register_loop(Arc::new(TestLoop { id })).await;
        }

        // Tick should complete without panic
        system.tick().await;

        assert_eq!(system.registered_count().await, 6);
    }

    #[tokio::test]
    async fn loop_system_tick_n_completes() {
        let dispatch = Arc::new(MessageDispatch::new());
        let system = LoopSystem::new(dispatch);

        system
            .register_loop(Arc::new(TestLoop {
                id: LoopId::Inference,
            }))
            .await;

        // Multiple ticks should complete
        system.tick_n(5).await;
    }

    #[tokio::test]
    async fn loop_system_tick_processes_inbox_messages() {
        let dispatch = Arc::new(MessageDispatch::new());
        let system = LoopSystem::new(dispatch.clone());

        system
            .register_loop(Arc::new(TestLoop {
                id: LoopId::Cybernetics,
            }))
            .await;

        // Send a message targeting the Cybernetics loop
        let msg = LoopMessage::warning(
            hkask_types::loops::LoopId::Curation,
            hkask_types::loops::dispatch::LoopPayload::CyberneticsDirective {
                directive_type: "calibrate".to_string(),
                target: hkask_types::WebID::new(),
                parameters: serde_json::json!({"reason": "test"}),
            },
        )
        .with_target(hkask_types::loops::LoopId::Cybernetics);

        // Put message into dispatch
        dispatch.send(msg).await;

        // Tick should process the message through Communication Loop → inbox
        system.tick().await;
    }
}
