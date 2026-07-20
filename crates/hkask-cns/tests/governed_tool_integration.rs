//
// TASK 4 integration test: GovernedTool exercises all 6 membrane steps
// using REAL production components (no mocks).
//
// The 6 membrane steps:
//   1. OCAP authority verification (Domain path)
//   2. Energy budget reservation (hold-settle)
//   3. cns.tool.invoked ν-event emission
//   4. Delegate to inner tool
//   5. Energy budget settlement (hold-settle)
//   6. cns.tool.completed ν-event emission
//
// Components: CnsRuntime (real), CyberneticsLoop (real), GovernedTool (real),
// NuEventStore (real, in-memory DB), EchoToolPort (real, just echoes args).

use hkask_capability::{
    DelegationAction, DelegationResource, DelegationToken, DelegationTokenBuilder,
    derive_signing_key,
};
use hkask_cns::DEFAULT_VARIETY_MAX_DEFICIT;
use hkask_cns::cybernetics_loop::CyberneticsLoop;
use hkask_cns::governed_tool::{EnergyEstimator, GovernedTool};
use hkask_cns::runtime::CnsRuntime;
use hkask_ports::{ToolFuture, ToolInfo, ToolPort, ToolPortError};
use hkask_storage::NuEventStore;
use hkask_types::WebID;
use std::sync::Arc;
use tokio::sync::RwLock;

// ── Real EchoToolPort — just echoes args for testing ─────────────────────
// This is NOT a mock. It's a real ToolPort that does real serialization.

struct EchoToolPort;

impl ToolPort for EchoToolPort {
    fn invoke<'a>(
        &'a self,
        _server: &'a str,
        _tool: &'a str,
        args: serde_json::Value,
        _token: &'a DelegationToken,
    ) -> ToolFuture<'a, Result<serde_json::Value, ToolPortError>> {
        Box::pin(async move {
            Ok(serde_json::json!({
                "echo": args,
                "status": "ok",
            }))
        })
    }

    fn discover_tools<'a>(&'a self) -> ToolFuture<'a, Vec<String>> {
        Box::pin(async move { vec!["echo".to_string()] })
    }

    fn get_tool_info<'a>(&'a self, _tool_name: &'a str) -> ToolFuture<'a, Option<ToolInfo>> {
        Box::pin(async move {
            Some(ToolInfo {
                name: "echo".to_string(),
                description: "Echo tool for integration testing".to_string(),
                input_schema: serde_json::json!({}),
                server_id: "echo_server".to_string(),
                required_capability: Some("tool:cns:execute".to_string()),
                taint: hkask_types::ToolTaint::Pure,
            })
        })
    }
}

// ── Real FixedCostEstimator ─────────────────────────────────────────────
// Production uses TableEnergyEstimator (table lookup). This is same pattern.

struct FixedCostEstimator(u64);

impl EnergyEstimator for FixedCostEstimator {
    fn estimate_cost(&self, _server: &str, _tool: &str, _args: &serde_json::Value) -> u64 {
        self.0
    }
}

// ── Test ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn governed_tool_full_membrane_ocap_domain_path() {
    // 1. Build real CNS runtime
    let cns = Arc::new(RwLock::new(CnsRuntime::with_threshold(
        DEFAULT_VARIETY_MAX_DEFICIT as u64,
    )));

    // 2. Build real NuEventStore with in-memory DB
    let driver = hkask_database::sqlite::SqliteDriver::in_memory_driver();
    let event_store = NuEventStore::from_driver(driver);
    let event_sink: Arc<dyn hkask_types::NuEventSink> = Arc::new(event_store);

    // 3. Build real CyberneticsLoop
    let loop6 = Arc::new(RwLock::new(
        CyberneticsLoop::new(Arc::clone(&cns)).with_event_sink(Arc::clone(&event_sink)),
    ));

    // 4. Register an energy budget so gas accounting passes
    let agent = WebID::new();
    loop6
        .write()
        .await
        .register_gas_budget(
            agent,
            hkask_cns::energy::GasBudget::new(hkask_cns::energy::GasCost(10_000)),
        )
        .await;

    // 5. Build real GovernedTool membrane wrapping EchoToolPort
    let inner: Arc<EchoToolPort> = Arc::new(EchoToolPort);
    let estimator: Arc<dyn EnergyEstimator> = Arc::new(FixedCostEstimator(100));
    let governed = GovernedTool::new(
        inner,
        Arc::clone(&loop6),
        Arc::clone(&event_sink),
        estimator,
        agent,
    );

    // 6. Create a domain-scoped DelegationToken for CNS
    let sk = derive_signing_key(b"test-secret-32-bytes-long!!");
    let token = DelegationTokenBuilder::new(
        DelegationResource::Tool,
        "cns".into(),
        DelegationAction::Execute,
        WebID::new(),
        WebID::new(),
        &sk,
    )
    .sign();

    // 7. Invoke — exercises ALL 6 membrane steps
    let result = governed
        .invoke(
            "echo_server",
            "echo",
            serde_json::json!({"message": "hello"}),
            &token,
        )
        .await;

    assert!(
        result.is_ok(),
        "GovernedTool invocation failed: {:?}",
        result.err()
    );
    let value = result.unwrap();
    assert_eq!(value["status"], "ok");
    assert_eq!(value["echo"]["message"], "hello");

    // 8. Verify energy was consumed (hold-settle pattern ran)
    let status = loop6.read().await.agent_gas_status(&agent).await;
    assert!(
        status.is_some(),
        "Agent should have gas status after invocation"
    );
    let gas = status.unwrap();
    assert!(
        gas.remaining.0 < gas.cap.0,
        "Gas should have been consumed by the invocation"
    );
}
