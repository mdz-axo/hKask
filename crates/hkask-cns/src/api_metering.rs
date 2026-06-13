//! API metering — per-key rate limiting, gas tracking, and CNS spans.
//!
//! # Design (essentialist G2)
//! - Rate limit state is in-memory (HashMap). Acceptable per handoff:
//!   "What happens to rate limit state on process restart? Acceptable?"
//! - Rate-limited vs allocation-exhausted: separate span fields, not separate span types.
//! - `endpoint_weight` table: hardcoded initially (configurable later).
//!
//! # Span: cns.api.request
//! Every API call with `Authorization: Bearer hk_...` opens a span tracking:
//! `key_id, endpoint, scope_matched, gas_consumed, allocation_remaining, rate_limit_status`

use hkask_types::wallet::{ApiKeyId, Encumbrance};
use std::collections::HashMap;
use std::time::Instant;

// ── Endpoint weight table (hardcoded, per essentialist G2) ────────────────────

/// Weight multiplier per endpoint category. Heavier endpoints cost more gas.
#[derive(Debug, Clone, Copy)]
pub struct EndpointWeight(pub f64);

impl Default for EndpointWeight {
    fn default() -> Self {
        EndpointWeight(1.0)
    }
}

/// Look up the weight for an endpoint path.
/// Hardcoded table — configurable in future release.
pub fn endpoint_weight(path: &str) -> EndpointWeight {
    if path.contains("embed-corpus") || path.contains("compose") {
        EndpointWeight(5.0)
    } else if path.contains("chat") || path.contains("invoke") {
        EndpointWeight(2.0)
    } else {
        EndpointWeight(1.0)
    }
}

// ── Rate limit state (in-memory) ──────────────────────────────────────────────

/// Per-key rate limit tracking.
#[derive(Debug, Clone)]
struct RateLimitBucket {
    /// Timestamps of requests in the current minute window.
    request_timestamps: Vec<Instant>,
    /// Tokens consumed today (UTC day boundary).
    tokens_today: u64,
    /// Day identifier (UTC date string) for token reset.
    day_key: String,
}

impl RateLimitBucket {
    fn new() -> Self {
        Self {
            request_timestamps: Vec::new(),
            tokens_today: 0,
            day_key: String::new(),
        }
    }

    /// Prune timestamps older than 60 seconds.
    fn prune(&mut self, now: Instant) {
        let cutoff = now - std::time::Duration::from_secs(60);
        self.request_timestamps.retain(|t| *t >= cutoff);
    }

    /// Check if a new request would exceed the per-minute limit.
    fn check_rpm(&mut self, now: Instant, max_rpm: u32) -> bool {
        self.prune(now);
        (self.request_timestamps.len() as u32) < max_rpm
    }

    /// Record a request.
    fn record_request(&mut self, now: Instant) {
        self.prune(now);
        self.request_timestamps.push(now);
    }

    /// Check and record token consumption for today.
    fn check_tokens(&mut self, tokens: u64, max_tokens_per_day: u64, today: &str) -> bool {
        if self.day_key != today {
            self.tokens_today = 0;
            self.day_key = today.to_string();
        }
        if self.tokens_today + tokens > max_tokens_per_day {
            return false;
        }
        self.tokens_today += tokens;
        true
    }
}

/// Rate limit status returned after a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitStatus {
    /// Request is within all limits.
    Ok,
    /// Per-minute request rate exceeded.
    RateExceeded,
    /// Daily token quota exceeded.
    TokensExceeded,
}

impl RateLimitStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::RateExceeded => "rate_exceeded",
            Self::TokensExceeded => "tokens_exceeded",
        }
    }
}

// ── API Meter ────────────────────────────────────────────────────────────────

/// In-memory API meter for per-key rate limiting and gas tracking.
///
/// # Design (essentialist G2)
/// - Single `HashMap<ApiKeyId, RateLimitBucket>` — no separate store abstraction.
/// - `check_and_record` is the single entry point for rate limit enforcement.
pub struct ApiMeter {
    buckets: HashMap<ApiKeyId, RateLimitBucket>,
}

impl ApiMeter {
    /// Create a new empty meter.
    pub fn new() -> Self {
        Self {
            buckets: HashMap::new(),
        }
    }

