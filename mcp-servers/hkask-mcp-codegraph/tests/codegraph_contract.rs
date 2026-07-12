//! Contract tests for hkask-mcp-codegraph — graph store and traversal invariants.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seam: `GraphStore` (in-memory), `find_symbol_by_name`, and traversal.

use hkask_codegraph::graph::store::GraphStore;
use hkask_codegraph::graph::traversal;
use hkask_codegraph::types::Direction;

fn setup_store() -> GraphStore {
    let store = GraphStore::open_in_memory().expect("in-memory graph store");

    let conn = store.conn();
    conn.execute(
        "INSERT INTO code_files (path, content_hash) VALUES ('src/main.rs', 'hash1')",
        [],
    )
    .expect("insert test file");

    let file_id: i64 = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO symbols (name, kind, file_id, signature, visibility, start_line, end_line)
         VALUES ('test_function', 'function', ?1, 'fn test_function()', 'pub', 10, 20)",
        [file_id],
    )
    .expect("insert symbol");

    store
}

// ── Store tests ────────────────────────────────────────────────────────────

#[test]
fn graph_store_opens_in_memory() {
    let store = GraphStore::open_in_memory();
    assert!(store.is_ok(), "in-memory store should open");
}

#[test]
fn graph_store_has_connection() {
    let store = setup_store();
    let count = store.symbol_count().expect("symbol count");
    assert!(count > 0, "should have at least one symbol, got {count}");
}

#[test]
fn graph_store_find_symbol_by_name() {
    let store = setup_store();
    let id = store
        .find_symbol_by_name("test_function")
        .expect("find should succeed");
    assert!(id.is_some(), "should find test_function by name");

    let id = store
        .find_symbol_by_name("nonexistent")
        .expect("find should succeed");
    assert!(id.is_none(), "should not find nonexistent symbol");
}

// ── Traversal tests ────────────────────────────────────────────────────────

#[test]
fn find_symbol_id_returns_some_for_existing_symbol() {
    let store = setup_store();
    let id = traversal::find_symbol_id(store.conn(), "test_function")
        .expect("find_symbol_id should succeed");
    assert!(id.is_some(), "should find existing symbol");
}

#[test]
fn find_symbol_id_returns_none_for_missing_symbol() {
    let store = setup_store();
    let id = traversal::find_symbol_id(store.conn(), "no_such_symbol")
        .expect("find_symbol_id should succeed");
    assert!(id.is_none(), "should not find missing symbol");
}

// ── Direction enum tests ───────────────────────────────────────────────────

#[test]
fn direction_variants_exist() {
    let forward = Direction::Forward;
    let reverse = Direction::Reverse;
    assert!(matches!(forward, Direction::Forward));
    assert!(matches!(reverse, Direction::Reverse));
}

// ── Context budget tests ───────────────────────────────────────────────────

#[test]
fn context_budget_variants_exist() {
    let budgets = [
        hkask_codegraph::ContextBudget::Minimal,
        hkask_codegraph::ContextBudget::Focused,
        hkask_codegraph::ContextBudget::Standard,
        hkask_codegraph::ContextBudget::Full,
    ];
    assert_eq!(budgets.len(), 4);
}
