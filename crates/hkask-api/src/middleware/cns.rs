//! CNS span middleware — emits tracing spans on every HTTP request.
//!
//! Applied as the outermost middleware layer so all requests are captured
//! regardless of auth or route-level filtering.

use axum::{body::Body, http::Request, middleware::Next, response::Response};
use std::time::Instant;

/// CNS middleware — emits `request` and `response` spans for every HTTP request.
///
/// expect: "API endpoints enforce OCAP boundaries"
/// pre:  req is an incoming HTTP request
/// post: cns.api request span emitted with method + path
/// post: cns.api response span emitted with status + latency_ms
pub async fn cns_middleware(req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let start = Instant::now();

    // P9: CNS span
    tracing::info!(target: "hkask.api", operation = "request", method = %method, path = %path, "CNS");

    let response = next.run(req).await;

    let status = response.status().as_u16();
    // P9: CNS span
    tracing::info!(target: "hkask.api", operation = "response", status = status, latency_ms = start.elapsed().as_millis(), "CNS");

    response
}
