//! Live smoke test — verifies end-to-end training completion detection.
//!
//! This is the FIRST test that verifies training can be detected as complete.
//! It creates a real RunPod pod, runs a tiny training job, and polls
//! training_status until the completion manifest is fetched from HuggingFace.
//!
//! Run with:
//!   set -a && source .env && set +a && \
//!   cargo test --test smoke_test -- --ignored --nocapture
//!
//! Requires: RUNPOD_API_KEY, HF_TOKEN, HKASK_HF_ARTIFACT_OWNER,
//!           HKASK_HF_DATASET_REPO, HKASK_HF_MODEL_REPO

use hkask_inference::InferenceConfig;
use hkask_mcp_training::TrainingServer;
use hkask_mcp_training::adapter::AdapterStore;
use hkask_mcp_training::adapters::JobStore;
use hkask_mcp_training::dataset::DatasetPipeline;
use hkask_mcp_training::providers::types::LoraInit;
use hkask_mcp_training::providers::{
    TrainingHarnessId, TrainingHostConfig, TrainingHostId, TrainingParams, create_host,
};
use hkask_mcp_training::types::{TrainStatusRequest, TrainSubmitRequest};
use hkask_storage::database::sqlite::SqliteDriver;
use hkask_types::WebID;
use rmcp::handler::server::wrapper::Parameters;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Parse the success envelope `{"content": <value>}`; falls back to the raw value.
fn parse_content(out: &str) -> serde_json::Value {
    let v: serde_json::Value = serde_json::from_str(out).expect("tool output is JSON");
    v.get("content").cloned().unwrap_or(v)
}

