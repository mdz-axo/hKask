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
    /// REQ: TYP-212
    /// pre:  self is a valid TemplateType variant
    /// post: returns the canonical PascalCase string ("WordAct", "KnowAct", "FlowDef")
    pub fn as_str(&self) -> &'static str {
        match self {
            TemplateType::WordAct => "WordAct",
            TemplateType::KnowAct => "KnowAct",
            TemplateType::FlowDef => "FlowDef",
        }
    }

    /// REQ: TYP-213
    /// pre:  s is a string in PascalCase or lowercase ("WordAct"/"wordact", "KnowAct"/"knowact", "FlowDef"/"flowdef")
    /// post: returns Some(TemplateType) if s matches a known variant; None otherwise
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "WordAct" | "wordact" => Some(TemplateType::WordAct),
            "KnowAct" | "knowact" => Some(TemplateType::KnowAct),
            "FlowDef" | "flowdef" => Some(TemplateType::FlowDef),
            _ => None,
        }
    }

    /// REQ: TYP-214
    /// pre:  self is a valid TemplateType variant
    /// post: returns the file extension: "j2" for all runtime template types
    pub fn file_extension(&self) -> &'static str {
        match self {
            TemplateType::WordAct => "j2",
            TemplateType::KnowAct => "j2",
            TemplateType::FlowDef => "j2",
        }
    }

    /// REQ: TYP-215
    /// pre:  self is a valid TemplateType variant
    /// post: returns the MDS specification name: WordAct→"Prompt", KnowAct→"Cognition", FlowDef→"Process"
    pub fn as_spec_name(&self) -> &'static str {
        match self {
            TemplateType::WordAct => "Prompt",
            TemplateType::KnowAct => "Cognition",
            TemplateType::FlowDef => "Process",
        }
    }

    /// REQ: TYP-216
    /// pre:  ext is a file extension string (e.g. "j2", "yaml", "yml")
    /// post: returns None because a file extension alone cannot distinguish the three runtime template types; `.j2` may be WordAct, KnowAct, or FlowDef
    pub fn infer_from_extension(ext: &str) -> Option<Self> {
        let _ = ext;
        None
    }
}

impl std::fmt::Display for TemplateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}