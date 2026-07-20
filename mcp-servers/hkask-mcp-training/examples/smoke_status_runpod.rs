//! Smoke test: query the status of a RunPod pod by pod_id.
//!
//! ```bash
//! cargo run --example smoke_status_runpod --release -p hkask-mcp-training -- <pod_id>
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

    let arg = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: smoke_status_runpod <job_id_or_pod_id>"))?;

    let api_key = std::env::var("RUNPOD_API_KEY")
        .map_err(|_| anyhow::anyhow!("RUNPOD_API_KEY not set in .env"))?;

    let host = RunpodHost::new(api_key, String::new(), Box::new(AxolotlHarness));

    // The arg can be either a job_id (looked up in the persisted pods file) or
    // a pod_id (used directly). Try job_id first; if not found, fall back to
    // treating the arg as a pod_id by injecting it into the jobs map.
    let job_id = {
        let map = host.jobs_for_lookup();
        if map.contains_key(&arg) {
            arg.clone()
        } else {
            // Fall back: assume arg is a pod_id. Inject a synthetic mapping.
            drop(map);
            host.inject_pod_id(&arg);
            arg.clone()
        }
    };

    let status = host.status(&job_id).await?;
    println!("pod {job_id} status: {status:?}");

    Ok(())
}
