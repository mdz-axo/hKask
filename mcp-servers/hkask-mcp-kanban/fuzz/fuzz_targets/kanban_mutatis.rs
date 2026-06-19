//! Structure-aware fuzz target for hkask-mcp-kanban.
//!
//! Uses the mutatis crate for structure-aware mutation of kanban tool inputs
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
use mutatis::Mutate;
use rmcp::handler::server::wrapper::Parameters;
use serde::{Deserialize, Serialize};
use std::panic::{self, AssertUnwindSafe};

// ── Method enum — one variant per kanban tool ──────────────────────────

/// A kanban tool invocation command. Each variant wraps the tool's request type.
/// Derives `Mutate` for structure-aware fuzzing and `Serialize`/`Deserialize`
/// for JSON round-tripping through the fuzz_mutator! hook.
#[derive(Debug, Mutate, Serialize, Deserialize)]
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

// ── Structure-aware mutator ────────────────────────────────────────────

/// Custom libfuzzer mutator: deserialize JSON → mutate structure → reserialize.
///
/// This is the key pattern from the Rust Fuzz Book. Instead of libfuzzer
/// mutating raw bytes (which would almost never produce valid JSON), we:
///   1. Deserialize the fuzzer's byte buffer into `Vec<KanbanMethod>`
///   2. Mutate the deserialized structure using mutatis (field-level mutation)
///   3. Reserialize back into the byte buffer for libfuzzer to use
///
/// This ensures every mutation produces valid JSON, dramatically improving
/// coverage compared to byte-level mutation on random data.
fuzz_mutator!(
    |data: &mut [u8], size: usize, max_size: usize, _seed: u32| {
        let mut methods: Vec<KanbanMethod> =
            serde_json::from_slice(&data[..size]).unwrap_or_default();
        let mut session = mutatis::Session::new();
        let _ = session.mutate(&mut methods);
        if let Ok(new_data) = serde_json::to_vec(&methods) {
            let n = new_data.len().min(max_size);
            data[..n].copy_from_slice(&new_data[..n]);
            n
        } else {
            size
        }
    }
);

// ── Fuzz target ────────────────────────────────────────────────────────

/// The actual fuzz harness. Receives structure-aware mutated bytes from
/// libfuzzer, deserializes them into kanban method invocations, and
/// dispatches each one through the full tool path under catch_unwind.
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
