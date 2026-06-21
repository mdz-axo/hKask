//! Adaptive Provider Monitor — background daemon for provider cost surveillance.
//!
//! Monitors configured providers at dynamically-adjusted intervals:
//!   usage < 50%  → daily
//!   50-70%       → every 6 hours
//!   70-90%       → hourly
//!   usage ≥ 90%  → every 10 minutes
//!
//! Emits CNS spans when a provider crosses from pre-paid/subscription
//! into marginal/overage pricing (`cns.provider.marginal_activated`).

use hkask_services_classify::{ProviderIntelligence, UsageStatus};
use std::time::Duration;
use tokio::time::Instant;

/// A single provider under surveillance.
struct WatchedProvider {
    provider: Box<dyn ProviderIntelligence>,
    api_key: String,
    /// Last known marginal state — used to detect transitions.
    was_marginal: bool,
    /// When to next check this provider.
    next_check: Instant,
    /// Current check interval (adjusted by usage fraction).
    interval: Duration,
}

impl WatchedProvider {
    fn new(provider: Box<dyn ProviderIntelligence>, api_key: String) -> Self {
        Self {
            provider,
            api_key,
            was_marginal: false,
            next_check: Instant::now(), // check immediately on first run
            interval: Duration::from_secs(24 * 3600), // start at daily
        }
    }

    /// Determine check interval from usage fraction.
    fn interval_for_fraction(fraction: f64) -> Duration {
        if fraction >= 0.90 {
            Duration::from_secs(10 * 60)
        } else if fraction >= 0.70 {
            Duration::from_secs(3600)
        } else if fraction >= 0.50 {
            Duration::from_secs(6 * 3600)
        } else {
            Duration::from_secs(24 * 3600)
        }
    }

    /// Run one check cycle for this provider.
    async fn check(&mut self) {
        let provider_id = self.provider.provider_id();

        // Query usage
        let usage = match self.provider.usage(&self.api_key).await {
            Ok(u) => u,
            Err(e) => {
                tracing::warn!(
                    target: "cns.provider",
                    provider = %provider_id,
                    error = %e,
                    "Failed to query provider usage"
                );
                return;
            }
        };

        // Query actual cost (use empty model name for base/provider-default rate)
        let cost = match self.provider.actual_cost(&self.api_key, "").await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    target: "cns.provider",
                    provider = %provider_id,
                    error = %e,
                    "Failed to query actual cost"
                );
                return;
            }
        };

        // Detect marginal activation (false → true transition)
        if cost.is_marginal && !self.was_marginal {
            tracing::warn!(
                target: "cns.provider.marginal_activated",
                provider = %provider_id,
                consumed = usage.consumed,
                limit = usage.limit,
                fraction = %format!("{:.1}%", usage.fraction * 100.0),
                "Provider crossed into marginal pricing — overage rates now apply"
            );
        }
        self.was_marginal = cost.is_marginal;

        // Adjust check interval based on usage fraction
        let new_interval = Self::interval_for_fraction(usage.fraction);
        if new_interval != self.interval {
            tracing::info!(
                target: "cns.provider",
                provider = %provider_id,
                old_interval_secs = self.interval.as_secs(),
                new_interval_secs = new_interval.as_secs(),
                fraction = %format!("{:.1}%", usage.fraction * 100.0),
                "Adjusted monitoring interval"
            );
            self.interval = new_interval;
        }

        self.next_check = Instant::now() + self.interval;

        tracing::debug!(
            target: "cns.provider",
            provider = %provider_id,
            consumed = usage.consumed,
            limit = usage.limit,
            fraction = %format!("{:.1}%", usage.fraction * 100.0),
            is_marginal = cost.is_marginal,
            next_check_secs = self.interval.as_secs(),
            "Provider check complete"
        );
    }
}

/// Adaptive monitoring daemon — watches multiple providers,
/// accelerating check frequency as usage approaches limits.
pub struct AdaptiveMonitor {
    providers: Vec<WatchedProvider>,
}

impl AdaptiveMonitor {
    /// Create a new adaptive monitor with no providers registered.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Register a provider for monitoring.
    ///
    /// `api_key` is the provider's API key for authenticated usage queries.
    pub fn add_provider(&mut self, provider: Box<dyn ProviderIntelligence>, api_key: String) {
        tracing::info!(
            target: "cns.provider",
            provider = %provider.provider_id(),
            "Provider registered for adaptive monitoring"
        );
        self.providers.push(WatchedProvider::new(provider, api_key));
    }

    /// Run the monitor daemon. Blocks indefinitely, checking each provider
    /// at its adaptive interval. Returns only on fatal error or if all
    /// providers are removed.
    pub async fn run(&mut self) {
        if self.providers.is_empty() {
            tracing::warn!(
                target: "cns.provider",
                "Adaptive monitor started with no providers — idle"
            );
            // Park forever — caller can add providers externally
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        }

        loop {
            // Find the provider with the earliest next_check
            let now = Instant::now();
            let mut next_deadline = now + Duration::from_secs(3600); // default: 1 hour

            for p in &mut self.providers {
                if p.next_check <= now {
                    p.check().await;
                }
                if p.next_check < next_deadline {
                    next_deadline = p.next_check;
                }
            }

            // Sleep until the next provider needs checking
            let sleep_dur = next_deadline.saturating_duration_since(Instant::now());
            if sleep_dur > Duration::ZERO {
                tokio::time::sleep(sleep_dur).await;
            }
        }
    }
}

