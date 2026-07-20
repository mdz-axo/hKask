//! hkask-llama-finetune — Rust orchestrator for llama.cpp LoRA training.
//!
//! Wraps llama.cpp's `finetune` binary. Downloads model + dataset from
//! HuggingFace via curl, runs training, uploads adapter.
//! No Python. The only external binary is llama.cpp's `finetune` (C++).

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(name = "hkask-llama-finetune")]
#[command(about = "Rust orchestrator for llama.cpp LoRA training")]
struct Args {
    /// HuggingFace base model repo (GGUF). e.g. "Qwen/Qwen3-1.7B-GGUF"
    #[arg(long, env = "HKASK_BASE_MODEL")]
    base_model: String,

    /// Specific GGUF file to download. e.g. "qwen3-1.7b-f16.gguf"
    #[arg(long, env = "HKASK_MODEL_FILE")]
    model_file: Option<String>,

    /// Path to training data file (local, already present).
    #[arg(long, env = "HKASK_TRAIN_DATA")]
    train_data: Option<PathBuf>,

    /// HuggingFace dataset repo for training data.
    #[arg(long, env = "HKASK_HF_DATASET_REPOSITORY")]
    dataset_repo: Option<String>,

    /// Dataset file within the repo.
    #[arg(long, env = "HKASK_HF_DATASET_PATH", default_value = "train.jsonl")]
    dataset_file: String,

    /// LoRA rank.
    #[arg(long, env = "HKASK_LORA_R", default_value_t = 16)]
    lora_r: u32,

    /// LoRA alpha.
    #[arg(long, env = "HKASK_LORA_ALPHA", default_value_t = 32)]
    lora_alpha: u32,

    /// Number of training epochs.
    #[arg(long, env = "HKASK_NUM_EPOCHS", default_value_t = 3)]
    epochs: u32,

    /// Learning rate.
    #[arg(long, env = "HKASK_LEARNING_RATE", default_value = "1e-4")]
    learning_rate: String,

    /// Batch size.
    #[arg(long, env = "HKASK_BATCH_SIZE", default_value_t = 1)]
    batch_size: u32,

    /// Gradient accumulation steps.
    #[arg(long, env = "HKASK_GRAD_ACCUM", default_value_t = 16)]
    grad_accum: u32,

    /// Context length.
    #[arg(long, env = "HKASK_SEQ_LEN", default_value_t = 4096)]
    seq_len: u32,

    /// Output directory for LoRA adapter.
    #[arg(long, env = "HKASK_OUTPUT_DIR", default_value = "/workspace/outputs")]
    output_dir: PathBuf,

    /// HuggingFace adapter upload repo (optional).
    #[arg(long, env = "HKASK_HF_MODEL_REPOSITORY")]
    upload_repo: Option<String>,

    /// Path to the llama.cpp finetune binary.
    #[arg(
        long,
        env = "HKASK_FINETUNE_BIN",
        default_value = "/usr/local/bin/llama-finetune"
    )]
    finetune_bin: PathBuf,

    /// Number of GPU layers to offload (-1 = all).
    #[arg(long, env = "HKASK_NGPU_LAYERS", default_value_t = 99)]
    ngpu: i32,

    /// Save checkpoint every N iterations.
    #[arg(long, env = "HKASK_SAVE_EVERY", default_value_t = 100)]
    save_every: u32,

    /// Warmup steps.
    #[arg(long, env = "HKASK_WARMUP_STEPS", default_value_t = 100)]
    warmup_steps: u32,

    /// Enable gradient checkpointing.
    #[arg(long, env = "HKASK_GRADIENT_CHECKPOINTING", default_value_t = true)]
    grad_checkpointing: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    eprintln!("=== hKask llama.cpp LoRA Training ===");
    eprintln!("  base_model:  {}", args.base_model);
    eprintln!("  lora_r:      {}", args.lora_r);
    eprintln!("  lora_alpha:  {}", args.lora_alpha);
    eprintln!("  epochs:      {}", args.epochs);
    eprintln!("  seq_len:     {}", args.seq_len);
    eprintln!("  output_dir:  {}", args.output_dir.display());

    // 1. Download base model (GGUF) from HuggingFace
    let model_path = download_model(&args)?;
    eprintln!("  model_path:  {}", model_path.display());

    // 2. Download or locate training data
    let train_data_path = download_dataset(&args)?;
    eprintln!("  train_data:  {}", train_data_path.display());

    // 3. Prepare output directory
    std::fs::create_dir_all(&args.output_dir).context("failed to create output directory")?;

    // 4. Run llama.cpp finetune
    let lora_output = args.output_dir.join("lora-LATEST.gguf");
    run_finetune(&args, &model_path, &train_data_path, &lora_output)?;

    // 5. Upload adapter to HuggingFace (if configured)
    if let Some(repo) = &args.upload_repo {
        upload_adapter(repo, &lora_output)?;
    }

    // 6. Write completion manifest
    let manifest_path = args.output_dir.join("completion.json");
    let manifest = serde_json::json!({
        "status": "completed",
        "lora_path": lora_output.display().to_string(),
        "base_model": args.base_model,
        "lora_r": args.lora_r,
        "lora_alpha": args.lora_alpha,
        "epochs": args.epochs,
    });
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)
        .context("failed to write completion manifest")?;
    eprintln!("=== Training complete ===");
    eprintln!("  adapter: {}", lora_output.display());
    eprintln!("  manifest: {}", manifest_path.display());

    Ok(())
}

