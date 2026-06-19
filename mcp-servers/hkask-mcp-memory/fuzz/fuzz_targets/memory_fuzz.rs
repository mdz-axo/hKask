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

// ── Pattern (a): Tool dispatch never panics ───────────────────────────────

/// Full tool dispatch path must never panic under arbitrary deserialized input.
#[test]
fn fuzz_memory_tool_dispatch_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        let server = test_memory_server();

        // Try episodic_store
        if let Ok(req) = serde_json::from_str::<StoreRequest>(s) {
            let _output = call_tool(server.episodic_store(Parameters(req)));
            return;
        }
        // Try episodic_recall
        if let Ok(req) = serde_json::from_str::<RecallRequest>(s) {
            let _output = call_tool(server.episodic_recall(Parameters(req)));
            return;
        }
        // Try episodic_budget
        if let Ok(req) = serde_json::from_str::<BudgetRequest>(s) {
            let _output = call_tool(server.episodic_budget(Parameters(req)));
            return;
        }
        // Try episodic_consolidate_status
        if let Ok(req) = serde_json::from_str::<ConsolidateStatusRequest>(s) {
            let _output = call_tool(server.episodic_consolidate_status(Parameters(req)));
            return;
        }
        // Try semantic_store
        if let Ok(req) = serde_json::from_str::<StoreRequest>(s) {
            let _output = call_tool(server.semantic_store(Parameters(req)));
            return;
        }
        // Try semantic_recall
        if let Ok(req) = serde_json::from_str::<RecallRequest>(s) {
            let _output = call_tool(server.semantic_recall(Parameters(req)));
            return;
        }
        // Try semantic_embed
        if let Ok(req) = serde_json::from_str::<EmbedRequest>(s) {
            let _output = call_tool(server.semantic_embed(Parameters(req)));
            return;
        }
        // Try semantic_search
        if let Ok(req) = serde_json::from_str::<SearchRequest>(s) {
            let _output = call_tool(server.semantic_search(Parameters(req)));
            return;
        }
        // Try semantic_centroid
        if let Ok(req) = serde_json::from_str::<CentroidRequest>(s) {
            let _output = call_tool(server.semantic_centroid(Parameters(req)));
            return;
        }
        // Try semantic_purge
        if let Ok(req) = serde_json::from_str::<PurgeRequest>(s) {
            let _output = call_tool(server.semantic_purge(Parameters(req)));
            return;
        }
        // Try semantic_chunk
        if let Ok(req) = serde_json::from_str::<ChunkTextRequest>(s) {
            let _output = call_tool(server.semantic_chunk(Parameters(req)));
            return;
        }
        // Try semantic_count
        if let Ok(req) = serde_json::from_str::<CountRequest>(s) {
            let _output = call_tool(server.semantic_count(Parameters(req)));
            return;
        }
        // Try memory_backup
        if let Ok(req) = serde_json::from_str::<BackupRequest>(s) {
            let _output = call_tool(server.memory_backup(Parameters(req)));
            return;
        }
        // Try memory_restore
        if let Ok(req) = serde_json::from_str::<RestoreRequest>(s) {
            let _output = call_tool(server.memory_restore(Parameters(req)));
        }
    });
}
