//! RegistryDataBridge — trait for template/skill/bundle data in the TUI.
//!
//! Provides the Registry and Skills windows with live data from
//! the SqliteRegistry. Implemented by the CLI.

use std::sync::Arc;

/// Summary of a single template for TUI display.
#[derive(Debug, Clone)]
pub struct TemplateListItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Summary of a single skill for TUI display.
#[derive(Debug, Clone)]
pub struct SkillSummary {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub description: Option<String>,
}

/// Summary of a single bundle for TUI display.
#[derive(Debug, Clone)]
pub struct BundleListItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub skill_count: usize,
}

/// Trait for querying registry state.
pub trait RegistryDataBridge: Send + Sync {
    fn template_count(&self) -> usize;
    fn skill_count(&self) -> usize;
    fn bundle_count(&self) -> usize;
    fn list_templates(&self) -> Vec<TemplateListItem>;
    fn list_skills(&self) -> Vec<SkillSummary>;
    fn list_bundles(&self) -> Vec<BundleListItem>;
}

/// Mock implementation for TUI development and testing.
pub struct MockRegistryBridge {
    pub template_count: usize,
    pub skill_count: usize,
    pub bundle_count: usize,
    pub templates: Vec<TemplateListItem>,
    pub skills: Vec<SkillSummary>,
    pub bundles: Vec<BundleListItem>,
}

impl MockRegistryBridge {
    pub fn new() -> Self {
        Self {
            template_count: 46,
            skill_count: 46,
            bundle_count: 3,
            templates: vec![
                TemplateListItem {
                    id: "coding-guidelines".into(),
                    name: "coding-guidelines".into(),
                    description: Some("Enforce Karpathy's four coding principles".into()),
                },
                TemplateListItem {
                    id: "bug-hunt".into(),
                    name: "bug-hunt".into(),
                    description: Some("Bug hunting with Weinberg's quality definition".into()),
                },
                TemplateListItem {
                    id: "tdd".into(),
                    name: "tdd".into(),
                    description: Some("Test-driven development RED→GREEN→REFACTOR".into()),
                },
            ],
            skills: vec![
                SkillSummary {
                    id: "coding-guidelines".into(),
                    name: "coding-guidelines".into(),
                    domain: "coding".into(),
                    description: Some("Code quality guardrails".into()),
                },
                SkillSummary {
                    id: "bug-hunt".into(),
                    name: "bug-hunt".into(),
                    domain: "quality".into(),
                    description: Some("Systematic bug hunting".into()),
                },
            ],
            bundles: vec![BundleListItem {
                id: "core-dev".into(),
                name: "Core Development Bundle".into(),
                version: "1.0.0".into(),
                description: Some("TDD + debug + refactor".into()),
                skill_count: 3,
            }],
        }
    }

    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl RegistryDataBridge for MockRegistryBridge {
    fn template_count(&self) -> usize {
        self.template_count
    }
    fn skill_count(&self) -> usize {
        self.skill_count
    }
    fn bundle_count(&self) -> usize {
        self.bundle_count
    }
    fn list_templates(&self) -> Vec<TemplateListItem> {
        self.templates.clone()
    }
    fn list_skills(&self) -> Vec<SkillSummary> {
        self.skills.clone()
    }
    fn list_bundles(&self) -> Vec<BundleListItem> {
        self.bundles.clone()
    }
}
