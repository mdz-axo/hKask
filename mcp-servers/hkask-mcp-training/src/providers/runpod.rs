//! Runpod GPU cloud training host.
//!
//! Dispatches training jobs to GPU pods via the Runpod GraphQL API.
//! Uses a pre-built template with axolotl installed; training is dispatched
//! via environment variables injected into the pod.
//!
//! ## API surface
//!
//! Pod creation uses the `podFindAndDeployOnDemand` GraphQL mutation — the
//! current RunPod API no longer exposes `podCreateAndDeploy` (it was removed
//! when RunPod migrated to the Pools/Reservations model). The mutation shape
//! here mirrors the RunPod Python SDK's `runpod.create_pod` (see
//! `runpod/api/mutations/pods.py`), which builds an inline `input` object.
//! Status queries use `pod(input: { podId })` and cancellation uses
//! `podTerminate(input: { podId })` — both still present in the current API.
//!
//! Environment variables (resolved keychain-first via `CredentialRequirement`
//! declarations in `hkask-mcp-training/src/lib.rs`, then flowed through
//! `ServerContext.credentials` → `TrainingHostConfig` → `RunpodHost` fields):
//! - `RUNPOD_API_KEY` — Runpod API key (required)
//! - `RUNPOD_TEMPLATE_ID` — GPU pod template ID (optional; defaults to
//!   `DEFAULT_RUNPOD_TEMPLATE_ID` = `f4wac8wrhz`, the `hkask-axolotl-config-from-env`
//!   template with axolotl pre-installed)
//! - `RUNPOD_DOCKER_IMAGE` — Docker image name (optional; takes precedence
//!   over template; defaults to `DEFAULT_RUNPOD_DOCKER_IMAGE` =
//!   `winglian/axolotl-cloud:main-latest`)
//! - `RUNPOD_GPU_TYPE_ID` — GPU type ID, e.g. "NVIDIA RTX 4090" or
//!   "NVIDIA A100-SXM4-80GB" (default: model-size heuristic). Note: the
//!   variable is `RUNPOD_GPU_TYPE_ID`, not `RUNPOD_GPU_TYPE` — the latter is
//!   ignored. When the operator sets this explicitly, it is authoritative and
//!   the heuristic does not fire.
//! - `RUNPOD_CONTAINER_DISK_GB` — Container disk in GB (default: model-size
//!   heuristic; 50/100/200 by model class)
//! - `RUNPOD_MIN_MEMORY_GB` — Minimum memory in GB (default: 24)
//! - `RUNPOD_MIN_VCPU_COUNT` — Minimum vCPU count (default: 8)
//! - `HKASK_DATASET_URL` — Remote-readable URL where the pod can download the dataset.
//!   Submission fails before creating a pod when this value is empty.
//!
//! `.env` is deprecated for this server — deployment settings must come from
//! the OS keychain (`kask keystore load`) or the explicit process environment.
use crate::providers::harness::HarnessAdapter;
use crate::providers::types::*;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// ── Default pod template configuration ─────────────────────────────────
//
// These are the canonical defaults for hKask training pods. They are exposed
// as module-level constants (not magic strings in submit()) so they can be
// referenced, documented, and overridden together.
//
// Override via keychain/env (resolved keychain-first in lib.rs):
//   RUNPOD_TEMPLATE_ID  — a RunPod template ID with axolotl pre-installed
//   RUNPOD_DOCKER_IMAGE — a Docker image name (takes precedence over template)
//
// See docs/how-to/runpod-lora-training-guide.md for the full rationale.

/// Default RunPod template ID for axolotl training.
///
/// This template (`hkask-axolotl-config-from-env`) uses the
/// `winglian/axolotl-cloud:main-latest` image with axolotl + all deps
/// pre-installed, and a startup script that reads the `HKASK_AXOLOTL_CONFIG`
/// env var (rendered by Rust `AxolotlHarness::render_config`).
///
/// Using a pre-built template avoids the pip-install restart loop documented
/// in Lesson 10 of runpod-lora-training-guide.md.
const DEFAULT_RUNPOD_TEMPLATE_ID: &str = "f4wac8wrhz";

/// Default Docker image for axolotl training.
///
/// RunPod's `podFindAndDeployOnDemand` requires `imageName` to be non-empty
/// even when a template is used. This is the image the template is based on.
const DEFAULT_RUNPOD_DOCKER_IMAGE: &str = "winglian/axolotl-cloud:main-latest";