/// Download a GGUF model file from HuggingFace using curl.
/// HF URL format: https://huggingface.co/{repo}/resolve/main/{filename}
fn download_model(args: &Args) -> Result<PathBuf> {
    let cache_dir = PathBuf::from(
        std::env::var("HF_HOME").unwrap_or_else(|_| "/workspace/.cache/huggingface".to_string()),
    );
    std::fs::create_dir_all(&cache_dir).ok();

    let model_file = args.model_file.clone().unwrap_or_else(|| {
        // Default: try common GGUF naming patterns
        // The user should specify --model-file, but we provide a fallback
        eprintln!("  WARNING: --model-file not specified, trying to auto-detect");
        "model.gguf".to_string()
    });

    let dest = cache_dir.join(&model_file);
    if dest.exists() {
        eprintln!("  model already cached: {}", dest.display());
        return Ok(dest);
    }

    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        args.base_model, model_file
    );
    eprintln!("  downloading from: {}", url);

    let status = Command::new("curl")
        .arg("-L")
        .arg("-o")
        .arg(&dest)
        .arg("-H")
        .arg(format!(
            "Authorization: Bearer {}",
            std::env::var("HF_TOKEN").unwrap_or_default()
        ))
        .arg(&url)
        .status()
        .context("failed to run curl")?;

    if !status.success() {
        anyhow::bail!("curl download failed with status {}", status);
    }

    Ok(dest)
}

/// Download dataset from HuggingFace or use local path.
fn download_dataset(args: &Args) -> Result<PathBuf> {
    if let Some(path) = &args.train_data {
        if path.exists() {
            return Ok(path.clone());
        }
    }

    let repo = args
        .dataset_repo
        .as_ref()
        .context("no train_data path and no dataset_repo set")?;

    let cache_dir = PathBuf::from(
        std::env::var("HF_HOME").unwrap_or_else(|_| "/workspace/.cache/huggingface".to_string()),
    );
    let dest = cache_dir.join(&args.dataset_file);

    if dest.exists() {
        eprintln!("  dataset already cached: {}", dest.display());
        return Ok(dest);
    }

    let url = format!(
        "https://huggingface.co/datasets/{}/resolve/main/{}",
        repo, args.dataset_file
    );
    eprintln!("  downloading from: {}", url);

    let status = Command::new("curl")
        .arg("-L")
        .arg("-o")
        .arg(&dest)
        .arg("-H")
        .arg(format!(
            "Authorization: Bearer {}",
            std::env::var("HF_TOKEN").unwrap_or_default()
        ))
        .arg(&url)
        .status()
        .context("failed to run curl")?;

    if !status.success() {
        anyhow::bail!("curl download failed with status {}", status);
    }

    Ok(dest)
}

/// Run llama.cpp finetune with LoRA.
fn run_finetune(
    args: &Args,
    model_path: &std::path::Path,
    train_data: &std::path::Path,
    lora_output: &std::path::Path,
) -> Result<()> {
    let mut cmd = Command::new(&args.finetune_bin);

    cmd.arg("--model-base")
        .arg(model_path)
        .arg("--train-data")
        .arg(train_data)
        .arg("--lora-out")
        .arg(lora_output)
        .arg("--threads")
        .arg(num_cpus())
        .arg("--batch-size")
        .arg(args.batch_size.to_string())
        .arg("--grad-accum")
        .arg(args.grad_accum.to_string())
        .arg("--epochs")
        .arg(args.epochs.to_string())
        .arg("--learning-rate")
        .arg(&args.learning_rate)
        .arg("--warmup")
        .arg(args.warmup_steps.to_string())
        .arg("--save-every")
        .arg(args.save_every.to_string())
        .arg("--ctx")
        .arg(args.seq_len.to_string())
        .arg("--lora-r")
        .arg(args.lora_r.to_string())
        .arg("--lora-alpha")
        .arg(args.lora_alpha.to_string())
        .arg("--n-gpu-layers")
        .arg(args.ngpu.to_string());

    if !args.grad_checkpointing {
        cmd.arg("--no-checkpointing");
    }

    let checkpoint_in = args.output_dir.join("checkpoint-LATEST.gguf");
    let checkpoint_out = args.output_dir.join("checkpoint-ITERATION.gguf");
    cmd.arg("--checkpoint-in")
        .arg(&checkpoint_in)
        .arg("--checkpoint-out")
        .arg(&checkpoint_out);

    eprintln!("  running: {:?}", cmd);

    let status = cmd.status().context("failed to execute llama-finetune")?;

    if !status.success() {
        anyhow::bail!("llama-finetune exited with status {}", status);
    }

    Ok(())
}

/// Upload LoRA adapter to HuggingFace using huggingface-cli.
fn upload_adapter(repo: &str, lora_path: &std::path::Path) -> Result<()> {
    eprintln!("  uploading adapter to {} ...", repo);

    let _ = Command::new("huggingface-cli")
        .arg("repo")
        .arg("create")
        .arg(repo)
        .arg("--type")
        .arg("model")
        .arg("--exist-ok")
        .status();

    let status = Command::new("huggingface-cli")
        .arg("upload")
        .arg(repo)
        .arg(lora_path)
        .status()
        .context("failed to upload adapter")?;

    if !status.success() {
        anyhow::bail!("huggingface-cli upload exited with status {}", status);
    }

    eprintln!("  adapter uploaded to {}", repo);
    Ok(())
}

fn num_cpus() -> String {
    std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "4".to_string())
}
