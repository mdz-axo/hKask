//! Companies MCP server fuzz targets.
//!
//! Covers all 19 company/portfolio request types.
//!
//! Pattern (a): deserialize_never_panics — arbitrary JSON → deserialize all request types.

use bolero::check;
use hkask_mcp_companies::CompaniesServer;
use hkask_mcp_companies::types::*;
use hkask_test_harness::TestWebId;
use rmcp::handler::server::wrapper::Parameters;
use std::panic::{self, AssertUnwindSafe};

// ── Helpers ────────────────────────────────────────────────────────────────

fn test_server() -> CompaniesServer {
    CompaniesServer::new(
        TestWebId::alice(),
        "fuzz-replicant".into(),
        None,
        "test-fmp-key".into(),
        "test-eodhd-key".into(),
    )
    .expect("companies server construction")
}

fn call_tool<F: std::future::Future<Output = String>>(f: F) -> String {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let result = panic::catch_unwind(AssertUnwindSafe(|| rt.block_on(f)));
    match result {
        Ok(output) => output,
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".to_string()
            };
            format!("{{\"error\":\"panic: {msg}\"}}")
        }
    }
}

// ── Pattern (a): Deserialize never panics ──────────────────────────────────

/// Deserialize arbitrary JSON into all companies request types — none may panic.
#[test]
fn fuzz_companies_deserialize_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        let _ = serde_json::from_str::<SymbolRequest>(s);
        let _ = serde_json::from_str::<SymbolLimitRequest>(s);
        let _ = serde_json::from_str::<HistoricalRequest>(s);
        let _ = serde_json::from_str::<SearchRequest>(s);
        let _ = serde_json::from_str::<ExpectationsGapRequest>(s);
        let _ = serde_json::from_str::<PortfolioNameRequest>(s);
        let _ = serde_json::from_str::<LedgerImportRequest>(s);
        let _ = serde_json::from_str::<LedgerExportRequest>(s);
        let _ = serde_json::from_str::<TransactionNoteRequest>(s);
        let _ = serde_json::from_str::<PortfolioCompareRequest>(s);
        let _ = serde_json::from_str::<PortfolioReturnsRequest>(s);
        let _ = serde_json::from_str::<NoteAddRequest>(s);
        let _ = serde_json::from_str::<NoteListRequest>(s);
        let _ = serde_json::from_str::<NoteDeleteRequest>(s);
        let _ = serde_json::from_str::<FileAttachRequest>(s);
        let _ = serde_json::from_str::<FileListRequest>(s);
        let _ = serde_json::from_str::<FileDeleteRequest>(s);
        let _ = serde_json::from_str::<AttributionRequest>(s);
        let _ = serde_json::from_str::<CharacteristicsRequest>(s);
    });
}

// ── Pattern (a): Tool dispatch never panics ───────────────────────────────

/// Full tool dispatch path must never panic under arbitrary deserialized input.
#[test]
fn fuzz_companies_tool_dispatch_never_panics() {
    check!().with_type::<String>().for_each(|s| {
        let server = test_server();

        // Try company_profile
        if let Ok(req) = serde_json::from_str::<SymbolRequest>(s) {
            let _output = call_tool(server.company_profile(Parameters(req)));
            return;
        }
        // Try income_statement
        if let Ok(req) = serde_json::from_str::<SymbolLimitRequest>(s) {
            let _output = call_tool(server.income_statement(Parameters(req)));
            return;
        }
        // Try historical_price
        if let Ok(req) = serde_json::from_str::<HistoricalRequest>(s) {
            let _output = call_tool(server.historical_price(Parameters(req)));
            return;
        }
        // Try symbol_search
        if let Ok(req) = serde_json::from_str::<SearchRequest>(s) {
            let _output = call_tool(server.symbol_search(Parameters(req)));
            return;
        }
        // Try moat_check
        if let Ok(req) = serde_json::from_str::<SymbolRequest>(s) {
            let _output = call_tool(server.moat_check(Parameters(req)));
            return;
        }
        // Try management_scorecard
        if let Ok(req) = serde_json::from_str::<SymbolRequest>(s) {
            let _output = call_tool(server.management_scorecard(Parameters(req)));
            return;
        }
        // Try working_capital_cycle
        if let Ok(req) = serde_json::from_str::<SymbolLimitRequest>(s) {
            let _output = call_tool(server.working_capital_cycle(Parameters(req)));
            return;
        }
        // Try expectations_gap
        if let Ok(req) = serde_json::from_str::<ExpectationsGapRequest>(s) {
            let _output = call_tool(server.expectations_gap(Parameters(req)));
            return;
        }
        // Try portfolio_delete
        if let Ok(req) = serde_json::from_str::<PortfolioNameRequest>(s) {
            let _output = call_tool(server.portfolio_delete(Parameters(req)));
            return;
        }
        // Try ledger_import
        if let Ok(req) = serde_json::from_str::<LedgerImportRequest>(s) {
            let _output = call_tool(server.ledger_import(Parameters(req)));
            return;
        }
        // Try ledger_export
        if let Ok(req) = serde_json::from_str::<LedgerExportRequest>(s) {
            let _output = call_tool(server.ledger_export(Parameters(req)));
            return;
        }
        // Try transaction_note_append
        if let Ok(req) = serde_json::from_str::<TransactionNoteRequest>(s) {
            let _output = call_tool(server.transaction_note_append(Parameters(req)));
            return;
        }
        // Try portfolio_comparison
        if let Ok(req) = serde_json::from_str::<PortfolioCompareRequest>(s) {
            let _output = call_tool(server.portfolio_comparison(Parameters(req)));
            return;
        }
        // Try portfolio_returns
        if let Ok(req) = serde_json::from_str::<PortfolioReturnsRequest>(s) {
            let _output = call_tool(server.portfolio_returns(Parameters(req)));
            return;
        }
        // Try note_add
        if let Ok(req) = serde_json::from_str::<NoteAddRequest>(s) {
            let _output = call_tool(server.note_add(Parameters(req)));
            return;
        }
        // Try note_list
        if let Ok(req) = serde_json::from_str::<NoteListRequest>(s) {
            let _output = call_tool(server.note_list(Parameters(req)));
            return;
        }
        // Try note_delete
        if let Ok(req) = serde_json::from_str::<NoteDeleteRequest>(s) {
            let _output = call_tool(server.note_delete(Parameters(req)));
            return;
        }
        // Try file_attach
        if let Ok(req) = serde_json::from_str::<FileAttachRequest>(s) {
            let _output = call_tool(server.file_attach(Parameters(req)));
            return;
        }
        // Try file_list
        if let Ok(req) = serde_json::from_str::<FileListRequest>(s) {
            let _output = call_tool(server.file_list(Parameters(req)));
            return;
        }
        // Try file_delete
        if let Ok(req) = serde_json::from_str::<FileDeleteRequest>(s) {
            let _output = call_tool(server.file_delete(Parameters(req)));
            return;
        }
        // Try portfolio_attribution
        if let Ok(req) = serde_json::from_str::<AttributionRequest>(s) {
            let _output = call_tool(server.portfolio_attribution(Parameters(req)));
            return;
        }
        // Try portfolio_characteristics
        if let Ok(req) = serde_json::from_str::<CharacteristicsRequest>(s) {
            let _output = call_tool(server.portfolio_characteristics(Parameters(req)));
            return;
        }
        // portfolio_list takes no parameters — always dispatchable fallback
        let _output = call_tool(server.portfolio_list());
    });
}
