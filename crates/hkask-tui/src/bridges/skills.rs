//! SkillsDataBridge — trait for skill listing and execution in the TUI.
//!
//! Backed by hkask-mcp-skill MCP server. Provides skill discovery,
//! listing, and execution status.

use std::sync::Arc;

/// Summary of a skill for TUI display.
#[derive(Debug, Clone)]
pub struct SkillListItem {
    pub id: String,
    pub description: String,
}

/// Result of skill execution.
#[derive(Debug, Clone)]
pub struct SkillExecResult {
    pub skill_id: String,
    pub output: String,
    pub tokens_used: u64,
}

/// Trait for querying the skill subsystem.
pub trait SkillsDataBridge: Send + Sync {
    /// List all registered skills.
    fn skill_list(&self) -> Vec<SkillListItem>;
    /// Execute a skill with context variables (returns output text).
    fn skill_execute(&self, skill_id: &str, context: &str) -> Option<SkillExecResult>;
    /// Get the count of registered skills.
    fn skill_count(&self) -> usize;
}

/// Mock implementation for TUI development and testing.
pub struct MockSkillsBridge {
    pub skills: Vec<SkillListItem>,
}

impl MockSkillsBridge {
    pub fn new() -> Self {
        Self { skills: Vec::new() }
    }
    pub fn with_sample() -> Self {
        Self {
            skills: vec![
                SkillListItem {
                    id: "coding-guidelines".into(),
                    description: "Enforce coding behavioral principles".into(),
                },
                SkillListItem {
                    id: "bug-hunt".into(),
                    description: "Bug hunting expeditions against target crates".into(),
                },
                SkillListItem {
                    id: "tdd".into(),
                    description: "Test-driven development with red-green-refactor loop".into(),
                },
                SkillListItem {
                    id: "deep-module".into(),
                    description: "Module design discipline".into(),
                },
            ],
        }
    }
    pub fn arc(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl SkillsDataBridge for MockSkillsBridge {
    fn skill_list(&self) -> Vec<SkillListItem> {
        self.skills.clone()
    }
    fn skill_execute(&self, _skill_id: &str, _context: &str) -> Option<SkillExecResult> {
        None
    }
    fn skill_count(&self) -> usize {
        self.skills.len()
    }
}
