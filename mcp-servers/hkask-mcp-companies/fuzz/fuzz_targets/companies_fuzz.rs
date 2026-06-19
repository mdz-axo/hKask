//! Companies MCP server fuzz targets.
//!
//! Covers all 19 company/portfolio request types.
//!
//! Pattern (a): deserialize_never_panics — arbitrary JSON → deserialize all request types.

use bolero::check;
use hkask_mcp_companies::types::*;

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