#[tokio::test]
#[ignore = "requires live RunPod + HuggingFace credentials (~$1-2 GPU cost)"]
async fn smoke_test_training_completion() {
    // ── 1. Create a tiny dataset (10 ChatML samples) ──────────────────────
    let dataset_path = std::env::temp_dir().join("hkask_smoke_test_dataset.jsonl");
    let mut dataset = String::new();
    for i in 0..10 {
        let record = serde_json::json!({
            "messages": [
                {"role": "user", "content": format!("What is {i}?")},
                {"role": "assistant", "content": format!("{i} is a number between {i} and {}.", i + 1)}
            ]
        });
        dataset.push_str(&record.to_string());
        dataset.push('\n');
    }
    std::fs::write(&dataset_path, &dataset).expect("write dataset");
    eprintln!("Dataset written: {} (10 samples)", dataset_path.display());

    // ── 2. Set up the TrainingServer with real credentials ────────────────
    let pool = SqliteDriver::in_memory_pool().expect("in-memory pool");
    let driver: Arc<dyn hkask_storage::database::driver::DatabaseDriver> =
        Arc::new(SqliteDriver::new(pool.clone()));
    let adapter_store = Arc::new(AdapterStore::from_driver(driver));
    let job_store = JobStore::new(pool).expect("job store");

    let host_config = TrainingHostConfig {
        host: TrainingHostId::Runpod,
        runpod_api_key: std::env::var("RUNPOD_API_KEY").expect("RUNPOD_API_KEY"),
        runpod_template_id: std::env::var("RUNPOD_TEMPLATE_ID").unwrap_or_default(),
        runpod_gpu_type_id: std::env::var("RUNPOD_GPU_TYPE_ID").unwrap_or_default(),
        runpod_container_disk_gb: std::env::var("RUNPOD_CONTAINER_DISK_GB")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        runpod_min_memory_gb: std::env::var("RUNPOD_MIN_MEMORY_GB")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        runpod_min_vcpu_count: std::env::var("RUNPOD_MIN_VCPU_COUNT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        runpod_docker_image: std::env::var("RUNPOD_DOCKER_IMAGE").unwrap_or_default(),
    };
    let host = create_host(&host_config).expect("create host");

    let server = TrainingServer::new(
        WebID::new(),
        "smoke-test-userpod".into(),
        None,
        None,
        host,
        TrainingHostId::Runpod,
        TrainingHarnessId::Axolotl,
        Mutex::new(DatasetPipeline::new(PathBuf::from(
            "/tmp/hkask-smoke-cache",
        ))),
        adapter_store,
        Some(job_store),
        None,
        InferenceConfig::default(),
    );

    // ── 3. Submit a minimal training job ──────────────────────────────────
    let mut params = TrainingParams::default();
    params.lora.init_lora_weights = Some(LoraInit::Eva);
    params.lora.r = 8;
    params.lora.alpha = 16;
    params.num_epochs = 1;
    params.batch_size = 1;
    params.learning_rate = 1e-4;
    params.optimization.gradient_accumulation_steps = 4;
    params.optimization.lr_scheduler = Some("cosine".to_string());
    params.sequence.sequence_len = Some(2048);
    params.advanced.bf16 = true;
    params.advanced.eval_split_ratio = Some(0.1);

    // Use a small model — Qwen2.5-0.5B-Instruct (~1GB, downloads fast)
    let req: TrainSubmitRequest = serde_json::from_value(serde_json::json!({
        "dataset_path": dataset_path.to_string_lossy(),
        "base_model": "Qwen/Qwen2.5-0.5B-Instruct",
        "params": params,
        "feedback_path": null,
        "skill_name": null,
        "adapter_name": null,
        "merged_output_path": null
    }))
    .expect("deserialize TrainSubmitRequest");

    eprintln!("Submitting training job (Qwen2.5-0.5B, 10 samples, 1 epoch, pre-built template)...");
    let submit_result = server.training_submit(Parameters(req)).await;
    let submit_content = parse_content(&submit_result);

    if submit_content.get("error").is_some() || submit_content.get("detail").is_some() {
        eprintln!("Submit failed: {submit_content}");
        panic!("training_submit failed");
    }

    let job_id = submit_content["job_id"]
        .as_str()
        .expect("job_id in submit response");
    let provider_job_id = submit_content["provider_job_id"]
        .as_str()
        .unwrap_or("unknown");
    eprintln!("Job submitted successfully!");
    eprintln!("  job_id: {job_id}");
    eprintln!("  provider_job_id: {provider_job_id}");

    // ── 4. Poll training_status until completion or timeout ───────────────
    // Expected timeline on H100 with 0.5B model:
    //   - Pod creation: ~1-2 min
    //   - pip install axolotl: ~5-10 min
    //   - Model download: ~1 min (0.5B is ~1GB)
    //   - Training (10 samples, 1 epoch): ~2-5 min
    //   - Manifest upload: ~30 sec
    //   Total: ~10-20 min
    // Timeout: 45 min (pip install can take 20+ min on a fresh pod)
    let timeout = tokio::time::Instant::now() + tokio::time::Duration::from_secs(2700);
    let mut last_status = String::new();
    let mut poll_count = 0u32;

    loop {
        if tokio::time::Instant::now() >= timeout {
            eprintln!("\nTIMEOUT after 45 minutes. Last status: {last_status}");
            eprintln!("Cancelling pod to stop billing...");
            let cancel_req: hkask_mcp_training::types::TrainCancelRequest =
                serde_json::from_value(serde_json::json!({"job_id": job_id}))
                    .expect("deserialize cancel");
            let _ = server.training_cancel(Parameters(cancel_req)).await;
            eprintln!("Pod cancelled.");
            panic!("timeout waiting for training completion");
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        poll_count += 1;

        let status_req: TrainStatusRequest =
            serde_json::from_value(serde_json::json!({"job_id": job_id}))
                .expect("deserialize status");
        let status_result = server.training_status(Parameters(status_req)).await;
        let status_content = parse_content(&status_result);

        let status = status_content["status"].as_str().unwrap_or("unknown");
        if status != last_status {
            eprintln!(
                "\n[poll #{poll_count}] {time} — Status: {status}",
                time = chrono::Utc::now().format("%H:%M:%S UTC")
            );
            if status == "running" && last_status.is_empty() {
                eprintln!("  Pod is running. Waiting for training to complete...");
                eprintln!("  (pip install + model download + training + manifest upload)");
            }
            last_status = status.to_string();
        } else {
            eprint!(".");
            use std::io::Write;
            std::io::stderr().flush().ok();
        }

        if status == "completed" {
            eprintln!("\n\n=== TRAINING COMPLETED ===");
            eprintln!("Full status response:\n{status_content}");

            // Verify adapter was auto-registered
            let adapter_registered = status_content["adapter_registered"]
                .as_bool()
                .unwrap_or(false);
            assert!(
                adapter_registered,
                "adapter must be auto-registered on completion"
            );
            eprintln!("\n✅ Adapter auto-registered successfully!");

            if let Some(repo) = status_content["adapter_repository"].as_str() {
                eprintln!("  adapter_repository: {repo}");
            }
            if let Some(path) = status_content["adapter_path"].as_str() {
                eprintln!("  adapter_path: {path}");
            }
            if let Some(name) = status_content["adapter_name"].as_str() {
                eprintln!("  adapter_name: {name}");
            }

            // Cancel the pod to stop billing
            eprintln!("\nCancelling pod to stop billing...");
            let cancel_req: hkask_mcp_training::types::TrainCancelRequest =
                serde_json::from_value(serde_json::json!({"job_id": job_id}))
                    .expect("deserialize cancel");
            let _ = server.training_cancel(Parameters(cancel_req)).await;
            eprintln!("Pod cancelled. Smoke test PASSED!");
            break;
        } else if status == "failed" {
            eprintln!("\n\n=== TRAINING FAILED ===");
            eprintln!("Full status response:\n{status_content}");

            // Cancel the pod to stop billing
            let cancel_req: hkask_mcp_training::types::TrainCancelRequest =
                serde_json::from_value(serde_json::json!({"job_id": job_id}))
                    .expect("deserialize cancel");
            let _ = server.training_cancel(Parameters(cancel_req)).await;
            panic!("training failed — check pod logs via SSH for details");
        }
    }
}
