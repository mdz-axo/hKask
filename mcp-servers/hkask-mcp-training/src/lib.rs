//! hKask MCP Training — Model training data ingestion, fine-tuning, deployment, and continuous improvement server.
//!
//! Exposes a full training + deployment surface:
//! - `training_ingest_qa` — Ingest QA pairs for expertise training
//! - `training_ingest_dataset` — Ingest a raw dataset into the normalized cache
//! - `training_assemble_dataset` — Assemble stored QA pairs into a ChatML JSONL dataset file
//! - `training_generate_traces` — Generate type-specialized decomposition traces from skill documents
//! - `training_generate_cot` — Generate chain-of-thought training traces
//! - `training_submit` — Submit a training job via harness-aware host dispatch
//! - `training_status` — Query training job status (auto-registers on completion + A/B comparison)
//! - `training_cancel` — Cancel a running job
//! - `training_evaluate` — Evaluate a trained adapter against a test dataset
//! - `training_sweep` — Parameter grid search across hyperparameters
//! - `training_deploy` — Deploy a trained adapter to a cloud inference endpoint (Together/Baseten/Runpod) with cost estimates and adapter validation
//! - `training_deployment_status` — Check deployment provisioning status, endpoint URL, and accrued cost
//! - `training_teardown` — Tear down a deployed adapter endpoint, release GPU resources
//! - `training_register_adapter` — Register a completed adapter in persistent storage
//! - `training_list_adapters` — List completed LoRA adapters
//! - `training_delete_adapter` — Remove a LoRA adapter
//! - `training_recommend_model` — Recommend a base model for fine-tuning
//! - `training_record_invocation` — Record an adapter invocation for continuous training
//! - `training_curate_feedback` — Curate feedback with failure categorization and quality gating
//! - `training_retrain` — Retrain an adapter with A/B baseline (closes the continuous loop)
//! - `training_merge_adapters` — Merge multiple LoRA adapters
//! - `training_compare_adapters` — Compare output quality across adapters
//!
//! Training modes:
//!   Expertise — factual domain knowledge from QA pairs ("what to know")
//!   Skill — procedural decomposition from SKILL.md traces ("how to think")
//!   Contrastive — judgment training from correct/incorrect trace pairs ("what to prefer")
//!
//! Architecture:
//!   SKILL.md → trace generation (type-specialized) → DatasetPipeline → TrainingJob
//!          → TrainingHost (harness-aware dispatch: Axolotl/Unsloth on Together/Runpod/Baseten)
//!          → LoRAAdapter (stored in hkask-storage)
//!          → training_deploy → TrainedInferenceEndpoint (Together/Baseten/Runpod)
//!          → continuous loop: evaluate → curate (failure-gated) → retrain (A/B baseline) → version++

pub mod adapters;
pub mod dataset;
pub mod endpoint;
pub mod expertise;
pub mod huggingface;
pub mod providers;

pub use adapters::{
    AdapterMetrics, AdapterStore, AdapterStoreError, InMemoryAdapterStore, JobStore, LoRAAdapter,
    SqliteAdapterStore, StoredJob,
};
