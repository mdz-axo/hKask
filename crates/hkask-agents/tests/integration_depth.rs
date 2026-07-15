//! Integration Depth Tests — Verifies the integration gaps from the code audit.
//!
//! These tests complement agent_pod_integration.rs by testing:
//! - recall_semantic through Curator path (PodContext routing)
//! - CNS cns.semantic.published observer notification
//! - CuratorPod singleton enforcement
//! - source_pod provenance round-trip
//! - pod_meta table population

use hkask_agents::pod::{ActivePods, AgentPersona, PodKind};
use hkask_ports::CnsObserver;
use hkask_types::AgentKind;
use hkask_types::event::{NuEvent, SpanNamespace};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Poll the Curator index until the expected entity appears or timeout expires.
///
/// CuratorSync runs on a 1-second polling interval with blocking filesystem I/O
/// and SQLCipher key derivation per tick. A fixed sleep cannot guarantee the
/// sync has completed; this polling loop absorbs scheduling jitter gracefully.
async fn wait_for_curator_sync(
    pods: &ActivePods,
    entity: &str,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(ci) = pods.curator_index().await {
            let idx = ci.read().map_err(|e| format!("lock: {e}"))?;
            let h_mems = idx.query_by_entity(entity).unwrap_or_default();
            if !h_mems.is_empty() {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Err(format!(
        "Curator did not sync entity '{}' within {:.1}s",
        entity,
        timeout.as_secs_f64()
    ))
}

/// Gracefully cancel the CuratorSync background task and wait for teardown.
///
/// Without this, the Tokio runtime may forcibly drop the CuratorSync while it
/// holds SQLCipher connections inside `spawn_blocking`, causing a SIGSEGV from
/// the `sqlite_vec` global auto-extension accessing freed memory.
async fn cleanup_curator(cancel_tx: tokio::sync::watch::Sender<bool>) {
    cancel_tx.send(true).ok();
    // Let the CuratorSync task exit its poll loop and drop connections cleanly
    tokio::time::sleep(Duration::from_millis(200)).await;
}

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
    let (pods, cancel_tx) = setup_curator(&tmp).await;
    let pod_id = create_replicant(&pods, "alice").await;
    let ctx = pods.context(&pod_id).await.expect("PodContext");

    // Write semantic h_mem — should be stored locally AND synced to Curator
    ctx.store_semantic("RoutingTest", "value", serde_json::json!("42"), 0.9)
        .expect("store_semantic");

    // Poll until CuratorSync picks it up (absorbs 1s tick interval + FS/DB overhead)
    wait_for_curator_sync(&pods, "RoutingTest", Duration::from_secs(60))
        .await
        .expect("CuratorSync should pick up the h_mem");

    // Now verify PodContext::recall_semantic routes through Curator
    let results = ctx.recall_semantic("RoutingTest").expect("recall_semantic");
    assert!(
        !results.is_empty(),
        "recall_semantic should return Curator-synced h_mem"
    );
    assert_eq!(results[0].entity, "RoutingTest");

    cleanup_curator(cancel_tx).await;
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
async fn cns_semantic_published_notifies_observer() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let (pods, cancel_tx) = setup_curator(&tmp).await;
    let pod_id = create_replicant(&pods, "alice").await;
    let ctx = pods.context(&pod_id).await.expect("PodContext");

    // Subscribe a test observer to the pod's CNS runtime
    let observer = std::sync::Arc::new(TestObserver::new(vec![
        SpanNamespace::new("cns.semantic.published").unwrap(),
    ]));
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

    cleanup_curator(cancel_tx).await;
}

// ── #3: CuratorPod singleton enforcement ─────────────────────────────────────

#[tokio::test]
async fn second_curator_pod_is_rejected() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let (pods, cancel_tx) = setup_curator(&tmp).await;

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

    cleanup_curator(cancel_tx).await;
}

// ── #4: source_pod provenance preserved ──────────────────────────────────────

#[tokio::test]
async fn source_pod_provenance_round_trips() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let (pods, cancel_tx) = setup_curator(&tmp).await;
    let pod_id = create_replicant(&pods, "alice").await;
    let ctx = pods.context(&pod_id).await.expect("PodContext");

    ctx.store_semantic("ProvTest", "value", serde_json::json!("provenance"), 0.9)
        .expect("store_semantic");

    // Poll until CuratorSync picks it up
    wait_for_curator_sync(&pods, "ProvTest", Duration::from_secs(60))
        .await
        .expect("CuratorSync should pick up the h_mem");

    // Check Curator index — the h_mem should carry source_pod provenance
    let (source_opt, source_pod) = {
        let ci = pods.curator_index().await.expect("curator index");
        let idx = ci.read().unwrap();
        let h_mems = idx.query_by_entity("ProvTest").unwrap_or_default();
        assert!(!h_mems.is_empty(), "HMem should be synced");

        // Extract source pod from h_mem provenance — must round-trip now that
        // PodIDs are deterministic (PodID::from_name("{kind}:{name}")).
        let source = hkask_agents::curator::SemanticIndex::source_pod_of(&h_mems[0]);
        assert!(source.is_some(), "HMem should have source_pod provenance");
        (source, pod_id)
    };
    assert_eq!(
        source_opt.unwrap(),
        source_pod,
        "Source pod should match the pod that wrote the h_mem"
    );

    cleanup_curator(cancel_tx).await;
}

// ── #5: pod_meta table populated ─────────────────────────────────────────────

#[tokio::test]
async fn pod_meta_table_contains_metadata() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let (_pods, cancel_tx) = setup_curator(&tmp).await;

    // Open the curator pod database directly with the canonical DB passphrase.
    use hkask_storage::Database;
    let db_path = tmp.path().join("agents").join("curator").join("pod.db");
    let webid_path = db_path.with_extension("webid");
    let webid = std::fs::read_to_string(&webid_path).expect("read webid");
    let passphrase =
        hkask_keystore::keychain::resolve_db_passphrase_string().expect("resolve DB passphrase");

    let db = Database::open(&db_path.to_string_lossy(), &passphrase).expect("open DB");
    let pool = db.sqlite_pool().expect("sqlite pool");
    let conn = pool.get().expect("pool get");

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

    cleanup_curator(cancel_tx).await;
}
