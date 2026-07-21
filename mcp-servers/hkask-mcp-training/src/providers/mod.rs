//! Training provider abstraction — pluggable backend adapter for model fine-tuning.
//!
//! Each provider wraps a different training framework behind a common
//! `TrainingHost` trait. The MCP server maps its tool surface (`submit`,
//! `status`, `cancel`) to provider methods, isolating the MCP surface from
//! host-specific API differences.
//!
//! Architecture (cloud-only — no local training):
//!   TrainingHostConfig × HarnessAdapter → cloud Host → TrainingJob
//!
//! Provider selection is driven by `training.harness` in settings.json,
//! routed through `hkask-services` shared config init. The host is fixed
//! to Runpod (cloud-only, single host).

pub mod harness;
pub mod runpod;
pub mod trl_harness;
pub mod types;

// ── Re-exports for lib.rs compatibility ──────────────────────────────────

pub use harness::{AxolotlHarness, HarnessAdapter, LudwigHarness};
pub use runpod::RunpodHost;
pub use trl_harness::TrlHarness;
pub use types::{
    AdvancedParams, CompletionMetadata, LoraParams, OptimizationParams, ProviderError,
    QuantizationParams, SequenceParams, TrainingHarnessId, TrainingHost, TrainingHostId,
    TrainingJob, TrainingJobStatus, TrainingParams, TrlTrainer,
};

// ── Host factory ───────────────────────────────────────────────────────────

/// Create a training host from configuration and a pre-built harness.
///
/// The harness selects the tooling (Axolotl). The host is fixed to
/// Runpod — the only cloud host. The harness is injected into the host at
/// construction.
pub fn create_host(
    config: &TrainingHostConfig,
    harness: Box<dyn HarnessAdapter>,
) -> Result<Box<dyn TrainingHost>, ProviderError> {
    if config.runpod_api_key.is_empty() {
        return Err(ProviderError::Unavailable(
            "Runpod API key not configured (set RUNPOD_API_KEY)".to_string(),
        ));
    }
    // Template ID and Docker image are both optional individually — but at
    // least one must be available so `RunpodHost::submit` can resolve an image
    // source. The canonical flow uses the pre-built axolotl template
    // (`DEFAULT_RUNPOD_TEMPLATE_ID`) plus its base image
    // (`DEFAULT_RUNPOD_DOCKER_IMAGE`); both are baked into `RunpodHost` as
    // constants, so construction succeeds out of the box without requiring
    // the operator to set either var.
    if config.runpod_template_id.is_empty() && config.runpod_docker_image.is_empty() {
        tracing::warn!(
            target: "hkask.training.runpod",
            "Neither RUNPOD_TEMPLATE_ID nor RUNPOD_DOCKER_IMAGE is set — \
             RunpodHost::submit() will default to the canonical axolotl \
             template and base image"
        );
    }
    Ok(Box::new(RunpodHost::new(
        runpod::RunpodHostInit {
            api_key: config.runpod_api_key.clone(),
            template_id: config.runpod_template_id.clone(),
            gpu_type_id: config.runpod_gpu_type_id.clone(),
            container_disk_gb: config.runpod_container_disk_gb,
            min_memory_gb: config.runpod_min_memory_gb,
            min_vcpu: config.runpod_min_vcpu_count,
            docker_image: config.runpod_docker_image.clone(),
        },
        harness,
    )))
}

// ── Training host config ──────────────────────────────────────────────────

/// Training host configuration resolved from hKask settings.
///
/// Selects the Runpod cloud host with API credentials. The harness is
/// selected separately and injected into the host at construction time —
/// this config only describes *where* compute runs.
///
/// Deployment settings (GPU type, disk, memory, vCPU, image) are resolved
/// keychain-first via `CredentialRequirement` declarations — never from
/// `.env`. They flow through `ServerContext.credentials` into this struct,
/// then into `RunpodHost`, where `submit()` reads them as authoritative
/// operator-accepted values. The model-size heuristic in `submit()` is a
/// last-resort fallback for `gpu_type_id` only, used when the operator
/// leaves it unset; it must never override an explicitly accepted value.
#[derive(Debug, Clone)]
pub struct TrainingHostConfig {
    /// Selected training host (always Runpod — kept for future extensibility).
    pub host: TrainingHostId,
    /// Runpod API key.
    pub runpod_api_key: String,
    /// Runpod GPU pod template ID with axolotl pre-installed.
    pub runpod_template_id: String,
    /// Runpod GPU type ID (e.g. `"NVIDIA H100 80GB HBM3"`). Empty defers to
    /// the model-size heuristic in `RunpodHost::submit`.
    pub runpod_gpu_type_id: String,
    /// Container disk in GB. `0` defers to the model-size heuristic.
    pub runpod_container_disk_gb: u32,
    /// Minimum pod memory in GB. `0` defers to the RunpodHost default.
    pub runpod_min_memory_gb: u32,
    /// Minimum vCPU count. `0` defers to the RunpodHost default.
    pub runpod_min_vcpu_count: u32,
    /// Docker image name. Empty defers to `DEFAULT_RUNPOD_DOCKER_IMAGE`.
    pub runpod_docker_image: String,
}

