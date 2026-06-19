use serde::{Deserialize, Serialize};

/// Template type discriminator — aligned with hKask domains.
///
/// Each variant corresponds to a domain in the architecture and a file format:
/// - **WordAct**: Jinja2 prompt templates — "what to say" — `.j2`
/// - **KnowAct**: Jinja2 cognition templates — "how to think" — `.j2`
/// - **FlowDef**: Jinja2 process templates — "what to do" — `.j2`
///
/// Specifications (DDMVSS Prompt/Cognition/Process) are aliases used only in
/// architecture documents; they never appear in `.j2` frontmatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TemplateType {
    /// Jinja2 prompt templates — "what to say"
    WordAct,
    /// Jinja2 cognition templates — "how to think"
    KnowAct,
    /// Jinja2 process templates — "what to do"
    FlowDef,
}

impl TemplateType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TemplateType::WordAct => "WordAct",
            TemplateType::KnowAct => "KnowAct",
            TemplateType::FlowDef => "FlowDef",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "WordAct" | "wordact" => Some(TemplateType::WordAct),
            "KnowAct" | "knowact" => Some(TemplateType::KnowAct),
            "FlowDef" | "flowdef" => Some(TemplateType::FlowDef),
            _ => None,
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            TemplateType::WordAct => "j2",
            TemplateType::KnowAct => "j2",
            TemplateType::FlowDef => "j2",
        }
    }

    pub fn as_spec_name(&self) -> &'static str {
        match self {
            TemplateType::WordAct => "Prompt",
            TemplateType::KnowAct => "Cognition",
            TemplateType::FlowDef => "Process",
        }
    }
}

impl std::fmt::Display for TemplateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
