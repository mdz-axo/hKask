//! Integration Depth Tests — Verifies the integration gaps from the code audit.
//!
//! These tests complement agent_pod_integration.rs by testing:
//! - recall_semantic through Curator path (PodContext routing)
//! - CNS cns.semantic.published observer notification
//! - CuratorPod singleton enforcement
//! - source_pod provenance round-trip
//! - pod_meta table population

use hkask_agents::pod::{ActivePods, AgentPersona, PodKind};
use hkask_types::AgentKind;

fn setup(tmp: &tempfile::TempDir) -> ActivePods {
    ActivePods::new_test_harness(tmp.path())
}

async fn setup_curator(tmp: &tempfile::TempDir) -> (ActivePods, tokio::sync::watch::Sender<bool>) {
    let pods = setup(tmp);
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    pods.ensure_curator(tmp.path().to_path_buf(), cancel_rx)
        .await
        .expect("ensure_curator");
    (pods, cancel_tx)
}

async fn create_replicant(pods: &ActivePods, name: &str) -> hkask_types::PodID {
    let persona = AgentPersona::system(name, AgentKind::Replicant);
    let pod_id = pods
        .create_pod("replicant", &persona, None, PodKind::Replicant)
        .await
        .expect("create");
    pods.activate_pod(&pod_id).await.expect("activate");
    pod_id
}

// ── #1: recall_semantic through Curator path ─────────────────────────────────

#[tokio::test]
async fn recall_semantic_routes_through_curator() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let (pods, _cancel) = setup_curator(&tmp).await;
    let pod_id = create_replicant(&pods, "alice").await;
    let ctx = pods.context(&pod_id).await.expect("PodContext");

    // Write semantic triple — should be stored locally AND synced to Curator
    ctx.store_semantic("RoutingTest", "value", serde_json::json!("42"), 0.9)
        .expect("store_semantic");

    // Wait for CuratorSync to pick it up
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;

    // Verify Curator index has it (direct check)
    let ci = pods.curator_index().await.expect("curator index");
    let idx = ci.read().unwrap();
    let triples = idx.query_by_entity("RoutingTest").unwrap_or_default();
    assert!(!triples.is_empty(), "Curator should have synced the triple");

    // Now verify PodContext::recall_semantic routes through Curator
    // (it should return the Curator index result, not local)
    let results = ctx.recall_semantic("RoutingTest").expect("recall_semantic");
    assert!(
        !results.is_empty(),
        "recall_semantic should return Curator-synced triple"
    );
    assert_eq!(results[0].entity, "RoutingTest");
}

#[tokio::test]
async fn recall_semantic_falls_back_to_local_when_curator_unavailable() {
    // No CuratorPod — recall should use local storage
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let pods = setup(&tmp);
    let pod_id = create_replicant(&pods, "solo").await;
    let ctx = pods.context(&pod_id).await.expect("PodContext");

    ctx.store_semantic("FallbackTest", "value", serde_json::json!("local"), 0.5)
        .expect("store_semantic");

    let results = ctx
        .recall_semantic("FallbackTest")
        .expect("recall_semantic");
    assert!(!results.is_empty(), "Should fall back to local storage");
    assert_eq!(results[0].entity, "FallbackTest");
}

// ── #2: CNS cns.semantic.published observer notification ─────────────────────

use hkask_ports::CnsObserver;
use hkask_types::event::{NuEvent, SpanNamespace};
use std::sync::Mutex;

/// Test observer that records received CNS events
struct TestObserver {
    events: Mutex<Vec<NuEvent>>,
    interest: Vec<SpanNamespace>,
}

impl TestObserver {
    fn new(interest: Vec<SpanNamespace>) -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            interest,
        }
    }
    fn received_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }
}

#[async_trait::async_trait]
impl CnsObserver for TestObserver {
    fn interest_mask(&self) -> Vec<SpanNamespace> {
        self.interest.clone()
    }
    async fn on_event(&self, event: &NuEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
    async fn on_depletion(&self, _signal: &hkask_ports::DepletionSignal) {}
    async fn on_backpressure(&self, _signal: &hkask_ports::BackpressureSignal) {}
}

#[tokio::test]
#[ignore = "TODO: CnsSpan::SemanticPublished not yet emitted by store_semantic — CNS notification wiring pending"]
async fn cns_semantic_published_notifies_observer() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let (pods, _cancel) = setup_curator(&tmp).await;
    let pod_id = create_replicant(&pods, "alice").await;
    let ctx = pods.context(&pod_id).await.expect("PodContext");

