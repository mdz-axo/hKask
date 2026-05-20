//! hKask MCP Inference — Okapi-backed LLM inference
//!
//! Provides LLM inference via Okapi with CNS span integration.
//! Uses hexagonal architecture with explicit ports and adapters.

mod metrics_translator;

use hkask_cns::CnsRuntime;
use hkask_ensemble::adapters::OkapiSseAdapter;
use hkask_types::WebID;
use metrics_translator::MetricsTranslator;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Clone)]
pub struct InferenceMcpServer {
    okapi_base_url: String,
    cns_runtime: Arc<CnsRuntime>,
}

impl InferenceMcpServer {
    pub fn new(okapi_base_url: String, cns_runtime: Arc<CnsRuntime>) -> Self {
        Self {
            okapi_base_url,
            cns_runtime,
        }
    }

    pub async fn start_cns_translator(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (cns_tx, mut cns_rx) = mpsc::channel(100);
        let observer_webid = WebID::new();

        // Create metrics source adapter (hexagonal port implementation)
        let metrics_source = OkapiSseAdapter::new(&self.okapi_base_url);

        // Create translator with injected dependencies
        let mut translator = MetricsTranslator::new(metrics_source, cns_tx, observer_webid);

        let cns_runtime = Arc::clone(&self.cns_runtime);

        // Spawn CNS span consumer
        tokio::spawn(async move {
            while let Some(event) = cns_rx.recv().await {
                info!(target: "cns", "Received CNS event: {:?}", event.id);

                let domain = match &event.span {
                    hkask_types::Span::Connector(s) => {
                        if s.contains("llm") {
                            "llm"
                        } else {
                            "connector"
                        }
                    }
                    hkask_types::Span::Tool(_) => "tool",
                    _ => "general",
                };

                cns_runtime
                    .increment_variety(domain, &event.id.to_string())
                    .await;
            }
        });

        // Spawn metrics translator (runs until stream ends)
        tokio::spawn(async move {
            if let Err(e) = translator.subscribe_and_translate().await {
                error!("CNS translator error: {}", e);
            }
        });

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt::init();

    let okapi_base_url =
        std::env::var("OKAPI_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:11435".to_string());

    let cns_runtime = Arc::new(CnsRuntime::new());
    let server = InferenceMcpServer::new(okapi_base_url.clone(), Arc::clone(&cns_runtime));

    info!("Starting CNS span translator for Okapi metrics");
    server.start_cns_translator().await?;

    info!(
        "hKask MCP Inference server initialized for Okapi: {}",
        okapi_base_url
    );
    info!("CNS translator running - metrics will be emitted on delta");

    // Keep the runtime alive
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
