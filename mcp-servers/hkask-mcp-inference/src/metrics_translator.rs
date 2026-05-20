//! CNS Span Translator for Okapi metrics
//!
//! Subscribes to Okapi's SSE metrics stream and emits CNS spans on delta.

use hkask_types::{NuEvent, Span, WebID};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc;
use tracing::info;

/// Okapi metrics as received from SSE stream
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OkapiMetrics {
    pub tokens_generated_total: i64,
    pub kv_cache_tokens: i64,
    pub context_length: i64,
    pub adapter_swap_latency_ms: i64,
    pub gpu_memory_used_bytes: u64,
    pub prompt_cache_hit_ratio: Option<f64>,
}

/// CNS span translator for Okapi metrics
pub struct MetricsTranslator {
    sse_url: String,
    cns_tx: mpsc::Sender<NuEvent>,
    observer_webid: WebID,
    last_metrics: Option<OkapiMetrics>,
}

impl MetricsTranslator {
    pub fn new(okapi_base_url: &str, cns_tx: mpsc::Sender<NuEvent>, observer_webid: WebID) -> Self {
        Self {
            sse_url: format!("{}/api/metrics/stream?interval=5", okapi_base_url),
            cns_tx,
            observer_webid,
            last_metrics: None,
        }
    }

    /// Subscribe to SSE stream and translate metrics to CNS spans
    pub async fn subscribe_and_translate(&mut self) -> Result<(), MetricsError> {
        info!("Subscribing to Okapi SSE stream: {}", self.sse_url);

        let client = reqwest::Client::new();
        let response = client
            .get(&self.sse_url)
            .send()
            .await
            .map_err(|e| MetricsError::SseError(e.to_string()))?;

        let stream = response.text().await?;
        for line in stream.lines() {
            if line.starts_with("data: ") {
                let data = line.strip_prefix("data: ").unwrap_or("");
                if let Ok(metrics) = serde_json::from_str::<OkapiMetrics>(data) {
                    if let Some(last) = &self.last_metrics {
                        self.emit_delta_spans(&metrics, last).await?;
                    }
                    self.last_metrics = Some(metrics);
                }
            }
        }

        Ok(())
    }

    /// Emit CNS spans for changed metrics only
    async fn emit_delta_spans(&self, current: &OkapiMetrics, last: &OkapiMetrics) -> Result<(), MetricsError> {
        if current.tokens_generated_total != last.tokens_generated_total {
            self.emit_span(
                Span::Connector("cns.connector.llm.tokens".to_string()),
                json!({
                    "tokens_generated": current.tokens_generated_total,
                    "delta": current.tokens_generated_total - last.tokens_generated_total,
                }),
            ).await?;
        }

        if current.kv_cache_tokens != last.kv_cache_tokens {
            let utilization_pct = if current.context_length > 0 {
                (current.kv_cache_tokens as f64 / current.context_length as f64) * 100.0
            } else {
                0.0
            };

            self.emit_span(
                Span::Connector("cns.connector.llm.context".to_string()),
                json!({
                    "kv_cache_tokens": current.kv_cache_tokens,
                    "context_length": current.context_length,
                    "utilization_pct": utilization_pct,
                }),
            ).await?;
        }

        if current.adapter_swap_latency_ms > 0 && current.adapter_swap_latency_ms != last.adapter_swap_latency_ms {
            self.emit_span(
                Span::Tool("cns.tool.adapter_swap".to_string()),
                json!({
                    "latency_ms": current.adapter_swap_latency_ms,
                }),
            ).await?;
        }

        if current.gpu_memory_used_bytes != last.gpu_memory_used_bytes {
            self.emit_span(
                Span::Connector("cns.connector.llm.gpu_memory".to_string()),
                json!({
                    "used_bytes": current.gpu_memory_used_bytes,
                    "delta": (current.gpu_memory_used_bytes as i64 - last.gpu_memory_used_bytes as i64).abs(),
                }),
            ).await?;
        }

        if current.prompt_cache_hit_ratio != last.prompt_cache_hit_ratio {
            if let Some(ratio) = current.prompt_cache_hit_ratio {
                self.emit_span(
                    Span::Connector("cns.connector.llm.cache_hit".to_string()),
                    json!({
                        "hit_ratio": ratio,
                    }),
                ).await?;
            }
        }

        Ok(())
    }

    async fn emit_span(&self, span: Span, observation: serde_json::Value) -> Result<(), MetricsError> {
        let event = NuEvent::new(
            self.observer_webid,
            span,
            hkask_types::Phase::Observe,
            observation,
            0,
        );

        self.cns_tx.send(event).await.map_err(|e| {
            MetricsError::CnsEmissionError(e.to_string())
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    #[error("SSE stream error: {0}")]
    SseError(String),

    #[error("JSON parse error: {0}")]
    ParseError(String),

    #[error("CNS emission error: {0}")]
    CnsEmissionError(String),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_translator_new() {
        let (tx, _rx) = mpsc::channel(100);
        let webid = WebID::new();
        let translator = MetricsTranslator::new("http://localhost:11435", tx, webid);

        assert_eq!(translator.sse_url, "http://localhost:11435/api/metrics/stream?interval=5");
        assert!(translator.last_metrics.is_none());
    }

    #[tokio::test]
    async fn test_delta_only_emission() {
        let (tx, mut rx) = mpsc::channel(100);
        let webid = WebID::new();

        let mut translator = MetricsTranslator::new("http://localhost:11435", tx, webid);

        let first_metrics = OkapiMetrics {
            tokens_generated_total: 1000,
            kv_cache_tokens: 500,
            context_length: 8192,
            adapter_swap_latency_ms: 0,
            gpu_memory_used_bytes: 4294967296,
            prompt_cache_hit_ratio: Some(0.75),
        };
        translator.last_metrics = Some(first_metrics.clone());

        translator.emit_delta_spans(&first_metrics, &first_metrics).await.unwrap();
        assert!(rx.try_recv().is_err());

        let changed_metrics = OkapiMetrics {
            tokens_generated_total: 1050,
            kv_cache_tokens: 500,
            context_length: 8192,
            adapter_swap_latency_ms: 0,
            gpu_memory_used_bytes: 4294967296,
            prompt_cache_hit_ratio: Some(0.75),
        };
        translator.emit_delta_spans(&changed_metrics, &first_metrics).await.unwrap();

        let event = rx.recv().await.unwrap();
        assert!(event.span.as_str().contains("tokens"));
        assert!(rx.try_recv().is_err());
    }
}
