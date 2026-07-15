//! Multi-Pod Integration Tests — Acceptance tests for the three-tier pod architecture.

use hkask_agents::pod::{ActivePods, AgentPersona, PodKind};
use hkask_types::AgentKind;

async fn wait_for_curator_h_mems(
    pods: &ActivePods,
    entity: &str,
    min_count: usize,
) -> Vec<hkask_storage::HMem> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        let h_mems = if let Some(index) = pods.curator_index().await {
            index
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .query_by_entity(entity)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if h_mems.len() >= min_count || tokio::time::Instant::now() >= deadline {
            return h_mems;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
}

async fn setup_with_curator(
    tmp: &tempfile::TempDir,
) -> (ActivePods, tokio::sync::watch::Sender<bool>) {
    let pods = ActivePods::new_test_harness(tmp.path());
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let result = pods
        .ensure_curator(tmp.path().to_path_buf(), cancel_rx)
        .await;
    assert!(result.is_ok(), "ensure_curator failed: {:?}", result.err());
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
    let idx = ci.read().unwrap();
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
    let all = wait_for_curator_h_mems(&pods, "Bitcoin", 1).await;
    assert!(
        !all.is_empty(),
        "Curator index should have Bitcoin h_mem after sync; got {} h_mems",
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
    let all = wait_for_curator_h_mems(&pods, "Bitcoin", 2).await;
    let price_triples: Vec<_> = all.iter().filter(|t| t.attribute == "price").collect();
    assert!(
        price_triples.len() >= 2,
        "Expected at least 2 contradictory h_mems, got {}",
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
    let all = wait_for_curator_h_mems(&pods, "team:discovery", 1).await;
    assert!(!all.is_empty(), "Curator should see TeamPod semantic h_mem");
}

#[tokio::test]
async fn recall_falls_back_to_local_when_no_curator() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let pods = ActivePods::new_test_harness(tmp.path());
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
    let pods = ActivePods::new_test_harness(tmp.path());
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
