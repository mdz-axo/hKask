//! Provider Intelligence — real-time provider cost and usage tracking.
//!
//! Provides the `ProviderIntelligence` trait for discovering current tier,
//! billing-period usage, and actual per-unit costs from provider APIs.
//! Implementations exist per provider (DeepInfra, OpenRouter, Together, etc.).

use chrono::Datelike;
use serde::Deserialize;

/// Errors from provider intelligence operations.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("JSON parse error: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("usage API not available for this provider")]
    NoUsageApi,
}

impl From<reqwest::Error> for ProviderError {
    fn from(e: reqwest::Error) -> Self {
        ProviderError::Http(format!("{e}"))
    }
}

/// Unit in which a provider measures consumption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LimitUnit {
    Tokens,
    Calls,
    Credits,
    Dollars,
}

/// Current provider tier and billing state.
#[derive(Debug, Clone)]
pub struct ProviderState {
    pub tier: String,
    pub monthly_limit: Option<u64>,
    pub limit_unit: LimitUnit,
    pub overage_rate: Option<CostRate>,
    pub billing_period_start: chrono::DateTime<chrono::Utc>,
}

/// Billing-period usage status.
#[derive(Debug, Clone)]
pub struct UsageStatus {
    pub consumed: u64,
    pub limit: u64,
    pub fraction: f64,
    pub estimated_exhaustion: Option<chrono::DateTime<chrono::Utc>>,
}

/// Per-unit cost rate, in nano-rJoules (nJ). 1 nJ = 0.001 µrJ.
/// 30 nJ/token = $0.03 per million tokens.
#[derive(Debug, Clone)]
pub struct CostRate {
    pub input_nj_per_unit: u64,
    pub output_nj_per_unit: u64,
    pub fixed_nj_per_call: u64,
    pub is_marginal: bool,
}

/// The ProviderIntelligence trait — discover, monitor, and cost-track any provider.
///
/// Implementations are per-provider. DeepInfra is the reference implementation
/// (always marginal, pay-as-you-go). Providers like Brave/Firecrawl without usage
/// APIs use self-tracking via the cost ledger.
#[async_trait::async_trait]
pub trait ProviderIntelligence: Send + Sync {
    /// Stable identifier for this provider (e.g., "deepinfra", "openrouter").
    fn provider_id(&self) -> &'static str;

    /// Discover current tier, limits, and pricing for the given API key.
    async fn discover(&self, api_key: &str) -> Result<ProviderState, ProviderError>;

    /// Query current billing-period usage.
    async fn usage(&self, api_key: &str) -> Result<UsageStatus, ProviderError>;

    /// The actual per-unit cost being charged RIGHT NOW.
    /// Returns the marginal cost rate if overage has been triggered,
    /// or the base (pre-paid/subscription) rate otherwise.
    async fn actual_cost(&self, api_key: &str) -> Result<CostRate, ProviderError>;
}

// ── DeepInfra Provider ──────────────────────────────────────────────────────────

/// DeepInfra provider — always marginal, pay-as-you-go.
///
/// No subscription tiers. Cost is always the per-token rate.
/// Usage data available at `GET https://api.deepinfra.com/v1/usage`.
pub struct DeepInfraProvider;

/// DeepInfra usage API response.
#[derive(Debug, Deserialize)]
struct DeepInfraUsage {
    /// Total tokens consumed this billing period.
    total_tokens: Option<u64>,
}

impl DeepInfraProvider {
    /// DeepInfra per-token pricing in nano-rJ (nJ).
    /// $0.03/M input = 30 nJ/token, $0.06/M output = 60 nJ/token.
    pub const INPUT_NJ_PER_TOKEN: u64 = 30;
    pub const OUTPUT_NJ_PER_TOKEN: u64 = 60;

    /// Default billing period start (assume 1st of current month if unknown).
    fn default_billing_start() -> chrono::DateTime<chrono::Utc> {
        let now = chrono::Utc::now();
        chrono::DateTime::<chrono::Utc>::from_timestamp(
            chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
                .and_then(|d| d.and_hms_opt(0, 0, 0))
                .map(|dt| dt.and_utc().timestamp())
                .unwrap_or(0),
            0,
        )
        .unwrap_or(now)
    }
}

#[async_trait::async_trait]
impl ProviderIntelligence for DeepInfraProvider {
    fn provider_id(&self) -> &'static str {
        "deepinfra"
    }

    async fn discover(&self, _api_key: &str) -> Result<ProviderState, ProviderError> {
        // DeepInfra is always marginal with no tier limits
        Ok(ProviderState {
            tier: "pay-as-you-go".into(),
            monthly_limit: None,
            limit_unit: LimitUnit::Tokens,
            overage_rate: None,
            billing_period_start: Self::default_billing_start(),
        })
    }

    async fn usage(&self, api_key: &str) -> Result<UsageStatus, ProviderError> {
        let url = "https://api.deepinfra.com/v1/usage";
        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .header("Authorization", format!("Bearer {api_key}"))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| ProviderError::Http(format!("DeepInfra usage request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!(
                "DeepInfra usage API returned {status}: {body}"
            )));
        }

        let data: DeepInfraUsage = resp.json().await?;
        let consumed = data.total_tokens.unwrap_or(0);

        Ok(UsageStatus {
            consumed,
            limit: u64::MAX, // no hard limit on pay-as-you-go
            fraction: 0.0,
            estimated_exhaustion: None,
        })
    }

    async fn actual_cost(&self, _api_key: &str) -> Result<CostRate, ProviderError> {
        // DeepInfra is always marginal — pay per token, no free tier/pre-paid
        Ok(CostRate {
            input_nj_per_unit: Self::INPUT_NJ_PER_TOKEN,
            output_nj_per_unit: Self::OUTPUT_NJ_PER_TOKEN,
            fixed_nj_per_call: 0,
            is_marginal: true,
        })
    }
}
