//! Live training submission — creates a real RunPod pod with the
//! axolotl-lora-trainer Docker image and runs EVA LoRA training.
//!
//! Cost: ~$35-80 for full training (26-55h on H100).
//! For a quick test (~$0.50), set HKASK_QUICK_TEST=1 (5 min on RTX 3090).
//!
//! Run with:
//!   cargo test -p hkask-mcp-training --test live_training_submit -- --ignored --nocapture

use hkask_mcp_training::huggingface::{TrainingArtifact, TrainingArtifacts};
use hkask_mcp_training::providers::{
    AxolotlHarness, TrainingHarnessId, TrainingHost, TrainingHostId, TrainingJob, TrainingParams,
};
use std::path::PathBuf;

#[tokio::test]
#[ignore = "requires RUNPOD_API_KEY — creates a real training pod (costs money)"]
async fn submit_real_training_job() {
    dotenvy::dotenv().ok();

    let api_key =
        std::env::var("RUNPOD_API_KEY").expect("RUNPOD_API_KEY must be set for live training");
    let template_id = std::env::var("RUNPOD_TEMPLATE_ID").unwrap_or_default();

    // Use a persistent pods file (not temp) so we can track the pod across restarts
    let pods_path = "data/training-pods.json";

    // SAFETY: test-only — no other threads are running at this point
    unsafe {
        std::env::set_var("HKASK_PODS_FILE", pods_path);
        // Use H100 for real training, RTX 3090 for quick test
        if std::env::var("HKASK_QUICK_TEST").is_ok() {
            std::env::set_var("RUNPOD_GPU_TYPE_ID", "NVIDIA GeForce RTX 3090");
            std::env::set_var("RUNPOD_CONTAINER_DISK_GB", "100");
            std::env::set_var("RUNPOD_MIN_MEMORY_GB", "16");
            std::env::set_var("RUNPOD_MIN_VCPU_COUNT", "4");
        } else {
            std::env::set_var("RUNPOD_GPU_TYPE_ID", "NVIDIA H100 80GB HBM3");
            std::env::set_var("RUNPOD_CONTAINER_DISK_GB", "200");
            std::env::set_var("RUNPOD_MIN_MEMORY_GB", "80");
            std::env::set_var("RUNPOD_MIN_VCPU_COUNT", "8");
        }
    }

    let harness = AxolotlHarness;
    let host = hkask_mcp_training::providers::runpod::RunpodHost::new(
        api_key,
        template_id,
        Box::new(harness),
    );

    // Construct the training job with HuggingFace artifacts
    let mut job = TrainingJob::new(
        PathBuf::from("/workspace/data/train_chat_full.jsonl"),
        "unsloth/Qwen3.6-27B".to_string(),
        TrainingParams::default(),
        TrainingHostId::Runpod,
        TrainingHarnessId::Axolotl,
    );
    job.artifacts = Some(TrainingArtifacts {
        dataset: TrainingArtifact {
            repository: "mdz-axo/capabilities-researcher-qa".to_string(),
            revision: "main".to_string(),
            path: "train_chat_full.jsonl".to_string(),
            sha256: String::new(),
        },
        model_repository: "mdz-axo/capabilities-researcher-v3-eva".to_string(),
        completion_manifest_path: "/workspace/completion.json".to_string(),
    });

    // Submit the training job
    let is_quick = std::env::var("HKASK_QUICK_TEST").is_ok();
    let gpu_type = if is_quick { "RTX 3090" } else { "H100 80GB" };
    println!("=== Submitting training job ===");
    println!("Job ID: {}", job.id);
    println!("Base model: {}", job.base_model);
    println!("GPU type: {}", gpu_type);
    println!("Image: mdzaxo/axolotl-lora-trainer:latest");
    println!("Model repo: mdz-axo/capabilities-researcher-v3-eva");
    println!("Dataset: mdz-axo/capabilities-researcher-qa/train_chat_full.jsonl");
    println!();

    let pod_id = host.submit(&job).await.expect("submit should succeed");
    println!("=== Pod created ===");
    println!("Pod ID: {}", pod_id);
    println!("Pods file: {}", pods_path);
    println!();
    println!("=== To monitor training ===");
    println!("  # Check pod status:");
    println!("  curl -s --request POST --header 'content-type: application/json' \\");
    println!("    --url 'https://api.runpod.io/graphql?api_key=$RUNPOD_API_KEY' \\");
    println!(
        "    --data '{{\"query\": \"{{ pod(input: {{ podId: \\\"{}\\\" }}) {{ desiredStatus runtime {{ uptimeInSeconds }} }} }}\"}}'",
        pod_id
    );
    println!();
    println!("=== To terminate the pod ===");
    println!(
        "  cargo test -p hkask-mcp-training --test live_runpod_persistence -- --ignored --nocapture"
    );
    println!();
    println!("=== Training job submitted successfully ===");
    println!("Job ID: {} (save this for status queries)", job.id);
    println!("Pod ID: {} (save this for manual termination)", pod_id);
}
