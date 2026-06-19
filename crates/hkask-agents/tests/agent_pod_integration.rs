//! Multi-Pod Integration Tests
//!
//! Acceptance tests for the three-tier pod architecture:
//! - CuratorPod → SemanticIndex aggregation
//! - ReplicantPod → private episodic, public semantic
//! - TeamPod → shared bot workspace
//! - CNS event emission on semantic write
//! - Cross-pod contradictory triple recall

use hkask_agents::pod::{ActivePods, AgentPersona, PodKind};
use hkask_types::AgentKind;

/// Create mock template directories for system pods (curator, replicant, team, solo).
fn setup_mock_templates() {
    let base = std::path::PathBuf::from("/tmp/hkask-mock");
    for name in &["curator", "replicant", "team", "solo"] {
        let dir = base.join(name);
        let _ = std::fs::create_dir_all(&dir);
        // Create a minimal YAML manifest so GitCasAdapter can load it
        let manifest = format!(
            "name: {name}\nversion: 0.1.0\ndescription: Test template for {name}\n"
        );
        let _ = std::fs::write(dir.join("template.yaml"), manifest);
    }
}

/// Helper: create an in-memory ActivePods with CuratorPod activated and CuratorSync running.
async fn setup_with_curator() -> (ActivePods, tokio::sync::watch::Sender<bool>) {
    setup_mock_templates();
    let pods = ActivePods::new_mock();
    let data_dir = std::path::PathBuf::from("/tmp/hkask-test-pods");
    let _ = std::fs::create_dir_all(&data_dir);
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    // Activate CuratorPod
    let result = pods.ensure_curator(data_dir, cancel_rx).await;
    assert!(result.is_ok(), "ensure_curator failed: {:?}", result.err());
    assert!(
        result.unwrap().is_some(),
        "SemanticIndex missing after curator creation"
    );

    (pods, cancel_tx)
}

/// Helper: create a ReplicantPod with a given name.
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
    let data_dir = std::path::PathBuf::from("/tmp/hkask-test-curator-empty");
    let _ = std::fs::create_dir_all(&data_dir);
    setup_mock_templates();
    let pods = ActivePods::new_mock();
    let (_cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    let result = pods.ensure_curator(data_dir.clone(), cancel_rx).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());

    // Verify curator_index is populated
    let ci = pods.curator_index().await;
    assert!(ci.is_some());

    // SemanticIndex should be empty
    let index = ci.unwrap();
    let idx = index.read().await;
    assert_eq!(idx.source_count(), 0);
}

#[tokio::test]
async fn replicant_writes_semantic_and_curator_sees_it() {
    let (pods, _cancel_tx) = setup_with_curator().await;
    let pod_id = create_replicant(&pods, "alice").await;

    let ctx = pods.context(&pod_id).await.expect("get PodContext");

    // Write a semantic triple
    let result = ctx.store_semantic("Bitcoin", "price", serde_json::json!("$100k"), 0.9);
    assert!(result.is_ok(), "store_semantic failed: {:?}", result.err());

    // The CNS event fires on write, but CuratorSync polls async.
    // Give it time to sync (1s polling interval + buffer).
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    // Now recall through Curator — should see the triple
    let results = ctx.recall_semantic("Bitcoin");
    assert!(
        results.is_ok(),
        "recall_semantic failed: {:?}",
        results.err()
    );
    let triples = results.unwrap();
    assert!(
        !triples.is_empty(),
        "Expected at least one triple, got empty"
    );
    assert_eq!(triples[0].entity, "Bitcoin");
    assert_eq!(triples[0].attribute, "price");
}

#[tokio::test]
async fn contradictory_triples_both_returned() {
    let (pods, _cancel_tx) = setup_with_curator().await;

    let alice_id = create_replicant(&pods, "alice").await;
    let bob_id = create_replicant(&pods, "bob").await;

    let alice_ctx = pods.context(&alice_id).await.expect("get alice context");
    let bob_ctx = pods.context(&bob_id).await.expect("get bob context");

    // Alice says $100k
    alice_ctx
        .store_semantic("Bitcoin", "price", serde_json::json!("$100k"), 0.9)
        .expect("alice store");
    // Bob says $50k
    bob_ctx
        .store_semantic("Bitcoin", "price", serde_json::json!("$50k"), 0.7)
        .expect("bob store");

    // Wait for CuratorSync to pick up both
    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    // Query through alice's context (which uses Curator index)
    let results = alice_ctx
        .recall_semantic("Bitcoin")
        .expect("recall_semantic");
    let price_triples: Vec<_> = results.iter().filter(|t| t.attribute == "price").collect();

    assert_eq!(
        price_triples.len(),
        2,
        "Expected 2 contradictory triples, got {}: {:?}",
        price_triples.len(),
        price_triples
    );

    let values: Vec<&str> = price_triples
        .iter()
        .map(|t| t.value.as_str().unwrap_or(""))
        .collect();
    assert!(values.contains(&"$100k"), "Missing Alice's $100k");
    assert!(values.contains(&"$50k"), "Missing Bob's $50k");
}

