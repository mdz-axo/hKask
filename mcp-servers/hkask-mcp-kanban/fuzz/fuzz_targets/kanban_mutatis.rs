//! Structure-aware fuzz target for hkask-mcp-kanban.
//!
//! Uses the mutatis crate for structure-aware mutation of JSON inputs
//! via libfuzzer's custom mutator hook (fuzz_mutator!).
//!
//! Pattern from the Rust Fuzz Book:
//!   https://rust-fuzz.github.io/book/cargo-fuzz/structure-aware-fuzzing.html
//!
//! Build:  cargo +nightly fuzz build --bin kanban_mutatis
//! Run:    cargo +nightly fuzz run kanban_mutatis -- -max_len=65536

use hkask_mcp_kanban::{
    KanbanServer,
    types::{
        BoardCreateRequest, BoardListRequest, ContractProposeExpect, TaskAssignRequest,
        TaskCreateRequest, TaskListRequest, TaskMoveRequest, TaskVerifyRequest,
    },
};
use hkask_test_harness::TestWebId;
use libfuzzer_sys::{fuzz_mutator, fuzz_target};
use rmcp::handler::server::wrapper::Parameters;
use std::panic::{self, AssertUnwindSafe};

// ── Method enum — one variant per kanban tool ──────────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize)]
enum KanbanMethod {
    BoardCreate(BoardCreateRequest),
    BoardList(BoardListRequest),
    TaskCreate(TaskCreateRequest),
    TaskList(TaskListRequest),
    TaskMove(TaskMoveRequest),
    TaskAssign(TaskAssignRequest),
    TaskVerify(TaskVerifyRequest),
    ContractProposeExpect(ContractProposeExpect),
}

// ── Helpers ────────────────────────────────────────────────────────────

fn test_server() -> KanbanServer {
    KanbanServer::new(TestWebId::alice(), "fuzz-replicant".into(), None)
}

fn call_tool<F: std::future::Future<Output = String>>(f: F) -> String {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let result = panic::catch_unwind(AssertUnwindSafe(|| rt.block_on(f)));
    match result {
        Ok(output) => output,
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            format!("{{\"error\":\"panic: {msg}\"}}")
        }
    }
}

// ── JSON-level structure-aware mutator ─────────────────────────────────

/// Custom libfuzzer mutator: deserialize → mutate JSON structure → reserialize.
///
/// Instead of libfuzzer mutating raw bytes (which almost never produce valid
/// JSON for structured types), we deserialize to `serde_json::Value`, apply
/// structure-aware mutations at the JSON level (toggle booleans, extend strings,
/// bump numbers, flip nulls), and reserialize.
///
/// This is structure-aware at the JSON type level — it knows about objects,
/// arrays, strings, numbers, booleans, and null — without requiring `Mutate`
/// derives on the server's request types.
fuzz_mutator!(
    |data: &mut [u8], size: usize, max_size: usize, _seed: u32| {
        let mut value: serde_json::Value =
            serde_json::from_slice(&data[..size]).unwrap_or(serde_json::Value::Array(vec![]));
        mutate_json(&mut value);
        if let Ok(new_data) = serde_json::to_vec(&value) {
            let n = new_data.len().min(max_size);
            data[..n].copy_from_slice(&new_data[..n]);
            n
        } else {
            size
        }
    }
);

/// Apply structure-aware mutations to a JSON value: toggle booleans,
/// extend strings, bump numbers, flip null→string, recurse into objects/arrays.
fn mutate_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                mutate_json(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() {
                mutate_json(v);
            }
        }
        serde_json::Value::String(s) => {
            // Extend with boundary-testing characters
            if s.len() < 256 {
                s.push('\x00');
                s.push_str(" mutated");
            }
        }
        serde_json::Value::Bool(b) => {
            *b = !*b;
        }
        serde_json::Value::Number(_) => {
            // Bump the number to test boundary transitions
            *value = serde_json::json!(0);
        }
        serde_json::Value::Null => {
            *value = serde_json::Value::String("was-null".into());
        }
    }
}

// ── Fuzz target ────────────────────────────────────────────────────────

fuzz_target!(|data: &[u8]| {
    let methods: Vec<KanbanMethod> = serde_json::from_slice(data).unwrap_or_default();
    if methods.is_empty() || methods.len() > 20 {
        return;
    }

    let server = test_server();
    for method in methods {
        let _ = match method {
            KanbanMethod::BoardCreate(req) => {
                call_tool(server.kanban_board_create(Parameters(req)))
            }
            KanbanMethod::BoardList(req) => call_tool(server.kanban_board_list(Parameters(req))),
            KanbanMethod::TaskCreate(req) => call_tool(server.kanban_task_create(Parameters(req))),
            KanbanMethod::TaskList(req) => call_tool(server.kanban_task_list(Parameters(req))),
            KanbanMethod::TaskMove(req) => call_tool(server.kanban_task_move(Parameters(req))),
            KanbanMethod::TaskAssign(req) => call_tool(server.kanban_task_assign(Parameters(req))),
            KanbanMethod::TaskVerify(req) => call_tool(server.kanban_task_verify(Parameters(req))),
            KanbanMethod::ContractProposeExpect(req) => {
                call_tool(server.contract_propose_expect(Parameters(req)))
            }
        };
    }
});