    /// Check rate limits and record the request if within limits.
    ///
    /// Returns `RateLimitStatus::Ok` if the request can proceed.
    /// Returns the appropriate exceeded status otherwise.
    ///
    /// # Arguments
    /// * `key_id` — The API key making the request.
    /// * `max_rpm` — Maximum requests per minute for this key.
    /// * `max_tokens_per_day` — Maximum tokens per day for this key.
    /// * `tokens_this_request` — Estimated tokens for this request.
    pub fn check_and_record(
        &mut self,
        key_id: ApiKeyId,
        max_rpm: u32,
        max_tokens_per_day: u64,
        tokens_this_request: u64,
    ) -> RateLimitStatus {
        let now = Instant::now();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let bucket = self
            .buckets
            .entry(key_id)
            .or_insert_with(RateLimitBucket::new);

        if !bucket.check_rpm(now, max_rpm) {
            return RateLimitStatus::RateExceeded;
        }

        if !bucket.check_tokens(tokens_this_request, max_tokens_per_day, &today) {
            return RateLimitStatus::TokensExceeded;
        }

        bucket.record_request(now);
        RateLimitStatus::Ok
    }

    /// Get the current request count in the last minute for a key.
    pub fn current_rpm(&self, key_id: ApiKeyId) -> u32 {
        let now = Instant::now();
        self.buckets
            .get(&key_id)
            .map(|b| {
                let cutoff = now - std::time::Duration::from_secs(60);
                b.request_timestamps
                    .iter()
                    .filter(|t| **t >= cutoff)
                    .count() as u32
            })
            .unwrap_or(0)
    }
}

impl Default for ApiMeter {
    fn default() -> Self {
        Self::new()
    }
}

// ── CNS span: cns.api.request ────────────────────────────────────────────────

/// Observation data for a `cns.api.request` span.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ApiRequestSpan {
    pub key_id: String,
    pub endpoint: String,
    pub scope_matched: bool,
    pub gas_consumed: u64,
    pub allocation_remaining: u64,
    pub rate_limit_status: String,
}

impl ApiRequestSpan {
    /// Build a span observation from metering data.
    pub fn new(
        key_id: &str,
        endpoint: &str,
        scope_matched: bool,
        gas_consumed: u64,
        encumbrance: Option<&Encumbrance>,
        rate_limit_status: RateLimitStatus,
    ) -> Self {
        Self {
            key_id: key_id.to_string(),
            endpoint: endpoint.to_string(),
            scope_matched,
            gas_consumed,
            allocation_remaining: encumbrance.map(|e| e.remaining_rj()).unwrap_or(0),
            rate_limit_status: rate_limit_status.as_str().to_string(),
        }
    }
}

// ── Alert types ──────────────────────────────────────────────────────────────

/// Alert types emitted by the API metering system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiMeteringAlert {
    /// Key exceeded its rate limit.
    RateLimitExceeded { key_id: ApiKeyId, endpoint: String },
    /// Key allocation dropped below 20%.
    AllocationLow {
        key_id: ApiKeyId,
        remaining_rj: u64,
        total_rj: u64,
    },
    /// Key allocation exhausted (≤ 0).
    AllocationExhausted { key_id: ApiKeyId },
    /// Potential abuse pattern detected (3+ anomalies).
    AnomalyAbuse { key_id: ApiKeyId, pattern: String },
    /// Key used for endpoint outside declared scope.
    ScopeViolation {
        key_id: ApiKeyId,
        endpoint: String,
        allowed_scope: Vec<String>,
    },
}

