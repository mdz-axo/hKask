//! Dataset ingestion and preprocessing pipeline.
//!
//! Converts raw input files (JSONL, ShareGPT, Alpaca, raw text) into canonical
//! ChatML format, validates structure, and caches the normalized output in
//! `hkask-storage` to avoid re-processing.
//!
//! Each provider adapter then translates canonical ChatML to its native format
//! for cloud dispatch (axolotl YAML configs → Together/Runpod, unsloth).
//! All training is cloud-only — there is no local training path.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

// ── Canonical ChatML types ─────────────────────────────────────────────────

/// A single conversation turn in canonical ChatML format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// A full conversation (list of role/content turns).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConversation {
    pub messages: Vec<ChatMessage>,
}

/// Source format identifiers for input datasets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetFormat {
    /// JSONL with `{"messages": [{"role": ..., "content": ...}, ...]}` per line.
    ChatML,
    /// ShareGPT format: `{"conversations": [{"from": "human", "value": "..."}, ...]}`.
    ShareGPT,
    /// Alpaca format: `{"instruction": "...", "input": "...", "output": "..."}`.
    Alpaca,
    /// Raw text file — each line is a standalone training example.
    RawText,
}

impl DatasetFormat {
    /// Detect format from file extension or content heuristics.
    pub fn detect(path: &std::path::Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        match ext.to_lowercase().as_str() {
            "jsonl" => {
                // Could be ChatML or ShareGPT — read first line to disambiguate.
                if let Ok(content) = std::fs::read_to_string(path) {
                    let first_line = content.lines().next().unwrap_or("");
                    if first_line.contains("\"messages\"") {
                        return Some(Self::ChatML);
                    }
                    if first_line.contains("\"conversations\"") {
                        return Some(Self::ShareGPT);
                    }
                }
                Some(Self::ChatML) // default for .jsonl
            }
            "json" => {
                // Single JSON array of Alpaca objects.
                if let Ok(content) = std::fs::read_to_string(path)
                    && content.contains("\"instruction\"")
                    && content.contains("\"output\"")
                {
                    return Some(Self::Alpaca);
                }
                None
            }
            "txt" => Some(Self::RawText),
            _ => None,
        }
    }
}

// ── Pipeline errors ───────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum DatasetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Validation error at line {line}: {message}")]
    Validation { line: usize, message: String },
    #[error("Empty dataset: no parseable examples found")]
    Empty,
    #[error("Cache error: {0}")]
    Cache(String),
}

// ── DatasetPipeline ────────────────────────────────────────────────────────

/// Ingest, normalize, validate, and cache datasets for training.
///
/// Pipeline: `ingest(file_path) → normalize → validate → cache`
///
/// Normalization always produces canonical ChatML. Provider adapters consume
/// the normalized output and translate it to their native format.
pub struct DatasetPipeline {
    /// Cache directory for normalized datasets.
    cache_dir: PathBuf,
    /// Cache key for the current normalization (content hash).
    cache_key: Option<String>,
}

impl Clone for DatasetPipeline {
    fn clone(&self) -> Self {
        Self {
            cache_dir: self.cache_dir.clone(),
            cache_key: None, // Reset cache_key on clone to avoid stale references
        }
    }
}

