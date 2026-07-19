//! Thinking Machines Tinker training host.
//!
//! Tinker is a training API where the training loop runs as a Python script on
//! the host CPU, while Tinker's service handles distributed GPU compute. This
//! contrasts with Runpod (remote pod) and Together AI (REST submission): the
//! "host" here is the local machine, and compute is dispatched to Tinker via
//! the SDK's `ServiceClient`.
//!
//! Architecture:
//!   1. `submit(job)` renders the Python training script via `TinkerHarness`,
//!      writes it to a temp file, and launches it as a `python3` subprocess.
//!      Stdout/stderr are teed to a per-job log file under the output dir.
//!      The returned job ID is a `Tinker:<pid>` string (the prefix
//!      distinguishes Tinker jobs from other hosts' IDs).
//!   2. `status(job_id)` checks whether the subprocess is still running and
//!      parses the log file for progress (step, loss, eval_loss). When the
//!      process exits cleanly and the completion marker exists, the job is
//!      `Completed`; a non-zero exit with no marker yields `Failed`.
//!   3. `cancel(job_id)` sends SIGTERM to the subprocess.
//!
//! Environment variables:
//! - `TINKER_API_KEY` — Thinking Machines Tinker API key (read by the Python
//!   SDK inside the subprocess; the Rust host only checks it is non-empty so
//!   submission fails fast with a clear error instead of a Python traceback).
//! - `HKASK_PYTHON_PATH` — Path to the `python3` interpreter that has the
//!   `tinker` package installed (default: `python3` from `PATH`).
//!
//! The Python training script is an ad-hoc generated artifact (per job, not
//! committed), like the Axolotl YAML rendered for Runpod.

use crate::providers::harness::HarnessAdapter;
use crate::providers::types::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

#[cfg(unix)]
/// Wrapper around the C `kill` syscall (avoids a `libc` crate dependency).
unsafe fn libc_kill(pid: i32, sig: i32) -> i32 {
    unsafe { kill(pid, sig) }
}
use std::sync::{Arc, Mutex};

/// Prefix that distinguishes a Tinker job ID (which carries a PID) from other
/// hosts' provider job IDs. The prefix is stripped before parsing the PID.
const TINKER_JOB_PREFIX: &str = "Tinker:";

/// Tinker training host — runs the training loop as a host-side Python
/// subprocess that dispatches GPU compute to Tinker's service.
///
/// Unlike `RunpodHost` and `TogetherHost`, the Tinker host actually uses its
/// injected harness (`TinkerHarness`) to render the Python training script.
pub struct TinkerHost {
    /// Path to the `python3` interpreter with the `tinker` package installed.
    python_path: String,
    /// Harness used to render the Python training script.
    harness: Box<dyn HarnessAdapter>,
    /// job_id (as returned by `submit`) → subprocess PID.
    jobs: Arc<Mutex<HashMap<String, u32>>>,
}

