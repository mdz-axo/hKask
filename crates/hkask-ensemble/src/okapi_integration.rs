//! Okapi Integration Module
//!
//! Unified interface for Okapi integration with hKask infrastructure.
//! Combines ports, adapters, capability security, and CNS integration.

use hkask_cns::CnsRuntime;
use hkask_types::{NuEvent, Span, WebID};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info, instrument};

use crate::adapters::OkapiSseAdapter;
use crate::capability::OkapiCapability;
use crate::ports::OkapiMetrics;

/// Okapi integration runtime
pub struct OkapiIntegration {
    base_url: String,
    capability: OkapiCapability,
    #[allow(dead_code)]
    cns_runtime: Arc<CnsRuntime>,
}

impl OkapiIntegration {
    /// Create new Okapi integration with default system capability
    pub fn new(base_url: String, cns_runtime: Arc<CnsRuntime>) -> Self {
        let holder = WebID::new();
        let key = [0x42; 32]; // TODO: Load from secure keystore
        let capability = crate::capability::default_system_capability(holder, &key);

        Self {
            base_url,
            capability,
            cns_runtime,
        }
    }

    /// Create with custom capability
    pub fn with_capability(
        base_url: String,
        capability: OkapiCapability,
        cns_runtime: Arc<CnsRuntime>,
    ) -> Self {
        Self {
            base_url,
            capability,
            cns_runtime,
        }
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the capability
    pub fn capability(&self) -> &OkapiCapability {
        &self.capability
    }

    /// Verify OCAP for generate operation
    pub async fn verify_generate_ocap(
        &self,
        requester: WebID,
    ) -> Result<(), OkapiIntegrationError> {
        // Check if requester has generate capability
        let key = [0x42; 32]; // TODO: Load from secure keystore
        if let Err(e) = self
            .capability
            .verify(&key, &[crate::OkapiOperation::Generate])
        {
            return Err(OkapiIntegrationError::CapabilityError(format!(
                "Capability verification failed: {:?}",
                e
            )));
        }

        // Check if capability holder matches requester
        if self.capability.holder != requester {
            return Err(OkapiIntegrationError::CapabilityError(
                "Capability holder does not match requester".to_string(),
            ));
        }

        Ok(())
    }

    /// Verify OCAP for chat operation
    pub async fn verify_chat_ocap(&self, requester: WebID) -> Result<(), OkapiIntegrationError> {
        let key = [0x42; 32];
        if let Err(e) = self.capability.verify(&key, &[crate::OkapiOperation::Chat]) {
            return Err(OkapiIntegrationError::CapabilityError(format!(
                "Capability verification failed: {:?}",
                e
            )));
        }

        if self.capability.holder != requester {
            return Err(OkapiIntegrationError::CapabilityError(
                "Capability holder does not match requester".to_string(),
            ));
        }

        Ok(())
    }

    /// Start metrics translation to CNS
    #[instrument(skip(self), fields(okapi_url = %self.base_url))]
    pub async fn start_metrics_translation(&self) -> Result<(), OkapiIntegrationError> {
        info!("Starting Okapi metrics translation to CNS");

        let (cns_tx, mut cns_rx): (mpsc::Sender<NuEvent>, mpsc::Receiver<NuEvent>) =
            mpsc::channel(100);
        let observer_webid = WebID::new();

        let metrics_source = OkapiSseAdapter::new(&self.base_url);

        // Spawn CNS span consumer
        let cns_runtime = Arc::clone(&self.cns_runtime);
        tokio::spawn(async move {
            while let Some(event) = cns_rx.recv().await {
                let domain = match &event.span {
                    Span::Connector(s) => {
                        if s.contains("llm") {
                            "llm"
                        } else {
                            "connector"
                        }
                    }
                    Span::Tool(_) => "tool",
                    _ => "general",
                };

                cns_runtime
                    .increment_variety(domain, &event.id.to_string())
                    .await;

                info!(
                    target: "cns.okapi",
                    event_id = %event.id,
                    domain = %domain,
                    "CNS event processed"
                );
            }
        });

        // Spawn metrics translator
        let mut translator = MetricsTranslator::new(metrics_source, cns_tx, observer_webid);

        tokio::spawn(async move {
            if let Err(e) = translator.subscribe_and_translate().await {
                error!("Metrics translator error: {}", e);
            }
        });

        info!("Okapi metrics translation started successfully");
        Ok(())
    }

    /// Emit CNS span for capability validation
    #[instrument(skip(self), fields(template_id = %template_id))]
    pub fn emit_capability_validation_span(
        &self,
        template_id: &str,
        success: bool,
        errors: Vec<String>,
    ) {
        info!(
            target: "cns.okapi.validation",
            template_id = %template_id,
            success = %success,
            errors = ?errors,
            "Capability validation span emitted"
        );
    }
}

/// Metrics translator for Okapi integration
pub struct MetricsTranslator<M> {
    metrics_source: M,
    cns_tx: mpsc::Sender<NuEvent>,
    observer_webid: WebID,
    last_metrics: Option<OkapiMetrics>,
}

impl<M> MetricsTranslator<M>
where
    M: crate::ports::MetricsSource<Metrics = OkapiMetrics>,
{
    pub fn new(metrics_source: M, cns_tx: mpsc::Sender<NuEvent>, observer_webid: WebID) -> Self {
        Self {
            metrics_source,
            cns_tx,
            observer_webid,
            last_metrics: None,
        }
    }

    /// Subscribe to metrics stream and translate to CNS spans
    pub async fn subscribe_and_translate(
        &mut self,
    ) -> Result<(), MetricsTranslatorError<M::Error>> {
        info!("Starting CNS span translator for Okapi metrics");

        loop {
            let metrics = self
                .metrics_source
                .next_metrics()
                .await
                .map_err(MetricsTranslatorError::MetricsSource)?;

            if let Some(last) = &self.last_metrics {
                self.emit_delta_spans(&metrics, last).await?;
            }

            self.last_metrics = Some(metrics);
        }
    }

    /// Emit CNS spans for changed metrics only
    async fn emit_delta_spans(
        &self,
        current: &OkapiMetrics,
        last: &OkapiMetrics,
    ) -> Result<(), MetricsTranslatorError<M::Error>> {
        if current.tokens_generated_total != last.tokens_generated_total {
            self.emit_span(
                Span::Connector("cns.connector.llm.tokens".to_string()),
                serde_json::json!({
                    "tokens_generated": current.tokens_generated_total,
                    "delta": current.tokens_generated_total - last.tokens_generated_total,
                }),
            )
            .await?;
        }

        if current.kv_cache_tokens != last.kv_cache_tokens {
            let utilization_pct = if current.context_length > 0 {
                (current.kv_cache_tokens as f64 / current.context_length as f64) * 100.0
            } else {
                0.0
            };

            self.emit_span(
                Span::Connector("cns.connector.llm.context".to_string()),
                serde_json::json!({
                    "kv_cache_tokens": current.kv_cache_tokens,
                    "context_length": current.context_length,
                    "utilization_pct": utilization_pct,
                }),
            )
            .await?;
        }

        if current.adapter_swap_latency_ms > 0
            && current.adapter_swap_latency_ms != last.adapter_swap_latency_ms
        {
            self.emit_span(
                Span::Tool("cns.tool.adapter_swap".to_string()),
                serde_json::json!({
                    "latency_ms": current.adapter_swap_latency_ms,
                }),
            )
            .await?;
        }

        if current.gpu_memory_used_bytes != last.gpu_memory_used_bytes {
            self.emit_span(
                Span::Connector("cns.connector.llm.gpu_memory".to_string()),
                serde_json::json!({
                    "used_bytes": current.gpu_memory_used_bytes,
                    "delta": (current.gpu_memory_used_bytes as i64
                        - last.gpu_memory_used_bytes as i64)
                    .abs(),
                }),
            )
            .await?;
        }

        if current.prompt_cache_hit_ratio != last.prompt_cache_hit_ratio
            && let Some(ratio) = current.prompt_cache_hit_ratio
        {
            self.emit_span(
                Span::Connector("cns.connector.llm.cache_hit".to_string()),
                serde_json::json!({
                    "hit_ratio": ratio,
                }),
            )
            .await?;
        }

        Ok(())
    }

    async fn emit_span(
        &self,
        span: Span,
        observation: serde_json::Value,
    ) -> Result<(), MetricsTranslatorError<M::Error>> {
        let event = NuEvent::new(
            self.observer_webid,
            span,
            hkask_types::Phase::Observe,
            observation,
            0,
        );

        self.cns_tx
            .send(event)
            .await
            .map_err(|e| MetricsTranslatorError::CnsEmissionError(e.to_string()))
    }
}

/// Metrics translator error
#[derive(Debug, thiserror::Error)]
pub enum MetricsTranslatorError<E: std::error::Error + Send + Sync> {
    #[error("Metrics source error: {0}")]
    MetricsSource(E),

