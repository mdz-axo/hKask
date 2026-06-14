//! hKask MCP Training — Model training data ingestion and fine-tuning server.
//!
//! Exposes a full training surface:
//! - `training_ingest_qa` — Ingest QA pairs for model fine-tuning
//! - `training_submit` — Submit a training job via pluggable provider
//! - `training_status` — Query training job status
//! - `training_cancel` — Cancel a running job
//! - `training_list_adapters` — List completed LoRA adapters
//! - `training_delete_adapter` — Remove a LoRA adapter
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
