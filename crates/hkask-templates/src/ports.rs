//! Port traits for registry and template execution
//!
//! Defines the hexagonal architecture ports for template dispatch system.
//! Per architecture v0.21.0: Rust is the loom, YAML/Jinja2 is the thread.

use hkask_types::NotFound;

/// Error type for template operations
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("Template not found: {0}")]
    NotFound(NotFound),

    #[error("Render error: {0}")]
    Render(String),
    #[error("Manifest error: {0}")]
    Manifest(String),
    #[error("Database error: {0}")]
    Database(#[from] hkask_types::InfrastructureError),
    #[error("Inference error: {0}")]
    Inference(#[from] hkask_ports::InferenceError),
    #[error("MCP error: {0}")]
    Mcp(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Path traversal attempt: {0}")]
    PathTraversal(String),
    #[error("Sandbox violation: {0}")]
    SandboxViolation(String),
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),

    /// Failed to load a skill from disk (typed replacement for `anyhow::anyhow!`
    /// in `skill_loader.rs`). Carries the path so callers can surface it in
    /// findings without re-formatting.
    #[error("skill load error at {path}: {source}")]
    SkillLoad {
        path: String,
        source: std::io::Error,
    },

    /// SKILL.md frontmatter is missing or malformed. `detail` names the
    /// exact repair (mirrors Nika's `SkillDefect` discipline: each variant
    /// names the fix, not just the failure).
    #[error("SKILL.md frontmatter error: {detail}")]
    Frontmatter { detail: String },
}

impl From<NotFound> for TemplateError {
    fn from(nf: NotFound) -> Self {
        TemplateError::NotFound(nf)
    }
}

pub type Result<T> = std::result::Result<T, TemplateError>;

/// One skill-system finding — a typed failure surfaced by skill loading or
/// manifest resolution (mirrors Nika's `SkillFinding`: `code` + `detail`,
/// one voice for check and run). The `code` is a stable `&'static str`
/// so consumers can switch on it without string-matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillFinding {
    /// The skill or manifest the finding refers to.
    pub skill_id: String,
    /// Stable code (e.g. `"HKASK-SKILL-001"`). Consumers switch on this.
    pub code: &'static str,
    /// Human-readable detail naming the exact repair.
    pub detail: String,
}

impl SkillFinding {
    /// The human-facing row (check rung, run refusal, log line).
    #[must_use]
    pub fn row(&self) -> String {
        format!(
            "[{code}] {skill}: {detail}",
            code = self.code,
            skill = self.skill_id,
            detail = self.detail
        )
    }

    /// The machine-facing JSON object (check --json, structured logs).
    #[must_use]
    pub fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "skill_id": self.skill_id,
            "code": self.code,
            "detail": self.detail,
        })
    }
}

/// Why a manifest reference did not resolve. Replaces the prior
/// `Option<BundleManifest>` return on `resolve_manifest` (which collapsed
/// three distinct failure modes into `None`).
#[derive(Debug, thiserror::Error)]
pub enum ManifestResolveError {
    /// The reference matched no registry entry and no file path.
    #[error("manifest not found: {reference}")]
    NotFound { reference: String },
    /// A file path matched but the manifest failed to load.
    #[error("manifest load failed for {reference}: {source}")]
    LoadFailed {
        reference: String,
        #[source]
        source: super::manifest_loader::ManifestLoadError,
    },
    /// The manifest loaded but is not a `skill` category (e.g. `qa-script`).
    #[error("manifest '{reference}' is not a skill (category={category})")]
    NotASkill { reference: String, category: String },
}

/// Injected filesystem reader for skill loading (purity seam — mirrors
/// Nika's `resolve_skills(wf, &mut dyn FnMut)` pattern). Production wires
/// `FsSkillReader`; tests wire a mock. This keeps `SkillLoader` testable
/// without a real filesystem and enables check≡run by construction.
pub trait SkillReader {
    /// Read a file's contents as UTF-8 text.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or is not valid UTF-8.
    fn read_to_string(&self, path: &std::path::Path) -> std::io::Result<String>;
}

/// Production filesystem reader — thin wrapper over `std::fs::read_to_string`.
#[derive(Debug, Clone, Copy)]
pub struct FsSkillReader;

impl SkillReader for FsSkillReader {
    fn read_to_string(&self, path: &std::path::Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }
}