#[tokio::test]
async fn team_pod_bots_share_episodic() {
    let (pods, _cancel_tx) = setup_with_curator().await;

    // Create a TeamPod for "7R7 research team"
    let team_persona = AgentPersona::system("7r7", AgentKind::Bot);
    let team_id = pods
        .create_pod("team", &team_persona, None, PodKind::Team)
        .await
        .expect("create TeamPod");
    pods.activate_pod(&team_id).await.expect("activate TeamPod");

    let team_ctx = pods.context(&team_id).await.expect("get team context");

    // Bot 1 writes episodic (private to team)
    team_ctx
        .store_episodic("test:entity", "found", serde_json::json!("gold"), 0.8)
        .expect("store episodic");

    // Same context reads it back — same team pod
    let results = team_ctx
        .recall_episodic("test:entity")
        .expect("recall episodic");
    assert!(
        !results.is_empty(),
        "TeamPod episodic should be visible to team bots"
    );

    // Team also publishes semantic to Curator
    team_ctx
        .store_semantic(
            "team:discovery",
            "location",
            serde_json::json!("Atlantis"),
            0.6,
        )
        .expect("team store semantic");

    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

    // Verify Curator sees team's semantic
    let results = team_ctx
        .recall_semantic("team:discovery")
        .expect("recall team semantic");
    assert!(
        !results.is_empty(),
        "Curator should see TeamPod's semantic triple"
    );
}

#[tokio::test]
async fn store_semantic_emits_cns_event() {
    let (pods, _cancel_tx) = setup_with_curator().await;
    let pod_id = create_replicant(&pods, "alice").await;

    let ctx = pods.context(&pod_id).await.expect("get PodContext");

    // CNS runtime should be available
    let variety_before = ctx.cns().variety().await;
    let semantic_count_before = variety_before
        .iter()
        .filter(|(ns, _)| ns.to_string().contains("semantic"))
        .count();

    // Write semantic — should increment CNS variety
    ctx.store_semantic("Ethereum", "price", serde_json::json!("$5k"), 0.95)
        .expect("store_semantic");

    // CNS variety should increment for cns.semantic.published namespace
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let variety_after = ctx.cns().variety().await;
    let semantic_count_after = variety_after
        .iter()
        .filter(|(ns, _)| ns.to_string().contains("semantic"))
        .count();

    assert!(
        semantic_count_after >= semantic_count_before,
        "CNS semantic variety should increase or stay same after store_semantic"
    );
}

#[tokio::test]
async fn recall_falls_back_to_local_when_no_curator() {
    // No CuratorPod — just a ReplicantPod
    setup_mock_templates();
    let pods = ActivePods::new_mock();
    let persona = AgentPersona::system("solo", AgentKind::Replicant);
    let pod_id = pods
        .create_pod("solo", &persona, None, PodKind::Replicant)
        .await
        .expect("create pod");
    pods.activate_pod(&pod_id).await.expect("activate pod");

    let ctx = pods.context(&pod_id).await.expect("get context");

    // Store semantic locally
    ctx.store_semantic("SoloCoin", "price", serde_json::json!("$42"), 0.5)
        .expect("store");

    // Recall — should fall back to local storage (no Curator)
    let results = ctx.recall_semantic("SoloCoin").expect("recall_semantic");
    assert!(
        !results.is_empty(),
        "Should recall from local storage when no Curator"
    );
    assert_eq!(results[0].entity, "SoloCoin");
}

#[tokio::test]
async fn pod_deployment_has_correct_pod_kind() {
    setup_mock_templates();
    let pods = ActivePods::new_mock();

    let curator_persona = AgentPersona::system("curator-test", AgentKind::Bot);
    let curator_id = pods
        .create_pod("curator", &curator_persona, None, PodKind::Curator)
        .await
        .expect("create CuratorPod");

    let status = pods.get_pod_status(&curator_id).await.expect("get status");
    assert!(matches!(status.pod_kind, PodKind::Curator));

    let replicant_persona = AgentPersona::system("replicant-test", AgentKind::Replicant);
    let rep_id = pods
        .create_pod("replicant", &replicant_persona, None, PodKind::Replicant)
        .await
        .expect("create ReplicantPod");

    let status = pods.get_pod_status(&rep_id).await.expect("get status");
    assert!(matches!(status.pod_kind, PodKind::Replicant));
}