/// Bundled construction parameters for `RunpodHost::new`.
///
/// Mirrors the `PodDeploySpec` pattern: keeps `RunpodHost::new` under clippy's
/// argument-count limit while making the operator-accepted deployment settings
/// (GPU type, disk, memory, vCPU, image) explicit and self-documenting. All
/// fields are resolved keychain-first in `lib.rs` and flowed through
/// `TrainingHostConfig` → `create_host` → here.
pub struct RunpodHostInit {
    pub api_key: String,
    pub template_id: String,
    /// Operator-accepted GPU type ID (e.g. `"NVIDIA H100 80GB HBM3"`).
    /// Empty defers to the model-size heuristic in `submit`.
    pub gpu_type_id: String,
    /// Operator-accepted container disk in GB. `0` defers to the heuristic.
    pub container_disk_gb: u32,
    /// Operator-accepted minimum memory in GB. `0` defers to the default.
    pub min_memory_gb: u32,
    /// Operator-accepted minimum vCPU count. `0` defers to the default.
    pub min_vcpu: u32,
    /// Operator-accepted Docker image. Empty defers to
    /// `DEFAULT_RUNPOD_DOCKER_IMAGE`.
    pub docker_image: String,
}

/// Runpod GPU cloud training host — dispatches training to GPU pods.
///
/// Uses the Runpod GraphQL API to create GPU pods from a pre-built template
/// (with axolotl installed), execute training, and retrieve LoRA adapters.
/// This is the "cloud dispatch" path for Axolotl — instead of running locally,
/// training runs on Runpod's GPU infrastructure.
///
/// **Template requirements:** The pod template must include a startup script
/// that reads `HKASK_*` environment variables, downloads the dataset from
/// `HKASK_DATASET_URL`, runs axolotl training, and uploads the resulting
/// adapter weights to a storage location.
pub struct RunpodHost {
    api_key: String,
    template_id: String,
    /// Operator-accepted GPU type ID (e.g. `"NVIDIA H100 80GB HBM3"`).
    /// Empty defers to the model-size heuristic in `submit`.
    gpu_type_id: String,
    /// Operator-accepted container disk in GB. `0` defers to the heuristic.
    container_disk_gb: u32,
    /// Operator-accepted minimum memory in GB. `0` defers to the default.
    min_memory_gb: u32,
    /// Operator-accepted minimum vCPU count. `0` defers to the default.
    min_vcpu: u32,
    /// Operator-accepted Docker image. Empty defers to
    /// `DEFAULT_RUNPOD_DOCKER_IMAGE`.
    docker_image: String,
    graphql_url: String,
    #[allow(dead_code)]
    harness: Box<dyn HarnessAdapter>,
    client: reqwest::Client,
    /// job_id -> pod_id mapping for status/cancel
    jobs: Arc<Mutex<HashMap<String, String>>>,
    /// Path to the pod ID persistence file (JSON: {job_id: pod_id}).
    pods_file: PathBuf,
}

fn map_pod_status(status: &str) -> TrainingJobStatus {
    match status {
        "CREATING" | "PENDING" => TrainingJobStatus::Queued,
        "RUNNING" => TrainingJobStatus::Running,
        "FAILED" | "ERROR" | "STOPPED" | "TERMINATED" => TrainingJobStatus::Failed,
        _ => TrainingJobStatus::Queued,
    }
}

impl RunpodHost {
    pub fn new(init: RunpodHostInit, harness: Box<dyn HarnessAdapter>) -> Self {
        let pods_file = PathBuf::from(
            std::env::var("HKASK_PODS_FILE")
                .unwrap_or_else(|_| "data/training-pods.json".to_string()),
        );
        // Load persisted pod IDs so we can cancel orphaned pods after a restart.
        let persisted = Self::load_pods(&pods_file);
        if !persisted.is_empty() {
            tracing::warn!(
                target: "hkask.training.runpod",
                count = persisted.len(),
                file = %pods_file.display(),
                "Loaded persisted pod IDs from previous session — call drain_all_pods() on shutdown to terminate them"
            );
        }
        Self {
            api_key: init.api_key,
            template_id: init.template_id,
            gpu_type_id: init.gpu_type_id,
            container_disk_gb: init.container_disk_gb,
            min_memory_gb: init.min_memory_gb,
            min_vcpu: init.min_vcpu,
            docker_image: init.docker_image,
            graphql_url: "https://api.runpod.io/graphql".to_string(),
            harness,
            client: reqwest::Client::new(),
            jobs: Arc::new(Mutex::new(persisted)),
            pods_file,
        }
    }