impl TinkerHost {
    /// Construct a new Tinker host.
    ///
    /// `python_path` should point at a `python3` interpreter that has the
    /// `tinker` SDK installed. If empty, `python3` from `PATH` is used.
    pub fn new(python_path: String, harness: Box<dyn HarnessAdapter>) -> Self {
        Self {
            python_path: if python_path.is_empty() {
                "python3".to_string()
            } else {
                python_path
            },
            harness,
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Resolve the python interpreter to launch.
    fn python(&self) -> &str {
        &self.python_path
    }

    /// Parse a Tinker job ID into the subprocess PID.
    fn parse_pid(job_id: &str) -> Option<u32> {
        job_id
            .strip_prefix(TINKER_JOB_PREFIX)
            .and_then(|s| s.parse::<u32>().ok())
    }

    /// Check whether a PID is currently running on this host.
    fn pid_running(pid: u32) -> bool {
        #[cfg(unix)]
        {
            // SAFETY: kill(pid, 0) is a standard existence probe — signal 0
            // performs no action, just checks permission/existence.
            let rc = unsafe { libc_kill(pid as i32, 0) };
            rc == 0
        }
        #[cfg(not(unix))]
        {
            let _ = pid;
            false
        }
    }

    /// Send SIGTERM to a PID. Returns `true` if the signal was delivered.
    fn terminate_pid(pid: u32) -> bool {
        #[cfg(unix)]
        {
            // SAFETY: kill(pid, SIGTERM) is a standard termination signal.
            let rc = unsafe { libc_kill(pid as i32, 15) };
            rc == 0
        }
        #[cfg(not(unix))]
        {
            let _ = pid;
            false
        }
    }

    /// Path to the per-job log file used for progress parsing.
    fn log_path(&self, job_id: &str) -> PathBuf {
        self.harness.output_dir(job_id).join("tinker.log")
    }

    /// Path to the completion marker written by the Python script on success.
    fn completion_marker_path(&self, job_id: &str) -> PathBuf {
        self.harness.completion_marker(job_id)
    }

    /// Parse the tail of the log file for the last reported step/loss/eval_loss.
    ///
    /// Lines are of the form `[tinker] step=N epoch=M loss=0.123456` and
    /// `[tinker] step=N eval_loss=0.123456`. Returns `None` if the log is
    /// missing or contains no progress lines.
    fn parse_log_progress(path: &std::path::Path) -> Option<ProgressLine> {
        let content = std::fs::read_to_string(path).ok()?;
        content.lines().rev().find_map(ProgressLine::parse)
    }
}

/// One parsed progress line from the Tinker Python script's log.
#[derive(Debug, Clone, PartialEq)]
struct ProgressLine {
    step: u64,
    loss: Option<f64>,
    eval_loss: Option<f64>,
}

impl ProgressLine {
    /// Parse a single `[tinker] step=N ...` log line.
    fn parse(line: &str) -> Option<Self> {
        let needle = "step=";
        let idx = line.find(needle)?;
        let rest = &line[idx + needle.len()..];
        let step_end = rest.find(' ').unwrap_or(rest.len());
        let step: u64 = rest[..step_end].parse().ok()?;

        let mut loss = None;
        let mut eval_loss = None;
        if let Some(li) = line.find("eval_loss=") {
            let tail = &line[li + "eval_loss=".len()..];
            let end = tail.find(' ').unwrap_or(tail.len());
            if let Ok(v) = tail[..end].parse::<f64>() {
                eval_loss = Some(v);
            }
        }
        // Only set `loss` when this is a training-step line (not an eval line).
        if eval_loss.is_none()
            && let Some(li) = line.find("loss=")
        {
            let tail = &line[li + "loss=".len()..];
            let end = tail.find(' ').unwrap_or(tail.len());
            if let Ok(v) = tail[..end].parse::<f64>() {
                loss = Some(v);
            }
        }
        Some(Self {
            step,
            loss,
            eval_loss,
        })
    }
}

#[async_trait::async_trait]
impl TrainingHost for TinkerHost {
    async fn submit(&self, job: &TrainingJob) -> Result<String, ProviderError> {
        // Fail fast if the API key is missing — the Python SDK would otherwise
        // emit a traceback that is harder to debug from the host's log file.
        if std::env::var("TINKER_API_KEY")
            .unwrap_or_default()
            .is_empty()
        {
            return Err(ProviderError::Unavailable(
                "Tinker API key not configured (set TINKER_API_KEY)".to_string(),
            ));
        }

        // Render the Python training script via the Tinker harness.
        let script = self.harness.render_config(job)?;

        // Write the script to a temp file.
        let script_path = std::env::temp_dir().join(format!("hkask-tinker-{}.py", job.id));
        std::fs::write(&script_path, &script)
            .map_err(|e| ProviderError::Backend(format!("Failed to write Tinker script: {}", e)))?;

        // Ensure the output directory exists so the subprocess can write logs
        // and the completion marker immediately.
        let output_dir = self.harness.output_dir(&job.id);
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| ProviderError::Backend(format!("Failed to create output dir: {}", e)))?;

        let log_path = self.log_path(&job.id);
        // Truncate any stale log from a prior run with the same job ID.
        let _ = std::fs::File::create(&log_path);

        // Spawn the Python subprocess with hKask env vars injected.
        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| ProviderError::Backend(format!("Failed to open Tinker log: {}", e)))?;

        let stderr = log_file
            .try_clone()
            .map_err(|e| ProviderError::Backend(format!("Failed to clone log handle: {}", e)))?;

        let mut cmd = std::process::Command::new(self.python());
        cmd.arg(&script_path)
            .env("HKASK_JOB_ID", &job.id)
            .env("HKASK_DATASET_PATH", &job.dataset_path)
            .env("HKASK_BASE_MODEL", &job.base_model)
            .env("HKASK_TINKER_OUTPUT_DIR", &output_dir)
            .env("HKASK_TINKER_LOG_PATH", &log_path)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(stderr));

        let child = cmd
            .spawn()
            .map_err(|e| ProviderError::Backend(format!("Failed to spawn python3: {}", e)))?;

        let pid = child.id();
        // Drop the Child handle — we track the PID manually and probe via
        // signal 0. The OS reaps the process when it exits; we don't want to
        // hold a Child that would block on drop waiting for exit.
        drop(child);

