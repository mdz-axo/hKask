//! Memory infrastructure assembly for the REPL.
//!
//! Builds storage ports and ConsolidationService from a shared Database
//! connection so that consolidation operates on the agent's actual
//! episodic and semantic triples.

use hkask_agents::adapters::MemoryLoopAdapter;
use hkask_agents::ports::{EpisodicStoragePort, SemanticStoragePort};
use hkask_memory::{ConsolidationBridge, ConsolidationService, EpisodicMemory, SemanticMemory};
use hkask_storage::{Database, EmbeddingStore, TripleStore};
use hkask_types::CuratorHandle;
use std::sync::Arc;

/// Build memory infrastructure from a Database: storage ports + ConsolidationService.
///
/// All components share the same underlying DB connection, so consolidation
/// operates on the agent's actual episodic and semantic triples.
pub(super) fn build_memory_infra(
    db: Database,
) -> (
    Arc<dyn EpisodicStoragePort>,
    Arc<dyn SemanticStoragePort>,
    ConsolidationService,
) {
    let conn = db.conn_arc();

    // EpisodicMemory + SemanticMemory for ConsolidationService
    let ts1 = TripleStore::new(Arc::clone(&conn));
    let episodic_memory = Arc::new(EpisodicMemory::new(ts1));
    let ts2 = TripleStore::new(Arc::clone(&conn));
    let emb = EmbeddingStore::new(Arc::clone(&conn));
    let semantic_memory = Arc::new(SemanticMemory::new(ts2, emb));

    // ConsolidationService from the shared memories
    let bridge = Arc::new(ConsolidationBridge::new(
        Arc::clone(&episodic_memory),
        Arc::clone(&semantic_memory),
    ));
    let handle = CuratorHandle::system();
    let token = handle.issue_consolidation_token();
    let service = ConsolidationService::new(bridge, semantic_memory, token);

    // Storage ports — new EpisodicMemory/SemanticMemory from the same
    // connection (same pattern as MemoryLoopAdapter::from_database)
    let epi_adapter = Arc::new(MemoryLoopAdapter::new(
        EpisodicMemory::new(TripleStore::new(Arc::clone(&conn))),
        SemanticMemory::new(
            TripleStore::new(Arc::clone(&conn)),
            EmbeddingStore::new(Arc::clone(&conn)),
        ),
    ));
    let sem_adapter = Arc::new(MemoryLoopAdapter::new(
        EpisodicMemory::new(TripleStore::new(Arc::clone(&conn))),
        SemanticMemory::new(
            TripleStore::new(Arc::clone(&conn)),
            EmbeddingStore::new(Arc::clone(&conn)),
        ),
    ));

    (
        epi_adapter as Arc<dyn EpisodicStoragePort>,
        sem_adapter as Arc<dyn SemanticStoragePort>,
        service,
    )
}
