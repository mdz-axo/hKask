//! Spec MCP server fuzz targets.
//!
//! Pattern (a): deserialize_never_panics — arbitrary JSON → deserialize all request types

use bolero::check;
use hkask_mcp_spec::types::*;

#[test]
fn fuzz_spec_deserialize_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        let _ = serde_json::from_str::<GoalCaptureRequest>(s);
        let _ = serde_json::from_str::<GoalDecomposeRequest>(s);
        let _ = serde_json::from_str::<WritingQualityRequest>(s);
        let _ = serde_json::from_str::<GraphQueryRequest>(s);
        let _ = serde_json::from_str::<GraphCoherenceRequest>(s);
        let _ = serde_json::from_str::<ReplicaRewriteRequest>(s);
        let _ = serde_json::from_str::<ContractAuditRequest>(s);
        let _ = serde_json::from_str::<ContractProposeRequest>(s);
        let _ = serde_json::from_str::<ContractAcceptRequest>(s);
        let _ = serde_json::from_str::<ContractRejectRequest>(s);
        let _ = serde_json::from_str::<TestRunRequest>(s);
    });
}
