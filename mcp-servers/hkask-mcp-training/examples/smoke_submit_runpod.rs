//! Smoke test: submit a real LoRA training job to RunPod using the
//! `docker.io/mdzaxo/axolotl-lora-trainer:latest` image.
//!
//! This is an integration test, not a unit test — it bills real GPU time.
//! Run it manually:
//!
//! ```bash
//! cargo run --example smoke_submit_runpod --release -p hkask-mcp-training
//! ```
//!
//! Requirements:
//!   - .env with RUNPOD_API_KEY and HF_TOKEN
//!   - /tmp/smoke_test_dataset.jsonl (50-line ChatML slice — created automatically)
//!   - The Docker image must be pushed to Docker Hub (already done).
//!
//! The job uses Qwen/Qwen3-1.7B (small, cheap) and 1 epoch on 50 examples.
//! Expected cost: < $0.50. Expected runtime: ~5-10 minutes on an H100.

use std::path::PathBuf;

use hkask_mcp_training::huggingface::TrainingArtifacts;
use hkask_mcp_training::providers::{
    self, AxolotlHarness, HarnessAdapter, TrainingHarnessId, TrainingHostConfig, TrainingHostId,
    TrainingJob, TrainingParams,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("hkask=info,cns=info,warn")),
        )
        .init();

    let api_key = std::env::var("RUNPOD_API_KEY")
        .map_err(|_| anyhow::anyhow!("RUNPOD_API_KEY not set in .env"))?;
    let hf_token =
        std::env::var("HF_TOKEN").map_err(|_| anyhow::anyhow!("HF_TOKEN not set in .env"))?;

    // 50-line ChatML slice for the smoke test.
    let dataset_path = PathBuf::from("/tmp/smoke_test_dataset.jsonl");
    if !dataset_path.exists() {
        let src = PathBuf::from("corpus/qa_pairs/train_chat.jsonl");
        let content = std::fs::read_to_string(&src)?;
        let lines: Vec<&str> = content.lines().take(50).collect();
        std::fs::write(&dataset_path, lines.join("\n") + "\n")?;
        eprintln!("wrote 50-line smoke dataset to {dataset_path:?}");
    }

    // Use a small, cheap base model for the smoke test.
    // Qwen/Qwen3-1.7B is ~3.5GB on disk, trains in minutes on an H100.
    let base_model = "Qwen/Qwen3-1.7B".to_string();

    // Minimal params: 1 epoch, default LoRA (r=16, alpha=32).
    let params = TrainingParams {
        num_epochs: 1,
        batch_size: 1,
        ..Default::default()
    };

    let mut job = TrainingJob::new(
        dataset_path,
        base_model.clone(),
        params,
        TrainingHostId::Runpod,
        TrainingHarnessId::Axolotl,
    );

    // Artifacts: the pod needs to know where to upload the adapter.
    // Use a unique repo name so we don't clobber existing adapters.
    let job_id_short = &job.id[..8];
    let model_repo = format!("mdz-axo/smoke-test-{}", job_id_short);
    job.artifacts = Some(TrainingArtifacts {
        dataset: hkask_mcp_training::huggingface::TrainingArtifact {
            repository: "mdz-axo/capabilities-researcher-qa".to_string(),
            revision: "main".to_string(),
            path: "train_chat_full.jsonl".to_string(),
            sha256: String::new(), // not enforced for public datasets
        },
        model_repository: model_repo.clone(),
        completion_manifest_path: "/workspace/completion.json".to_string(),
    });

    eprintln!("submitting smoke training job:");
    eprintln!("  job_id:           {}", job.id);
    eprintln!("  base_model:       {}", job.base_model);
    eprintln!("  model_repository: {model_repo}");
    eprintln!("  dataset_path:     {}", job.dataset_path.display());
    eprintln!("  image:            docker.io/mdzaxo/axolotl-lora-trainer:latest");
    eprintln!(
        "  gpu:              NVIDIA H100 80GB HBM3 (default for small models is RTX 4090; override below)"
    );

    // Build the host. We use create_host() to exercise the real factory path.
    let host_config = TrainingHostConfig {
        host: TrainingHostId::Runpod,
        runpod_api_key: api_key,
        runpod_template_id: String::new(), // use Docker image, not template
    };
    let harness = Box::new(AxolotlHarness);
    let host = providers::create_host(&host_config, harness)?;

    // Render and print the axolotl YAML so we can verify the config visually.
    let yaml = AxolotlHarness.render_config(&job)?;
    eprintln!("--- axolotl config ---");
    eprintln!("{yaml}");
    eprintln!("--- end config ---");

    // Submit the job.
    let pod_id = host.submit(&job).await?;
    eprintln!("✓ pod created on RunPod:");
    eprintln!("  pod_id: {pod_id}");
    eprintln!("  HF_TOKEN set: {}", !hf_token.is_empty());
    eprintln!();
    eprintln!("monitor with:");
    eprintln!(
        "  cargo run --example smoke_status_runpod --release -p hkask-mcp-training -- {pod_id}"
    );
    eprintln!();
    eprintln!("cancel with:");
    eprintln!(
        "  cargo run --example smoke_cancel_runpod --release -p hkask-mcp-training -- {pod_id}"
    );

    // Persist the job_id → pod_id mapping so the status example can find it.
    let mapping =
        serde_json::json!({ "job_id": job.id, "pod_id": pod_id, "model_repo": model_repo });
    std::fs::write(
        "/tmp/smoke_last_job.json",
        serde_json::to_string_pretty(&mapping)?,
    )?;

    Ok(())
}
