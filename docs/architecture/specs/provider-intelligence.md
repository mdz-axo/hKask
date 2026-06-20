# Provider Intelligence Service — Specification

**Version:** v0.30.0
**Status:** Draft
**Last updated:** 2026-06-20
**Depends on:** `docs/architecture/specs/rjoule-cost-system.md`, planned `crates/hkask-ledger`

---

## 1. Purpose

Track actual provider costs in real-time, not estimated from price cards. Detect when usage shifts from pre-paid/subscription to marginal/overage pricing. Feed actual costs into the rJoule ledger so hKask knows its true cash cost per operation.

## 2. Provider Intelligence Trait

Each API provider implements a common trait:

```rust
#[async_trait]
pub trait ProviderIntelligence: Send + Sync {
    /// Unique identifier matching classifier config `provider` field.
    fn provider_id(&self) -> &'static str;

    /// Discover the current tier, limits, and pricing for this API key.
    /// Called once at startup and when key/environment changes.
    async fn discover(&self, api_key: &str) -> Result<ProviderState, ProviderError>;

    /// Query current billing period usage. Returns fraction of limit consumed.
    /// Providers that don't support this return provider-defined error.
    async fn usage(&self, api_key: &str) -> Result<UsageStatus, ProviderError>;

    /// The actual per-unit cost being charged RIGHT NOW.
    /// Returns (input_cost_nj, output_cost_nj) for token-based APIs.
    /// Returns per-call cost for fixed-price APIs.
    /// This is what the provider is charging, not the price card.
    async fn actual_cost(&self, api_key: &str) -> Result<CostRate, ProviderError>;
}

pub struct ProviderState {
    pub tier: String,              // "free", "pro", "enterprise"
    pub monthly_limit: Option<u64>, // units (tokens, calls, credits)
    pub limit_unit: LimitUnit,     // tokens, calls, credits, dollars
    pub overage_rate: Option<CostRate>, // cost when past the limit
    pub billing_period_start: DateTime<Utc>,
}

pub struct UsageStatus {
    pub consumed: u64,           // units consumed this period
    pub limit: u64,              // total limit for this period
    pub fraction: f64,           // consumed / limit
    pub estimated_exhaustion: Option<DateTime<Utc>>, // when we'll hit 100%
}

pub enum LimitUnit {
    Tokens,
    Calls,
    Credits,
    Dollars,
}

pub struct CostRate {
    pub input_nj_per_unit: u64,   // nano-rJ per input unit (0 if not token-based)
    pub output_nj_per_unit: u64,  // nano-rJ per output unit
    pub fixed_nj_per_call: u64,   // nano-rJ per call (for fixed-price APIs)
    pub is_marginal: bool,        // true if we're past the subscription limit
}
```

## 3. Adaptive Usage Monitoring

The daemon checks each provider on a schedule that accelerates as usage approaches the limit:

```
IF usage_pct < 50%   → check daily
IF 50% ≤ usage_pct < 70% → check every 6 hours  
IF 70% ≤ usage_pct < 90% → check hourly
IF usage_pct ≥ 90%        → check every 10 minutes
```

When the daemon detects the shift from pre-paid to marginal:
1. `CostRate.is_marginal` flips from `false` to `true`
2. A CNS span `cns.provider.marginal_activated` is emitted
3. All subsequent per-call costs use the marginal `CostRate`
4. The shift point is recorded in the cost ledger for later analysis

## 4. Per-Provider Implementations

### 4.1 DeepInfra
- **Pricing model:** Pure pay-as-you-go. No subscription tiers, no limits.
- **Cost discovery:** `GET /v1/usage` returns token counts. Cost = tokens × published rate.
- **is_marginal:** Always true (there is no free tier).
- **Complexity:** Low. Token counts from API response already captured.

### 4.2 OpenRouter
- **Pricing model:** Credit-based (prepaid). Users deposit credits.
- **Cost discovery:** `GET /api/v1/auth/key` returns `credits_remaining`.
- **is_marginal:** Always true (credits are always being consumed).
- **Acceleration trigger:** When `credits_remaining < 30%` of initial deposit.
- **Complexity:** Medium. Need to track credit balance.

### 4.3 Together AI
- **Pricing model:** Free credits + pay-as-you-go after exhaustion.
- **Cost discovery:** `GET /api/v1/usage` returns token counts. Free credits tracked in billing.
- **is_marginal:** False until free credits exhausted, then true.
- **Acceleration trigger:** When free credits < 30% remaining.
- **Complexity:** Medium. Need to know free credit allocation.

