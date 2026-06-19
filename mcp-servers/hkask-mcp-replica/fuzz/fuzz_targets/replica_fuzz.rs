//! Replica MCP server fuzz targets.
//!
//! Pattern (a): deserialize_never_panics — arbitrary JSON → deserialize all request types

use bolero::check;
use hkask_mcp_replica::types::*;

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
