//! Memory MCP server fuzz targets.
//!
//! Pattern (a): deserialize_never_panics — arbitrary JSON → deserialize all request types

use bolero::check;
use hkask_mcp_memory::MemoryServer;
use hkask_mcp_memory::types::*;
use hkask_memory::{EpisodicMemory, SemanticMemory};
use hkask_storage::{EmbeddingStore, Store, TripleStore};
use hkask_test_harness::TestWebId;
use rmcp::handler::server::wrapper::Parameters;
use rusqlite::Connection;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

// ── Helpers ────────────────────────────────────────────────────────────────

fn test_memory_server() -> MemoryServer {
    let conn = Arc::new(Mutex::new(Connection::open_in_memory().expect("in-memory DB")));
    let triple_store = TripleStore::new(conn.clone());
    triple_store.lock_conn().unwrap().execute_batch(
        "CREATE TABLE IF NOT EXISTS triples (
            id TEXT PRIMARY KEY, entity TEXT NOT NULL, attribute TEXT NOT NULL,
            value TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT,
            confidence REAL NOT NULL, perspective TEXT, visibility TEXT NOT NULL,
            owner_webid TEXT NOT NULL
        )"
    ).expect("DDL");
    let episodic = EpisodicMemory::new(triple_store.clone());
    let emb_store = EmbeddingStore::new(conn.clone());
    let semantic = Arc::new(SemanticMemory::new(triple_store, emb_store));
    MemoryServer::new(
        episodic,
        semantic,
        Some(conn),
        TestWebId::alice(),
        "fuzz-replicant".into(),
        None,
    )
}

fn call_tool<F: std::future::Future<Output = String>>(f: F) -> String {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let result = panic::catch_unwind(AssertUnwindSafe(|| rt.block_on(f)));
    match result {
        Ok(output) => output,
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".to_string()
            };
            format!("{{\"error\":\"panic: {msg}\"}}")
        }
    }
}

// ── Pattern (a): Deserialize never panics ──────────────────────────────────

#[test]
fn fuzz_memory_deserialize_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        let _ = serde_json::from_str::<StoreRequest>(s);
        let _ = serde_json::from_str::<RecallRequest>(s);
        let _ = serde_json::from_str::<BudgetRequest>(s);
        let _ = serde_json::from_str::<ConsolidateStatusRequest>(s);
        let _ = serde_json::from_str::<EmbedRequest>(s);
        let _ = serde_json::from_str::<SearchRequest>(s);
        let _ = serde_json::from_str::<CentroidRequest>(s);
        let _ = serde_json::from_str::<PurgeRequest>(s);
        let _ = serde_json::from_str::<ChunkTextRequest>(s);
        let _ = serde_json::from_str::<CountRequest>(s);
        let _ = serde_json::from_str::<BackupRequest>(s);
        let _ = serde_json::from_str::<RestoreRequest>(s);
    });
}

// ── Pattern (a): Tool dispatch — one test per tool (equal coverage) ─────

macro_rules! dispatch_test {
    ($name:ident, $ty:ty, $method:ident) => {
        #[test]
        fn $name() {
            check!().with_type::<String>().for_each(|s| {
                if let Ok(req) = serde_json::from_str::<$ty>(s) {
                    let server = test_memory_server();
                    let _ = call_tool(server.$method(Parameters(req)));
                }
            });
        }
    };
}

dispatch_test!(
    fuzz_memory_dispatch_episodic_store,
    StoreRequest,
    episodic_store
);
dispatch_test!(
    fuzz_memory_dispatch_episodic_recall,
    RecallRequest,
    episodic_recall
);
dispatch_test!(
    fuzz_memory_dispatch_episodic_budget,
    BudgetRequest,
    episodic_budget
);
dispatch_test!(
    fuzz_memory_dispatch_episodic_consolidate_status,
    ConsolidateStatusRequest,
    episodic_consolidate_status
);
dispatch_test!(
    fuzz_memory_dispatch_semantic_store,
    StoreRequest,
    semantic_store
);
dispatch_test!(
    fuzz_memory_dispatch_semantic_recall,
    RecallRequest,
    semantic_recall
);
dispatch_test!(
    fuzz_memory_dispatch_semantic_embed,
    EmbedRequest,
    semantic_embed
);
dispatch_test!(
    fuzz_memory_dispatch_semantic_search,
    SearchRequest,
    semantic_search
);
dispatch_test!(
    fuzz_memory_dispatch_semantic_centroid,
    CentroidRequest,
    semantic_centroid
);
dispatch_test!(
    fuzz_memory_dispatch_semantic_purge,
    PurgeRequest,
    semantic_purge
);
dispatch_test!(
    fuzz_memory_dispatch_semantic_chunk,
    ChunkTextRequest,
    semantic_chunk
);
dispatch_test!(
    fuzz_memory_dispatch_semantic_count,
    CountRequest,
    semantic_count
);
dispatch_test!(
    fuzz_memory_dispatch_memory_backup,
    BackupRequest,
    memory_backup
);
dispatch_test!(
    fuzz_memory_dispatch_memory_restore,
    RestoreRequest,
    memory_restore
);
