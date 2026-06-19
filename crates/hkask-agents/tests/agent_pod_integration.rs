//! Multi-Pod Integration Tests — Acceptance tests for the three-tier pod architecture.

use hkask_agents::AllowAllConsent;
use hkask_agents::pod::{ActivePods, AgentPersona, PodKind};
use hkask_types::AgentKind;
use std::sync::Arc;

fn setup_mock_templates(base: &std::path::Path) {
    unsafe {
        std::env::set_var(
            "HKASK_MASTER_KEY",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
    }
    let tmpl = base.join("templates");
    let persona_yaml = "agent:\n  name: test\n  type: Bot\n  version: \"0.1.0\"\ncharter:\n  description: Test\n  editor: test\n";
    for name in &["curator", "replicant", "team", "solo"] {
        let dir = tmpl.join(name);
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("agent_persona.yaml"), persona_yaml);
        let _ = std::fs::write(dir.join("dispatch_manifest.yaml"), "selector: test\n");
    }
}

fn make_test_pods(data_dir: &std::path::Path) -> ActivePods {
    use hkask_agents::a2a::A2ARuntime;
    use hkask_agents::adapters::mcp_runtime::CapabilityOnlyAdapter;
    use hkask_agents::adapters::memory_loop_adapter::MemoryLoopAdapter;
    use hkask_agents::pod::PodFactory;
    use hkask_types::CapabilityChecker;

    let adapter = Arc::new(MemoryLoopAdapter::in_memory_unchecked());
    let mcp = Arc::new(CapabilityOnlyAdapter::new(Arc::new(
        CapabilityChecker::new(b"mock"),
    )));
    let a2a = Arc::new(A2ARuntime::new(b"mock"));
    let factory = Arc::new(PodFactory::new(
        Arc::new(hkask_mcp::GitCasAdapter::from_path(
            data_dir.join("templates"),
        )),
        Arc::new(AllowAllConsent),
        data_dir.to_path_buf(),
    ));
    ActivePods::new()
        .with_a2a_runtime(a2a)
        .with_factory_and_ports(
            factory,
            mcp.clone(),
            None,
            None,
            None,
            adapter.clone() as Arc<dyn hkask_agents::ports::EpisodicStoragePort>,
            adapter as Arc<dyn hkask_agents::ports::SemanticStoragePort>,
        )
}

async fn setup_with_curator(
    tmp: &tempfile::TempDir,
) -> (ActivePods, tokio::sync::watch::Sender<bool>) {
    setup_mock_templates(tmp.path());
    let pods = make_test_pods(tmp.path());
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let result = pods
        .ensure_curator(tmp.path().to_path_buf(), cancel_rx)
        .await;
    assert!(result.is_ok(), "ensure_curator failed: {:?}", result.err());
    assert!(result.unwrap().is_some(), "SemanticIndex missing");
    (pods, cancel_tx)
}

async fn create_replicant(pods: &ActivePods, name: &str) -> hkask_types::PodID {
    let persona = AgentPersona::system(name, AgentKind::Replicant);
    let pod_id = pods
        .create_pod("replicant", &persona, None, PodKind::Replicant)
        .await
        .expect("create ReplicantPod");
    pods.activate_pod(&pod_id)
        .await
        .expect("activate ReplicantPod");
    pod_id
}

#[tokio::test]
async fn curator_starts_and_has_empty_index() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let (pods, _cancel) = setup_with_curator(&tmp).await;
    let ci = pods.curator_index().await.expect("curator index");
    let idx = ci.read().await;
    assert_eq!(
        idx.source_count(),
        0,
        "Fresh curator should have empty index"
    );
}

#[tokio::test]
async fn replicant_writes_semantic_and_curator_sees_it() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let (pods, _cancel) = setup_with_curator(&tmp).await;
    let pod_id = create_replicant(&pods, "alice").await;
    let ctx = pods.context(&pod_id).await.expect("get PodContext");
    ctx.store_semantic("Bitcoin", "price", serde_json::json!("$100k"), 0.9)
        .expect("store_semantic");
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
    let ci = pods.curator_index().await.expect("curator index exists");
    let idx = ci.read().await;
    let all = idx.query_by_entity("Bitcoin").unwrap_or_default();
    assert!(
        !all.is_empty(),
        "Curator index should have Bitcoin triple after sync; got {} triples",
        all.len()
    );
}

