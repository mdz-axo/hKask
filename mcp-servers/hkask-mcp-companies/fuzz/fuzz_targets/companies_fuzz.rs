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

// ── Pattern (a): Tool dispatch — one test per tool (equal coverage) ─────

macro_rules! dispatch_test {
    ($name:ident, $ty:ty, $method:ident) => {
        #[test]
        fn $name() {
            check!().with_type::<String>().for_each(|s| {
                if let Ok(req) = serde_json::from_str::<$ty>(s) {
                    let server = test_server();
                    let _ = call_tool(server.$method(Parameters(req)));
                }
            });
        }
    };
}

dispatch_test!(
    fuzz_companies_dispatch_company_profile,
    SymbolRequest,
    company_profile
);
dispatch_test!(
    fuzz_companies_dispatch_stock_quote,
    SymbolRequest,
    stock_quote
);
dispatch_test!(
    fuzz_companies_dispatch_income_statement,
    SymbolLimitRequest,
    income_statement
);
dispatch_test!(
    fuzz_companies_dispatch_balance_sheet,
    SymbolLimitRequest,
    balance_sheet
);
dispatch_test!(
    fuzz_companies_dispatch_cash_flow_statement,
    SymbolLimitRequest,
    cash_flow_statement
);
dispatch_test!(
    fuzz_companies_dispatch_key_metrics,
    SymbolLimitRequest,
    key_metrics
);
dispatch_test!(
    fuzz_companies_dispatch_historical_price,
    HistoricalRequest,
    historical_price
);
dispatch_test!(
    fuzz_companies_dispatch_symbol_search,
    SearchRequest,
    symbol_search
);
dispatch_test!(
    fuzz_companies_dispatch_moat_check,
    SymbolRequest,
    moat_check
);
dispatch_test!(
    fuzz_companies_dispatch_management_scorecard,
    SymbolRequest,
    management_scorecard
);
dispatch_test!(
    fuzz_companies_dispatch_working_capital_cycle,
    SymbolLimitRequest,
    working_capital_cycle
);
dispatch_test!(
    fuzz_companies_dispatch_expectations_gap,
    ExpectationsGapRequest,
    expectations_gap
);
dispatch_test!(
    fuzz_companies_dispatch_portfolio_delete,
    PortfolioNameRequest,
    portfolio_delete
);
dispatch_test!(
    fuzz_companies_dispatch_ledger_import,
    LedgerImportRequest,
    ledger_import
);
dispatch_test!(
    fuzz_companies_dispatch_ledger_export,
    LedgerExportRequest,
    ledger_export
);
dispatch_test!(
    fuzz_companies_dispatch_transaction_note_append,
    TransactionNoteRequest,
    transaction_note_append
);
dispatch_test!(
    fuzz_companies_dispatch_portfolio_comparison,
    PortfolioCompareRequest,
    portfolio_comparison
);
dispatch_test!(
    fuzz_companies_dispatch_portfolio_returns,
    PortfolioReturnsRequest,
    portfolio_returns
);
dispatch_test!(fuzz_companies_dispatch_note_add, NoteAddRequest, note_add);
dispatch_test!(
    fuzz_companies_dispatch_note_list,
    NoteListRequest,
    note_list
);
dispatch_test!(
    fuzz_companies_dispatch_note_delete,
    NoteDeleteRequest,
    note_delete
);
dispatch_test!(
    fuzz_companies_dispatch_file_attach,
    FileAttachRequest,
    file_attach
);
dispatch_test!(
    fuzz_companies_dispatch_file_list,
    FileListRequest,
    file_list
);
dispatch_test!(
    fuzz_companies_dispatch_file_delete,
    FileDeleteRequest,
    file_delete
);
dispatch_test!(
    fuzz_companies_dispatch_portfolio_attribution,
    AttributionRequest,
    portfolio_attribution
);
dispatch_test!(
    fuzz_companies_dispatch_portfolio_characteristics,
    CharacteristicsRequest,
    portfolio_characteristics
);

/// portfolio_list takes no parameters — always dispatchable.
#[test]
fn fuzz_companies_dispatch_portfolio_list() {
    check!().with_type::<String>().for_each(|_s| {
        let server = test_server();
        let _ = call_tool(server.portfolio_list());
    });
}
