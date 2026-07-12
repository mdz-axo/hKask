use serde::{Deserialize, Serialize};

/// Template type discriminator — aligned with hKask domains.
///
/// Each variant corresponds to a domain in the architecture and a file format:
/// - **WordAct**: Jinja2 prompt templates — "what to say" — `.j2` (inference-invoked)
/// - **KnowAct**: Jinja2 cognition templates — "how to think" — `.j2` (inference-invoked)
/// - **FlowDef**: YAML pipeline manifests — "what to do" — `.yaml`
/// - **RenderAct**: Jinja2 render templates — "what to render" — `.j2` (NOT inference-invoked)
///
/// WordAct, KnowAct, and FlowDef are the cognitive-act triad (Pattern A) — all
/// are invoked in the cascade (WordAct/KnowAct sent to inference; FlowDef
/// orchestrates). RenderAct is the non-inference layer: Jinja2 components that
/// produce text via rendering (reference content, `{% macro %}` libraries, error
/// views included via `{% include %}`/`{% from %}`) and are never sent to the LLM.
///
/// FlowDef templates are declared in `manifest.yaml` with `type: FlowDef`
/// but use `.yaml` files (not `.j2`). Only WordAct, KnowAct, and RenderAct
/// appear in `.j2` frontmatter `template_type`.
///
/// Specifications (DDMVSS Prompt/Cognition/Process) are aliases used only in
/// architecture documents; they never appear in `.j2` frontmatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TemplateType {
    /// Jinja2 prompt templates — "what to say" (inference-invoked)
    WordAct,
    /// Jinja2 cognition templates — "how to think" (inference-invoked)
    KnowAct,
    /// Jinja2 process templates — "what to do"
    FlowDef,
    /// Jinja2 render templates — "what to render" (NOT inference-invoked).
    /// Reference content, macro libraries, and error/display views included
    /// into other templates via `{% include %}`/`{% from %}`. The action is the
    /// rendering; the output is produced by Jinja, not by an LLM.
    RenderAct,
}

crate::enum_str_ops!(TemplateType, {
    WordAct => ("WordAct", "wordact"),
    KnowAct => ("KnowAct", "knowact"),
    FlowDef => ("FlowDef", "flowdef"),
    RenderAct => ("RenderAct", "renderact"),
});

impl TemplateType {
    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a valid TemplateType variant
    /// post: returns the file extension: "j2" for WordAct/KnowAct/RenderAct, "yaml" for FlowDef
    pub fn file_extension(&self) -> &'static str {
        match self {
            TemplateType::WordAct => "j2",
            TemplateType::KnowAct => "j2",
            TemplateType::FlowDef => "yaml",
            TemplateType::RenderAct => "j2",
        }
    }

    /// expect: "System types preserve semantic identity and are provenance-aware"
    /// pre:  self is a valid TemplateType variant
    /// post: returns the MDS specification name: WordAct→"Prompt", KnowAct→"Cognition", FlowDef→"Process", RenderAct→"Render"
    pub fn as_spec_name(&self) -> &'static str {
        match self {
            TemplateType::WordAct => "Prompt",
            TemplateType::KnowAct => "Cognition",
            TemplateType::FlowDef => "Process",
            TemplateType::RenderAct => "Render",
        }
    }
}

impl std::fmt::Display for TemplateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
