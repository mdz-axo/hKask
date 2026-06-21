//! Pod Portability Test — Solid Pod portability guarantee acceptance test.
//!
//! "Create a pod on server A. Export it. Import it on server B.
//!  Activate it. The agent retains its memory, identity, and capabilities."

use hkask_agents::pod::{ActivePods, AgentPersona, PodKind};
use hkask_agents::AgentKind;

#[tokio::test]
async fn pod_portability_across_servers() {
    // ── Server A: Create and populate a pod ──────────────────────────────
    let server_a = tempfile::TempDir::new().expect("server A tempdir");
    let pods_a = ActivePods::new_test_harness(server_a.path());

    let persona = AgentPersona::system("portable-alice", AgentKind::Replicant);
    let pod_id = pods_a
        .create_pod("replicant", &persona, None, PodKind::Replicant)
        .await
        .expect("create pod A");
    pods_a.activate_pod(&pod_id).await.expect("activate pod A");

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

    // ── Deactivate to flush database ─────────────────────────────────────
    pods_a
        .deactivate_pod(&pod_id)
        .await
        .expect("deactivate pod A");

    // Remove from ActivePods to close the database handle
    pods_a.remove(&pod_id).await;

    // ── Export: copy the database and webid files ─────────────────────────
    let db_path = server_a
        .path()
        .join("pods")
        .join("replicant.portable-alice.db");
    let webid_path = server_a
        .path()
        .join("pods")
        .join("replicant.portable-alice.webid");
    let salt_path = server_a
        .path()
        .join("pods")
        .join("replicant.portable-alice.db.salt");
    let export_dir = server_a.path().join("export");
    std::fs::create_dir_all(export_dir.join("pods")).expect("create export dir");
    std::fs::copy(
        &db_path,
        export_dir.join("pods").join("replicant.portable-alice.db"),
    )
    .expect("copy db");
    std::fs::copy(
        &webid_path,
        export_dir
            .join("pods")
            .join("replicant.portable-alice.webid"),
    )
    .expect("copy webid");
    if salt_path.exists() {
        std::fs::copy(
            &salt_path,
            export_dir
                .join("pods")
                .join("replicant.portable-alice.db.salt"),
        )
        .expect("copy salt");
    }

    // ── Server B: Import the pod ─────────────────────────────────────────
    let server_b = tempfile::TempDir::new().expect("server B tempdir");
    let pods_dir_b = server_b.path().join("pods");
    std::fs::create_dir_all(&pods_dir_b).expect("create pods dir B");
    std::fs::copy(&db_path, pods_dir_b.join("replicant.portable-alice.db")).expect("copy DB to B");
    std::fs::copy(
        &webid_path,
        pods_dir_b.join("replicant.portable-alice.webid"),
    )
    .expect("copy webid to B");
    if salt_path.exists() {
        std::fs::copy(
            &salt_path,
            pods_dir_b.join("replicant.portable-alice.db.salt"),
        )
        .expect("copy salt to B");
    }

    // Copy template mocks
    let tmpl_a = server_a.path().join("templates");
    let tmpl_b = server_b.path().join("templates");
    if tmpl_a.exists() {
        std::fs::create_dir_all(&tmpl_b).unwrap();
        for entry in std::fs::read_dir(&tmpl_a).unwrap() {
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

    // Create ActivePods on server B
    let pods_b = ActivePods::new_test_harness(server_b.path());

    // ── Verify: Same persona → deterministic PodID match ─────────────────
    let pod_id_b = pods_b
        .create_pod("replicant", &persona, None, PodKind::Replicant)
        .await
        .expect("create pod on B");
    assert_eq!(
        pod_id_b, pod_id,
        "Deterministic PodID should match across servers"
    );

    // Activate on server B — opens the copied database
    pods_b
        .activate_pod(&pod_id_b)
        .await
        .expect("activate pod on B");

    let ctx_b = pods_b.context(&pod_id_b).await.expect("PodContext B");

    // ── Verify: memory retained ──────────────────────────────────────────
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

    // ── Verify: identity retained ────────────────────────────────────────
    assert_eq!(
        ctx_b.webid,
        persona.webid(),
        "WebID should be identical across servers"
    );
    assert_eq!(
        ctx_b.pod_id, pod_id,
        "PodID should be deterministic across servers"
    );
}