        let provider_job_id = format!("{}{}", TINKER_JOB_PREFIX, pid);
        if let Ok(mut map) = self.jobs.lock() {
            map.insert(provider_job_id.clone(), pid);
        }

        tracing::info!(
            target: "hkask.training.job.submit",
            job_id = %job.id,
            tinker_pid = pid,
            host = "tinker",
            harness = ?job.harness,
            script_path = %script_path.display(),
            "Tinker training subprocess launched"
        );

        tracing::info!(
            target: "cns.training.provider.tinker.submit",
            pid = pid,
            script_path = %script_path.display(),
            "Tinker subprocess launched"
        );

        Ok(provider_job_id)
    }

    async fn status(&self, job_id: &str) -> Result<TrainingJobStatus, ProviderError> {
        let pid: Option<u32> = Self::parse_pid(job_id).or_else(|| {
            self.jobs
                .lock()
                .map(|m| m.get(job_id).copied())
                .unwrap_or(None)
        });

        let pid = match pid {
            Some(p) => p,
            None => {
                return Err(ProviderError::JobFailed(format!(
                    "No Tinker subprocess found for job {}",
                    job_id
                )));
            }
        };

        let marker = self.completion_marker_path(job_id);
        let running = Self::pid_running(pid);

        tracing::debug!(
            target: "cns.training.provider.tinker.status",
            pid = pid,
            running = running,
            "Tinker subprocess status"
        );

        if !running {
            // Process has exited. If the completion marker exists, the job
            // completed successfully; otherwise it crashed or was cancelled.
            if marker.exists() {
                return Ok(TrainingJobStatus::Completed);
            }
            // Distinguish cancelled from failed: a cancelled job is removed
            // from the in-memory map by `cancel`. If it's not in the map and
            // there's no marker, treat it as cancelled (explicit cancel).
            let in_map = self
                .jobs
                .lock()
                .map(|m| m.contains_key(job_id))
                .unwrap_or(false);
            if !in_map {
                return Ok(TrainingJobStatus::Cancelled);
            }
            return Ok(TrainingJobStatus::Failed);
        }

        // Process is still running — surface progress via tracing.
        if let Some(progress) = TinkerHost::parse_log_progress(&self.log_path(job_id)) {
            tracing::debug!(
                target: "hkask.training.job.status",
                job_id = %job_id,
                tinker_pid = pid,
                step = progress.step,
                loss = ?progress.loss,
                eval_loss = ?progress.eval_loss,
                "Tinker training in progress"
            );
        }
        Ok(TrainingJobStatus::Running)
    }

    async fn cancel(&self, job_id: &str) -> Result<(), ProviderError> {
        let pid = Self::parse_pid(job_id).or_else(|| {
            self.jobs
                .lock()
                .map(|m| m.get(job_id).copied())
                .unwrap_or(None)
        });

        let pid = match pid {
            Some(p) => p,
            None => {
                tracing::warn!(
                    target: "hkask.training.job.cancel",
                    job_id = %job_id,
                    "No Tinker subprocess found for job"
                );
                return Ok(());
            }
        };

        let delivered = Self::terminate_pid(pid);
        if !delivered {
            tracing::warn!(
                target: "hkask.training.job.cancel",
                job_id = %job_id,
                tinker_pid = pid,
                "SIGTERM delivery failed (process may have already exited)"
            );
        }

        if let Ok(mut map) = self.jobs.lock() {
            map.remove(job_id);
        }

        tracing::info!(
            target: "hkask.training.job.cancel",
            job_id = %job_id,
            tinker_pid = pid,
            host = "tinker",
            "Tinker training subprocess terminated"
        );
        tracing::info!(
            target: "cns.training.provider.tinker.cancel",
            pid = pid,
            "Tinker subprocess cancelled"
        );
        Ok(())
    }

    async fn list_adapters(&self) -> Result<Vec<String>, ProviderError> {
        let map = self
            .jobs
            .lock()
            .map_err(|e| ProviderError::Backend(format!("Lock error: {}", e)))?;
        tracing::info!(
            target: "cns.training.provider.tinker.list",
            count = map.len(),
            "Tinker adapter list"
        );
        Ok(map.keys().cloned().collect())
    }

    async fn delete_adapter(&self, adapter_id: &str) -> Result<(), ProviderError> {
        // Kill any still-running subprocess, then remove the output directory.
        let _ = self.cancel(adapter_id).await;
        let output_dir = self.harness.output_dir(adapter_id);
        if output_dir.exists()
            && let Err(e) = std::fs::remove_dir_all(&output_dir)
        {
            tracing::warn!(
                target: "hkask.training.adapter.deleted",
                adapter_id = %adapter_id,
                error = %e,
                "Failed to remove Tinker output directory"
            );
        }
        tracing::info!(
            target: "hkask.training.adapter.deleted",
            adapter_id = %adapter_id,
            host = "tinker",
            "Tinker adapter artifacts removed"
        );
        Ok(())
    }

    async fn completion_metadata(
        &self,
        job_id: &str,
    ) -> Result<Option<CompletionMetadata>, ProviderError> {
        let marker = self.completion_marker_path(job_id);
        let content = match std::fs::read_to_string(&marker) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        let json: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
            ProviderError::Backend(format!("Invalid Tinker completion marker: {}", e))
        })?;

        let base_model = json["base_model"].as_str().unwrap_or("unknown").to_string();
        let output_name = json["output_name"].as_str().map(|s| s.to_string());

        // Loss / tokens / duration: parse from the log file's last training line.
        let (loss, tokens_processed, training_duration_secs) =
            TinkerHost::parse_log_progress(&self.log_path(job_id))
                .map(|p| (p.loss.map(|v| v as f32), None::<u64>, None::<u64>))
                .unwrap_or((None, None, None));

        Ok(Some(CompletionMetadata {
            base_model,
            output_name,
            loss,
            training_duration_secs,
            tokens_processed,
        }))
    }

    async fn adapter_weight_path(
        &self,
        adapter_id: &str,
    ) -> Result<Option<PathBuf>, ProviderError> {
        // Tinker stores weights server-side and downloads them via the
        // `tinker checkpoint download` CLI. The completion marker is the
        // local artifact; weights are not on disk locally by default.
        let marker = self.completion_marker_path(adapter_id);
        if marker.exists() {
            // Return the output directory — the user can run `tinker checkpoint
            // download` to populate it with weights.
            Ok(Some(self.harness.output_dir(adapter_id)))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_line_parses_step_and_loss() {
        let line = "[tinker] step=42 epoch=1 loss=0.123456";
        let parsed = ProgressLine::parse(line).expect("parses");
        assert_eq!(parsed.step, 42);
        assert_eq!(parsed.loss, Some(0.123456));
        assert_eq!(parsed.eval_loss, None);
    }

    #[test]
    fn progress_line_parses_eval_loss() {
        let line = "[tinker] step=50 eval_loss=0.234567";
        let parsed = ProgressLine::parse(line).expect("parses");
        assert_eq!(parsed.step, 50);
        assert_eq!(parsed.loss, None);
        assert_eq!(parsed.eval_loss, Some(0.234567));
    }

    #[test]
    fn progress_line_rejects_non_step_lines() {
        assert!(ProgressLine::parse("[tinker] service client created").is_none());
        assert!(ProgressLine::parse("not a tinker line at all").is_none());
    }

    #[test]
    fn progress_line_handles_step_without_loss() {
        let line = "[tinker] step=10 epoch=0";
        let parsed = ProgressLine::parse(line).expect("parses");
        assert_eq!(parsed.step, 10);
        assert_eq!(parsed.loss, None);
        assert_eq!(parsed.eval_loss, None);
    }

    #[test]
    fn parse_pid_round_trips_prefix() {
        let job_id = format!("{}{}", TINKER_JOB_PREFIX, 12345);
        assert_eq!(TinkerHost::parse_pid(&job_id), Some(12345));
    }

    #[test]
    fn parse_pid_rejects_non_prefixed() {
        assert_eq!(TinkerHost::parse_pid("12345"), None);
        assert_eq!(TinkerHost::parse_pid("together-abc"), None);
    }

    #[test]
    fn parse_log_progress_finds_latest_step() {
        let tmp_path = tempfile::tempdir().unwrap();
        let log = tmp_path.path().join("tinker.log");
        std::fs::write(
            &log,
            "[tinker] step=10 epoch=0 loss=0.500000\n\
             [tinker] step=20 epoch=0 loss=0.400000\n\
             [tinker] step=30 epoch=1 loss=0.300000\n",
        )
        .unwrap();
        let progress = TinkerHost::parse_log_progress(&log).expect("parses");
        assert_eq!(progress.step, 30);
        assert_eq!(progress.loss, Some(0.3));
    }

    #[test]
    fn parse_log_progress_missing_file_returns_none() {
        let progress =
            TinkerHost::parse_log_progress(std::path::Path::new("/nonexistent/tinker.log"));
        assert!(progress.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn pid_running_detects_current_process() {
        let pid = std::process::id();
        assert!(TinkerHost::pid_running(pid));
    }

    #[cfg(unix)]
    #[test]
    fn pid_running_rejects_nonexistent_pid() {
        let bogus = 0x7FFF_FFFF;
        assert!(!TinkerHost::pid_running(bogus));
    }
}