#[tokio::test]
async fn contradictory_triples_both_returned() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let (pods, _cancel) = setup_with_curator(&tmp).await;
    let alice_id = create_replicant(&pods, "alice").await;
    let bob_id = create_replicant(&pods, "bob").await;
    let alice_ctx = pods.context(&alice_id).await.expect("get alice context");
    let bob_ctx = pods.context(&bob_id).await.expect("get bob context");
    alice_ctx
        .store_semantic("Bitcoin", "price", serde_json::json!("$100k"), 0.9)
        .expect("alice store");
    bob_ctx
        .store_semantic("Bitcoin", "price", serde_json::json!("$50k"), 0.7)
        .expect("bob store");
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
    let ci = pods.curator_index().await.expect("curator index");
    let idx = ci.read().await;
    let all = idx.query_by_entity("Bitcoin").unwrap_or_default();
    let price_triples: Vec<_> = all.iter().filter(|t| t.attribute == "price").collect();
    assert!(
        price_triples.len() >= 2,
        "Expected at least 2 contradictory triples, got {}",
        price_triples.len()
    );
}

#[tokio::test]
async fn team_pod_bots_share_episodic() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let (pods, _cancel) = setup_with_curator(&tmp).await;
    let team_persona = AgentPersona::system("7r7", AgentKind::Bot);
    let team_id = pods
        .create_pod("team", &team_persona, None, PodKind::Team)
        .await
        .expect("create TeamPod");
    pods.activate_pod(&team_id).await.expect("activate TeamPod");
    let team_ctx = pods.context(&team_id).await.expect("get team context");
    team_ctx
        .store_episodic("test:entity", "found", serde_json::json!("gold"), 0.8)
        .expect("store episodic");
    let results = team_ctx
        .recall_episodic("test:entity")
        .expect("recall episodic");
    assert!(!results.is_empty(), "TeamPod episodic should be visible");
    team_ctx
        .store_semantic(
            "team:discovery",
            "location",
            serde_json::json!("Atlantis"),
            0.6,
        )
        .expect("team store semantic");
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
    let ci = pods.curator_index().await.expect("curator index");
    let idx = ci.read().await;
    let all = idx.query_by_entity("team:discovery").unwrap_or_default();
    assert!(
        !all.is_empty(),
        "Curator should see TeamPod semantic triple"
    );
}

#[tokio::test]
async fn recall_falls_back_to_local_when_no_curator() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    setup_mock_templates(tmp.path());
    let pods = make_test_pods(tmp.path());
    let persona = AgentPersona::system("solo", AgentKind::Replicant);
    let pod_id = pods
        .create_pod("solo", &persona, None, PodKind::Replicant)
        .await
        .expect("create pod");
    pods.activate_pod(&pod_id).await.expect("activate pod");
    let ctx = pods.context(&pod_id).await.expect("get context");
    ctx.store_semantic("SoloCoin", "price", serde_json::json!("$42"), 0.5)
        .expect("store");
    let results = ctx.recall_semantic("SoloCoin").expect("recall_semantic");
    assert!(
        !results.is_empty(),
        "Should recall from local storage when no Curator"
    );
}

#[tokio::test]
async fn pod_deployment_has_correct_pod_kind() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    setup_mock_templates(tmp.path());
    let pods = make_test_pods(tmp.path());
    let team_persona = AgentPersona::system("podkind-team", AgentKind::Bot);
    let team_id = pods
        .create_pod("team", &team_persona, None, PodKind::Team)
        .await
        .expect("create TeamPod");
    let status = pods.get_pod_status(&team_id).await.expect("get status");
    assert!(matches!(status.pod_kind, PodKind::Team));
    let rep_persona = AgentPersona::system("podkind-replicant", AgentKind::Replicant);
    let rep_id = pods
        .create_pod("replicant", &rep_persona, None, PodKind::Replicant)
        .await
        .expect("create ReplicantPod");
    let status = pods.get_pod_status(&rep_id).await.expect("get status");
    assert!(matches!(status.pod_kind, PodKind::Replicant));
}