impl Default for AdaptiveMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mock provider for testing interval adjustment.
    struct MockProvider {
        id: &'static str,
        usage_status: UsageStatus,
        cost_rate: hkask_services_classify::CostRate,
        usage_ok: bool,
    }

    #[async_trait::async_trait]
    impl ProviderIntelligence for MockProvider {
        fn provider_id(&self) -> &'static str {
            self.id
        }
        async fn discover(
            &self,
            _api_key: &str,
        ) -> Result<hkask_services_classify::ProviderState, hkask_services_classify::ProviderError>
        {
            unimplemented!()
        }
        async fn usage(
            &self,
            _api_key: &str,
        ) -> Result<UsageStatus, hkask_services_classify::ProviderError> {
            if self.usage_ok {
                Ok(self.usage_status.clone())
            } else {
                Err(hkask_services_classify::ProviderError::Http(
                    "mock error".into(),
                ))
            }
        }
        async fn actual_cost(
            &self,
            _api_key: &str,
            _model_name: &str,
        ) -> Result<hkask_services_classify::CostRate, hkask_services_classify::ProviderError>
        {
            Ok(self.cost_rate.clone())
        }
    }

    fn mock_provider(id: &'static str, fraction: f64, is_marginal: bool) -> MockProvider {
        MockProvider {
            id,
            usage_status: UsageStatus {
                consumed: (fraction * 1000.0) as u64,
                limit: 1000,
                fraction,
                estimated_exhaustion: None,
            },
            cost_rate: hkask_services_classify::CostRate {
                input_nj_per_unit: 30,
                output_nj_per_unit: 60,
                cache_read_nj_per_unit: 0,
                cache_write_nj_per_unit: 0,
                fixed_nj_per_call: 0,
                image_nj_per_unit: 0,
                is_marginal,
            },
            usage_ok: true,
        }
    }

    #[test]
    fn interval_below_50_percent_is_daily() {
        assert_eq!(
            WatchedProvider::interval_for_fraction(0.30).as_secs(),
            24 * 3600
        );
    }

    #[test]
    fn interval_50_to_70_percent_is_6_hours() {
        assert_eq!(
            WatchedProvider::interval_for_fraction(0.60).as_secs(),
            6 * 3600
        );
    }

    #[test]
    fn interval_70_to_90_percent_is_hourly() {
        assert_eq!(WatchedProvider::interval_for_fraction(0.80).as_secs(), 3600);
    }

    #[test]
    fn interval_above_90_percent_is_10_minutes() {
        assert_eq!(
            WatchedProvider::interval_for_fraction(0.95).as_secs(),
            10 * 60
        );
    }

    #[test]
    fn interval_boundary_exact_50_is_6_hours() {
        assert_eq!(
            WatchedProvider::interval_for_fraction(0.50).as_secs(),
            6 * 3600
        );
    }

    #[test]
    fn interval_boundary_exact_70_is_hourly() {
        assert_eq!(WatchedProvider::interval_for_fraction(0.70).as_secs(), 3600);
    }

    #[test]
    fn interval_boundary_exact_90_is_10_minutes() {
        assert_eq!(
            WatchedProvider::interval_for_fraction(0.90).as_secs(),
            10 * 60
        );
    }

    #[tokio::test]
    async fn monitor_can_be_created_empty() {
        let mut monitor = AdaptiveMonitor::new();
        // Add a mock provider and run one check
        let mock = mock_provider("test-provider", 0.30, false);
        monitor.add_provider(Box::new(mock), "test-key".into());

        // Run checks synchronously by iterating
        for p in &mut monitor.providers {
            p.check().await;
        }

        // After check at 30%, interval should be daily
        assert_eq!(monitor.providers[0].interval.as_secs(), 24 * 3600);
    }

    #[tokio::test]
    async fn marginal_activation_is_detected() {
        // Create a provider that reports high usage and just became marginal
        let mock = MockProvider {
            id: "test-marginal",
            usage_status: UsageStatus {
                consumed: 950,
                limit: 1000,
                fraction: 0.95,
                estimated_exhaustion: None,
            },
            cost_rate: hkask_services_classify::CostRate {
                input_nj_per_unit: 30,
                output_nj_per_unit: 60,
                cache_read_nj_per_unit: 0,
                cache_write_nj_per_unit: 0,
                fixed_nj_per_call: 0,
                image_nj_per_unit: 0,
                is_marginal: true, // just became marginal
            },
            usage_ok: true,
        };

        let mut monitor = AdaptiveMonitor::new();
        monitor.add_provider(Box::new(mock), "test-key".into());

        // was_marginal starts false, check should detect transition
        monitor.providers[0].check().await;

        // After detection, was_marginal should be true
        assert!(monitor.providers[0].was_marginal);

        // Interval should be 10 minutes (≥90%)
        assert_eq!(monitor.providers[0].interval.as_secs(), 10 * 60);
    }
}