### 4.4 Brave Search
- **Pricing model:** Tiered — Free (2,000 calls/mo), Base, Pro, Custom.
- **Cost discovery:** No public usage API. Must trust hKask's own call counter.
- **is_marginal:** False until hKask counter exceeds tier limit, then true.
- **Acceleration trigger:** When hKask counter > 70% of tier limit.
- **Complexity:** Medium. Self-tracked usage with provider-verified tier.

### 4.5 Firecrawl
- **Pricing model:** Tiered — Free, Hobby, Standard, Growth. Credit-based.
- **Cost discovery:** `GET /v1/account` may return usage. Fallback: self-tracked.
- **Complexity:** Medium.

### 4.6 Tavily, Exa, FMP, EODHD
- **Pricing model:** Tiered subscriptions with monthly call limits.
- **Cost discovery:** No public usage API. Must trust hKask's own call counter.
- **is_marginal:** False until hKask counter exceeds tier limit, then true.
- **Complexity:** Medium. Self-tracked with periodic manual verification.

### 4.7 Runpod, Baseten
- **Pricing model:** Pay-as-you-go GPU time. No subscription limits.
- **Cost discovery:** Runpod has billing API. Baseten dashboard only.
- **is_marginal:** Always true.
- **Complexity:** Low for Runpod, medium for Baseten.

## 5. Provider Config

Each provider's settings live alongside the classifier config in `registry/providers/`:

```yaml
# registry/providers/deepinfra.yaml
provider:
  id: deepinfra
  name: DeepInfra
  pricing_model: pay_as_you_go
  usage_api: https://api.deepinfra.com/v1/usage
  cost_input_nj_per_token: 30
  cost_output_nj_per_token: 60
  subscription_tiers: []  # no tiers for pay-as-you-go

# registry/providers/brave.yaml
provider:
  id: brave
  name: Brave Search
  pricing_model: tiered
  usage_api: null  # no usage API — self-tracked
  subscription_tiers:
    - name: free
      monthly_call_limit: 2000
      overage_per_call_nj: 1000000  # $0.001/call in nJ
    - name: base
      monthly_call_limit: 20000
      overage_per_call_nj: 800000
    - name: pro
      monthly_call_limit: null  # unlimited
      overage_per_call_nj: 0
```

## 6. Integration with rJoule Cost Tracker

The QA CostTracker uses `ProviderIntelligence` to get actual costs per classify call:

```
QA Script classify step
  → classify_batch calls DeepInfra API
  → API response includes token usage
  → ProviderIntelligence.actual_cost(api_key) returns current CostRate
  → CostRate applied to token counts → ClassifyResult.cost_urj
  → CostTracker accumulates actual cost
```

If the provider has shifted to marginal pricing mid-run, the cost for subsequent calls uses the marginal rate. The ledger records two entries for the same run: pre-marginal calls at one rate, post-marginal at another.

## 7. Ledger Integration

When `hKask-ledger` exists, the provider intelligence writes:
```
Transaction {
    id: uuid,
    timestamp: now,
    reference: "provider:deepinfra:api-key-abc123:rate-change",
    postings: [
        Posting { source: "cost:qa-run-xyz", destination: "api:deepinfra", asset: "rJ", amount: 30 },
    ],
    metadata: { "shift": "pre-paid to marginal", "old_rate": {...}, "new_rate": {...} }
}
```

## 8. Open Questions

1. **Self-tracked providers:** For Brave, Tavily, Exa, etc. — should we implement a local call counter that persists between runs, or trust the QA script's per-run CostSummary and aggregate post-hoc?

2. **Manual tier input:** For providers with no usage API AND no way to query tier, should the user manually declare their current tier in settings? How often do they need to update it?

3. **Cost dispute/reconciliation:** When the provider's actual charge differs from our tracked cost (billing error, delayed usage reporting), how do we reconcile? A reconciliation entry in the ledger?

4. **Multi-key environments:** A single hKask instance may use multiple API keys for the same provider (development, production, different agents). Does provider intelligence track per-key or aggregate across keys?

5. **Rate limit interaction:** When a provider returns 429 (rate limited), the call costs input tokens but produces no output. Should this be tracked as a separate cost category (similar to our existing `failed_api_cost_urj`)?
