//! Dual-layer skill audit harness.
//!
//! Scans the Zed agent layer (`.agents/skills/*/SKILL.md`) and the registry layer
//! (`registry/templates/*/manifest.yaml` + `*.j2`) and produces a health report.
//!
//! REQ: P5-svc-skills-095 — Implement dual-layer skill audit as a reusable service.
//! expect: "The service layer exposes minimal, essential interfaces shared by all surfaces"

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use hkask_ports::{RegistryIndex, SkillRegistryIndex};
use hkask_templates::SkillLoader;
use hkask_types::template_type::TemplateType;
use hkask_types::visibility::Visibility;
use serde::{Deserialize, Serialize};

// ── Public API ───────────────────────────────────────────────────────────

/// Auditor for the dual-layer skill corpus.
///
/// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
/// pre:  registry and skill_index are valid; project_root points to hKask root
/// post: returns an auditor configured for both layers
pub struct SkillAuditor<'a> {
    registry: &'a dyn RegistryIndex,
    skill_index: &'a dyn SkillRegistryIndex,
    project_root: PathBuf,
}

impl<'a> SkillAuditor<'a> {
    /// Create a new auditor.
    pub fn new(
        registry: &'a dyn RegistryIndex,
        skill_index: &'a dyn SkillRegistryIndex,
        project_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            registry,
            skill_index,
            project_root: project_root.into(),
        }
    }

    /// Audit every skill name found in either layer.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// post: returns a report with a health score and defects per skill
    pub fn audit_all(&self) -> Result<SkillAuditReport, SkillAuditError> {
        let names = self.collect_skill_names()?;
        let mut entries = Vec::with_capacity(names.len());
        for name in names {
            entries.push(self.audit_skill_internal(&name)?);
        }
        Ok(SkillAuditReport {
            workspace_version: WORKSPACE_VERSION.to_string(),
            entries,
        })
    }

    /// Audit a single skill by name.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// pre:  name is non-empty
    /// post: returns the skill's health score or an error if audit fails
    pub fn audit_skill(&self, name: &str) -> Result<SkillHealthScore, SkillAuditError> {
        self.audit_skill_internal(name)
    }
}

/// Full audit report for the skill corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAuditReport {
    pub workspace_version: String,
    pub entries: Vec<SkillHealthScore>,
}

impl SkillAuditReport {
    /// Serialize the report to JSON.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// post: returns a JSON string representation of the report
    pub fn to_json(&self) -> Result<String, SkillAuditError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| SkillAuditError::Serialize(format!("JSON serialize: {e}")))
    }

    /// Count of active agent skills (category `skill`, health_score >= 0.8).
    /// Non-skill template crates (pipelines, daemon-processes, etc.) are
    /// audited for template health but not counted here.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// post: returns number of `skill`-category entries with health_score >= 0.8
    pub fn active_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.is_active() && e.category == "skill")
            .count()
    }

    /// Count of active agent skills (category `skill`, health_score >= 0.8).
    pub fn active_skill_count(&self) -> usize {
        self.active_count()
    }

    /// Count of non-skill template crates audited (infrastructure sharing the
    /// FlowDef form: pipelines, daemon-processes, qa-scripts, runtime-config).
    pub fn non_skill_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.category != "skill")
            .count()
    }

    /// Count of .j2 files that incorrectly declare template_type FlowDef.
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// post: returns number of defects matching "FlowDef declared on .j2"
    pub fn flowdef_on_j2_count(&self) -> usize {
        self.entries
            .iter()
            .map(|e| {
                e.defects
                    .iter()
                    .filter(|d| d.contains("FlowDef declared on .j2"))
                    .count()
            })
            .sum()
    }
}

/// Health score and defect list for one skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillHealthScore {
    pub skill_name: String,
    pub zed_layer_present: bool,
    pub registry_layer_present: bool,
    pub health_score: f64,
    pub status: SkillStatus,
    pub defects: Vec<String>,
    pub template_summary: TemplateSummary,
    /// Manifest category (`skill` | `qa-script` | `runtime-config` | `daemon-process` |
    /// `pipeline`), read from `registry/manifests/<name>.yaml`. Defaults to `skill`
    /// when the FlowDef manifest is absent or carries no `category`. Non-`skill`
    /// entries are template crates for infrastructure that shares the FlowDef
    /// form — audited for template health but not counted as agent skills.
    #[serde(default = "default_category")]
    pub category: String,
}

