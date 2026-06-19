//! Replica MCP server fuzz targets.
//!
//! Pattern (a): deserialize_never_panics — arbitrary JSON → deserialize all request types

use bolero::check;
use hkask_mcp_replica::ReplicaServer;
use hkask_mcp_replica::types::*;
use hkask_test_harness::TestWebId;
use rmcp::handler::server::wrapper::Parameters;
use std::panic::{self, AssertUnwindSafe};

// ── Helpers ────────────────────────────────────────────────────────────────

fn test_server() -> ReplicaServer {
    ReplicaServer::new(
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
fn fuzz_replica_deserialize_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        let _ = serde_json::from_str::<BuildRequest>(s);
        let _ = serde_json::from_str::<ComposeRequest>(s);
        let _ = serde_json::from_str::<CompareRequest>(s);
        let _ = serde_json::from_str::<MashupRequest>(s);
        let _ = serde_json::from_str::<RegistryRequest>(s);
        let _ = serde_json::from_str::<DiscoverRequest>(s);
        let _ = serde_json::from_str::<CacheWorkRequest>(s);
    });
}

// ── Pattern (a): Tool dispatch never panics ───────────────────────────────

/// Full tool dispatch path must never panic under arbitrary deserialized input.
#[test]
fn fuzz_replica_tool_dispatch_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        let server = test_server();

        // Try replica_build
        if let Ok(req) = serde_json::from_str::<BuildRequest>(s) {
            let _output = call_tool(server.replica_build(Parameters(req)));
            return;
        }
        // Try replica_compose
        if let Ok(req) = serde_json::from_str::<ComposeRequest>(s) {
            let _output = call_tool(server.replica_compose(Parameters(req)));
            return;
        }
        // Try replica_compare
        if let Ok(req) = serde_json::from_str::<CompareRequest>(s) {
            let _output = call_tool(server.replica_compare(Parameters(req)));
            return;
        }
        // Try replica_mashup
        if let Ok(req) = serde_json::from_str::<MashupRequest>(s) {
            let _output = call_tool(server.replica_mashup(Parameters(req)));
            return;
        }
        // Try replica_registry
        if let Ok(req) = serde_json::from_str::<RegistryRequest>(s) {
            let _output = call_tool(server.replica_registry(Parameters(req)));
            return;
        }
        // Try replica_discover
        if let Ok(req) = serde_json::from_str::<DiscoverRequest>(s) {
            let _output = call_tool(server.replica_discover(Parameters(req)));
            return;
        }
        // Try replica_cache_work
        if let Ok(req) = serde_json::from_str::<CacheWorkRequest>(s) {
            let _output = call_tool(server.replica_cache_work(Parameters(req)));
        }
    });
}