    /// Borrow the job_id → pod_id map for lookup (used by smoke test examples).
    pub fn jobs_for_lookup(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        self.jobs.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Inject a synthetic job_id → pod_id mapping where the job_id equals the
    /// pod_id (used by smoke test examples that only have the pod_id).
    pub fn inject_pod_id(&self, pod_id: &str) {
        let mut map = self.jobs.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(pod_id.to_string(), pod_id.to_string());
    }

    /// Load persisted pod IDs from the JSON file.
    fn load_pods(path: &std::path::Path) -> HashMap<String, String> {
        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => HashMap::new(),
        }
    }

    /// Persist the current pod ID map to the JSON file atomically.
    /// Writes to a temp file then renames — crash-safe.
    fn persist_pods(&self) {
        let map = match self.jobs.lock() {
            Ok(m) => m.clone(),
            Err(_) => return,
        };
        if let Ok(json) = serde_json::to_string_pretty(&map) {
            let tmp = self.pods_file.with_extension("json.tmp");
            if std::fs::write(&tmp, &json).is_ok() {
                let _ = std::fs::rename(&tmp, &self.pods_file);
            }
        }
    }

    /// Terminate all known pods via GraphQL `podTerminate`.
    /// Call on shutdown to prevent orphaned pods from billing.
    pub async fn drain_all_pods(&self) -> Result<usize, ProviderError> {
        let pod_ids: Vec<(String, String)> = {
            let map = self
                .jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };
        let count = pod_ids.len();
        tracing::info!(
            target: "cns.training.provider.runpod.drain",
            count = count,
            "Draining all RunPod pods"
        );
        for (job_id, pod_id) in &pod_ids {
            let mutation = r#"
                mutation TerminatePod($id: String!) {
                    podTerminate(input: { podId: $id })
                }
            "#;
            match self.graphql_query(mutation, json!({ "id": pod_id })).await {
                Ok(_) => tracing::info!(
                    target: "hkask.training.runpod",
                    job_id = %job_id,
                    pod_id = %pod_id,
                    "Pod terminated during drain"
                ),
                Err(e) => tracing::warn!(
                    target: "hkask.training.runpod",
                    job_id = %job_id,
                    pod_id = %pod_id,
                    error = %e,
                    "Failed to terminate pod during drain — may need manual deletion via RunPod console"
                ),
            }
        }
        if let Ok(mut map) = self.jobs.lock() {
            map.clear();
        }
        self.persist_pods();
        Ok(count)
    }

    async fn graphql_query(
        &self,
        query: &str,
        variables: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        // Classify the GraphQL operation for observability — read-only inspection
        // of the query string, no behavior change.
        let query_type = if query.contains("podFindAndDeployOnDemand") {
            "create"
        } else if query.contains("podTerminate") {
            "terminate"
        } else if query.contains("pod(input") {
            "status"
        } else {
            "unknown"
        };
        let body = json!({
            "query": query,
            "variables": variables,
        });

        tracing::debug!(
            target: "cns.training.provider.runpod.graphql",
            query_type = query_type,
            "RunPod GraphQL request"
        );

        let response = self
            .client
            .post(&self.graphql_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Backend(format!("Runpod API request failed: {}", e)))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ProviderError::Backend(format!("Runpod API parse error: {}", e)))?;

        if let Some(errors) = json.get("errors") {
            tracing::warn!(
                target: "cns.training.provider.runpod.graphql",
                query_type = query_type,
                "RunPod GraphQL returned errors"
            );
            return Err(ProviderError::Backend(format!(
                "Runpod GraphQL errors: {}",
                serde_json::to_string_pretty(errors).unwrap_or_default()
            )));
        }