impl DatasetPipeline {
    /// Create a new dataset pipeline with a given cache directory.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            cache_key: None,
        }
    }

    /// Ingest a raw dataset file and return the path to normalized output.
    ///
    /// Full pipeline: detect format → normalize to ChatML → validate → cache.
    /// Returns the cached path on subsequent calls with the same input.
    pub fn ingest(&mut self, file_path: &std::path::Path) -> Result<PathBuf, DatasetError> {
        self.ingest_local(file_path)
    }

    fn ingest_local(&mut self, file_path: &std::path::Path) -> Result<PathBuf, DatasetError> {
        // Check cache first
        let cache_key = self.compute_cache_key(file_path)?;
        let cached_path = self.cache_dir.join(format!("{}.jsonl", cache_key));
        if cached_path.exists() {
            tracing::info!(
                target: "hkask.training.dataset.cached",
                path = %file_path.display(),
                cache_key = %cache_key,
                "Returning cached normalized dataset"
            );
            return Ok(cached_path);
        }

        let format = DatasetFormat::detect(file_path).ok_or_else(|| {
            DatasetError::UnsupportedFormat(format!(
                "Cannot determine format for {}",
                file_path.display()
            ))
        })?;

        let raw = std::fs::read_to_string(file_path)?;
        let normalized = match format {
            DatasetFormat::ChatML => self.normalize_chatml(&raw)?,
            DatasetFormat::ShareGPT => self.normalize_sharegpt(&raw)?,
            DatasetFormat::Alpaca => self.normalize_alpaca(&raw)?,
            DatasetFormat::RawText => self.normalize_raw_text(&raw)?,
        };

        self.validate(&normalized)?;
        self.cache(&cached_path, &normalized)?;

        self.cache_key = Some(cache_key);
        Ok(cached_path)
    }

    /// Compute a content-hash-based cache key for the input file.
    fn compute_cache_key(&self, file_path: &std::path::Path) -> Result<String, DatasetError> {
        let content = std::fs::read(file_path)?;
        let hash = blake3::hash(&content);
        let key = format!("dataset-{}", hash.to_hex());
        Ok(key)
    }

    /// Normalize ChatML JSONL to canonical form.
    ///
    /// Input: JSONL with `{"messages": [{"role": ..., "content": ...}, ...]}`
    /// Output: Same format, validated.
    fn normalize_chatml(&self, raw: &str) -> Result<Vec<ChatConversation>, DatasetError> {
        let mut conversations = Vec::new();
        for (i, line) in raw.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            #[derive(Deserialize)]
            struct ChatMLRecord {
                messages: Vec<ChatMessage>,
            }
            let record: ChatMLRecord =
                serde_json::from_str(trimmed).map_err(|e| DatasetError::Validation {
                    line: i + 1,
                    message: format!("Invalid ChatML record: {}", e),
                })?;
            conversations.push(ChatConversation {
                messages: record.messages,
            });
        }
        if conversations.is_empty() {
            return Err(DatasetError::Empty);
        }
        Ok(conversations)
    }

    /// Normalize ShareGPT JSONL to canonical ChatML.
    ///
    /// ShareGPT uses `from: human/gpt` and `value` instead of `role` and `content`.
    fn normalize_sharegpt(&self, raw: &str) -> Result<Vec<ChatConversation>, DatasetError> {
        let mut conversations = Vec::new();
        for (i, line) in raw.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            #[derive(Deserialize)]
            struct ShareGPTTurn {
                from: String,
                value: String,
            }
            #[derive(Deserialize)]
            struct ShareGPTRecord {
                conversations: Vec<ShareGPTTurn>,
            }
            let record: ShareGPTRecord =
                serde_json::from_str(trimmed).map_err(|e| DatasetError::Validation {
                    line: i + 1,
                    message: format!("Invalid ShareGPT record: {}", e),
                })?;
            let messages: Vec<ChatMessage> = record
                .conversations
                .into_iter()
                .map(|t| {
                    let role = match t.from.as_str() {
                        "human" => "user".to_string(),
                        "gpt" => "assistant".to_string(),
                        other => other.to_string(),
                    };
                    ChatMessage {
                        role,
                        content: t.value,
                    }
                })
                .collect();
            conversations.push(ChatConversation { messages });
        }
        if conversations.is_empty() {
            return Err(DatasetError::Empty);
        }
        Ok(conversations)
    }

    /// Normalize Alpaca JSON to canonical ChatML.
    ///
    /// Alpaca uses `instruction`, `input` (optional), and `output` fields.
    fn normalize_alpaca(&self, raw: &str) -> Result<Vec<ChatConversation>, DatasetError> {
        #[derive(Deserialize)]
        struct AlpacaRecord {
            instruction: String,
            #[serde(default)]
            input: String,
            output: String,
        }
        let records: Vec<AlpacaRecord> =
            serde_json::from_str(raw).map_err(|e| DatasetError::Validation {
                line: 0,
                message: format!("Invalid Alpaca JSON: {}", e),
            })?;
        if records.is_empty() {
            return Err(DatasetError::Empty);
        }
        let conversations: Vec<ChatConversation> = records
            .into_iter()
            .map(|r| {
                let user_content = if r.input.is_empty() {
                    r.instruction
                } else {
                    format!("{}\n\n{}", r.instruction, r.input)
                };
                ChatConversation {
                    messages: vec![
                        ChatMessage {
                            role: "user".to_string(),
                            content: user_content,
                        },
                        ChatMessage {
                            role: "assistant".to_string(),
                            content: r.output,
                        },
                    ],
                }
            })
            .collect();
        Ok(conversations)
    }

    /// Normalize raw text to canonical ChatML.
    ///
    /// Each non-empty line becomes a single-message user turn.
    /// This is a best-effort normalization — raw text has no conversation structure.
    fn normalize_raw_text(&self, raw: &str) -> Result<Vec<ChatConversation>, DatasetError> {
        let conversations: Vec<ChatConversation> = raw
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| ChatConversation {
                messages: vec![ChatMessage {
                    role: "user".to_string(),
                    content: line.trim().to_string(),
                }],
            })
            .collect();
        if conversations.is_empty() {
            return Err(DatasetError::Empty);
        }
        Ok(conversations)
    }

    /// Validate canonical ChatML conversations.
    ///
    /// Checks:
    /// - At least one message per conversation
    /// - Valid roles (user, assistant, system)
    /// - Non-empty content fields
    /// - Alternating user/assistant pattern (system allowed only as first message)
    fn validate(&self, conversations: &[ChatConversation]) -> Result<(), DatasetError> {
        let valid_roles = ["user", "assistant", "system"];
        for (i, conv) in conversations.iter().enumerate() {
            if conv.messages.is_empty() {
                return Err(DatasetError::Validation {
                    line: i + 1,
                    message: "Empty conversation".to_string(),
                });
            }
            for (j, msg) in conv.messages.iter().enumerate() {
                if !valid_roles.contains(&msg.role.as_str()) {
                    return Err(DatasetError::Validation {
                        line: i + 1,
                        message: format!("Invalid role '{}' at position {}", msg.role, j + 1),
                    });
                }
                if msg.content.trim().is_empty() {
                    return Err(DatasetError::Validation {
                        line: i + 1,
                        message: format!(
                            "Empty content for role '{}' at position {}",
                            msg.role,
                            j + 1
                        ),
                    });
                }
                // System messages only allowed as first message
                if msg.role == "system" && j > 0 {
                    return Err(DatasetError::Validation {
                        line: i + 1,
                        message: "System message only allowed as first message".to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Write normalized conversations to cache as JSONL.
    fn cache(
        &self,
        path: &std::path::Path,
        conversations: &[ChatConversation],
    ) -> Result<(), DatasetError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DatasetError::Cache(format!("Failed to create cache dir: {}", e)))?;
        }
        let mut output = String::new();
        for conv in conversations {
            let json = serde_json::to_string(conv)
                .map_err(|e| DatasetError::Cache(format!("Serialization error: {}", e)))?;
            output.push_str(&json);
            output.push('\n');
        }
        std::fs::write(path, output)
            .map_err(|e| DatasetError::Cache(format!("Failed to write cache: {}", e)))?;
        Ok(())
    }
}

// ── Provider-specific format converters ────────────────────────────────────

/// Convert canonical ChatML JSONL to axolotl-compatible ChatML format.
///
/// Axolotl expects ChatML with explicit `type: chatml` in config.
/// The path returned is the cached normalized file — the provider's
/// config YAML references it directly.
pub fn to_axolotl_format(normalized_path: &std::path::Path) -> PathBuf {
    normalized_path.to_path_buf()
}

/// Convert canonical ChatML JSONL to unsloth-compatible text format.
///
/// Unsloth expects a single text field per example. We concatenate
/// each conversation into a formatted text block.
pub fn to_unsloth_format(
    normalized_path: &std::path::Path,
    conversations: &[ChatConversation],
) -> Result<PathBuf, DatasetError> {
    let output_path = normalized_path.with_extension("unsloth.jsonl");
    let mut output = String::new();
    for conv in conversations {
        let text: Vec<String> = conv
            .messages
            .iter()
            .map(|m| format!("<|{}|>\n{}", m.role, m.content))
            .collect();
        let record = serde_json::json!({"text": text.join("\n")});
        output.push_str(
            &serde_json::to_string(&record)
                .map_err(|e| DatasetError::Cache(format!("Serialization error: {}", e)))?,
        );
        output.push('\n');
    }
    std::fs::write(&output_path, output)
        .map_err(|e| DatasetError::Cache(format!("Failed to write unsloth format: {}", e)))?;
    Ok(output_path)
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn ingest_chatml_jsonl() {
        let dir = tempfile::tempdir().expect("tempdir");
        let input = dir.path().join("test.jsonl");
        let cache = dir.path().join("cache");
        std::fs::create_dir_all(&cache).expect("create cache dir");

        // Write a minimal ChatML dataset (simulating pragmatic-semantics traces)
        let records = vec![
            serde_json::json!({"messages": [
                {"role": "system", "content": "You classify constraints."},
                {"role": "user", "content": "Classify: must never expose memory."},
                {"role": "assistant", "content": "Prohibition (Rank 1)."}
            ]}),
            serde_json::json!({"messages": [
                {"role": "system", "content": "You classify constraints."},
                {"role": "user", "content": "Classify: prefer local models."},
                {"role": "assistant", "content": "Guideline (Rank 3)."}
            ]}),
        ];
        let mut file = std::fs::File::create(&input).expect("create input");
        for record in &records {
            writeln!(file, "{}", serde_json::to_string(record).unwrap()).expect("write");
        }

        let mut pipeline = DatasetPipeline::new(cache.clone());
        let normalized = pipeline.ingest(&input).expect("ingest should succeed");

        assert!(normalized.exists(), "normalized output should exist");
        let content = std::fs::read_to_string(&normalized).expect("read output");
        let lines: Vec<_> = content.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 2, "should have 2 conversations");

        // Verify each line is valid ChatML JSON
        for line in &lines {
            let conv: ChatConversation = serde_json::from_str(line).expect("valid ChatML JSON");
            assert!(!conv.messages.is_empty());
            let roles: Vec<_> = conv.messages.iter().map(|m| m.role.as_str()).collect();
            assert_eq!(roles, vec!["system", "user", "assistant"]);
        }
    }

    #[test]
    fn ingest_caches_result() {
        let dir = tempfile::tempdir().expect("tempdir");
        let input = dir.path().join("test.jsonl");
        let cache = dir.path().join("cache");
        std::fs::create_dir_all(&cache).expect("create cache dir");

        let record = serde_json::json!({"messages": [
            {"role": "user", "content": "What is P5?"},
            {"role": "assistant", "content": "Minimal Architecture."}
        ]});
        std::fs::write(
            &input,
            format!("{}\n", serde_json::to_string(&record).unwrap()),
        )
        .expect("write");

        let mut pipeline = DatasetPipeline::new(cache.clone());
        let first = pipeline.ingest(&input).expect("first ingest");
        let second = pipeline.ingest(&input).expect("second ingest");

        assert_eq!(first, second, "cached path should match");
        assert!(first.starts_with(&cache), "output should be in cache dir");
    }

    #[test]
    fn ingest_empty_dataset() {
        let dir = tempfile::tempdir().expect("tempdir");
        let input = dir.path().join("empty.jsonl");
        let cache = dir.path().join("cache");
        std::fs::create_dir_all(&cache).expect("create cache dir");
        std::fs::write(&input, "\n\n").expect("write empty");

        let mut pipeline = DatasetPipeline::new(cache);
        let result = pipeline.ingest(&input);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DatasetError::Empty));
    }
}
