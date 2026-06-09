//! Loop System — Bootstrap and lifecycle for the 4-loop model
//
//! Manages loop registration, tick scheduling, and lifecycle.
//! Inter-loop communication uses direct `tokio::mpsc` channels.
//!
//! **Authority DAG:** Curation → Cybernetics → {Inference, Memory}

use hkask_cns::CyberneticsLoop;
use hkask_types::loops::HkaskLoop;
use hkask_types::loops::LoopId;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tracing::info;

/// Adapter to share a CyberneticsLoop between the loop system and GovernedTool.
pub struct CyberneticsLoopHandle(pub Arc<tokio::sync::RwLock<CyberneticsLoop>>);

#[async_trait::async_trait]
impl HkaskLoop for CyberneticsLoopHandle {
    fn id(&self) -> LoopId {
        LoopId::Cybernetics
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

/// Default tick interval for the Inference loop (500ms).
pub const INFERENCE_TICK_MS: u64 = 500;

/// Default tick interval for the Memory sub-loops (5s).
pub const MEMORY_TICK_SECS: u64 = 5;

/// Default tick interval for the Cybernetics loop (2s).
pub const CYBERNETICS_TICK_SECS: u64 = 2;

/// Default tick interval for the Curation loop (10s).
pub const CURATION_TICK_SECS: u64 = 10;

/// Fallback tick interval for unregistered loops (1s).
pub const DEFAULT_FALLBACK_TICK_SECS: u64 = 1;

pub fn default_tick_interval(loop_id: LoopId) -> Duration {
    match loop_id {
        LoopId::Inference => Duration::from_millis(INFERENCE_TICK_MS),
        LoopId::Memory => Duration::from_secs(MEMORY_TICK_SECS),
        LoopId::Cybernetics => Duration::from_secs(CYBERNETICS_TICK_SECS),
        LoopId::Curation => Duration::from_secs(CURATION_TICK_SECS),
    }
}

/// Authority DAG tick order: meta-loops first, then domain loops.
/// Curation → Cybernetics → Inference → Memory
pub const AUTHORITY_ORDER: [LoopId; 4] = [
    LoopId::Curation,
    LoopId::Cybernetics,
    LoopId::Inference,
    LoopId::Memory,
];

/// Multiple loops may share a `LoopId` (e.g., Episodic + Semantic both register
/// as `Memory`). They are ticked in registration order within the same ID.
type LoopRegistry = Arc<RwLock<HashMap<LoopId, Vec<Arc<dyn HkaskLoop>>>>>;

/// Loop System — manages loop registration, tick scheduling, and lifecycle.
///
/// Inter-loop communication uses direct `tokio::mpsc` channels wired during
/// `ServiceContext::build()`. This struct handles only registration and ticking.
pub struct LoopSystem {
    /// All registered loops keyed by LoopId. Vec supports multiple loops per ID.
    loops: LoopRegistry,
    /// Cancellation token for graceful shutdown
    cancel: tokio_util::sync::CancellationToken,
    /// Per-loop tick intervals (keyed by LoopId)
    tick_intervals: HashMap<LoopId, Duration>,
}

impl LoopSystem {
    /// Create a new LoopSystem.
    pub fn new() -> Self {
        Self {
            loops: Arc::new(RwLock::new(HashMap::new())),
            cancel: CancellationToken::new(),
            tick_intervals: [
                LoopId::Inference,
                LoopId::Memory,
                LoopId::Cybernetics,
                LoopId::Curation,
            ]
            .into_iter()
            .map(|id| (id, default_tick_interval(id)))
            .collect(),
        }
    }

    /// Customize the tick interval for a specific loop.
    pub fn with_tick_interval(mut self, loop_id: LoopId, interval: Duration) -> Self {
        self.tick_intervals.insert(loop_id, interval);
        self
    }

    /// Register a loop with the system.
    ///
    /// Adds the loop to the registry so it can be ticked by `start()` or `tick()`.
    /// Multiple loops may share the same `LoopId`.
    pub async fn register_loop(&self, loop_instance: Arc<dyn HkaskLoop>) {
        let id = loop_instance.id();
        let mut loops = self.loops.write().await;
        loops.entry(id).or_default().push(loop_instance);
        info!(
            target: "loop_system",
            loop_id = %id,
            total = loops.get(&id).map(|v| v.len()).unwrap_or(0),
            "Registered loop"
        );
    }

    /// Get the cancellation token for external cancellation.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Start all loop tick tasks.
    ///
    /// Spawns per-loop tick tasks — each registered loop runs its
    /// `sense → compare → compute → act` cycle on a timer.
    pub async fn start(&self) -> Result<(), hkask_types::InfrastructureError> {
        let cancel = self.cancel.clone();

        let loops_map = self.loops.read().await.clone();
        for (id, loop_instances) in loops_map {
            let cancel = cancel.clone();
            let tick_interval = self
                .tick_intervals
                .get(&id)
                .copied()
                .unwrap_or(Duration::from_secs(DEFAULT_FALLBACK_TICK_SECS));

            for loop_instance in loop_instances {
                let cancel = cancel.clone();
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
        }

        info!(
            target: "loop_system",
            "LoopSystem started with per-loop tick intervals"
        );

        Ok(())
    }

    /// Run a single regulation cycle across all loops in authority order.
    ///
    /// Authority DAG: Curation → Cybernetics → {Inference, Memory}
    pub async fn tick(&self) {
        for loop_id in AUTHORITY_ORDER {
            let loops = self.loops.read().await;
            if let Some(instances) = loops.get(&loop_id) {
                for loop_instance in instances {
                    loop_instance.tick().await;
                }
            }
        }
    }

    /// Run multiple regulation cycles.
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

    /// Total number of loop instances across all IDs.
    pub async fn registered_count(&self) -> usize {
        self.loops.read().await.values().map(|v| v.len()).sum()
    }

    /// Get the IDs of all registered loops.
    pub async fn registered_loop_ids(&self) -> Vec<LoopId> {
        self.loops.read().await.keys().copied().collect()
    }
}

impl Default for LoopSystem {
    fn default() -> Self {
        Self::new()
    }
}
