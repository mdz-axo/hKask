//! Training MCP server fuzz targets.
//!
//! Pattern (a): deserialize_never_panics — arbitrary JSON → deserialize all request types

use bolero::check;
use hkask_mcp_training::types::*;

#[test]
fn fuzz_training_deserialize_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        let _ = serde_json::from_str::<IngestQaRequest>(s);
        let _ = serde_json::from_str::<TrainSubmitRequest>(s);
        let _ = serde_json::from_str::<TrainStatusRequest>(s);
        let _ = serde_json::from_str::<TrainCancelRequest>(s);
        let _ = serde_json::from_str::<TrainDeleteAdapterRequest>(s);
        let _ = serde_json::from_str::<AssembleDatasetRequest>(s);
        let _ = serde_json::from_str::<GenerateTracesRequest>(s);
        let _ = serde_json::from_str::<TrainEvaluateRequest>(s);
        let _ = serde_json::from_str::<TrainRegisterAdapterRequest>(s);
        let _ = serde_json::from_str::<TrainRecommendModelRequest>(s);
        let _ = serde_json::from_str::<TrainRecordInvocationRequest>(s);
        let _ = serde_json::from_str::<TrainCurateFeedbackRequest>(s);
        let _ = serde_json::from_str::<TrainRetrainRequest>(s);
        let _ = serde_json::from_str::<TrainIngestDatasetRequest>(s);
        let _ = serde_json::from_str::<TrainSweepRequest>(s);
        let _ = serde_json::from_str::<GenerateChainOfThoughtRequest>(s);
        let _ = serde_json::from_str::<MergeAdaptersRequest>(s);
        let _ = serde_json::from_str::<TrainDeployRequest>(s);
        let _ = serde_json::from_str::<TrainTeardownRequest>(s);
    });
}
