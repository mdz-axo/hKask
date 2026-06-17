//! hKask MCP Training — Model training data ingestion and fine-tuning server.
//!
//! Exposes a full training surface:
//! - `training_ingest_qa` — Ingest QA pairs for model fine-tuning
//! - `training_ingest_dataset` — Ingest a raw dataset into the normalized cache
//! - `training_assemble_dataset` — Assemble stored QA pairs into a ChatML JSONL dataset file
//! - `training_generate_traces` — Generate type-specialized decomposition traces from skill documents
//! - `training_generate_cot` — Generate chain-of-thought training traces
//! - `training_submit` — Submit a training job via harness-aware host dispatch
//! - `training_status` — Query training job status (auto-registers on completion + A/B comparison)
//! - `training_cancel` — Cancel a running job
//! - `training_evaluate` — Evaluate a trained adapter against a test dataset
//! - `training_sweep` — Parameter grid search across hyperparameters
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
//! Architecture:
//!   SKILL.md → trace generation (type-specialized) → DatasetPipeline → TrainingJob
//!          → TrainingHost (harness-aware dispatch: Axolotl/Unsloth on Together/Runpod/Baseten)
//!          → LoRAAdapter (stored in hkask-storage)
//!          → continuous loop: evaluate → curate (failure-gated) → retrain (A/B baseline) → version++

pub mod adapters;
pub mod dataset;
pub mod huggingface;
pub mod providers;

pub use adapters::{
    AdapterMetrics, AdapterStore, AdapterStoreError, InMemoryAdapterStore, JobStore, LoRAAdapter,
    SqliteAdapterStore, StoredJob,
};
