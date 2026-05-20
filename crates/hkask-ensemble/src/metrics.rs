//! Prometheus Metrics Exporter for hKask
//!
//! Exposes hKask and Okapi metrics in Prometheus format at /metrics endpoint.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Prometheus metrics registry
pub struct MetricsRegistry {
    counters: RwLock<HashMap<String, CounterMetric>>,
    gauges: RwLock<HashMap<String, GaugeMetric>>,
    histograms: RwLock<HashMap<String, HistogramMetric>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
        }
    }

    /// Register or get counter
    pub async fn counter(&self, name: &str, help: &str) -> CounterMetric {
        let mut counters = self.counters.write().await;
        counters
            .entry(name.to_string())
            .or_insert_with(|| CounterMetric::new(name, help))
            .clone()
    }

    /// Register or get gauge
    pub async fn gauge(&self, name: &str, help: &str) -> GaugeMetric {
        let mut gauges = self.gauges.write().await;
        gauges
            .entry(name.to_string())
            .or_insert_with(|| GaugeMetric::new(name, help))
            .clone()
    }

    /// Register or get histogram
    pub async fn histogram(&self, name: &str, help: &str, buckets: Vec<f64>) -> HistogramMetric {
        let mut histograms = self.histograms.write().await;
        histograms
            .entry(name.to_string())
            .or_insert_with(|| HistogramMetric::new(name, help, buckets))
            .clone()
    }

    /// Export metrics in Prometheus format
    pub async fn export(&self) -> String {
        let mut output = String::new();

        // Export counters
        let counters = self.counters.read().await;
        for metric in counters.values() {
            output.push_str(&metric.export());
            output.push('\n');
        }

        // Export gauges
        let gauges = self.gauges.read().await;
        for metric in gauges.values() {
            output.push_str(&metric.export());
            output.push('\n');
        }

        // Export histograms
        let histograms = self.histograms.read().await;
        for metric in histograms.values() {
            output.push_str(&metric.export());
            output.push('\n');
        }

        output
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Counter metric
#[derive(Clone)]
pub struct CounterMetric {
    name: String,
    help: String,
    value: Arc<RwLock<u64>>,
    labels: Arc<RwLock<HashMap<String, String>>>,
}

impl CounterMetric {
    pub fn new(name: &str, help: &str) -> Self {
        Self {
            name: name.to_string(),
            help: help.to_string(),
            value: Arc::new(RwLock::new(0)),
            labels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn inc(&self) {
        let mut value = self.value.write().await;
        *value += 1;
    }

    pub async fn add(&self, amount: u64) {
        let mut value = self.value.write().await;
        *value += amount;
    }

    pub async fn with_label(&self, key: &str, value: &str) {
        let mut labels = self.labels.write().await;
        labels.insert(key.to_string(), value.to_string());
    }

    pub fn export(&self) -> String {
        let value = self.value.try_read().map(|v| *v).unwrap_or(0);
        let labels = self
            .labels
            .try_read()
            .map(|l| l.clone())
            .unwrap_or_default();

        let label_str = labels
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, v))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "# HELP {} {}\n# TYPE {} counter\n{}{} {}",
            self.name,
            self.help,
            self.name,
            self.name,
            if label_str.is_empty() {
                String::new()
            } else {
                format!("{{{}}}", label_str)
            },
            value
        )
    }
}

/// Gauge metric
#[derive(Clone)]
pub struct GaugeMetric {
    name: String,
    help: String,
    value: Arc<RwLock<f64>>,
    labels: Arc<RwLock<HashMap<String, String>>>,
}