    // Subscribe a test observer to the pod's CNS runtime
    let observer = std::sync::Arc::new(TestObserver::new(vec![SpanNamespace::from(
        hkask_types::cns::CnsSpan::SemanticPublished,
    )]));
    ctx.cns().inner().subscribe_async(observer.clone()).await;

    // Write semantic — should notify observer
    ctx.store_semantic("CnsTest", "value", serde_json::json!("observed"), 0.8)
        .expect("store_semantic");

    // Give the spawned CNS task time to complete
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let count = observer.received_count();
    assert!(
        count > 0,
        "CNS observer should receive cns.semantic.published event, got {}",
        count
    );
}

// ── #3: CuratorPod singleton enforcement ─────────────────────────────────────

#[tokio::test]
async fn second_curator_pod_is_rejected() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let (pods, _cancel) = setup_curator(&tmp).await;

    // Attempt to create a second CuratorPod
    let persona = AgentPersona::system("curator2", AgentKind::Bot);
    let result = pods
        .create_pod("curator", &persona, None, PodKind::Curator)
        .await;

    assert!(result.is_err(), "Second CuratorPod should be rejected");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("CuratorPod"),
        "Error should mention CuratorPod: {}",
        err
    );
}

// ── #4: source_pod provenance preserved ──────────────────────────────────────

#[tokio::test]
async fn source_pod_provenance_round_trips() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let (pods, _cancel) = setup_curator(&tmp).await;
    let pod_id = create_replicant(&pods, "alice").await;
    let ctx = pods.context(&pod_id).await.expect("PodContext");

    ctx.store_semantic("ProvTest", "value", serde_json::json!("provenance"), 0.9)
        .expect("store_semantic");

    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;

    // Check Curator index — the triple should carry source_pod provenance
    let ci = pods.curator_index().await.expect("curator index");
    let idx = ci.read().unwrap();
    let triples = idx.query_by_entity("ProvTest").unwrap_or_default();
    assert!(!triples.is_empty(), "Triple should be synced");

    // Extract source pod from triple provenance — must round-trip now that
    // PodIDs are deterministic (PodID::from_name("{kind}:{name}")).
    let source = hkask_agents::curator::SemanticIndex::source_pod_of(&triples[0]);
    assert!(source.is_some(), "Triple should have source_pod provenance");
    assert_eq!(
        source.unwrap(),
        pod_id,
        "Source pod should match the pod that wrote the triple"
    );
}

// ── #5: pod_meta table populated ─────────────────────────────────────────────

#[tokio::test]
async fn pod_meta_table_contains_metadata() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let (_pods, _cancel) = setup_curator(&tmp).await;

    // Open the curator.db directly and check pod_meta
    use hkask_storage::Database;
    let db_path = tmp.path().join("agents").join("curator").join("pod.db");
    let webid_path = db_path.with_extension("webid");
    let webid = std::fs::read_to_string(&webid_path).expect("read webid");
    let passphrase = {
        use hkask_types::secret::{SecretRef, derivation_contexts};
        let webid_parsed: hkask_types::WebID = webid.trim().parse().expect("parse webid");
        let ctx = format!("{}:{}", derivation_contexts::OCAP_SECRET, webid_parsed);
        let secret = SecretRef::derived(derivation_contexts::MASTER_KEY_ENV, &ctx);
        let bytes = hkask_keystore::resolve(&secret).expect("resolve key");
        hex::encode(&*bytes)
    };

    let db = Database::open(&db_path.to_string_lossy(), &passphrase).expect("open DB");
    let conn = db.conn_arc();
    let conn = conn.lock().unwrap();

    // Check webid
    let stored_webid: String = conn
        .query_row("SELECT value FROM pod_meta WHERE key = 'webid'", [], |r| {
            r.get(0)
        })
        .expect("pod_meta.webid");
    assert_eq!(
        stored_webid.trim(),
        webid.trim(),
        "pod_meta.webid should match"
    );

    // Check pod_kind
    let kind: String = conn
        .query_row(
            "SELECT value FROM pod_meta WHERE key = 'pod_kind'",
            [],
            |r| r.get(0),
        )
        .expect("pod_meta.pod_kind");
    assert!(
        kind.contains("Curator"),
        "pod_meta.pod_kind should contain Curator: {}",
        kind
    );

    // Check created_at exists
    let created: String = conn
        .query_row(
            "SELECT value FROM pod_meta WHERE key = 'created_at'",
            [],
            |r| r.get(0),
        )
        .expect("pod_meta.created_at");
    assert!(
        !created.is_empty(),
        "pod_meta.created_at should be non-empty"
    );
}