        tracing::info!(
            target: "cns.training.provider.runpod.graphql",
            query_type = query_type,
            "RunPod GraphQL request succeeded"
        );
        Ok(json)
    }

    /// Escape a string for safe interpolation into a GraphQL literal.
    /// Backslashes first, then double quotes — standard GraphQL string escaping.
    fn escape_graphql_string(s: &str) -> String {
        // GraphQL spec (June 2018 §2.9.2) requires escaping:
        //   " \ / \b \f \n \r \t and all control chars U+0000–U+001F
        // We escape backslash, quote, newline, CR, tab, and all remaining
        // control characters. Forward slash is not escaped (not required by
        // GraphQL, only by JSON — and RunPod's parser accepts it raw).
        let mut out = String::with_capacity(s.len() + 8);
        for c in s.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                '\u{0008}' => out.push_str("\\b"),
                '\u{000C}' => out.push_str("\\f"),
                c if c.is_control() && c as u32 <= 0x1F => {
                    out.push_str(&format!("\\u{:04x}", c as u32));
                }
                c => out.push(c),
            }
        }
        out
    }

    /// Build the inline `podFindAndDeployOnDemand` GraphQL mutation.
    ///
    /// This mirrors the RunPod Python SDK's `generate_pod_deployment_mutation`
    /// (`runpod/api/mutations/pods.py`) — the current API no longer exposes
    /// `podCreateAndDeploy`, so we deploy on-demand pods via
    /// `podFindAndDeployOnDemand` with an inline `input` object. Inline
    /// construction (rather than GraphQL variables) matches the SDK exactly
    /// and avoids depending on the schema's input-type name.
    ///
    /// The image source is resolved by the caller and passed via
    /// `PodDeploySpec.docker_image`: when non-empty it becomes `imageName`; when
    /// empty, `self.template_id` (if set) becomes `templateId` and carries the
    /// image + startup script. At least one of the two must be available,
    /// mirroring the SDK's validation (enforced by `submit`).
    fn build_pod_deploy_mutation(
        &self,
        job_id: &str,
        spec: &PodDeploySpec<'_>,
        env_entries: &[(&str, String)],
    ) -> String {
        let pod_name = format!("hkask-training-{}", &job_id[..8.min(job_id.len())]);
        let mut fields: Vec<String> = Vec::with_capacity(16);

        // Required fields (match SDK ordering).
        fields.push(format!(
            "name: \"{}\"",
            Self::escape_graphql_string(&pod_name)
        ));
        // The SDK always emits imageName; when a template is used instead of a
        // bare image, imageName is the empty string and templateId carries the
        // image + startup script.
        fields.push(format!(
            "imageName: \"{}\"",
            Self::escape_graphql_string(spec.docker_image)
        ));
        fields.push("cloudType: ALL".to_string());
        fields.push("startSsh: true".to_string());

        // Docker args (startup command) — read from RUNPOD_DOCKER_ARGS env var.
        // When non-empty, this becomes the Docker CMD, allowing pods without a
        // template to run a startup script (e.g. install axolotl + train).
        if !spec.docker_args.is_empty() {
            fields.push(format!(
                "dockerArgs: \"{}\"",
                Self::escape_graphql_string(spec.docker_args)
            ));
        }

        // GPU pod fields.
        fields.push(format!(
            "gpuTypeId: \"{}\"",
            Self::escape_graphql_string(spec.gpu_type_id)
        ));
        fields.push("supportPublicIp: true".to_string());
        fields.push("gpuCount: 1".to_string());
        fields.push(format!("containerDiskInGb: {}", spec.container_disk_gb));
        fields.push(format!("minVcpuCount: {}", spec.min_vcpu));
        fields.push(format!("minMemoryInGb: {}", spec.min_memory_gb));
        fields.push("dataCenterId: null".to_string());

        // Template ID (if set) — provides the image + startup script.
        // Takes precedence over self.template_id (which comes from config)
        // so submit() can resolve a default template at runtime.
        let template_id = if !spec.template_id.is_empty() {
            spec.template_id
        } else {
            &self.template_id
        };
        if !template_id.is_empty() {
            fields.push(format!(
                "templateId: \"{}\"",
                Self::escape_graphql_string(template_id)
            ));
        }

        // Environment variables injected into the pod.
        let env_items: Vec<String> = env_entries
            .iter()
            .map(|(k, v)| {
                format!(
                    "{{ key: \"{}\", value: \"{}\" }}",
                    Self::escape_graphql_string(k),
                    Self::escape_graphql_string(v)
                )
            })
            .collect();
        fields.push(format!("env: [{}]", env_items.join(", ")));

        let input_string = fields.join(", ");
        format!(
            "mutation {{\n  podFindAndDeployOnDemand(\n    input: {{ {} }}\n  ) {{\n    id\n    imageName\n    env\n    machineId\n    machine {{ podHostId }}\n  }}\n}}",
            input_string
        )
    }
}

/// Resolved pod deployment parameters passed to `build_pod_deploy_mutation`.
///
/// Bundling these keeps the helper under clippy's argument-count limit while
/// mirroring the RunPod SDK's `create_pod` parameter surface.
struct PodDeploySpec<'a> {
    gpu_type_id: &'a str,
    container_disk_gb: u32,
    min_memory_gb: u32,
    min_vcpu: u32,
    docker_image: &'a str,
    docker_args: &'a str,
    template_id: &'a str,
}