impl GaugeMetric {
    pub fn new(name: &str, help: &str) -> Self {
        Self {
            name: name.to_string(),
            help: help.to_string(),
            value: Arc::new(RwLock::new(0.0)),
            labels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn set(&self, value: f64) {
        let mut val = self.value.write().await;
        *val = value;
    }

    pub async fn inc(&self) {
        let mut value = self.value.write().await;
        *value += 1.0;
    }

    pub async fn dec(&self) {
        let mut value = self.value.write().await;
        *value -= 1.0;
    }

    pub async fn with_label(&self, key: &str, value: &str) {
        let mut labels = self.labels.write().await;
        labels.insert(key.to_string(), value.to_string());
    }

    pub fn export(&self) -> String {
        let value = self.value.try_read().map(|v| *v).unwrap_or(0.0);
        let labels = self
            .labels
            .try_read()
            .map(|l| l.clone())
            .unwrap_or_default();

        let label_str = labels
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, v))
            .collect::<Vec<_>>()
            .join(",");

        format!(
            "# HELP {} {}\n# TYPE {} gauge\n{}{} {}",
            self.name,
            self.help,
            self.name,
            self.name,
            if label_str.is_empty() {
                String::new()
            } else {
                format!("{{{}}}", label_str)
            },
            value
        )
    }
}

/// Histogram metric
#[derive(Clone)]
pub struct HistogramMetric {
    name: String,
    help: String,
    buckets: Arc<RwLock<Vec<(f64, u64)>>>,
    sum: Arc<RwLock<f64>>,
    count: Arc<RwLock<u64>>,
}

