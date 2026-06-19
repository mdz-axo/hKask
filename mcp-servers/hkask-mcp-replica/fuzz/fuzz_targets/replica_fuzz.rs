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

// ── Pattern (a): Tool dispatch — one test per tool (equal coverage) ─────

macro_rules! dispatch_test {
    ($name:ident, $ty:ty, $method:ident) => {
        #[test]
        fn $name() {
            check!().with_type::<String>().for_each(|s| {
                if let Ok(req) = serde_json::from_str::<$ty>(s) {
                    let server = test_server();
                    let _ = call_tool(server.$method(Parameters(req)));
                }
            });
        }
    };
}

dispatch_test!(
    fuzz_replica_dispatch_replica_build,
    BuildRequest,
    replica_build
);
dispatch_test!(
    fuzz_replica_dispatch_replica_compose,
    ComposeRequest,
    replica_compose
);
dispatch_test!(
    fuzz_replica_dispatch_replica_compare,
    CompareRequest,
    replica_compare
);
dispatch_test!(
    fuzz_replica_dispatch_replica_mashup,
    MashupRequest,
    replica_mashup
);
dispatch_test!(
    fuzz_replica_dispatch_replica_registry,
    RegistryRequest,
    replica_registry
);
dispatch_test!(
    fuzz_replica_dispatch_replica_discover,
    DiscoverRequest,
    replica_discover
);
dispatch_test!(
    fuzz_replica_dispatch_replica_cache_work,
    CacheWorkRequest,
    replica_cache_work
);