    #[error("CNS emission error: {0}")]
    CnsEmissionError(String),
}

/// Okapi integration error
#[derive(Debug, thiserror::Error)]
pub enum OkapiIntegrationError {
    #[error("Metrics translation error: {0}")]
    MetricsError(String),

    #[error("CNS integration error: {0}")]
    CnsError(String),

    #[error("Capability error: {0}")]
    CapabilityError(String),
}

impl<E> From<MetricsTranslatorError<E>> for OkapiIntegrationError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: MetricsTranslatorError<E>) -> Self {
        OkapiIntegrationError::MetricsError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_cns::CnsRuntime;

    #[test]
    fn test_okapi_integration_new() {
        let cns_runtime = Arc::new(CnsRuntime::new());
        let integration = OkapiIntegration::new("http://localhost:11435".to_string(), cns_runtime);

        assert_eq!(integration.base_url(), "http://localhost:11435");
        assert!(!integration.capability().is_expired());
    }

    #[test]
    fn test_okapi_integration_with_capability() {
        let cns_runtime = Arc::new(CnsRuntime::new());
        let holder = WebID::new();
        let key = [0x42; 32];
        let capability = crate::capability::default_system_capability(holder, &key);

        let integration = OkapiIntegration::with_capability(
            "http://localhost:11435".to_string(),
            capability.clone(),
            cns_runtime,
        );

        assert_eq!(integration.capability().id, capability.id);
    }
}
