//! Live RunPod integration test — verifies pod persistence and drain.
//!
//! This test creates a REAL RunPod pod, verifies the pod ID is persisted to
//! the JSON file, then immediately terminates it via drain_all_pods.
//! Cost: < $0.05 (pod runs for < 30 seconds).
//!
//! Requires:
//!   RUNPOD_API_KEY — RunPod API key
//!   RUNPOD_DOCKER_IMAGE or RUNPOD_TEMPLATE_ID — pod image source
//!   RUNPOD_GPU_TYPE_ID — GPU type (optional, defaults to "NVIDIA GeForce RTX 4090")
//!
//! Run with:
//!   RUNPOD_DOCKER_IMAGE=runpod/pytorch:2.4.0-py3.11-cuda12.4.1-devel-ubuntu22.04 \
//!   RUNPOD_GPU_TYPE_ID="NVIDIA GeForce RTX 3090" \
//!   cargo test -p hkask-mcp-training --test live_runpod_persistence -- --ignored --nocapture

use hkask_mcp_training::huggingface::{TrainingArtifact, TrainingArtifacts};
use hkask_mcp_training::providers::{
    AxolotlHarness, TrainingHarnessId, TrainingHost, TrainingHostId, TrainingJob, TrainingParams,
};
use std::path::PathBuf;

#[tokio::test]
#[ignore = "requires RUNPOD_API_KEY — creates a real pod"]
async fn runpod_pod_persistence_and_drain() {
    dotenvy::dotenv().ok();

    let api_key =
        std::env::var("RUNPOD_API_KEY").expect("RUNPOD_API_KEY must be set for live test");
    let template_id = std::env::var("RUNPOD_TEMPLATE_ID").unwrap_or_default();

    // Use a temp pods file so we don't clobber any existing persistence file
    let pods_file = tempfile::NamedTempFile::new().expect("tempfile");
    let pods_path = pods_file.path().to_string_lossy().to_string();

    // SAFETY: test-only — no other threads are running at this point
    unsafe {
        std::env::set_var("HKASK_PODS_FILE", &pods_path);
    }

    // GPU type, docker image, and resource limits come from the environment
    // (or code defaults). The test does not override them.

    let harness = AxolotlHarness;
    let host = hkask_mcp_training::providers::runpod::RunpodHost::new(
        api_key,
        template_id,
        Box::new(harness),
    );

    // Construct a minimal training job with HuggingFace artifacts
    let mut job = TrainingJob::new(
        PathBuf::from("/tmp/test_dataset_100.jsonl"),
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
        model_repository: "mdz-axo/test-adapter".to_string(),
        completion_manifest_path: "/workspace/completion.json".to_string(),
    });

    // Step 1: Submit — creates a real pod on RunPod
    println!("Submitting training job {}...", &job.id[..8]);
    let pod_id = host.submit(&job).await.expect("submit should succeed");
    println!("Pod created: {}", pod_id);
    assert!(!pod_id.is_empty(), "pod_id should not be empty");

    // Step 2: Verify pod ID was persisted to the JSON file
    let persisted_content =
        std::fs::read_to_string(&pods_path).expect("pods file should exist after submit");
    println!("Persisted pods: {}", persisted_content);
    assert!(
        persisted_content.contains(&pod_id),
        "pods file should contain the pod_id: {}",
        persisted_content
    );

    // Step 3: Verify status query works
    let status = host.status(&job.id).await.expect("status should succeed");
    println!("Pod status: {:?}", status);

    // Step 4: Drain all pods — terminates the pod via GraphQL podTerminate
    println!("Draining all pods...");
    let drained = host.drain_all_pods().await.expect("drain should succeed");
    println!("Drained {} pod(s)", drained);
    assert!(drained >= 1, "at least one pod should have been drained");

    // Step 5: Verify pods file is now empty
    let after_drain =
        std::fs::read_to_string(&pods_path).expect("pods file should still exist after drain");
    println!("Pods file after drain: {}", after_drain);
    assert!(
        after_drain.trim() == "{}" || after_drain.trim().is_empty(),
        "pods file should be empty after drain: {}",
        after_drain
    );

    println!("PASSED: Pod persistence and drain verified end-to-end");
}