impl Default for TrainingHostConfig {
    fn default() -> Self {
        Self {
            host: TrainingHostId::Runpod,
            runpod_api_key: String::new(),
            runpod_template_id: String::new(),
            runpod_gpu_type_id: String::new(),
            runpod_container_disk_gb: 0,
            runpod_min_memory_gb: 0,
            runpod_min_vcpu_count: 0,
            runpod_docker_image: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_id_from_str() {
        assert_eq!(
            TrainingHarnessId::from_str("axolotl"),
            Some(TrainingHarnessId::Axolotl)
        );
        assert_eq!(
            TrainingHarnessId::from_str("AXOLOTL"),
            Some(TrainingHarnessId::Axolotl)
        );
        assert_eq!(
            TrainingHarnessId::from_str("trl"),
            Some(TrainingHarnessId::Trl)
        );
        assert_eq!(
            TrainingHarnessId::from_str("ludwig"),
            Some(TrainingHarnessId::Ludwig)
        );
        assert_eq!(
            TrainingHarnessId::from_str("LUDWIG"),
            Some(TrainingHarnessId::Ludwig)
        );
        assert_eq!(TrainingHarnessId::from_str("unknown"), None);
    }

    #[test]
    fn host_id_from_str() {
        assert_eq!(
            TrainingHostId::from_str("runpod"),
            Some(TrainingHostId::Runpod)
        );
        assert_eq!(TrainingHostId::from_str("unknown"), None);
    }

    #[test]
    fn model_size_multiplier() {
        assert_eq!(types::extract_model_size_multiplier("Qwen3:8b"), 1);
        assert_eq!(types::extract_model_size_multiplier("Llama-3.3-70B"), 4);
        assert_eq!(types::extract_model_size_multiplier("Mixtral-8x7b"), 1);
        assert_eq!(types::extract_model_size_multiplier("unknown-model"), 2);
    }

    #[test]
    fn estimate_cost_is_positive() {
        let cost = types::estimate_training_cost_urj(&TrainingHostId::Runpod, 3, "Qwen3:8b");
        assert_eq!(cost, 1_500_000);
        let cost = types::estimate_training_cost_urj(&TrainingHostId::Runpod, 2, "Llama-3.3-70B");
        assert_eq!(cost, 4_000_000);
    }

    #[test]
    fn training_job_new_has_valid_defaults() {
        let params = TrainingParams::default();
        let job = TrainingJob::new(
            std::path::PathBuf::from("/tmp/test.jsonl"),
            "Qwen3:8b".into(),
            params,
            TrainingHostId::Runpod,
            TrainingHarnessId::Axolotl,
        );
        assert!(!job.id.is_empty());
        assert_eq!(job.base_model, "Qwen3:8b");
        assert_eq!(job.host, TrainingHostId::Runpod);
        assert_eq!(job.harness, TrainingHarnessId::Axolotl);
        assert_eq!(job.status, TrainingJobStatus::Queued);
        assert!(job.estimated_cost_urj > 0);
    }

    #[test]
    fn lora_params_default() {
        let params = LoraParams::default();
        assert_eq!(params.r, 16);
        assert_eq!(params.alpha, 32);
        assert_eq!(params.dropout, 0.0);
        assert_eq!(params.target_modules.len(), 7);
        assert!(!params.use_rslora);
    }

    #[test]
    fn training_params_default() {
        let params = TrainingParams::default();
        assert_eq!(params.num_epochs, 3);
        assert!(params.batch_size > 0);
    }

    #[test]
    fn training_job_status_is_serializable() {
        let status = TrainingJobStatus::Queued;
        let json = serde_json::to_string(&status).expect("serialize");
        assert!(json.contains("queued"));
    }

    #[test]
    fn axolotl_harness_output_dir() {
        let harness = AxolotlHarness;
        let path = harness.output_dir("job-123");
        assert!(path.to_string_lossy().contains("job-123"));
    }

    /// expect: The Capabilities Researcher job renders every operator-selected Axolotl setting.
    /// [P2] Motivating: User Sovereignty — the submitted job must match the declared training config.
    /// pre: A training job contains the canonical EVA LoRA parameters.
    /// post: The rendered YAML preserves those parameters without silent defaults.
    /// [P4] Constraining: Clear Boundaries — only explicit parameters cross the provider boundary.
    #[test]
    fn axolotl_harness_renders_capabilities_researcher_config() {
        let mut params = TrainingParams {
            num_epochs: 3,
            batch_size: 1,
            learning_rate: 1e-4,
            ..TrainingParams::default()
        };
        params.lora.r = 32;
        params.lora.alpha = 64;
        params.lora.init_lora_weights = Some(types::LoraInit::Eva);
        params.optimization.gradient_accumulation_steps = 16;
        params.optimization.lr_scheduler = Some("cosine".to_string());
        params.optimization.warmup_steps = Some(100);
        params.sequence.sequence_len = Some(4096);
        params.advanced.bf16 = true;
        params.advanced.eval_split_ratio = Some(0.0012);

        let mut job = TrainingJob::new(
            std::path::PathBuf::from("/tmp/train_chat_full.jsonl"),
            "unsloth/Qwen3.6-27B".to_string(),
            params,
            TrainingHostId::Runpod,
            TrainingHarnessId::Axolotl,
        );
        job.artifacts = Some(crate::huggingface::TrainingArtifacts {
            dataset: crate::huggingface::TrainingArtifact {
                repository: "mdz-axo/capabilities-researcher-qa".to_string(),
                revision: "main".to_string(),
                path: "train_chat_full.jsonl".to_string(),
                sha256: String::new(),
            },
            model_repository: "mdz-axo/capabilities-researcher-v3-eva".to_string(),
            completion_manifest_path: "/workspace/completion.json".to_string(),
        });

        let yaml = AxolotlHarness.render_config(&job).expect("render config");

        for expected in [
            "peft_init_lora_weights: eva",
            "num_epochs: 3",
            "micro_batch_size: 1",
            "gradient_accumulation_steps: 16",
            "path: mdz-axo/capabilities-researcher-qa",
            "data_files: train_chat_full.jsonl",
            "optim: adamw_8bit",
            "eval_batch_size: 1",
            "val_set_size: 0.0012",
            "early_stopping_patience: 25",
            "liger_kernel: true",
            "flash_attention: false",
            "cut_cross_entropy: true",
            "trust_remote_code: true",
            "strict: false",
        ] {
            assert!(yaml.contains(expected), "missing `{expected}` in:\n{yaml}");
        }
    }

    #[test]
    fn host_config_default() {
        let config = TrainingHostConfig::default();
        assert_eq!(config.host, TrainingHostId::Runpod);
    }

    /// expect: The TRL harness renders a valid SFTTrainer Python script from canonical TrainingParams.
    /// [P2] Motivating: User Sovereignty — the operator can choose TRL as an alternative to Axolotl.
    /// pre: A training job contains canonical EVA LoRA parameters with harness=Trl.
    /// post: The rendered Python script contains SFTTrainer, SFTConfig, LoraConfig, and all params.
    /// [P4] Constraining: Clear Boundaries — only explicit parameters cross the provider boundary.
    #[test]
    fn trl_harness_renders_sft_script() {
        let mut params = TrainingParams {
            num_epochs: 3,
            batch_size: 1,
            learning_rate: 1e-4,
            ..TrainingParams::default()
        };
        params.lora.r = 32;
        params.lora.alpha = 64;
        params.lora.init_lora_weights = Some(types::LoraInit::Eva);
        params.optimization.gradient_accumulation_steps = 16;
        params.optimization.lr_scheduler = Some("cosine".to_string());
        params.optimization.warmup_steps = Some(100);
        params.sequence.sequence_len = Some(4096);
        params.advanced.bf16 = true;
        params.harness = Some(TrainingHarnessId::Trl);
        params.trl_trainer = Some(types::TrlTrainer::Sft);

        let mut job = TrainingJob::new(
            std::path::PathBuf::from("/tmp/train_chat_full.jsonl"),
            "unsloth/Qwen3.6-27B".to_string(),
            params,
            TrainingHostId::Runpod,
            TrainingHarnessId::Trl,
        );
        job.artifacts = Some(crate::huggingface::TrainingArtifacts {
            dataset: crate::huggingface::TrainingArtifact {
                repository: "mdz-axo/capabilities-researcher-qa".to_string(),
                revision: "main".to_string(),
                path: "train_chat_full.jsonl".to_string(),
                sha256: String::new(),
            },
            model_repository: "mdz-axo/capabilities-researcher-v3-eva".to_string(),
            completion_manifest_path: "/workspace/completion.json".to_string(),
        });

        let script = crate::providers::TrlHarness
            .render_config(&job)
            .expect("render TRL SFT script");

        for expected in [
            "from trl import SFTConfig, SFTTrainer",
            "from peft import LoraConfig",
            "base_model = \"unsloth/Qwen3.6-27B\"",
            "r=32",
            "lora_alpha=64",
            "init_lora_weights=\"eva\"",
            "num_train_epochs=3",
            "per_device_train_batch_size=1",
            "gradient_accumulation_steps=16",
            "learning_rate=",
            "lr_scheduler_type=\"cosine\"",
            "warmup_steps=100",
            "max_length=4096",
            "bf16=True",
            "gradient_checkpointing=True",
            "load_dataset(\"mdz-axo/capabilities-researcher-qa\"",
            "data_files=\"train_chat_full.jsonl\"",
            "trainer = SFTTrainer(",
            "trainer.train()",
            "trainer.save_model",
        ] {
            assert!(
                script.contains(expected),
                "missing `{expected}` in:\n{script}"
            );
        }
    }

    /// expect: The TRL harness renders a valid DPOTrainer Python script from canonical TrainingParams.
    /// [P2] Motivating: User Sovereignty — the operator can choose DPO for preference optimization.
    /// pre: A training job contains canonical LoRA parameters with harness=Trl, trl_trainer=Dpo.
    /// post: The rendered Python script contains DPOTrainer, DPOConfig, LoraConfig, and all params.
    /// [P4] Constraining: Clear Boundaries — only explicit parameters cross the provider boundary.
    #[test]
    fn trl_harness_renders_dpo_script() {
        let script = render_preference_script_for(types::TrlTrainer::Dpo);
        for expected in [
            "from trl import DPOConfig, DPOTrainer",
            "from peft import LoraConfig",
            "trainer = DPOTrainer(",
            "config = DPOConfig(",
            "trainer_name = \"dpo\"",
            "trainer.train()",
            "trainer.save_model",
        ] {
            assert!(
                script.contains(expected),
                "missing `{expected}` in:\n{script}"
            );
        }
    }

    /// expect: The TRL harness renders a valid KTOTrainer Python script.
    #[test]
    fn trl_harness_renders_kto_script() {
        let script = render_preference_script_for(types::TrlTrainer::Kto);
        for expected in [
            "from trl import KTOConfig, KTOTrainer",
            "trainer = KTOTrainer(",
            "config = KTOConfig(",
            "trainer_name = \"kto\"",
        ] {
            assert!(
                script.contains(expected),
                "missing `{expected}` in:\n{script}"
            );
        }
    }

    /// expect: The TRL harness renders a valid ORPOTrainer Python script.
    #[test]
    fn trl_harness_renders_orpo_script() {
        let script = render_preference_script_for(types::TrlTrainer::Orpo);
        for expected in [
            "from trl import ORPOConfig, ORPOTrainer",
            "trainer = ORPOTrainer(",
            "config = ORPOConfig(",
            "trainer_name = \"orpo\"",
        ] {
            assert!(
                script.contains(expected),
                "missing `{expected}` in:\n{script}"
            );
        }
    }

    /// expect: The TRL harness renders a valid RewardTrainer Python script.
    #[test]
    fn trl_harness_renders_reward_script() {
        let script = render_preference_script_for(types::TrlTrainer::Reward);
        for expected in [
            "from trl import RewardConfig, RewardTrainer",
            "trainer = RewardTrainer(",
            "config = RewardConfig(",
            "trainer_name = \"reward\"",
        ] {
            assert!(
                script.contains(expected),
                "missing `{expected}` in:\n{script}"
            );
        }
    }

    /// expect: The Ludwig harness renders valid Ludwig YAML from canonical TrainingParams.
    /// [P2] Motivating: User Sovereignty — the operator can choose Ludwig as a third harness.
    /// pre: A training job contains canonical EVA LoRA parameters with harness=Ludwig.
    /// post: The rendered YAML contains model_type: llm, base_model, adapter: lora,
    ///       trainer: finetune, and all canonical params without silent defaults.
    /// [P4] Constraining: Clear Boundaries — only explicit parameters cross the provider boundary.
    #[test]
    fn ludwig_harness_renders_sft_yaml() {
        let mut params = TrainingParams {
            num_epochs: 3,
            batch_size: 1,
            learning_rate: 1e-4,
            ..TrainingParams::default()
        };
        params.lora.r = 32;
        params.lora.alpha = 64;
        params.lora.init_lora_weights = Some(types::LoraInit::Eva);
        params.optimization.gradient_accumulation_steps = 16;
        params.optimization.lr_scheduler = Some("cosine".to_string());
        params.optimization.warmup_steps = Some(100);
        params.sequence.sequence_len = Some(4096);
        params.advanced.bf16 = true;
        params.advanced.eval_split_ratio = Some(0.0012);
        params.quantization.load_in_4bit = true;
        params.harness = Some(TrainingHarnessId::Ludwig);

        let mut job = TrainingJob::new(
            std::path::PathBuf::from("/tmp/train_chat_full.jsonl"),
            "unsloth/Qwen3.6-27B".to_string(),
            params,
            TrainingHostId::Runpod,
            TrainingHarnessId::Ludwig,
        );
        job.artifacts = Some(crate::huggingface::TrainingArtifacts {
            dataset: crate::huggingface::TrainingArtifact {
                repository: "mdz-axo/capabilities-researcher-qa".to_string(),
                revision: "main".to_string(),
                path: "train_chat_full.jsonl".to_string(),
                sha256: String::new(),
            },
            model_repository: "mdz-axo/capabilities-researcher-v3-eva".to_string(),
            completion_manifest_path: "/workspace/completion.json".to_string(),
        });

        let yaml = crate::providers::LudwigHarness
            .render_config(&job)
            .expect("render Ludwig SFT YAML");

        for expected in [
            "model_type: llm",
            "base_model: unsloth/Qwen3.6-27B",
            "adapter:",
            "type: lora",
            "r: 32",
            "alpha: 64",
            "init_weights: eva",
            "quantization:",
            "bits: 4",
            "input_features:",
            "name: prompt",
            "output_features:",
            "name: output",
            "trainer:",
            "type: finetune",
            "epochs: 3",
            "batch_size: 1",
            "gradient_accumulation_steps: 16",
            "learning_rate:",
            "warmup_steps: 100",
            "decay: cosine",
            "max_sequence_length: 4096",
            "validation_split: 0.0012",
            "output_dir:",
        ] {
            assert!(yaml.contains(expected), "missing `{expected}` in:\n{yaml}");
        }
    }

    /// expect: The Ludwig harness output_dir and completion_marker match the
    /// harness-agnostic RunPod pod contract (/workspace/outputs/{job_id}).
    #[test]
    fn ludwig_harness_output_dir_and_completion_marker() {
        let harness = crate::providers::LudwigHarness;
        let path = harness.output_dir("job-ludwig-123");
        assert!(path.to_string_lossy().contains("job-ludwig-123"));
        assert!(path.to_string_lossy().contains("/workspace/outputs"));
        let marker = harness.completion_marker("job-ludwig-123");
        assert!(
            marker
                .to_string_lossy()
                .ends_with("adapter_model.safetensors"),
            "completion marker should be adapter_model.safetensors, got: {}",
            marker.display()
        );
        assert_eq!(harness.harness_id(), TrainingHarnessId::Ludwig);
    }

    /// Helper: render a TRL preference script for the given trainer.
    /// Builds a canonical TrainingJob with the specified trainer and renders it.
    fn render_preference_script_for(trainer: types::TrlTrainer) -> String {
        let mut params = TrainingParams {
            num_epochs: 3,
            batch_size: 1,
            learning_rate: 5e-6, // DPO/KTO/ORPO use lower LR than SFT
            ..TrainingParams::default()
        };
        params.lora.r = 32;
        params.lora.alpha = 64;
        params.lora.init_lora_weights = Some(types::LoraInit::Eva);
        params.optimization.gradient_accumulation_steps = 16;
        params.sequence.sequence_len = Some(4096);
        params.advanced.bf16 = true;
        params.harness = Some(TrainingHarnessId::Trl);
        params.trl_trainer = Some(trainer);

        let mut job = TrainingJob::new(
            std::path::PathBuf::from("/tmp/preference_data.jsonl"),
            "unsloth/Qwen3.6-27B".to_string(),
            params,
            TrainingHostId::Runpod,
            TrainingHarnessId::Trl,
        );
        job.artifacts = Some(crate::huggingface::TrainingArtifacts {
            dataset: crate::huggingface::TrainingArtifact {
                repository: "mdz-axo/preference-data".to_string(),
                revision: "main".to_string(),
                path: "train.jsonl".to_string(),
                sha256: String::new(),
            },
            model_repository: "mdz-axo/preference-adapter".to_string(),
            completion_manifest_path: "/workspace/completion.json".to_string(),
        });

        crate::providers::TrlHarness
            .render_config(&job)
            .expect("render TRL preference script")
    }
}