#[async_trait::async_trait]
impl TrainingHost for RunpodHost {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        // GPU selection: operator-accepted `RUNPOD_GPU_TYPE_ID` (resolved
        // keychain-first into `self.gpu_type_id`) is authoritative when set.
        // When unset, fall back to the model-size heuristic — small models
        // (≤14B) use RTX 4090, large models (20B–70B) use A100, very large
        // (120B+) use H100. GPU type IDs must match RunPod's gpuTypes query
        // exactly. This heuristic is the lora-training skill's G2 gate
        // (memory budget vs model size) — it informs, never overrides, an
        // explicitly accepted operator value.
        let gpu_type_id = if !self.gpu_type_id.is_empty() {
            self.gpu_type_id.clone()
        } else {
            let lower = job.base_model.to_lowercase();
            if ["70b", "72b", "120b", "405b"]
                .iter()
                .any(|p| lower.contains(p))
            {
                "NVIDIA H100 80GB HBM3".to_string()
            } else if ["20b", "30b", "34b", "35b"]
                .iter()
                .any(|p| lower.contains(p))
            {
                "NVIDIA A100-SXM4-80GB".to_string()
            } else {
                "NVIDIA GeForce RTX 4090".to_string()
            }
        };
        // Container disk: operator-accepted value is authoritative when set;
        // otherwise larger models need more disk for weights + checkpoints.
        let container_disk_gb: u32 = if self.container_disk_gb > 0 {
            self.container_disk_gb
        } else {
            let lower = job.base_model.to_lowercase();
            if ["70b", "72b", "120b", "405b"]
                .iter()
                .any(|p| lower.contains(p))
            {
                200 // 70B model weights ~140GB + checkpoints
            } else if ["13b", "14b", "20b", "30b"]
                .iter()
                .any(|p| lower.contains(p))
            {
                100
            } else {
                50
            }
        };
        let min_memory_gb: u32 = if self.min_memory_gb > 0 {
            self.min_memory_gb
        } else {
            24
        };
        let min_vcpu: u32 = if self.min_vcpu > 0 { self.min_vcpu } else { 8 };
        let artifacts = job.artifacts.as_ref().ok_or_else(|| {
            ProviderError::DatasetError(
                "RunPod requires a published Hugging Face artifact path before creating a billable pod"
                    .to_string(),
            )
        })?;

        // Resolve the pod template and image. The operator-accepted values
        // (resolved keychain-first into `self.template_id` and
        // `self.docker_image`) are authoritative when set. Defaults use the
        // pre-built axolotl template (DEFAULT_RUNPOD_TEMPLATE_ID) which has
        // axolotl + all deps pre-installed and reads config from
        // HKASK_AXOLOTL_CONFIG, plus its base image
        // (DEFAULT_RUNPOD_DOCKER_IMAGE).
        // See docs/how-to/runpod-lora-training-guide.md Lesson 10.
        let template_id = if !self.template_id.is_empty() {
            self.template_id.clone()
        } else {
            DEFAULT_RUNPOD_TEMPLATE_ID.to_string()
        };
        // RunPod's podFindAndDeployOnDemand requires imageName to be non-empty
        // even when using a template. Default to the template's base image.
        let docker_image = if !self.docker_image.is_empty() {
            self.docker_image.clone()
        } else {
            DEFAULT_RUNPOD_DOCKER_IMAGE.to_string()
        };
        if docker_image.is_empty() && template_id.is_empty() {
            return Err(ProviderError::InvalidConfig(
                "Either RUNPOD_DOCKER_IMAGE or RUNPOD_TEMPLATE_ID must be set to create a RunPod pod"
                    .to_string(),
            ));
        }

