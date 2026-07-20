//! Smoke test: cancel a RunPod pod by pod_id.
//!
//! ```bash
//! cargo run --example smoke_cancel_runpod --release -p hkask-mcp-training -- <pod_id>
//! ```

use hkask_mcp_training::providers::{AxolotlHarness, RunpodHost, TrainingHost};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("hkask=info,warn")),
        )
        .init();

    let pod_id = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: smoke_cancel_runpod <pod_id>"))?;

    let api_key = std::env::var("RUNPOD_API_KEY")
        .map_err(|_| anyhow::anyhow!("RUNPOD_API_KEY not set in .env"))?;

    let host = RunpodHost::new(api_key, String::new(), Box::new(AxolotlHarness));

    host.cancel(&pod_id).await?;
    println!("✓ cancelled pod {pod_id}");

    Ok(())
}
