//! Loop system construction for the API server.
//!
//! Creates CnsRuntime, MessageDispatch, LoopSystem, and registers:
//! Cybernetics, Episodic, Semantic, Curation, and Snapshot loops.
//! Communication Loop is managed internally by LoopSystem.
//! Inference Loop is registered only if an inference port is provided.

use std::sync::Arc;

use hkask_agents::CyberneticsLoopHandle;
use hkask_agents::communication::dispatch::MessageDispatch;
use hkask_agents::curator::context::CuratorContext;
use hkask_agents::curator_agent::CuratorAgent;
use hkask_agents::escalation::EscalationQueue;
use hkask_agents::loop_system::LoopSystem;
use hkask_agents::ports::EpisodicStoragePort;
use hkask_cns::{CnsRuntime, CyberneticsLoop, SnapshotLoop};
use hkask_memory::{
    ConsolidationBridge, EpisodicLoop, EpisodicMemory, SemanticLoop, SemanticMemory,
};
use hkask_storage::{EmbeddingStore, TripleStore};
use hkask_types::WebID;
use hkask_types::event::NuEventSink;
use hkask_types::loops::HkaskLoop;
use hkask_types::loops::curation::CuratorHandle;
use hkask_types::ports::git_cas::GitCASPort;

use crate::error::ApiError;

/// Build the LoopSystem with all loops.
///
/// Creates CnsRuntime, MessageDispatch, LoopSystem, and registers:
/// Cybernetics, Episodic, Semantic, Curation, and Snapshot loops.
/// Communication Loop is managed internally by LoopSystem.
/// Inference Loop is registered only if an inference port is provided.
#[allow(clippy::type_complexity)]
pub(crate) fn build_loop_system(
    escalation_queue: Arc<EscalationQueue>,
    dispatch: Arc<MessageDispatch>,
    inference_port: Option<Arc<dyn hkask_types::ports::InferencePort>>,
    system_webid: WebID,
    acp: Option<Arc<dyn hkask_agents::ports::AcpPort>>,
    event_sink: Option<Arc<dyn NuEventSink>>,
    git_cas_port: Arc<dyn GitCASPort>,
) -> Result<
    (
        Arc<LoopSystem>,
        Arc<dyn EpisodicStoragePort>,
        Arc<tokio::sync::RwLock<CyberneticsLoop>>,
    ),
    ApiError,
> {
    let loop_system = LoopSystem::new(Arc::clone(&dispatch));

    // Cybernetics Loop
    let cns_rwlock: Arc<tokio::sync::RwLock<CnsRuntime>> = Arc::new(tokio::sync::RwLock::new(
        CnsRuntime::with_threshold(hkask_cns::DEFAULT_THRESHOLD),
    ));
    let cybernetics_dispatch_tx = loop_system.dispatch_sender();
    let set_points = hkask_cns::load_set_points();
    let cybernetics_loop = CyberneticsLoop::with_set_points(
        Arc::clone(&cns_rwlock),
        set_points,
        cybernetics_dispatch_tx,
    );
    let cybernetics_loop = match event_sink {
        Some(sink) => cybernetics_loop.with_event_sink(sink),
        None => cybernetics_loop,
    };
    // Wire CommunicationLoop↔CyberneticsLoop queue depth counter.
    // CommunicationLoop writes, CyberneticsLoop reads — lock-free, Relaxed ordering.
    let cybernetics_loop = cybernetics_loop
        .with_communication_queue_depth(loop_system.communication_queue_depth_counter());
    let cybernetics_loop_rwlock = Arc::new(tokio::sync::RwLock::new(cybernetics_loop));
    // Register loops (register_loop is async, use a small runtime for sync callers)
    let rt = tokio::runtime::Runtime::new().map_err(|e| ApiError::Internal {
        message: format!("Failed to create tokio runtime for loop system: {e}"),
    })?;
    rt.block_on(async {
        loop_system
            .register_loop(Arc::new(CyberneticsLoopHandle(Arc::clone(
                &cybernetics_loop_rwlock,
            ))))
            .await;
    });

    // Inference Loop (optional)
    if inference_port.is_some() {
        let inference_loop =
            hkask_agents::InferenceLoop::new().with_dispatch(loop_system.dispatch_sender());
        rt.block_on(async {
            loop_system.register_loop(Arc::new(inference_loop)).await;
        });
    }

    // Episodic Loop
    let db = hkask_storage::in_memory_db();
    let conn = db.conn_arc();
    let triple_store = TripleStore::new(Arc::clone(&conn));
    let episodic_memory = Arc::new(EpisodicMemory::new(triple_store));
    let storage_budget = episodic_memory.storage_budget();
    let episodic_loop =
        EpisodicLoop::new(Arc::clone(&episodic_memory), system_webid, storage_budget);
    rt.block_on(async {
        loop_system.register_loop(Arc::new(episodic_loop)).await;
    });

    // Semantic Loop
    let db2 = hkask_storage::in_memory_db();
    let conn2 = db2.conn_arc();
    let triple_store2 = TripleStore::new(Arc::clone(&conn2));
    let embedding_store = EmbeddingStore::new(Arc::clone(&conn2));
    let semantic_memory = Arc::new(SemanticMemory::new(triple_store2, embedding_store));
    let semantic_loop = SemanticLoop::new(Arc::clone(&semantic_memory));
    rt.block_on(async {
        loop_system.register_loop(Arc::new(semantic_loop)).await;
    });

    // API-facing memory adapter — shares the same DB connections as the loops
    // so budget reads see API writes immediately.
    let memory_adapter = Arc::new(
        hkask_agents::adapters::memory_loop_adapter::MemoryLoopAdapter::new(
            EpisodicMemory::new(TripleStore::new(conn)),
            SemanticMemory::new(
                TripleStore::new(Arc::clone(&conn2)),
                EmbeddingStore::new(conn2),
            ),
        ),
    );
    let episodic_storage: Arc<dyn EpisodicStoragePort> = memory_adapter.clone();

    // Curation Loop (via CuratorAgent)
    let curator_handle = CuratorHandle::system();
    let mut curator_context = CuratorContext::new(
        curator_handle.clone(),
        Arc::new(CnsRuntime::with_threshold(hkask_cns::DEFAULT_THRESHOLD)),
        dispatch,
        escalation_queue,
    );
    if let Some(acp_port) = acp {
        curator_context = curator_context.with_acp(acp_port);
    }
    curator_context = curator_context.with_loop_dispatch_tx(loop_system.dispatch_sender());
    let curator_context = Arc::new(curator_context);
    let consolidation_bridge = Arc::new(ConsolidationBridge::new(
        Arc::clone(&episodic_memory),
        Arc::clone(&semantic_memory),
    ));
    let curator_agent = CuratorAgent::with_consolidation(
        curator_context,
        Default::default(),
        Arc::clone(&consolidation_bridge),
    );
    let curation_loop: Arc<dyn HkaskLoop> = curator_agent.curation_loop().clone();
    rt.block_on(async {
        loop_system.register_loop(curation_loop).await;
    });

    // Snapshot Loop (CAS — scheduled snapshots based on RetentionPolicy)
    let snapshot_loop = SnapshotLoop::new(Arc::clone(&git_cas_port));
    rt.block_on(async {
        loop_system.register_loop(Arc::new(snapshot_loop)).await;
    });

    drop(rt);
    Ok((
        Arc::new(loop_system),
        episodic_storage,
        cybernetics_loop_rwlock,
    ))
}