        let mut env_entries: Vec<(&str, String)> = vec![
            ("HKASK_JOB_ID", job.id.clone()),
            ("HKASK_BASE_MODEL", job.base_model.clone()),
            (
                "HKASK_HF_DATASET_REPOSITORY",
                artifacts.dataset.repository.clone(),
            ),
            (
                "HKASK_HF_DATASET_REVISION",
                artifacts.dataset.revision.clone(),
            ),
            ("HKASK_HF_DATASET_PATH", artifacts.dataset.path.clone()),
            (
                "HKASK_EXPECTED_DATASET_SHA256",
                artifacts.dataset.sha256.clone(),
            ),
            (
                "HKASK_HF_MODEL_REPOSITORY",
                artifacts.model_repository.clone(),
            ),
            (
                "HKASK_COMPLETION_MANIFEST_PATH",
                artifacts.completion_manifest_path.clone(),
            ),
            ("HKASK_HARNESS", format!("{:?}", job.harness).to_lowercase()),
            ("HKASK_NUM_EPOCHS", job.params.num_epochs.to_string()),
            ("HKASK_LORA_R", job.params.lora.r.to_string()),
            ("HKASK_LORA_ALPHA", job.params.lora.alpha.to_string()),
            ("HKASK_LORA_DROPOUT", job.params.lora.dropout.to_string()),
            (
                "HKASK_LORA_TARGET_MODULES",
                job.params.lora.target_modules.join(","),
            ),
            (
                "HKASK_LORA_USE_RSLORA",
                job.params.lora.use_rslora.to_string(),
            ),
            ("HKASK_LORA_USE_DORA", job.params.lora.use_dora.to_string()),
            (
                "HKASK_LORA_INIT_WEIGHTS",
                job.params
                    .lora
                    .init_lora_weights
                    .as_ref()
                    .map(|i| i.as_config_value())
                    .unwrap_or_default(),
            ),
            (
                "HKASK_LORA_BIAS",
                format!("{:?}", job.params.lora.bias).to_lowercase(),
            ),
            ("HKASK_LEARNING_RATE", job.params.learning_rate.to_string()),
            ("HKASK_BATCH_SIZE", job.params.batch_size.to_string()),
            (
                "HKASK_GRAD_ACCUM",
                job.params
                    .optimization
                    .gradient_accumulation_steps
                    .to_string(),
            ),
            (
                "HKASK_WEIGHT_DECAY",
                job.params.optimization.weight_decay.to_string(),
            ),
            (
                "HKASK_MAX_GRAD_NORM",
                job.params
                    .optimization
                    .max_grad_norm
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            ),
            (
                "HKASK_WARMUP_STEPS",
                job.params
                    .optimization
                    .warmup_steps
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            ),
            (
                "HKASK_LR_SCHEDULER",
                job.params
                    .optimization
                    .lr_scheduler
                    .clone()
                    .unwrap_or_default(),
            ),
            (
                "HKASK_SEQ_LEN",
                job.params
                    .sequence
                    .sequence_len
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            ),
            (
                "HKASK_LOAD_IN_4BIT",
                job.params.quantization.load_in_4bit.to_string(),
            ),
            (
                "HKASK_BNB_4BIT_QUANT_TYPE",
                job.params
                    .quantization
                    .bnb_4bit_quant_type
                    .clone()
                    .unwrap_or_default(),
            ),
            (
                "HKASK_BNB_4BIT_COMPUTE_DTYPE",
                job.params
                    .quantization
                    .bnb_4bit_compute_dtype
                    .clone()
                    .unwrap_or_default(),
            ),
            (
                "HKASK_BNB_4BIT_USE_DOUBLE_QUANT",
                job.params
                    .quantization
                    .bnb_4bit_use_double_quant
                    .to_string(),
            ),
            ("HKASK_BF16", job.params.advanced.bf16.to_string()),
            ("HKASK_FP16", job.params.advanced.fp16.to_string()),
            (
                "HKASK_GRADIENT_CHECKPOINTING",
                job.params
                    .advanced
                    .gradient_checkpointing
                    .clone()
                    .unwrap_or_default(),
            ),
            (
                "HKASK_ATTN_IMPLEMENTATION",
                job.params
                    .advanced
                    .attn_implementation
                    .clone()
                    .unwrap_or_default(),
            ),
        ];

        // Render the axolotl YAML config from TrainingParams and pass it to
        // the pod as an env var. The pod's startup script writes it to
        // /workspace/config.yml and runs `axolotl train /workspace/config.yml`.
        let axolotl_yaml = crate::providers::AxolotlHarness
            .render_config(job)
            .map_err(|e| {
                ProviderError::InvalidConfig(format!("Failed to render axolotl YAML: {e}"))
            })?;
        env_entries.push(("HKASK_AXOLOTL_CONFIG", axolotl_yaml));

        // Generate docker args if not provided via env var.
        // The image `docker.io/mdzaxo/axolotl-lora-trainer:latest` ships a bash
        // entrypoint at /usr/local/bin/entrypoint.sh (set as Docker ENTRYPOINT)
        // that handles the full training lifecycle: pip install axolotl, write
        // config from HKASK_AXOLOTL_CONFIG, run `axolotl train`, upload adapter
        // via `huggingface-cli upload`, write completion manifest, and
        // `exec sleep infinity` for SSH debugging.
        //
        // We do NOT set dockerArgs by default — RunPod's dockerArgs overrides
        // the Docker CMD, and our image uses ENTRYPOINT (not CMD) to invoke
        // the entrypoint. Setting dockerArgs would pass the script path as
        // arguments to the entrypoint, causing unexpected behavior. Leaving
        // dockerArgs empty lets the image's ENTRYPOINT run naturally.
        //
        // RUNPOD_DOCKER_ARGS remains available as an override for operators
        // who need to customize the startup command.
        let docker_args = std::env::var("RUNPOD_DOCKER_ARGS").unwrap_or_default();

