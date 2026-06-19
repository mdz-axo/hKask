//! Condenser MCP server fuzz targets.
//!
//! Covers all 5 condenser request types (from hkask-condenser crate).
//!
//! Pattern (a): deserialize_never_panics — arbitrary JSON → deserialize all request types.

use bolero::check;
use hkask_condenser::types::*;

// ── Pattern (a): Deserialize never panics ──────────────────────────────────

/// Deserialize arbitrary JSON into all condenser request types — none may panic.
#[test]
fn fuzz_condenser_deserialize_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        let _ = serde_json::from_str::<CompressRequest>(s);
        let _ = serde_json::from_str::<SetProfileRequest>(s);
        let _ = serde_json::from_str::<ClassifyRequest>(s);
        let _ = serde_json::from_str::<PersistRequest>(s);
        let _ = serde_json::from_str::<ThreadSummaryRequest>(s);
    });
}
