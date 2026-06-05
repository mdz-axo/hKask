//! Loops command handlers for `kask loops`
//!
//! Implements the CLI display logic for starting the cybernetic loop system.

pub fn run(rt: &tokio::runtime::Runtime) {
    use hkask_agents::{
        AcpPort, AcpRuntime, CuratorAgent, CuratorContext, EscalationQueue, LoopSystem,
        MessageDispatch,
    };
    use hkask_cns::load_set_points;
    use hkask_cns::{CnsRuntime, CyberneticsLoop};
    use hkask_memory::{
        ConsolidationBridge, EpisodicLoop, EpisodicMemory, SemanticLoop, SemanticMemory,
    };
    use hkask_storage::{Database, EmbeddingStore, TripleStore};
    use hkask_types::WebID;
    use hkask_types::event::NuEventSink;
    use hkask_types::loops::HkaskLoop;
    use hkask_types::loops::curation::CuratorHandle;
    use std::sync::Arc;

    // 1. Create shared infrastructure
    let dispatch = Arc::new(MessageDispatch::new());

    // 2. Create the LoopSystem (per-loop default intervals)
    let loop_system = LoopSystem::new(Arc::clone(&dispatch));

    // 3. Register Cybernetics Loop
    let cns_rwlock: Arc<tokio::sync::RwLock<CnsRuntime>> = Arc::new(tokio::sync::RwLock::new(
        CnsRuntime::with_threshold(hkask_cns::DEFAULT_THRESHOLD),
    ));
    let cybernetics_dispatch_tx = loop_system.dispatch_sender();
    let set_points = load_set_points();
    let cns_event_sink: Arc<dyn NuEventSink> = Arc::new(hkask_storage::NuEventStore::new(
        Database::in_memory().expect("cns event db").conn_arc(),
    ));
    let cybernetics_loop = CyberneticsLoop::with_set_points(
        Arc::clone(&cns_rwlock),
        set_points,
        cybernetics_dispatch_tx,
    )
    .with_event_sink(cns_event_sink);
    rt.block_on(loop_system.register_loop(Arc::new(cybernetics_loop)));

    // 4. Inference Loop skipped — requires Okapi connection (not available at CLI bootstrap)

    // 5. Register Episodic Loop
    let db = Database::in_memory().expect("in-memory db");
    let conn = db.conn_arc();
    let triple_store = TripleStore::new(Arc::clone(&conn));
    let episodic_memory = Arc::new(EpisodicMemory::new(triple_store));
    let system_webid = WebID::new();
    let storage_budget = episodic_memory.storage_budget();
    let episodic_loop =
        EpisodicLoop::new(Arc::clone(&episodic_memory), system_webid, storage_budget);
    rt.block_on(loop_system.register_loop(Arc::new(episodic_loop)));

    // 6. Register Semantic Loop
    let triple_store2 = TripleStore::new(Arc::clone(&conn));
    let embedding_store = EmbeddingStore::new(conn);
    let semantic_memory = Arc::new(SemanticMemory::new(triple_store2, embedding_store));
    let semantic_loop = SemanticLoop::new(Arc::clone(&semantic_memory));
    rt.block_on(loop_system.register_loop(Arc::new(semantic_loop)));

    // 7. Register Curation Loop (via CuratorAgent)
    let curator_handle = CuratorHandle::system();
    let escalation_queue = Arc::new(EscalationQueue::new(db.conn_arc()).expect("escalation queue"));
    let acp_secret = super::helpers::or_exit(
        super::config::resolve_acp_secret(),
        "Failed to resolve ACP secret for loop system",
    );
    let acp_runtime: Arc<AcpRuntime> = Arc::new(AcpRuntime::new(acp_secret.as_bytes()));
    let curator_context = Arc::new(
        CuratorContext::new(
            curator_handle.clone(),
            Arc::new(CnsRuntime::with_threshold(hkask_cns::DEFAULT_THRESHOLD)),
            Arc::clone(&dispatch),
            escalation_queue,
        )
        .with_acp(Arc::clone(&acp_runtime) as Arc<dyn AcpPort>),
    );
    let consolidation_bridge = Arc::new(ConsolidationBridge::new(
        Arc::clone(&episodic_memory),
        Arc::clone(&semantic_memory),
    ));
    let curator_agent =
        CuratorAgent::with_consolidation(curator_context, Default::default(), consolidation_bridge);
    let curation_loop: Arc<dyn HkaskLoop> = curator_agent.curation_loop().clone();
    rt.block_on(loop_system.register_loop(curation_loop));

    // 8. Start the loop system
    println!("Starting Loop System (per-loop default tick intervals)");
    println!("Registered loops:");
    let ids = rt.block_on(loop_system.registered_loop_ids());
    for id in &ids {
        println!("  • {:?}", id);
    }
    println!();
    println!("Note: Inference Loop not registered (requires Okapi connection)");
    println!();

    rt.block_on(loop_system.start());

    // 9. Run until Ctrl+C
    println!("Loop system running. Press Ctrl+C to shutdown.");
    rt.block_on(async {
        tokio::signal::ctrl_c().await.ok();
    });

    loop_system.shutdown();
    println!("Loop system shut down.");
}
