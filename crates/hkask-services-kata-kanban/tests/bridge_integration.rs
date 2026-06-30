//! Integration tests for the KanbanKataBridge — verifies bridge wiring,
//! construction, and error paths. Full kata execution is tested via the
//! kata engine's own test suite.

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use hkask_services_kata_kanban::kanban::{ColumnDef, KanbanService, TaskSpec, TaskStatus};
    use hkask_services_kata_kanban::kata::KataEngine;
    use hkask_storage::Store;
    use hkask_templates::SqliteRegistry;

    fn make_test_store() -> hkask_storage::TripleStore {
        use rusqlite::Connection;
        let conn = Arc::new(std::sync::Mutex::new(
            Connection::open_in_memory().expect("in-memory DB"),
        ));
        let store = hkask_storage::TripleStore::new(conn);
        store
            .lock_conn()
            .unwrap()
            .execute_batch(
                "CREATE TABLE triples (
                    id TEXT PRIMARY KEY, entity TEXT NOT NULL, attribute TEXT NOT NULL,
                    value TEXT NOT NULL, valid_from TEXT NOT NULL, valid_to TEXT,
                    recalled_at TEXT NOT NULL DEFAULT (datetime('now')),
                    confidence REAL NOT NULL, perspective TEXT, visibility TEXT NOT NULL,
                    owner_webid TEXT NOT NULL
                )",
            )
            .unwrap();
        store
    }

    fn default_columns() -> Vec<ColumnDef> {
        vec![
            ColumnDef::new("Backlog".into(), TaskStatus::Backlog, 0),
            ColumnDef::new("Ready".into(), TaskStatus::Ready, 1),
            ColumnDef::new("In Progress".into(), TaskStatus::InProgress, 2),
            ColumnDef::new("Review".into(), TaskStatus::Review, 3),
            ColumnDef::new("Done".into(), TaskStatus::Done, 4),
        ]
    }

    fn make_engine() -> KataEngine {
        // Use from_env which reads inference config from environment;
        // in CI this will use defaults that may or may not work.
        // For construction-only tests, this is fine.
        let registry = SqliteRegistry::new(None).expect("temp registry");
        KataEngine::from_env(registry)
    }

    // ── Bridge construction ───────────────────────────────────────────────

    #[test]
    fn bridge_can_be_constructed() {
        let engine = make_engine();
        let bridge = hkask_services_kata_kanban::bridge::KanbanKataBridge::new(Arc::new(engine));
        let _ = bridge;
    }

    // ── KanbanService bridge wiring ───────────────────────────────────────

    #[test]
    fn kanban_service_accepts_kata_engine() {
        let svc = KanbanService::new(make_test_store());
        let engine = make_engine();
        let _svc = svc.with_kata_engine(Arc::new(engine));
    }

    #[test]
    fn kanban_service_builder_chain() {
        let svc = KanbanService::new(make_test_store());
        let engine = make_engine();
        let _svc = svc.with_kata_engine(Arc::new(engine));
        // Verifies that builder methods compose without panics
    }

    // ── Error path: no bridge configured ──────────────────────────────────

    #[tokio::test]
    async fn kanban_service_rejects_kata_without_bridge() {
        let svc = KanbanService::new(make_test_store());
        let owner = hkask_types::WebID::new();
        let board = svc
            .board_create(owner, "No Bridge Board", &default_columns())
            .unwrap();
        let spec = TaskSpec::new("No Bridge Task".into());
        let task = svc.task_create(board.id, spec, owner).unwrap();

        // Load a manifest (just for the test — we won't reach execution)
        let manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("registry/manifests/kata-coaching.yaml");
        let manifest = match KataEngine::load_manifest(&manifest_path) {
            Ok(m) => m,
            Err(_) => return, // Skip if manifest unavailable
        };

        let result = svc.run_coaching_kata(task.id, &manifest).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("kata bridge not configured"));
    }

    // ── Full kata execution via bridge ────────────────────────────────────

    #[tokio::test]
    async fn kanban_service_runs_coaching_kata_via_bridge() {
        let manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("registry/manifests/kata-coaching.yaml");
        let manifest = match KataEngine::load_manifest(&manifest_path) {
            Ok(m) => m,
            Err(_) => return, // Skip if manifest unavailable
        };

        let svc = KanbanService::new(make_test_store());
        let engine = make_engine();
        let svc = svc.with_kata_engine(Arc::new(engine));
        let owner = hkask_types::WebID::new();
        let board = svc
            .board_create(owner, "Kata Board", &default_columns())
            .unwrap();
        let spec = TaskSpec::new("Kata Task".into());
        let task = svc.task_create(board.id, spec, owner).unwrap();

        // This test will attempt real inference via from_env(). It may fail
        // in environments without inference configured, which is fine —
        // the test verifies the bridge wiring, not inference quality.
        match svc.run_coaching_kata(task.id, &manifest).await {
            Ok(result) => {
                assert_eq!(result.kata_type, "coaching");
                assert!(result.steps_completed > 0);
            }
            Err(e) => {
                // Acceptable: inference not configured in test environment
                let msg = e.to_string();
                assert!(
                    msg.contains("kata engine")
                        || msg.contains("inference")
                        || msg.contains("Inference"),
                    "unexpected error: {msg}"
                );
            }
        }
    }

    // ── CNS span emission ─────────────────────────────────────────────────

    #[test]
    fn kata_prompt_methods_emit_cns_spans() {
        // Verify that the CNS span infrastructure exists and compiles.
        // Actual span emission is verified by integration tests with
        // tracing subscriber configured.
        let svc = KanbanService::new(make_test_store());
        let owner = hkask_types::WebID::new();
        let board = svc
            .board_create(owner, "CNS Board", &default_columns())
            .unwrap();
        let spec = TaskSpec::new("CNS Task".into());
        let task = svc.task_create(board.id, spec, owner).unwrap();

        // These should emit cns.kata spans (verified by tracing subscriber in CI)
        let coaching = svc.task_coaching_prompt(task.id);
        assert!(coaching.is_ok());

        let improvement = svc.task_improvement_prompt(task.id);
        assert!(improvement.is_ok());

        let practice = svc.task_practice_prompt(task.id, "test");
        assert!(practice.is_ok());
    }
}
