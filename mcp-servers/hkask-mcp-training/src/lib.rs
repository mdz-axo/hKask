//! hKask MCP Training — Model training data ingestion and fine-tuning server.
//!
//! Exposes a full training surface:
//! - `training_ingest_qa` — Ingest QA pairs for model fine-tuning
//! - `training_submit` — Submit a training job via pluggable provider
//! - `training_status` — Query training job status
//! - `training_cancel` — Cancel a running job
//! - `training_list_adapters` — List completed LoRA adapters
//! - `training_delete_adapter` — Remove a LoRA adapter
//! - `training_assemble_dataset` — Assemble stored QA pairs into a ChatML JSONL dataset file
//! - `training_generate_traces` — Generate decomposition traces from skill documents
//! - `training_evaluate` — Evaluate a trained adapter against a test dataset
//! - `training_register_adapter` — Register a completed adapter in persistent storage
//! - `training_recommend_model` — Recommend a base model for fine-tuning
//! - `training_record_invocation` — Record an adapter invocation for continuous training
//! - `training_curate_feedback` — Curate feedback from stored QA pairs for continuous skills training
//! - `training_retrain` — Retrain an adapter with curated feedback (closes the continuous loop)
//!
//! Architecture:
//!   Dataset → DatasetPipeline (ingest/normalize/validate/cache)
//!          → TrainingJob (canonical representation)
//!          → TrainingProvider (axolotl/unsloth adapter)
//!          → ProviderBackend (local CLI or cloud dispatch)
//!          → LoRAAdapter (stored in hkask-storage)

pub mod adapters;
pub mod dataset;
pub mod providers;

pub use adapters::{
    AdapterMetrics, AdapterStore, AdapterStoreError, InMemoryAdapterStore, JobStore, LoRAAdapter,
    SqliteAdapterStore, StoredJob,
};
