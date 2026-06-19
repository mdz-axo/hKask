//! Memory MCP server fuzz targets.
//!
//! Pattern (a): deserialize_never_panics — arbitrary JSON → deserialize all request types

use bolero::check;
use hkask_mcp_memory::types::*;

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