        let mutation = self.build_pod_deploy_mutation(
            &job.id,
            &PodDeploySpec {
                gpu_type_id: &gpu_type_id,
                container_disk_gb,
                min_memory_gb,
                min_vcpu,
                docker_image: &docker_image,
                docker_args: &docker_args,
                template_id: &template_id,
            },
            &env_entries,
        );

        tracing::debug!(
            target: "hkask.training.runpod.mutation",
            mutation_len = mutation.len(),
            docker_args_len = docker_args.len(),
            "Built pod deploy mutation"
        );

        let result = self.graphql_query(&mutation, json!({})).await?;

        let pod_id = result["data"]["podFindAndDeployOnDemand"]["id"]
            .as_str()
            .ok_or_else(|| ProviderError::Backend("No pod ID in Runpod response".to_string()))?
            .to_string();

        // Store pod_id for status/cancel
        if let Ok(mut map) = self.jobs.lock() {
            map.insert(job.id.clone(), pod_id.clone());
        }
        self.persist_pods();

        tracing::info!(
            target: "hkask.training.job.submit",
            job_id = %job.id,
            pod_id = %pod_id,
            host = "runpod",
            harness = ?job.harness,
            "Training pod created on Runpod"
        );

        tracing::info!(
            target: "cns.training.provider.runpod.submit",
            pod_id = %pod_id,
            gpu_type = %gpu_type_id,
            "RunPod pod submitted"
        );

        Ok(pod_id)
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        let pod_id = {
            let map = self
                .jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
            map.get(job_id).cloned()
        };

        let pod_id = match pod_id {
            Some(id) => id,
            None => {
                return Err(ProviderError::JobFailed(format!(
                    "No pod found for job {}",
                    job_id
                )));
            }
        };

        let query = r#"
            query GetPod($id: String!) {
                pod(input: { podId: $id }) {
                    id
                    desiredStatus
                    runtime { uptimeInSeconds }
                    machine { gpuTypeId }
                }
            }
        "#;

        let result = self.graphql_query(query, json!({ "id": pod_id })).await?;

        let status_str = result["data"]["pod"]["desiredStatus"]
            .as_str()
            .unwrap_or("UNKNOWN");

        tracing::debug!(
            target: "cns.training.provider.runpod.status",
            pod_id = %pod_id,
            desired_status = %status_str,
            "RunPod pod status"
        );

