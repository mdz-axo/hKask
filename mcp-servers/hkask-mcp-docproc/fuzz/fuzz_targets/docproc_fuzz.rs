//! DocProc MCP server fuzz targets.
//!
//! Covers all 9 docproc request types.
//!
//! Pattern (a): deserialize_never_panics — arbitrary JSON → deserialize all request types.

use bolero::check;
use hkask_mcp_docproc::tools::*;

// ── Pattern (a): Deserialize never panics ──────────────────────────────────

/// Deserialize arbitrary JSON into all docproc request types — none may panic.
#[test]
fn fuzz_docproc_deserialize_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        let _ = serde_json::from_str::<ConvertRequest>(s);
        let _ = serde_json::from_str::<OcrRequest>(s);
        let _ = serde_json::from_str::<ChunkRequest>(s);
        let _ = serde_json::from_str::<GenerateQaRequest>(s);
        let _ = serde_json::from_str::<ExtractTriplesRequest>(s);
        let _ = serde_json::from_str::<EmbedRequest>(s);
        let _ = serde_json::from_str::<CacheRequest>(s);
        let _ = serde_json::from_str::<QueryRequest>(s);
        let _ = serde_json::from_str::<ClearIndexRequest>(s);
    });
}