impl ApiMeteringAlert {
    /// CNS alert type string for span emission.
    pub fn alert_type(&self) -> &'static str {
        match self {
            Self::RateLimitExceeded { .. } => "cns.api.rate_limit_exceeded",
            Self::AllocationLow { .. } => "cns.api.allocation_low",
            Self::AllocationExhausted { .. } => "cns.api.allocation_exhausted",
            Self::AnomalyAbuse { .. } => "cns.api.anomaly_abuse",
            Self::ScopeViolation { .. } => "cns.api.scope_violation",
        }
    }

    /// Severity level for CNS algedonic signaling.
    pub fn severity(&self) -> &'static str {
        match self {
            Self::RateLimitExceeded { .. } => "warning",
            Self::AllocationLow { .. } => "info",
            Self::AllocationExhausted { .. } => "critical",
            Self::AnomalyAbuse { .. } => "critical",
            Self::ScopeViolation { .. } => "warning",
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // REQ: cns-api-meter-001 — endpoint_weight returns correct multipliers
    #[test]
    fn endpoint_weight_embed_corpus_is_heavy() {
        assert!((endpoint_weight("embed-corpus").0 - 5.0).abs() < f64::EPSILON);
        assert!((endpoint_weight("compose").0 - 5.0).abs() < f64::EPSILON);
    }

    // REQ: cns-api-meter-002 — endpoint_weight defaults to 1.0
    #[test]
    fn endpoint_weight_default_is_one() {
        assert!((endpoint_weight("read-specs").0 - 1.0).abs() < f64::EPSILON);
        assert!((endpoint_weight("unknown").0 - 1.0).abs() < f64::EPSILON);
    }

    // REQ: cns-api-meter-003 — rate limit bucket prunes old requests
    #[test]
    fn rate_limit_bucket_prunes_old_requests() {
        let mut bucket = RateLimitBucket::new();
        let now = Instant::now();
        let old = now - std::time::Duration::from_secs(61);

        bucket.request_timestamps.push(old);
        bucket.request_timestamps.push(now);
        bucket.prune(now);

        assert_eq!(bucket.request_timestamps.len(), 1);
    }

    // REQ: cns-api-meter-004 — rate limit bucket enforces RPM
    #[test]
    fn rate_limit_bucket_enforces_rpm() {
        let mut bucket = RateLimitBucket::new();
        let now = Instant::now();

        // Fill to limit (max_rpm = 3)
        for _ in 0..3 {
            assert!(bucket.check_rpm(now, 3));
            bucket.record_request(now);
        }
        // 4th should be rejected
        assert!(!bucket.check_rpm(now, 3));
    }

    // REQ: cns-api-meter-005 — token tracking resets on new day
    #[test]
    fn token_tracking_resets_on_new_day() {
        let mut bucket = RateLimitBucket::new();
        assert!(bucket.check_tokens(500, 1000, "2026-06-13"));
        assert_eq!(bucket.tokens_today, 500);
        // New day resets
        assert!(bucket.check_tokens(800, 1000, "2026-06-14"));
        assert_eq!(bucket.tokens_today, 800);
    }

    // REQ: cns-api-meter-006 — ApiMeter check_and_record enforces limits
    #[test]
    fn api_meter_enforces_limits() {
        let mut meter = ApiMeter::new();
        let key = ApiKeyId::new();

        // First 3 requests within limit
        for _ in 0..3 {
            assert_eq!(
                meter.check_and_record(key, 5, 10000, 100),
                RateLimitStatus::Ok
            );
        }

        // RPM should show 3
        assert_eq!(meter.current_rpm(key), 3);
    }

    // REQ: cns-api-meter-007 — ApiRequestSpan serializes correctly
    #[test]
    fn api_request_span_serialization() {
        let span = ApiRequestSpan::new(
            "k_test",
            "/api/specs/123",
            true,
            500,
            None,
            RateLimitStatus::Ok,
        );
        let json = serde_json::to_string(&span).unwrap();
        assert!(json.contains("k_test"));
        assert!(json.contains("/api/specs/123"));
        assert!(json.contains("ok"));
    }

    // REQ: cns-api-meter-008 — alert types have correct severity
    #[test]
    fn alert_severity_levels() {
        assert_eq!(
            ApiMeteringAlert::AllocationExhausted {
                key_id: ApiKeyId::new()
            }
            .severity(),
            "critical"
        );
        assert_eq!(
            ApiMeteringAlert::RateLimitExceeded {
                key_id: ApiKeyId::new(),
                endpoint: "/test".into()
            }
            .severity(),
            "warning"
        );
        assert_eq!(
            ApiMeteringAlert::AllocationLow {
                key_id: ApiKeyId::new(),
                remaining_rj: 100,
                total_rj: 1000
            }
            .severity(),
            "info"
        );
    }
}