        Ok(map_pod_status(status_str))
    }

    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError> {
        let pod_id = {
            let map = self
                .jobs
                .lock()
                .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
            map.get(job_id).cloned()
        };

        let pod_id = match pod_id {
            Some(id) => id,
            None => {
                tracing::warn!(
                    target: "hkask.training.job.cancel",
                    job_id = %job_id,
                    "No pod found for job"
                );
                tracing::warn!(
                    target: "cns.training.provider.runpod.cancel",
                    "No pod found for job"
                );
                return Ok(());
            }
        };

        let mutation = r#"
            mutation TerminatePod($id: String!) {
                podTerminate(input: { podId: $id })
            }
        "#;

        self.graphql_query(mutation, json!({ "id": pod_id }))
            .await?;

        if let Ok(mut map) = self.jobs.lock() {
            map.remove(job_id);
        }
        self.persist_pods();

        tracing::info!(
            target: "hkask.training.job.cancel",
            job_id = %job_id,
            pod_id = %pod_id,
            host = "runpod",
            "Training pod terminated"
        );
        tracing::info!(
        target: "cns.training.provider.runpod.cancel",
        pod_id = %pod_id,
        "RunPod pod cancelled"
        );
        Ok(())
    }

    async fn completion_metadata(
        &self,
        _job_id: &str,
    ) -> Result<Option<CompletionMetadata>, ProviderError> {
        // Runpod doesn't provide structured training metrics via API.
        Ok(None)
    }

    async fn adapter_weight_path(
        &self,
        _adapter_id: &str,
    ) -> Result<Option<PathBuf>, ProviderError> {
        // Weights are on the Runpod pod — need to be downloaded separately.
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::harness::AxolotlHarness;

    #[test]
    fn terminal_pods_are_not_reported_as_successful_without_artifacts() {
        assert_eq!(map_pod_status("STOPPED"), TrainingJobStatus::Failed);
        assert_eq!(map_pod_status("TERMINATED"), TrainingJobStatus::Failed);
    }

    #[test]
    fn running_pod_remains_running() {
        assert_eq!(map_pod_status("RUNNING"), TrainingJobStatus::Running);
    }

    #[test]
    fn escape_graphql_string_handles_quotes_and_backslashes() {
        assert_eq!(RunpodHost::escape_graphql_string("simple"), "simple");
        assert_eq!(
            RunpodHost::escape_graphql_string(r#"has "quote""#),
            r#"has \"quote\""#
        );
        assert_eq!(
            RunpodHost::escape_graphql_string("path\\to\\file"),
            "path\\\\to\\\\file"
        );
    }

    #[test]
    fn escape_graphql_string_handles_newlines() {
        // GraphQL string literals cannot contain raw newlines — they must be \n
        let script = "#!/bin/bash\necho hello\n";
        let escaped = RunpodHost::escape_graphql_string(script);
        assert!(
            !escaped.contains('\n'),
            "escaped string must not contain raw newlines"
        );
        assert!(
            escaped.contains("\\n"),
            "escaped string must contain \\n for newlines"
        );
        assert_eq!(escaped, "#!/bin/bash\\necho hello\\n");
    }

    fn make_host(template_id: &str) -> RunpodHost {
        RunpodHost::new(
            RunpodHostInit {
                api_key: "test-key".to_string(),
                template_id: template_id.to_string(),
                gpu_type_id: String::new(),
                container_disk_gb: 0,
                min_memory_gb: 0,
                min_vcpu: 0,
                docker_image: String::new(),
            },
            Box::new(AxolotlHarness),
        )
    }

    #[test]
    fn build_mutation_uses_pod_find_and_deploy_on_demand() {
        let host = make_host("tpl-123");
        let mutation = host.build_pod_deploy_mutation(
            "abcdefgh-1234-5678-90ab-1234567890ab",
            &PodDeploySpec {
                gpu_type_id: "NVIDIA A100-SXM4-80GB",
                container_disk_gb: 60,
                min_memory_gb: 80,
                min_vcpu: 8,
                docker_image: "",
                docker_args: "",
                template_id: "tpl-123",
            },
            &[("HKASK_JOB_ID", "job-1".to_string())],
        );
        assert!(
            mutation.contains("podFindAndDeployOnDemand"),
            "mutation must use podFindAndDeployOnDemand, got: {mutation}"
        );
        assert!(!mutation.contains("podCreateAndDeploy"));
        assert!(mutation.contains("templateId: \"tpl-123\""));
        assert!(mutation.contains("gpuTypeId: \"NVIDIA A100-SXM4-80GB\""));
        assert!(mutation.contains("containerDiskInGb: 60"));
        assert!(mutation.contains("minMemoryInGb: 80"));
        assert!(mutation.contains("minVcpuCount: 8"));
        assert!(mutation.contains("name: \"hkask-training-abcdefgh\""));
        assert!(mutation.contains("HKASK_JOB_ID"));
        assert!(mutation.contains("job-1"));
    }

    #[test]
    fn build_mutation_uses_docker_image_when_set() {
        let host = make_host("");
        let mutation = host.build_pod_deploy_mutation(
            "abcdefgh-1234-5678-90ab-1234567890ab",
            &PodDeploySpec {
                gpu_type_id: "NVIDIA RTX 4090",
                container_disk_gb: 50,
                min_memory_gb: 24,
                min_vcpu: 8,
                docker_image: "runpod/pytorch:2.6.0",
                docker_args: "",
                template_id: "",
            },
            &[],
        );
        assert!(mutation.contains("imageName: \"runpod/pytorch:2.6.0\""));
        assert!(!mutation.contains("templateId"));
    }

    #[test]
    fn build_mutation_omits_template_id_when_empty() {
        let host = make_host("");
        let mutation = host.build_pod_deploy_mutation(
            "abcdefgh-1234-5678-90ab-1234567890ab",
            &PodDeploySpec {
                gpu_type_id: "NVIDIA RTX 4090",
                container_disk_gb: 50,
                min_memory_gb: 24,
                min_vcpu: 8,
                docker_image: "",
                docker_args: "",
                template_id: "",
            },
            &[],
        );
        assert!(mutation.contains("imageName: \"\""));
        assert!(!mutation.contains("templateId"));
    }
}
