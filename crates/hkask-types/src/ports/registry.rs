// G2 Justification: 7 public items — registry domain (entry, zone, skill, error, three index traits). These form the template/skill registry boundary. Co-located because Skill, SkillZone, and RegistryEntry compose together; the three index traits share the same error type. ≤7 cap met.

use crate::BundleManifest;
use crate::bundle::SkillPolarity;
use crate::lexicon::TemplateType;
use crate::visibility::Visibility;
use serde::{Deserialize, Serialize};

/// Unified registry entry covering all template types with cascade metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub id: String,
    pub template_type: TemplateType,
    pub name: String,
    pub lexicon_terms: Vec<String>,
    pub description: String,
    pub source_path: String,
    pub required_capabilities: Vec<String>,
    pub cascade_level: u32,
    pub matroshka_limit: u32,
}

impl RegistryEntry {
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.id.is_empty() {
            warnings.push("entry id is empty".into());
        }
        if self.source_path.is_empty() {
            warnings.push(format!("entry '{}' has empty source_path", self.id));
        }
        if self.name.is_empty() {
            warnings.push(format!("entry '{}' has empty name", self.id));
        }
        if self.cascade_level >= self.matroshka_limit {
            warnings.push(format!(
                "entry '{}' cascade_level ({}) >= matroshka_limit ({}) — nesting exhausted",
                self.id, self.cascade_level, self.matroshka_limit
            ));
        }
        warnings
    }
    pub fn can_nest(&self) -> bool {
        self.cascade_level < self.matroshka_limit
    }
}

/// Two-zone model: Private (`.agents/skills/` source), Public (`skills/` build artifact).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default,
)]
#[serde(rename_all = "lowercase")]
pub enum SkillZone {
    #[default]
    Private,
    Public,
}