fn default_category() -> String {
    "skill".to_string()
}

impl SkillHealthScore {
    /// True iff the skill is active (health_score >= 0.8).
    ///
    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// post: returns true iff health_score >= 0.8
    pub fn is_active(&self) -> bool {
        self.health_score >= 0.8
    }
}

/// Audit status derived from health score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillStatus {
    Active,
    StaleWarning,
    Critical,
    RecommendDeprecation,
}

fn status_from_score(score: f64) -> SkillStatus {
    if score >= 0.8 {
        SkillStatus::Active
    } else if score >= 0.5 {
        SkillStatus::StaleWarning
    } else if score >= 0.2 {
        SkillStatus::Critical
    } else {
        SkillStatus::RecommendDeprecation
    }
}

/// Errors emitted by the skill audit harness.
#[derive(Debug, thiserror::Error)]
pub enum SkillAuditError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("YAML error: {0}")]
    Yaml(String),
    #[error("JSON error: {0}")]
    Serialize(String),
}

/// Counts of templates per type for a skill.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplateSummary {
    pub total: usize,
    pub word_act: usize,
    pub know_act: usize,
    pub flow_def: usize,
    pub render_act: usize,
}

// ── Internal implementation ──────────────────────────────────────────────

impl<'a> SkillAuditor<'a> {
    fn collect_skill_names(&self) -> Result<Vec<String>, SkillAuditError> {
        let mut names = HashSet::new();

        let zed_dir = self.project_root.join(".agents").join("skills");
        if zed_dir.exists() {
            for entry in fs::read_dir(&zed_dir).map_err(|e| SkillAuditError::Io(e.to_string()))? {
                let entry = entry.map_err(|e| SkillAuditError::Io(e.to_string()))?;
                if entry.path().is_dir() {
                    names.insert(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }

        let reg_dir = self.project_root.join("registry").join("templates");
        if reg_dir.exists() {
            for entry in fs::read_dir(&reg_dir).map_err(|e| SkillAuditError::Io(e.to_string()))? {
                let entry = entry.map_err(|e| SkillAuditError::Io(e.to_string()))?;
                if entry.path().is_dir() {
                    names.insert(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }

        // Cross-check with the loaded runtime indexes.
        for skill in self.skill_index.list_skills() {
            names.insert(skill.id);
        }
        for entry in self.registry.list(None) {
            if let Some(skill_name) = entry.id.split('/').next() {
                names.insert(skill_name.to_string());
            }
        }

        let mut names: Vec<String> = names.into_iter().collect();
        names.sort();
        Ok(names)
    }

    fn audit_skill_internal(&self, name: &str) -> Result<SkillHealthScore, SkillAuditError> {
        let zed = self.audit_zed_layer(name)?;
        let reg = self.audit_registry_layer(name)?;

        let mut score = 1.0_f64;
        let mut defects = Vec::new();

        if !zed.present {
            score -= 0.05;
            defects.push("missing SKILL.md companion (info — registry is canonical)".to_string());
        } else {
            if !zed.has_frontmatter {
                score -= 0.10;
                defects.push("SKILL.md missing frontmatter".to_string());
            }
            if !zed.name_matches_dir {
                score -= 0.10;
                defects.push(format!(
                    "SKILL.md name '{}' does not match directory '{}'",
                    zed.name, name
                ));
            }
            if zed.description_len < 20 {
                score -= 0.05;
                defects.push("SKILL.md description too short".to_string());
            }
        }

        if !reg.present {
            score -= 0.50;
            defects.push("missing registry crate (CRITICAL — not executable)".to_string());
        } else {
            if !reg.manifest_present {
                score -= 0.15;
                defects.push("missing manifest.yaml".to_string());
            }
            for j2 in &reg.j2_files {
                if j2.frontmatter_missing {
                    score -= 0.10;
                    defects.push(format!("{}: missing [inference] frontmatter", j2.filename));
                    continue;
                }
                if j2.ddmvss_alias {
                    score -= 0.15;
                    defects.push(format!(
                        "{}: DDMVSS alias template_type {:?} (must be WordAct/KnowAct/FlowDef)",
                        j2.filename, j2.template_type_raw
                    ));
                } else if j2.template_type == Some(TemplateType::FlowDef) {
                    score -= 0.15;
                    defects.push(format!(
                        "{}: FlowDef declared on .j2 file (runtime says FlowDef = YAML .yaml)",
                        j2.filename
                    ));
                } else if j2.template_type.is_none() {
                    score -= 0.10;
                    defects.push(format!("{}: missing or invalid template_type", j2.filename));
                }
                if !j2.visibility_valid {
                    score -= 0.10;
                    defects.push(format!(
                        "{}: invalid visibility {:?}",
                        j2.filename, j2.visibility
                    ));
                }
                if !j2.contract_valid {
                    score -= 0.10;
                    defects.push(format!("{}: missing/empty contract", j2.filename));
                }
                if !j2.energy_cap_valid {
                    score -= 0.05;
                    defects.push(format!(
                        "{}: energy_cap {:?} out of range [2048, 8192]",
                        j2.filename, j2.energy_cap
                    ));
                }
            }
        }

        if zed.present && reg.present && zed.name != reg.crate_name && !reg.crate_name.is_empty() {
            score -= 0.10;
            defects.push(format!(
                "name mismatch: SKILL.md '{}' vs manifest '{}'",
                zed.name, reg.crate_name
            ));
        }

        score = score.clamp(0.0, 1.0);
        let status = status_from_score(score);

        // Read the manifest category from registry/manifests/<name>.yaml (if
        // present). A template crate with no FlowDef manifest is infrastructure
        // (not a skill) — read_manifest_category returns "infrastructure" in
        // that case. Only `skill`-category entries are counted as agent skills.
        let category = self.read_manifest_category(name);

        Ok(SkillHealthScore {
            skill_name: name.to_string(),
            zed_layer_present: zed.present,
            registry_layer_present: reg.present,
            health_score: score,
            status,
            template_summary: reg.template_summary,
            defects,
            category,
        })
    }

    /// Determine the category of a template crate.
    ///
    /// The **authoritative skill discriminator** is the `.agents/skills/<name>/`
    /// directory — the curated set of agent-facing skills. If that directory
    /// exists, the crate is a `"skill"`, regardless of what the FlowDef manifest
    /// declares (the directory is the ground truth, not a heuristic). If it does
    /// not exist, the crate is infrastructure: the declared `manifest.category`
    /// is returned (e.g. `qa-script`, `runtime-config`, `daemon-process`,
    /// `pipeline`, `infrastructure`), defaulting to `"infrastructure"` when no
    /// FlowDef manifest is present.
    fn read_manifest_category(&self, name: &str) -> String {
        // Ground truth: the curated .agents/skills/<name>/ directory.
        let skill_dir = self.project_root.join(".agents").join("skills").join(name);
        if skill_dir.is_dir() {
            return "skill".to_string();
        }
        // Not in the curated skill set — read the declared category from the
        // FlowDef manifest, or default to infrastructure.
        let path = self
            .project_root
            .join("registry")
            .join("manifests")
            .join(format!("{name}.yaml"));
        let Ok(content) = fs::read_to_string(&path) else {
            return "infrastructure".to_string();
        };
        // Light parse: extract the `category:` field under the `manifest:` block
        // without deserializing the full (schema-varied) manifest.
        let mut in_manifest = false;
        for line in content.lines() {
            if line.starts_with("manifest:") {
                in_manifest = true;
                continue;
            }
            if in_manifest {
                // A top-level key (no leading indent) ends the manifest block.
                if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
                    break;
                }
                let trimmed = line.trim_start();
                if let Some(rest) = trimmed.strip_prefix("category:") {
                    return rest.trim().trim_matches('"').to_string();
                }
            }
        }
        "skill".to_string()
    }

    fn audit_zed_layer(&self, name: &str) -> Result<ZedLayerInfo, SkillAuditError> {
        let path = self
            .project_root
            .join(".agents")
            .join("skills")
            .join(name)
            .join("SKILL.md");
        if !path.exists() {
            return Ok(ZedLayerInfo::default());
        }
        let content = fs::read_to_string(&path).map_err(|e| SkillAuditError::Io(e.to_string()))?;
        let front = SkillLoader::parse_front_matter(&content)
            .map_err(|e| SkillAuditError::Yaml(e.to_string()))?;
        let dir_name = name.to_string();
        let name_matches_dir = front.name == dir_name;
        Ok(ZedLayerInfo {
            present: true,
            has_frontmatter: content.trim_start().starts_with("---"),
            name: front.name,
            description_len: front.description.as_ref().map(|s| s.len()).unwrap_or(0),
            name_matches_dir,
        })
    }

    fn audit_registry_layer(&self, name: &str) -> Result<RegistryLayerInfo, SkillAuditError> {
        let dir = self
            .project_root
            .join("registry")
            .join("templates")
            .join(name);
        if !dir.exists() {
            return Ok(RegistryLayerInfo::default());
        }

        let manifest_path = dir.join("manifest.yaml");
        let manifest_present = manifest_path.exists();
        let mut crate_name = String::new();
        let mut manifest_templates: std::collections::HashMap<String, serde_yaml_neo::Value> =
            std::collections::HashMap::new();
        let mut j2_files = Vec::new();
        let mut summary = TemplateSummary::default();

        if manifest_present {
            let content = fs::read_to_string(&manifest_path)
                .map_err(|e| SkillAuditError::Io(e.to_string()))?;
            let manifest: serde_yaml_neo::Value = serde_yaml_neo::from_str(&content)
                .map_err(|e| SkillAuditError::Yaml(e.to_string()))?;
            if let Some(c) = manifest.get("crate").and_then(|v| v.get("name")) {
                crate_name = c.as_str().unwrap_or("").to_string();
            }
            // Collect template metadata from manifest for fallback when [inference] is missing
            if let Some(templates) = manifest.get("templates").and_then(|v| v.as_sequence()) {
                for tmpl in templates {
                    if let Some(path) = tmpl.get("path").and_then(|v| v.as_str()) {
                        manifest_templates.insert(path.to_string(), tmpl.clone());
                    }
                }
            }
        }

        for entry in fs::read_dir(&dir).map_err(|e| SkillAuditError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| SkillAuditError::Io(e.to_string()))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("j2") {
                let filename = path
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let info =
                    self.audit_j2_file_with_fallback(&path, &filename, &manifest_templates)?;
                match info.template_type {
                    Some(TemplateType::WordAct) => summary.word_act += 1,
                    Some(TemplateType::KnowAct) => summary.know_act += 1,
                    Some(TemplateType::FlowDef) => summary.flow_def += 1,
                    Some(TemplateType::RenderAct) => summary.render_act += 1,
                    None => {}
                }
                summary.total += 1;
                j2_files.push(info);
            }
        }

        Ok(RegistryLayerInfo {
            present: true,
            manifest_present,
            crate_name,
            j2_files,
            template_summary: summary,
        })
    }

    fn audit_j2_file_with_fallback(
        &self,
        path: &Path,
        filename: &str,
        manifest_templates: &std::collections::HashMap<String, serde_yaml_neo::Value>,
    ) -> Result<J2FileInfo, SkillAuditError> {
        let content = fs::read_to_string(path).map_err(|e| SkillAuditError::Io(e.to_string()))?;
        let front = parse_j2_frontmatter(&content);

        // If [inference] frontmatter is missing, try manifest.yaml fallback.
        // Templates that declare their metadata in manifest.yaml (not [inference])
        // should not be penalized for missing [inference] frontmatter.
        let frontmatter_missing = front.is_none();
        let has_manifest_fallback = manifest_templates.contains_key(filename);

        let mut info = J2FileInfo {
            filename: filename.to_string(),
            frontmatter_missing: frontmatter_missing && !has_manifest_fallback,
            ..Default::default()
        };

        if let Some(front) = front {
            info.template_type = front.template_type;
            info.template_type_raw = front.template_type_raw.clone();
            info.visibility = front.visibility.clone();
            info.energy_cap = front.energy_cap;
            info.contract_valid = front.contract_input.is_some() && front.contract_output.is_some();

            const DDMVSS_ALIASES: [&str; 3] = ["Cognition", "Prompt", "Process"];
            if let Some(ref raw) = front.template_type_raw {
                if DDMVSS_ALIASES.contains(&raw.as_str()) {
                    info.ddmvss_alias = true;
                }
                if TemplateType::parse_str(raw).is_some() {
                    info.template_type_valid = true;
                }
            }

            if let Some(ref vis) = front.visibility {
                info.visibility_valid = Visibility::parse_str(vis).is_some();
            }

            if let Some(ec) = front.energy_cap {
                info.energy_cap_valid = (2048..=8192).contains(&ec);
            }
        } else if let Some(tmpl_meta) = manifest_templates.get(filename) {
            // Fallback: use manifest.yaml metadata.
            // Manifest templates declare type/lexicon in manifest.yaml, not [inference].
            // They don't have visibility/energy_cap/contract fields — skip those checks
            // by marking them valid (the manifest format doesn't use those fields).
            //
            // RenderAct templates (reference content, macro libraries, error views)
            // are declared here with `type: RenderAct`; as a first-class TemplateType
            // they parse cleanly and need no special-casing — the manifest fallback's
            // valid-marks below handle their lack of inference-only fields.
            if let Some(ttype) = tmpl_meta.get("type").and_then(|v| v.as_str()) {
                info.template_type = TemplateType::parse_str(ttype);
                info.template_type_raw = Some(ttype.to_string());
                info.template_type_valid = info.template_type.is_some();
            }
            // Mark these valid to avoid penalizing manifest-format templates
            info.visibility_valid = true;
            info.energy_cap_valid = true;
            info.contract_valid = true;
        }

        Ok(info)
    }
}

// ── Layer info structs ───────────────────────────────────────────────────

#[derive(Debug, Default)]
struct ZedLayerInfo {
    present: bool,
    has_frontmatter: bool,
    name: String,
    description_len: usize,
    name_matches_dir: bool,
}

#[derive(Debug, Default)]
struct RegistryLayerInfo {
    present: bool,
    manifest_present: bool,
    crate_name: String,
    j2_files: Vec<J2FileInfo>,
    template_summary: TemplateSummary,
}

#[derive(Debug, Default)]
struct J2FileInfo {
    filename: String,
    frontmatter_missing: bool,
    template_type: Option<TemplateType>,
    template_type_valid: bool,
    template_type_raw: Option<String>,
    ddmvss_alias: bool,
    visibility: Option<String>,
    visibility_valid: bool,
    energy_cap: Option<i64>,
    energy_cap_valid: bool,
    contract_valid: bool,
}

#[derive(Debug, Default)]
#[allow(dead_code)] // populated by serde deserialization; some fields consumed in specific code paths
struct J2FrontMatter {
    template_type: Option<TemplateType>,
    template_type_raw: Option<String>,
    lexicon_terms: Vec<String>,
    contract_input: Option<serde_yaml_neo::Value>,
    contract_output: Option<serde_yaml_neo::Value>,
    energy_cap: Option<i64>,
    visibility: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────

const WORKSPACE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn parse_j2_frontmatter(content: &str) -> Option<J2FrontMatter> {
    // Skip leading Jinja comments ({# ... #}) and whitespace before [inference].
    // Templates may have documentation comments before the frontmatter block.
    let mut content = content.trim_start();
    while content.starts_with("{#") {
        if let Some(end) = content.find("#}") {
            content = content[end + 2..].trim_start();
        } else {
            break;
        }
    }
    if !content.starts_with("[inference]") {
        return None;
    }
    let after_header = &content["[inference]".len()..].trim_start_matches('\n');
    let sep = after_header.find("\n---")?;
    let mut yaml_text = &after_header[..sep];
    // Strip TOML-style [contract] section if present — it's not valid YAML
    // and is parsed separately by the contract parser.
    if let Some(contract_pos) = yaml_text.find("\n[contract]") {
        yaml_text = &yaml_text[..contract_pos];
    }
    let yaml: serde_yaml_neo::Value = serde_yaml_neo::from_str(yaml_text).ok()?;
    let map = yaml.as_mapping()?;

    let template_type_raw = map
        .get(serde_yaml_neo::Value::String("template_type".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let template_type = template_type_raw
        .as_deref()
        .and_then(TemplateType::parse_str);

    let lexicon_terms = map
        .get(serde_yaml_neo::Value::String("lexicon_terms".to_string()))
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let contract = map
        .get(serde_yaml_neo::Value::String("contract".to_string()))
        .and_then(|v| v.as_mapping());
    let (contract_input, contract_output, nested_energy_cap, nested_visibility) =
        if let Some(c) = contract {
            (
                c.get(serde_yaml_neo::Value::String("input".to_string()))
                    .cloned(),
                c.get(serde_yaml_neo::Value::String("output".to_string()))
                    .cloned(),
                c.get(serde_yaml_neo::Value::String("energy_cap".to_string()))
                    .and_then(|v| v.as_i64()),
                c.get(serde_yaml_neo::Value::String("visibility".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            )
        } else {
            (None, None, None, None)
        };

    // If YAML contract is missing, check for TOML-style [contract] section.
    // The [contract] section uses inline format: input: {field: type, ...}
    // We mark contract_input/output as Some to indicate contract is present.
    let (contract_input, contract_output) = if contract_input.is_some() {
        (contract_input, contract_output)
    } else {
        let full_text = &after_header[..sep];
        if full_text.contains("[contract]")
            && full_text.contains("input:")
            && full_text.contains("output:")
        {
            (
                Some(serde_yaml_neo::Value::Null),
                Some(serde_yaml_neo::Value::Null),
            )
        } else {
            (None, None)
        }
    };

    let top_level_energy_cap = map
        .get(serde_yaml_neo::Value::String("energy_cap".to_string()))
        .and_then(|v| v.as_i64())
        .or(nested_energy_cap);

    let top_level_visibility = map
        .get(serde_yaml_neo::Value::String("visibility".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or(nested_visibility);

    Some(J2FrontMatter {
        template_type,
        template_type_raw,
        lexicon_terms,
        contract_input,
        contract_output,
        energy_cap: top_level_energy_cap,
        visibility: top_level_visibility,
    })
}

// ── Default version helper ────────────────────────────────────────────────

// Workspace version constant used in audit reports until the harness can read
// Cargo.toml workspace metadata at runtime.

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// \[P5\] Motivating: Essentialism — service-layer orchestration earns its existence; no raw domain logic.
    /// scores >= 0.8 and is_active() returns true.
    #[test]
    fn complete_skill_is_active() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();

        // Zed layer
        let zed_dir = root.join(".agents").join("skills").join("test-skill");
        fs::create_dir_all(&zed_dir).unwrap();
        fs::write(
            zed_dir.join("SKILL.md"),
            "---\nname: test-skill\nvisibility: public\ndescription: \"A minimal test skill for the audit harness.\"\n---\n\n# Test Skill\n\nInstructions.\n",
        )
        .unwrap();

        // Registry layer
        let reg_dir = root.join("registry").join("templates").join("test-skill");
        fs::create_dir_all(&reg_dir).unwrap();
        fs::write(
            reg_dir.join("manifest.yaml"),
            "crate:\n  name: test-skill\n  version: 0.28.0\n  description: Minimal test skill.\n\ntemplates:\n  - id: test-skill/test\n    path: test.j2\n    type: KnowAct\n    lexicon_terms: [classify]\n    description: Minimal cognition template.\n\nlexicon_terms:\n  - classify\n",
        )
        .unwrap();
        fs::write(
            reg_dir.join("test.j2"),
            "[inference]\ntemplate_type: KnowAct\nlexicon_terms:\n- classify\ncontract:\n  input:\n    x: string\n  output:\n    y: string\n  energy_cap: 4096\n  visibility: Shared\n---\n{# goal: Minimal test template. #}\n{{ x }}\n",
        )
        .unwrap();

        let mut registry = hkask_templates::Registry::new();
        let loader = SkillLoader::new(root);
        let mut skill_index: hkask_templates::Registry = hkask_templates::Registry::new();
        loader.load_into(&mut skill_index);
        // Seed registry with the template entry so list(None) returns it.
        registry.register(hkask_ports::RegistryEntry {
            id: "test-skill/test".to_string(),
            template_type: TemplateType::KnowAct,
            name: "test".to_string(),
            lexicon_terms: vec!["classify".to_string()],
            description: "Minimal cognition template".to_string(),
            source_path: "registry/templates/test-skill/test.j2".to_string(),
            required_capabilities: vec![],
            cascade_level: 0,
            matroshka_limit: 7,
        });

        let auditor = SkillAuditor::new(
            &registry, &registry, // Registry implements both traits
            root,
        );

        let score = auditor.audit_skill("test-skill").expect("audit");
        assert!(
            score.is_active(),
            "complete skill should be active, got score {} with defects {:?}",
            score.health_score,
            score.defects
        );
    }

    /// Property-based skeleton: once proptest is wired, assert that any skill
    /// with both layers present and all .j2 files valid scores >= 0.8.
    #[test]
    #[ignore = "requires proptest fixture for arbitrary complete skills"]
    fn complete_skill_scores_above_threshold() {
        // TODO: implement with proptest once arbitrary skill fixtures exist.
    }
}
