//! Pod Portability Test — Solid Pod portability guarantee acceptance test.
//!
//! "Create a pod on server A. Export it as a SQLCipher file. Import it on server B.
//!  Activate it. The agent retains its memory, identity, and capabilities."

use hkask_agents::pod::{ActivePods, AgentPersona, PodKind};
use hkask_types::AgentKind;

#[tokio::test]
async fn pod_portability_across_servers() {
    // ── Server A: Create and populate a pod ──────────────────────────────
    let server_a = tempfile::TempDir::new().expect("server A tempdir");
    let pods_a = ActivePods::new_test_harness(server_a.path());

    // Create and activate CuratorPod on server A
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    pods_a
        .ensure_curator(server_a.path().to_path_buf(), cancel_rx)
        .await
        .expect("curator on A");

    // Create and activate a ReplicantPod
    let persona = AgentPersona::system("portable-alice", AgentKind::Replicant);
    let pod_id = pods_a
        .create_pod("replicant", &persona, None, PodKind::Replicant)
        .await
        .expect("create pod A");
    pods_a.activate_pod(&pod_id).await.expect("activate pod A");

    // Write some memory
    let ctx_a = pods_a.context(&pod_id).await.expect("PodContext A");
    ctx_a
        .store_episodic(
            "memory:fact",
            "learned",
            serde_json::json!("hKask is portable"),
            0.95,
        )
        .expect("store episodic");
    ctx_a
        .store_semantic(
            "knowledge:fact",
            "truth",
            serde_json::json!("pods are sovereign"),
            0.99,
        )
        .expect("store semantic");

    // ── Export: copy the pod's database file ─────────────────────────────
    let db_path_a = server_a
        .path()
        .join("pods")
        .join("replicant.portable-alice.db");
    let webid_path_a = server_a
        .path()
        .join("pods")
        .join("replicant.portable-alice.webid");
    assert!(db_path_a.exists(), "Pod DB file should exist on server A");
    assert!(
        webid_path_a.exists(),
        "WebID sidecar should exist on server A"
    );

    // ── Server B: Import the pod ─────────────────────────────────────────
    let server_b = tempfile::TempDir::new().expect("server B tempdir");
    // Create pods directory and copy the database
    let pods_dir_b = server_b.path().join("pods");
    std::fs::create_dir_all(&pods_dir_b).expect("create pods dir B");
    std::fs::copy(&db_path_a, pods_dir_b.join("replicant.portable-alice.db")).expect("copy DB");
    std::fs::copy(
        &webid_path_a,
        pods_dir_b.join("replicant.portable-alice.webid"),
    )
    .expect("copy webid");

    // Also copy template mocks (needed for PodFactory activation)
    let tmpl_a = server_a.path().join("templates");
    let tmpl_b = server_b.path().join("templates");
    if tmpl_a.exists() {
        // Recursive copy of template directories
        for entry in std::fs::read_dir(&tmpl_a).expect("read templates A") {
            let entry = entry.unwrap();
            let dest = tmpl_b.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                std::fs::create_dir_all(&dest).unwrap();
                for file in std::fs::read_dir(entry.path()).unwrap() {
                    let file = file.unwrap();
                    std::fs::copy(file.path(), dest.join(file.file_name())).unwrap();
                }
            }
        }
    }

    // Create ActivePods on server B with the same template infrastructure
    let pods_b = ActivePods::new_test_harness(server_b.path());

    // Create the pod on server B (same persona → deterministic PodID matches)
    let pod_id_b = pods_b
        .create_pod("replicant", &persona, None, PodKind::Replicant)
        .await
        .expect("create pod on B");
    assert_eq!(
        pod_id_b, pod_id,
        "Deterministic PodID should match across servers"
    );

    // Activate it
    pods_b
        .activate_pod(&pod_id_b)
        .await
        .expect("activate pod on B");

    let ctx_b = pods_b.context(&pod_id_b).await.expect("PodContext B");

    // ── Verify: memory, identity, capabilities retained ──────────────────
    let episodes = ctx_b
        .recall_episodic("memory:fact")
        .expect("recall episodic");
    assert!(
        !episodes.is_empty(),
        "Episodic memory should survive migration"
    );
    assert_eq!(episodes[0].value, serde_json::json!("hKask is portable"));

    let semantics = ctx_b
        .recall_semantic("knowledge:fact")
        .expect("recall semantic");
    assert!(
        !semantics.is_empty(),
        "Semantic memory should survive migration"
    );

    let webid_b = ctx_b.webid;
    assert_eq!(
        webid_b,
        persona.webid(),
        "WebID should be identical across servers"
    );

    assert_eq!(ctx_b.pod_id, pod_id, "PodID should be deterministic");
}
