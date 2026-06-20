# Provider Intelligence Service — Specification

**Version:** v0.30.0
**Status:** Approved — pending implementation
**Last updated:** 2026-06-20
**Depends on:** `docs/architecture/specs/rjoule-cost-system.md`, `docs/architecture/specs/hkask-ledger.md`

---

## 1. Purpose

Track actual provider costs in real-time, not estimated from price cards. Detect when usage shifts from pre-paid/subscription to marginal/overage pricing. Feed actual costs into the rJoule cost ledger so hKask knows its true cash cost per operation.

## 2. Provider Intelligence Trait

```rust
#[async_trait]
pub trait ProviderIntelligence: Send + Sync {
    fn provider_id(&self) -> &'static str;

    /// Discover current tier, limits, and pricing for this API key.
    async fn discover(&self, api_key: &str) -> Result<ProviderState, ProviderError>;

    /// Query current billing period usage.
    async fn usage(&self, api_key: &str) -> Result<UsageStatus, ProviderError>;

    /// The actual per-unit cost being charged RIGHT NOW.
    async fn actual_cost(&self, api_key: &str) -> Result<CostRate, ProviderError>;
}

pub struct ProviderState {
    pub tier: String,
    pub monthly_limit: Option<u64>,
    pub limit_unit: LimitUnit,
    pub overage_rate: Option<CostRate>,
    pub billing_period_start: DateTime<Utc>,
}

pub struct UsageStatus {
    pub consumed: u64,
    pub limit: u64,
    pub fraction: f64,
    pub estimated_exhaustion: Option<DateTime<Utc>>,
}

pub enum LimitUnit { Tokens, Calls, Credits, Dollars }

pub struct CostRate {
    pub input_nj_per_unit: u64,
    pub output_nj_per_unit: u64,
    pub fixed_nj_per_call: u64,
    pub is_marginal: bool,
}
```

## 3. Adaptive Usage Monitoring

```
IF usage_pct < 50%   → check daily
IF 50% ≤ usage_pct < 70% → check every 6 hours
IF 70% ≤ usage_pct < 90% → check hourly
IF usage_pct ≥ 90%        → check every 10 minutes
```

When the daemon detects pre-paid → marginal shift:
1. `CostRate.is_marginal` flips to `true`
2. CNS span `cns.provider.marginal_activated` emitted
3. All subsequent per-call costs use marginal `CostRate`
4. Shift point recorded in cost ledger for analysis

## 4. Per-Provider Profiles

### 4.1 DeepInfra — Pay-as-you-go
- `is_marginal`: Always true. No subscription tiers.
- Cost discovery: `GET /v1/usage` returns token counts.
- Complexity: Low.

### 4.2 OpenRouter — Credit-based
- `is_marginal`: Always true (credits always consumed).
- Cost discovery: `GET /api/v1/auth/key` returns `credits_remaining`.
- Acceleration trigger: `credits_remaining < 30%`.
- Complexity: Medium.

### 4.3 Together AI — Free credits + pay-as-you-go
- `is_marginal`: False until free credits exhausted.
- Cost discovery: `GET /api/v1/usage`.
- Complexity: Medium.

### 4.4 Brave Search — Tiered, self-tracked
- No public usage API. hKask tracks calls via ledger.
- `is_marginal`: False until ledger call count > tier limit.
- Acceleration trigger: ledger count > 70% of tier limit.
- Complexity: Medium.

### 4.5 Firecrawl — Tiered, credit-based
- `GET /v1/account` may return usage. Fallback: self-tracked.
- Complexity: Medium.

### 4.6 Tavily, Exa, FMP, EODHD — Tiered, self-tracked
- No public usage API. hKask tracks via ledger.
- Complexity: Medium.

### 4.7 Runpod, Baseten — Pay-as-you-go GPU
- Runpod: billing API available. Baseten: dashboard only.
- `is_marginal`: Always true.
- Complexity: Low for Runpod, medium for Baseten.

## 5. Provider Config Template

```yaml
# registry/providers/brave.yaml
provider:
  id: brave
  name: Brave Search
  pricing_model: tiered
  usage_api: null
  api_key_env: BRAVE_API_KEY
  subscription_tiers:
    - name: free
      monthly_call_limit: 2000
      overage_per_call_nj: 1000000
    - name: base
      monthly_call_limit: 20000
      overage_per_call_nj: 800000
    - name: pro
      monthly_call_limit: null
      overage_per_call_nj: 0
  current_tier: free
  billing_cycle_start_day: 1
```

## 6. Resolved Design Decisions

1. **Self-tracked providers:** Persistent call counter survives restarts. Each API call writes a ledger transaction. Balance of `cost:api/<provider>` IS the call count or cost total. Ledger is the source of truth.

2. **"Dumb" API config:** YAML files at `registry/providers/<name>.yaml`. Admin declares tier, limits, overage rates. Template in §5.

3. **Multi-key:** Aggregate tracking. All keys for same provider aggregate to one cost account. Key rotation is transparent.

4. **Reconciliation:** Write a reconciliation transaction when billing diverges: `Posting { source: "cost:api/<provider>", destination: "cost:reconciliation", asset: "rJ", amount: <delta> }`.

5. **Rate limit (429):** Already tracked as `failed_api_cost_urj` in CostTracker.
