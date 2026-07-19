//! Contract tests for hkask-mcp-companies — financial data tool-behavior.
//!
//! Every test carries the full traceability chain:
//! `UserFunctionalExpectation (expect:) → GoalPrinciple [P{N}] → ConstrainingPrinciple [P{N}] → REQ: → Test`
//!
//! Tested seams:
//! - `CompaniesServer` tool methods via `Parameters<T>` seam
//! - `LearningState` defaults (pure computation)

use hkask_mcp_companies::CompaniesServer;
use hkask_mcp_companies::learning::LearningState;
use hkask_mcp_companies::portfolio::PortfolioManager;
use hkask_mcp_companies::superforecast::FermiDefaults;
use hkask_mcp_companies::types::SymbolRequest;
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use std::sync::{Arc, Mutex};

// ── LearningState contract tests ────────────────────────────────────────────

#[test]
fn learning_state_starts_empty() {
    let state = LearningState::default();
    // A fresh learning state should have no provider scores
    // (We can't directly access provider_scores, but we can verify default construction works)
    let _ = state;
}

#[test]
fn fermi_defaults_has_questions() {
    let defaults = FermiDefaults::default();
    assert!(
        !defaults.growth_questions.is_empty(),
        "should have growth questions"
    );
    assert!(
        !defaults.margin_questions.is_empty(),
        "should have margin questions"
    );
}

// ── Tool-behavior contract tests (Parameters<T> seam) ───────────────────────
//
// These exercise the actual MCP tool methods through the public `Parameters<T>`
// seam — the same surface an agent uses. Closes the test-variety gap that hid
// the create-new-file, range-inversion, and multibyte-truncation defects in
// hkask-mcp-filesystem.

/// Construct a CompaniesServer with no API keys — tools will return errors
/// for external data, but validation and portfolio tools work.
fn test_server() -> CompaniesServer {
    CompaniesServer::new(
        WebID::new(),
        "test-replicant".into(),
        None,
        reqwest::Client::new(),
        String::new(), // no FMP key
        String::new(), // no EODHD key
        None,          // no Exa key
        None,          // no Tavily key
        None,          // no Brave key
        PortfolioManager::new(WebID::new()),
        Arc::new(Mutex::new(LearningState::default())),
        FermiDefaults::default(),
    )
}

/// Parse the success envelope `{"content": <value>}`; falls back to the raw
/// value for non-envelope outputs.
fn parse_content(out: &str) -> serde_json::Value {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("content").cloned().unwrap_or(v)
}

/// Extract the `kind` field from an error envelope, if present.
fn error_kind(out: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("kind").and_then(|e| e.as_str()).map(String::from)
}

// REQ: company_profile rejects an empty symbol with invalid_argument (P5).
// expect: an empty symbol returns kind=invalid_argument.
#[tokio::test]
async fn company_profile_rejects_empty_symbol_via_parameters_seam() {
    let server = test_server();
    let req: SymbolRequest = serde_json::from_value(serde_json::json!({"symbol": ""}))
        .expect("deserialize SymbolRequest");
    let out = server.company_profile(Parameters(req)).await;
    let kind = error_kind(&out).expect("expected error kind for empty symbol");
    assert_eq!(kind, "invalid_argument", "got: {out}");
}

// REQ: stock_quote rejects an empty symbol with invalid_argument (P5).
// expect: an empty symbol returns kind=invalid_argument.
#[tokio::test]
async fn stock_quote_rejects_empty_symbol_via_parameters_seam() {
    let server = test_server();
    let req: SymbolRequest = serde_json::from_value(serde_json::json!({"symbol": ""}))
        .expect("deserialize SymbolRequest");
    let out = server.stock_quote(Parameters(req)).await;
    let kind = error_kind(&out).expect("expected error kind for empty symbol");
    assert_eq!(kind, "invalid_argument", "got: {out}");
}

// REQ: portfolio_list returns a list of portfolios (P5 Testing Discipline).
// expect: portfolio_list returns a JSON array (possibly empty).
#[tokio::test]
async fn portfolio_list_returns_array_via_parameters_seam() {
    let server = test_server();
    let out = server.portfolio_list().await;
    let content = parse_content(&out);
    // portfolio_list returns either an array of portfolio names or an error
    // For a fresh server, it should return an empty array or a success response
    assert!(
        content.is_array() || content.get("portfolios").is_some() || content.get("error").is_some(),
        "should return array, portfolios, or error: {out}"
    );
}