impl SkillZone {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Public => "public",
        }
    }
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "private" | "Private" => Some(Self::Private),
            "public" | "Public" => Some(Self::Public),
            _ => None,
        }
    }
    pub fn directory(&self) -> &'static str {
        match self {
            Self::Private => ".agents/skills",
            Self::Public => "skills",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Skill {
    pub id: String,
    pub domain: TemplateType,
    pub word_act: Option<String>,
    pub flow_def: Option<String>,
    pub know_act: Option<String>,
    pub polarity: Option<SkillPolarity>,
    pub content_hash: Option<String>,
    pub visibility: Visibility,
    pub zone: SkillZone,
    /// Namespace (replicant handle) for collision-free public sharing.
    ///
    /// [DECLARATIVE] Always a user replicant name (e.g. "alice"), never a system agent. (P6 — Space for Replicants).
    /// System agents (bots) don't author or share skills — only human replicants do.
    ///
    /// In the public zone, skills are stored as `<namespace>--<id>/` directories.
    /// In the private zone, namespace is typically `None` (user-local, no collision).
    /// When set, `qualified_id()` returns `<namespace>--<id>`.
    pub namespace: Option<String>,
}

impl Skill {
    pub fn new(id: &str, domain: TemplateType) -> Self {
        Self {
            id: id.to_string(),
            domain,
            word_act: None,
            flow_def: None,
            know_act: None,
            polarity: None,
            content_hash: None,
            visibility: Visibility::Private,
            zone: SkillZone::Private,
            namespace: None,
        }
    }

    /// Builders with `Option<String>` from `&str`.
    pub fn with_word_act(mut self, v: &str) -> Self {
        self.word_act = Some(v.to_string());
        self
    }
    pub fn with_flow_def(mut self, v: &str) -> Self {
        self.flow_def = Some(v.to_string());
        self
    }
    pub fn with_know_act(mut self, v: &str) -> Self {
        self.know_act = Some(v.to_string());
        self
    }
    pub fn with_polarity(mut self, v: SkillPolarity) -> Self {
        self.polarity = Some(v);
        self
    }
    pub fn with_content_hash(mut self, v: String) -> Self {
        self.content_hash = Some(v);
        self
    }
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_visibility(mut self, v: Visibility) -> Self {
        self.visibility = v;
        self
    }
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_zone(mut self, v: SkillZone) -> Self {
        self.zone = v;
        self
    }
    #[must_use = "builder methods must be chained or assigned"]
    pub fn with_namespace(mut self, v: impl Into<String>) -> Self {
        self.namespace = Some(v.into());
        self
    }

    /// Qualified ID: `<namespace>--<id>` if namespace set, else just `id`. Double-dash is unambiguous for filesystem dirs.
    pub fn qualified_id(&self) -> String {
        match &self.namespace {
            Some(ns) => format!("{}--{}", ns, self.id),
            None => self.id.clone(),
        }
    }
    /// Parse `<namespace>--<id>` into `(namespace, id)`. Returns `None` if not a qualified ID.
    pub fn parse_qualified_id(qualified: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = qualified.splitn(2, "--").collect();
        if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }

    /// Compute and set SHA-256 content hash from key fields.
    pub fn compute_content_hash(&mut self) {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.id.as_bytes());
        h.update(self.domain.as_str().as_bytes());
        h.update(self.visibility.as_str().as_bytes());
        h.update(self.zone.as_str().as_bytes());
        if let Some(ref v) = self.namespace {
            h.update(v.as_bytes());
        }
        if let Some(ref v) = self.word_act {
            h.update(v.as_bytes());
        }
        if let Some(ref v) = self.flow_def {
            h.update(v.as_bytes());
        }
        if let Some(ref v) = self.know_act {
            h.update(v.as_bytes());
        }
        self.content_hash = Some(hex::encode(h.finalize()));
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum RegistryError {
    #[error("Entry not found: {0}")]
    NotFound(String),
    #[error("Registry error: {0}")]
    Other(String),
}

/// CRUD for skills. Read methods return owned `Skill` for HashMap/SQLite compat.
pub trait SkillRegistryIndex {
    fn register_skill(&mut self, skill: Skill);
    fn get_skill(&self, id: &str) -> Option<Skill>;
    fn list_skills(&self) -> Vec<Skill>;
    fn list_skills_by_visibility(&self, visibility: Visibility) -> Vec<Skill>;
    fn skills_by_domain(&self, domain: TemplateType) -> Vec<Skill>;
    fn skills_referencing_template(&self, template_id: &str) -> Vec<Skill>;
    fn remove_skill(&mut self, id: &str) -> Option<Skill>;
    /// P2 (Affirmative Consent): default-deny access. Private context sees all skills. Public/Shared sees only Public or Shared.
    fn list_skills_visible_to(&self, caller_visibility: Visibility) -> Vec<Skill> {
        match caller_visibility {
            Visibility::Private => self.list_skills(),
            _ => {
                let mut result = self.list_skills_by_visibility(Visibility::Public);
                result.extend(self.list_skills_by_visibility(Visibility::Public));
                result
            }
        }
    }
}

/// CRUD for bundle manifests. Read methods return owned values for HashMap/SQLite compat.
pub trait BundleRegistryIndex {
    fn register_bundle(&mut self, bundle: BundleManifest);
    fn get_bundle(&self, id: &str) -> Option<BundleManifest>;
    fn list_bundles(&self) -> Vec<BundleManifest>;
    fn remove_bundle(&mut self, id: &str) -> Option<BundleManifest>;
    fn find_bundle_by_skills(&self, skill_ids: &[String]) -> Option<BundleManifest>;
}

/// Template registry lookups. Moved to hkask-types for Authority DAG.
/// Impls: `Registry` (in-memory, hkask-templates), `SqliteRegistry` (hkask-templates)
pub trait RegistryIndex {
    fn list(&self, domain_hint: Option<TemplateType>) -> Vec<RegistryEntry>;

    fn list_with_capabilities(&self, capabilities: &[String]) -> Vec<RegistryEntry> {
        self.list(None)
            .into_iter()
            .filter(|e| {
                e.required_capabilities.is_empty()
                    || e.required_capabilities
                        .iter()
                        .all(|c| capabilities.contains(c))
            })
            .collect()
    }

    fn get(&self, id: &str) -> Result<RegistryEntry, RegistryError>;
}