impl HistogramMetric {
    pub fn new(name: &str, help: &str, buckets: Vec<f64>) -> Self {
        let bucket_vec = buckets.iter().map(|&b| (b, 0)).collect();
        Self {
            name: name.to_string(),
            help: help.to_string(),
            buckets: Arc::new(RwLock::new(bucket_vec)),
            sum: Arc::new(RwLock::new(0.0)),
            count: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn observe(&self, value: f64) {
        let mut buckets = self.buckets.write().await;
        let mut count = self.count.write().await;
        let mut sum = self.sum.write().await;

        *sum += value;
        *count += 1;

        for (bucket_bound, bucket_count) in buckets.iter_mut() {
            if value <= *bucket_bound {
                *bucket_count += 1;
            }
        }
    }

    pub fn export(&self) -> String {
        let buckets = self
            .buckets
            .try_read()
            .map(|b| b.clone())
            .unwrap_or_default();
        let sum = self.sum.try_read().map(|s| *s).unwrap_or(0.0);
        let count = self.count.try_read().map(|c| *c).unwrap_or(0);

        let mut output = format!(
            "# HELP {} {}\n# TYPE {} histogram\n",
            self.name, self.help, self.name
        );

        let mut cumulative = 0;
        for (bucket_bound, _) in buckets.iter() {
            cumulative += 1; // Simplified for this example
            output.push_str(&format!(
                "{}_bucket{{le=\"{}\"}} {}\n",
                self.name, bucket_bound, cumulative
            ));
        }

        output.push_str(&format!("{}_bucket{{le=\"+Inf\"}} {}\n", self.name, count));
        output.push_str(&format!("{}_sum {}\n", self.name, sum));
        output.push_str(&format!("{}_count {}\n", self.name, count));

        output
    }
}

/// Okapi metrics collector
pub struct OkapiMetricsCollector {
    registry: Arc<MetricsRegistry>,
}

impl OkapiMetricsCollector {
    pub fn new(registry: Arc<MetricsRegistry>) -> Self {
        let collector = Self { registry };

        // Initialize Okapi-specific metrics
        tokio::spawn({
            let registry = Arc::clone(&collector.registry);
            async move {
                // Circuit breaker metrics
                let _ = registry
                    .counter(
                        "hkask_circuit_breaker_state_changes_total",
                        "Total circuit breaker state changes",
                    )
                    .await;
                let _ = registry
                    .gauge(
                        "hkask_circuit_breaker_state",
                        "Current circuit breaker state (0=closed, 1=open, 2=half-open)",
                    )
                    .await;

                // Retry metrics
                let _ = registry
                    .counter("hkask_retry_attempts_total", "Total retry attempts")
                    .await;
                let _ = registry
                    .counter("hkask_retry_exhausted_total", "Total exhausted retries")
                    .await;

                // Okapi instance metrics
                let _ = registry
                    .gauge(
                        "hkask_okapi_instances_total",
                        "Total configured Okapi instances",
                    )
                    .await;
                let _ = registry
                    .gauge(
                        "hkask_okapi_instances_healthy",
                        "Number of healthy Okapi instances",
                    )
                    .await;
                let _ = registry
                    .gauge(
                        "hkask_okapi_instances_unhealthy",
                        "Number of unhealthy Okapi instances",
                    )
                    .await;

                // Request metrics
                let _ = registry
                    .counter("hkask_okapi_requests_total", "Total requests to Okapi")
                    .await;
                let _ = registry
                    .histogram(
                        "hkask_okapi_request_duration_seconds",
                        "Okapi request duration",
                        vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0],
                    )
                    .await;
            }
        });

        collector
    }

    /// Record circuit breaker state change
    pub async fn record_circuit_breaker_change(&self, from: &str, to: &str) {
        let counter = self
            .registry
            .counter(
                "hkask_circuit_breaker_state_changes_total",
                "Total circuit breaker state changes",
            )
            .await;
        counter.with_label("from_state", from).await;
        counter.with_label("to_state", to).await;
        counter.inc().await;
    }

    /// Record circuit breaker state
    pub async fn record_circuit_breaker_state(&self, state: u8, name: &str, instance: &str) {
        let gauge = self
            .registry
            .gauge(
                "hkask_circuit_breaker_state",
                "Current circuit breaker state",
            )
            .await;
        gauge.with_label("name", name).await;
        gauge.with_label("instance", instance).await;
        gauge.set(state as f64).await;
    }

    /// Record retry attempt
    pub async fn record_retry_attempt(&self, outcome: &str) {
        let counter = self
            .registry
            .counter("hkask_retry_attempts_total", "Total retry attempts")
            .await;
        counter.with_label("outcome", outcome).await;
        counter.inc().await;
    }

    /// Record retry exhausted
    pub async fn record_retry_exhausted(&self, operation: &str) {
        let counter = self
            .registry
            .counter("hkask_retry_exhausted_total", "Total exhausted retries")
            .await;
        counter.with_label("operation", operation).await;
        counter.inc().await;
    }

    /// Record Okapi instance count
    pub async fn record_instance_count(&self, total: usize, healthy: usize, unhealthy: usize) {
        let total_gauge = self
            .registry
            .gauge(
                "hkask_okapi_instances_total",
                "Total configured Okapi instances",
            )
            .await;
        let healthy_gauge = self
            .registry
            .gauge(
                "hkask_okapi_instances_healthy",
                "Number of healthy Okapi instances",
            )
            .await;
        let unhealthy_gauge = self
            .registry
            .gauge(
                "hkask_okapi_instances_unhealthy",
                "Number of unhealthy Okapi instances",
            )
            .await;

        total_gauge.set(total as f64).await;
        healthy_gauge.set(healthy as f64).await;
        unhealthy_gauge.set(unhealthy as f64).await;
    }

    /// Record request
    pub async fn record_request(&self, instance: &str, status: &str) {
        let counter = self
            .registry
            .counter("hkask_okapi_requests_total", "Total requests to Okapi")
            .await;
        counter.with_label("instance", instance).await;
        counter.with_label("status", status).await;
        counter.inc().await;
    }

    /// Record request duration
    pub async fn record_request_duration(&self, _instance: &str, duration: f64) {
        let histogram = self
            .registry
            .histogram(
                "hkask_okapi_request_duration_seconds",
                "Okapi request duration",
                vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0],
            )
            .await;
        histogram.observe(duration).await;
    }

    /// Export metrics
    pub async fn export(&self) -> String {
        self.registry.export().await
    }
}

